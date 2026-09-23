import { EventEmitter } from "node:events";
import { TextDecoder } from "node:util";

export const MAX_FRAME_BYTES = 8 * 1024 * 1024;
const MAX_PENDING = 32;

export class CoreTransportError extends Error {
  constructor(code) {
    super(code);
    this.name = "CoreTransportError";
    this.code = code;
  }
}

// The transport never logs or places frame contents, executable paths, or child stderr in errors.
export class CoreClient extends EventEmitter {
  #child;
  #buffer = Buffer.alloc(0);
  #pending = new Map();
  #sequence = 0;
  #state = "starting";
  #startupTimer;
  #requestTimeout;
  #expectedVersion;
  #ready;
  #resolveReady;
  #rejectReady;

  constructor(child, { startupTimeout = 15_000, requestTimeout = 120_000, expectedVersion } = {}) {
    super();
    this.#child = child;
    this.#requestTimeout = requestTimeout;
    this.#expectedVersion = expectedVersion;
    this.#ready = new Promise((resolve, reject) => {
      this.#resolveReady = resolve;
      this.#rejectReady = reject;
    });
    // Failure can arrive before the host starts awaiting ready().
    void this.#ready.catch(() => {});
    this.#startupTimer = setTimeout(() => this.#fail("CoreStartupTimeout"), startupTimeout);
    this.#startupTimer.unref?.();
    child.stdout.on("data", (chunk) => this.#receive(Buffer.from(chunk)));
    child.stdout.on("error", () => this.#fail("CoreDisconnected"));
    child.stdout.on("end", () => this.#fail(this.#buffer.length ? "CoreInvalidResponse" : "CoreDisconnected"));
    child.stdin.on("error", () => this.#fail("CoreDisconnected"));
    child.on("error", () => this.#fail("CoreUnavailable"));
    child.on("exit", () => this.#fail("CoreDisconnected"));
    child.stderr.on("error", () => {});
    child.stderr.resume();
  }

  get state() { return this.#state; }
  ready() { return this.#ready; }

  #receive(chunk) {
    if (this.#state === "disconnected") return;
    // Split before concatenating so even a hostile frame cannot make an unbounded allocation.
    let offset = 0;
    while (offset < chunk.length) {
      const newline = chunk.indexOf(10, offset);
      const end = newline === -1 ? chunk.length : newline + 1;
      const part = chunk.subarray(offset, end);
      if (this.#buffer.length + part.length > MAX_FRAME_BYTES) {
        this.#fail("CoreInvalidResponse");
        return;
      }
      this.#buffer = Buffer.concat([this.#buffer, part]);
      offset = end;
      if (newline === -1) return;
      const frame = this.#buffer;
      this.#buffer = Buffer.alloc(0);
      try {
        this.#handle(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(frame)));
      } catch {
        this.#fail("CoreInvalidResponse");
        return;
      }
    }
  }

  #handle(frame) {
    if (!frame || typeof frame !== "object" || Array.isArray(frame)) throw new Error();
    if (frame.event) {
      if (frame.event === "startupError" && this.#state === "starting") {
        const code = ["IsolationInvalid", "AlreadyRunning", "OwnershipUnavailable"].includes(frame.code)
          ? frame.code : "CoreUnavailable";
        this.#fail(code);
      } else if (frame.event === "ready" && this.#state === "starting" && frame.protocolVersion === 1) {
        if (this.#expectedVersion !== undefined && frame.version !== this.#expectedVersion) {
          this.#fail("CoreVersionMismatch"); return;
        }
        clearTimeout(this.#startupTimer);
        this.#state = "ready";
        this.#resolveReady();
        this.emit("state", this.#state);
      } else if (frame.event === "focus" && this.#state === "ready") {
        this.emit("focus");
      } else {
        throw new Error();
      }
      return;
    }
    if (this.#state !== "ready" || !Number.isSafeInteger(frame.id)) throw new Error();
    const pending = this.#pending.get(frame.id);
    if (!pending || (Object.hasOwn(frame, "result") === Object.hasOwn(frame, "error"))) throw new Error();
    this.#pending.delete(frame.id);
    clearTimeout(pending.timer);
    if (frame.error) {
      // Static local taxonomy only: do not echo a child-provided message.
      const allowed = ["InvalidArguments", "UnknownCommand", "CoreBusy", "CommandInterrupted", "ResponseTooLarge", "SerializationFailed"];
      pending.reject(new CoreTransportError(allowed.includes(frame.error.code) ? frame.error.code : "CoreCommandFailed"));
      if (["CommandInterrupted", "ResponseTooLarge", "SerializationFailed"].includes(frame.error.code)) {
        this.#fail(frame.error.code);
      }
    } else {
      pending.resolve(frame.result);
    }
  }

  async request(command, args = {}) {
    await this.#ready;
    if (this.#state !== "ready") throw new CoreTransportError("CoreDisconnected");
    if (this.#pending.size >= MAX_PENDING) throw new CoreTransportError("CoreBusy");
    if (this.#sequence >= Number.MAX_SAFE_INTEGER) throw new CoreTransportError("CoreRequestLimit");
    const id = ++this.#sequence;
    let bytes;
    try { bytes = Buffer.from(JSON.stringify({ id, command, args }) + "\n"); }
    catch { throw new CoreTransportError("InvalidArguments"); }
    if (bytes.length > MAX_FRAME_BYTES) throw new CoreTransportError("RequestTooLarge");
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => this.#fail("CoreRequestTimeout"), this.#requestTimeout);
      timer.unref?.();
      this.#pending.set(id, { resolve, reject, timer });
      this.#child.stdin.write(bytes, (error) => { if (error) this.#fail("CoreDisconnected"); });
    });
  }

  #fail(code) {
    if (this.#state === "disconnected") return;
    const error = new CoreTransportError(code);
    clearTimeout(this.#startupTimer);
    this.#state = "disconnected";
    this.#rejectReady(error);
    for (const pending of this.#pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.#pending.clear();
    this.#buffer = Buffer.alloc(0);
    this.#child.stdin.end();
    this.emit("state", this.#state, code);
  }

  async stop(graceMs = 12_000) {
    this.#fail("CoreStopped");
    if (!this.#child.pid || this.#child.exitCode !== null || this.#child.signalCode) return;
    await new Promise((resolve, reject) => {
      let finalTimer;
      const done = () => { clearTimeout(timer); clearTimeout(finalTimer); resolve(); };
      const timer = setTimeout(() => {
        finalTimer = setTimeout(() => {
          this.#child.removeListener("exit", done);
          reject(new CoreTransportError("CoreStopTimeout"));
        }, 5_000);
        this.#child.kill("SIGKILL");
      }, graceMs);
      this.#child.once("exit", done);
    });
  }
}
