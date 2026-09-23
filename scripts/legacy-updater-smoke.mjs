// Execute the actual updater library from v0.4.18, headlessly, against an ephemeral signed feed
// and the built Electron archive. The old application directory and its data are disposable.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash, generateKeyPairSync, randomBytes, sign } from "node:crypto";
import { createServer } from "node:http";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildLegacyManifest } from "./electron-release-manifest.mjs";
import { inspectMacBundle } from "../desktop/update-install.mjs";

if (process.platform !== "darwin") throw new Error("Native macOS legacy replacement acceptance only");
const repo = resolve(fileURLToPath(new URL("..", import.meta.url)));
const { version } = JSON.parse(await readFile(join(repo, "package.json"), "utf8"));
const bytes = await readFile(join(repo, `dist-desktop/CodexMinus_${version}_aarch64.app.tar.gz`));
const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const id = randomBytes(8);
const encodedKey = Buffer.from(`untrusted comment: disposable upgrade-test key\n${Buffer.concat([Buffer.from("Ed"), id, publicKey.export({ type: "spki", format: "der" }).subarray(-32)]).toString("base64")}\n`).toString("base64");
const signature = sign(null, createHash("blake2b512").update(bytes).digest(), privateKey);
const comment = "timestamp:1\tfile:disposable-electron.app.tar.gz";
const encodedSignature = Buffer.from(`untrusted comment: disposable upgrade-test signature\n${Buffer.concat([Buffer.from("ED"), id, signature]).toString("base64")}\ntrusted comment: ${comment}\n${sign(null, Buffer.concat([signature, Buffer.from(comment)]), privateKey).toString("base64")}\n`).toString("base64");
const entry = filename => ({ url: `https://github.com/nxxxsooo/codex-minus/releases/download/v${version}/${filename}`, signature: encodedSignature, size: bytes.length, sha256: createHash("sha256").update(bytes).digest("hex") });
const feed = buildLegacyManifest({ schemaVersion: 1, runtime: "electron", version, platforms: {
  "darwin-aarch64": entry(`CodexMinus_${version}_aarch64.app.tar.gz`),
  "windows-x86_64": entry(`CodexMinus_${version}_x64-setup.exe`),
  "windows-aarch64": entry(`CodexMinus_${version}_arm64-setup.exe`),
} });
await mkdir(join(homedir(), ".cache"), { recursive: true });
const root = await mkdtemp(join(homedir(), ".cache/cm-legacy-"));
const target = join(root, "Codex Minus.app");
const requests = [];
const server = createServer((request, response) => {
  requests.push(request.url);
  const contents = request.url === "/latest.json" ? Buffer.from(JSON.stringify(feed)) : bytes;
  response.writeHead(200, { "content-type": request.url === "/latest.json" ? "application/json" : "application/octet-stream", "content-length": contents.length });
  response.end(contents);
});
try {
  await writeFile(join(root, ".legacy-update-fixture"), "disposable-test-only");
  await mkdir(join(target, "Contents/MacOS"), { recursive: true });
  await writeFile(join(target, "Contents/MacOS/codex-minus"), "old-runtime-sentinel");
  for (const directory of [".codex", ".codex-session-delete"]) await mkdir(join(root, directory));
  const sentinels = { ".codex/auth.json": "official-auth-sentinel", ".codex/config.toml": "protected-context-sentinel", ".codex-session-delete/settings.json": "saved-profiles-sentinel" };
  for (const [path, value] of Object.entries(sentinels)) await writeFile(join(root, path), value);
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  feed.platforms["darwin-aarch64"].url = base + "/update";
  const child = spawn(join(repo, "scripts/legacy-updater/target/debug/codex-minus-legacy-updater-acceptance"), [
    root, join(target, "Contents/MacOS/codex-minus"), base + "/latest.json", encodedKey, "0.4.18",
  ], { env: { ...process.env, HOME: root, CODEX_HOME: join(root, ".codex") }, stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  child.stdout.on("data", data => { output += data; }); child.stderr.on("data", data => { output += data; });
  const exit = await new Promise((resolve, reject) => { child.once("error", reject); child.once("exit", resolve); });
  assert.equal(exit, 0, output);
  assert.deepEqual(requests, ["/latest.json", "/update"]);
  assert.equal((await inspectMacBundle(target)).version, version);
  for (const [path, value] of Object.entries(sentinels)) assert.equal(await readFile(join(root, path), "utf8"), value);
  console.log(JSON.stringify({ updater: "tauri-plugin-updater@2.10.1", fromVersion: "0.4.18", toVersion: version, realSignedDownloadAndReplacement: true, dataSentinelsUnchanged: true, windowsOpened: 0 }));
} finally { await new Promise(resolve => server.close(resolve)); await rm(root, { recursive: true, force: true }); }
