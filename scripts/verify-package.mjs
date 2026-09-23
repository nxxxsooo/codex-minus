// Headless consumer checks: packaged renderer bytes, matching Rust resources, Electron's own Node
// entry point, and the packaged Rust stdio handshake. Cross-architecture CI stops at byte checks.
import assert from "node:assert/strict";
import { spawn, execFile } from "node:child_process";
import { createServer } from "node:net";
import { mkdtemp, readFile, mkdir, rm, stat } from "node:fs/promises";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { promisify } from "node:util";
import { extractFile, listPackage } from "@electron/asar";
import { CoreClient } from "../desktop/core-client.mjs";

const [location, platform = process.platform, arch = process.arch] = process.argv.slice(2);
if (!location || !["darwin-arm64", "win32-arm64", "win32-x64"].includes(`${platform}-${arch}`)) throw new Error("verify-package.mjs <bundle-or-install-directory> <platform> <arch>");
const root = resolve(location);
const resources = join(root, platform === "darwin" ? "Contents/Resources" : "resources");
const executable = join(root, platform === "darwin" ? "Contents/MacOS/codex-minus" : "codex-minus.exe");
const core = join(resources, "core", platform === "darwin" ? "codex-minus-core" : "codex-minus-core.exe");
const archive = join(resources, "app.asar");
const packed = JSON.parse(extractFile(archive, "package.json").toString());
const expected = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
assert.equal(packed.version, expected.version);
assert.equal(packed.main, "desktop/main.mjs");
const files = listPackage(archive);
for (const required of ["/dist/index.html", "/desktop/main.mjs", "/desktop/preload.cjs", "/desktop/update-helper.mjs", "/desktop/update-fs.mjs"]) assert(files.includes(required), required);
assert(!files.some(path => path.endsWith(".test.mjs") || path.startsWith("/node_modules/")));
const bytes = await readFile(core);
if (platform === "darwin") {
  assert.equal(bytes.readUInt32LE(0), 0xfeedfacf);
  assert.equal(bytes.readUInt32LE(4), 0x100000c);
} else {
  assert.equal(bytes.readUInt16LE(0), 0x5a4d);
  const pe = bytes.readUInt32LE(0x3c);
  assert.equal(bytes.readUInt32LE(pe), 0x4550);
  assert.equal(bytes.readUInt16LE(pe + 4), arch === "x64" ? 0x8664 : 0xaa64);
}
const native = platform === process.platform && arch === process.arch;
if (native) {
  const { stdout } = await promisify(execFile)(executable, ["-e", "console.log(JSON.stringify({electron:process.versions.electron,arch:process.arch}))"], {
    env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" }, timeout: 15_000, windowsHide: true,
  });
  assert.deepEqual(JSON.parse(stdout.trim()), { electron: expected.devDependencies.electron, arch });
  await mkdir(join(homedir(), ".cache"), { recursive: true });
  const home = await mkdtemp(join(homedir(), ".cache/cm-pkg-"));
  const server = createServer();
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  let client;
  try {
    await mkdir(join(home, ".codex"));
    client = new CoreClient(spawn(core, ["--stdio"], {
      env: { ...process.env, HOME: home, USERPROFILE: home, CODEX_HOME: join(home, ".codex"), CODEX_MINUS_TEST_HOME: home, CODEX_PLUS_MANAGER_GUARD_PORT: String(port) },
      stdio: ["pipe", "pipe", "pipe"], windowsHide: true,
    }), { expectedVersion: expected.version });
    await client.ready();
    assert.equal((await client.request("load_settings", {})).status, "ok");
  } finally { await client?.stop(); await rm(home, { recursive: true, force: true }); }
}
console.log(JSON.stringify({ version: packed.version, platform, arch, nativeRuntimeVerified: native, coreBytes: (await stat(core)).size, rendererArchiveBytes: (await stat(archive)).size }));
