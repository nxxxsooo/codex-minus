import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { PassThrough, Writable } from "node:stream";
import { setTimeout as delay } from "node:timers/promises";
import { test } from "node:test";
import { readFileSync } from "node:fs";
import { CoreClient, MAX_FRAME_BYTES } from "./core-client.mjs";
import { CORE_COMMANDS, allowedCommand, trustedSender } from "./command-policy.mjs";

function fixture(options) {
  const child = new EventEmitter();
  child.stdout = new PassThrough(); child.stderr = new PassThrough();
  child.exitCode = null; child.signalCode = null; child.pid = 123;
  const writes = [];
  child.stdin = new Writable({ write(chunk, _encoding, done) { writes.push(JSON.parse(chunk)); done(); } });
  child.kill = () => { child.signalCode = "SIGKILL"; child.emit("exit", null, "SIGKILL"); };
  const client = new CoreClient(child, options);
  const send = (frame) => child.stdout.write(JSON.stringify(frame) + "\n");
  send({ event: "ready", protocolVersion: 1, version: "0.4.18" });
  return { client, child, writes, send };
}

test("correlates out-of-order responses without changing command payloads", async () => {
  const { client, child, writes, send } = fixture();
  const a = client.request("first", { request: { draftRevision: 7 } });
  const b = client.request("second", { expectedFingerprint: "generation" });
  await delay(0);
  assert.equal(writes.length, 2);
  assert.deepEqual(writes[0].args, { request: { draftRevision: 7 } });
  send({ id: writes[1].id, result: { value: "second" } });
  send({ id: writes[0].id, result: { value: "first" } });
  assert.deepEqual(await Promise.all([a, b]), [{ value: "first" }, { value: "second" }]);
  child.kill();
});

test("a dead child rejects every pending request and never replays", async () => {
  const { client, child, writes } = fixture();
  const requests = [client.request("commit_provider_detail"), client.request("load_settings")];
  const settled = Promise.allSettled(requests);
  await delay(0); child.kill();
  for (const result of await settled) {
    assert.equal(result.status, "rejected");
    assert.equal(result.reason.code, "CoreDisconnected");
  }
  await assert.rejects(client.request("load_settings"), { code: "CoreDisconnected" });
  assert.equal(writes.length, 2);
});

test("timeout makes the whole connection uncertain and requires reconnect", async () => {
  const { client, child, writes } = fixture({ requestTimeout: 15 });
  const failed = assert.rejects(client.request("commit_provider_detail"), { code: "CoreRequestTimeout" });
  await delay(30); await failed;
  assert.equal(client.state, "disconnected");
  assert.equal(writes.length, 1);
  child.kill();
});

test("split UTF-8 response frames work and malformed frames reveal no input", async () => {
  const { client, child, writes } = fixture();
  const request = client.request("test"); await delay(0);
  const bytes = Buffer.from(JSON.stringify({ id: writes[0].id, result: "供应商" }) + "\n");
  for (const byte of bytes) child.stdout.write(Buffer.from([byte]));
  assert.equal(await request, "供应商");
  const next = client.request("commit_provider_detail", { apiKey: "secret-sentinel" });
  const check = assert.rejects(next, (error) => error.code === "CoreInvalidResponse" && !error.message.includes("secret-sentinel"));
  await delay(0); child.stdout.write("secret-sentinel invalid json\n"); await check;
  child.kill();
});

test("rejects oversized response and forged IDs before accepting success", async () => {
  for (const bad of [Buffer.alloc(MAX_FRAME_BYTES + 1, 120), Buffer.from('{"id":999,"result":true}\n')]) {
    const { client, child } = fixture();
    const request = client.request("load_settings");
    const check = assert.rejects(request, { code: "CoreInvalidResponse" });
    await delay(0); child.stdout.write(bad); await check; child.kill();
  }
});

test("command registration and IPC sender checks reject stale writers and foreign frames", () => {
  assert(allowedCommand("commit_provider_detail"));
  for (const command of ["save_relay_file", "apply_relay_injection", "health", "constructor", null]) assert(!allowedCommand(command));
  const frame = { url: "codex-minus://app/index.html", parent: null };
  const contents = { mainFrame: frame };
  const window = { webContents: contents, isDestroyed: () => false };
  assert(trustedSender({ sender: contents, senderFrame: frame }, window, "codex-minus://app"));
  assert(!trustedSender({ sender: {}, senderFrame: frame }, window, "codex-minus://app"));
  assert(!trustedSender({ sender: contents, senderFrame: { ...frame } }, window, "codex-minus://app"));
  for (const url of ["https://app.attacker.test", "file:///index.html", "codex-minus://app.attacker.test/", "codex-minus://app/assets/app.js"]) {
    frame.url = url;
    assert(!trustedSender({ sender: contents, senderFrame: frame }, window, "codex-minus://app"));
  }
});

test("the renderer allowlist exactly matches the registered Rust domain boundary", () => {
  const source = readFileSync(new URL("../src-tauri/src/rpc.rs", import.meta.url), "utf8");
  const dispatch = source.slice(source.indexOf("async fn dispatch"), source.indexOf("async fn read_frame"));
  const registered = [...dispatch.matchAll(/^\s*"([a-z_]+)"\s*=>/gm)].map(m => m[1]).filter(name => name !== "health");
  assert.deepEqual([...CORE_COMMANDS].sort(), registered.sort());
});

test("a mismatched core build cannot become ready", async () => {
  const { client, child } = fixture({ expectedVersion: "0.4.19" });
  await assert.rejects(client.ready(), { code: "CoreVersionMismatch" });
  assert.equal(client.state, "disconnected");
  child.kill();
});

test("uncertain command errors invalidate the connection without leaking child messages", async () => {
  const { client, child, send, writes } = fixture();
  const pending = client.request("commit_provider_detail");
  const rejected = assert.rejects(pending, error => error.code === "CommandInterrupted" && !error.message.includes("secret-sentinel"));
  await delay(0);
  send({ id: writes[0].id, error: { code: "CommandInterrupted", message: "secret-sentinel" } });
  await rejected;
  await assert.rejects(client.request("load_settings"), { code: "CoreDisconnected" });
  assert.equal(writes.length, 1);
  child.kill();
});

test("shutdown waits for the child's actual exit after forced termination", async () => {
  const { client, child } = fixture();
  let exited = false;
  child.kill = () => { setTimeout(() => { exited = true; child.exitCode = 0; child.emit("exit", 0); }, 15); };
  await client.stop(1);
  assert(exited);
});
