import assert from "node:assert/strict";
import { test } from "node:test";
import { selectElectronUpdate } from "./update-manifest.mjs";

const version = "0.4.19";
const macFile = `CodexMinus_${version}_aarch64.app.tar.gz`;
const macUrl = `https://github.com/nxxxsooo/codex-minus/releases/download/v${version}/${macFile}`;
const signature = Buffer.from("untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1556193335\tfile:test\ny/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==\n").toString("base64");
const entry = () => ({ url: macUrl, signature, size: 1_234_567, sha256: "a".repeat(64) });
const manifest = () => ({ schemaVersion: 1, runtime: "electron", version, platforms: {
  "darwin-aarch64": entry(),
  "windows-x86_64": { ...entry(), url: `https://github.com/nxxxsooo/codex-minus/releases/download/v${version}/CodexMinus_${version}_x64-setup.exe` },
  "windows-aarch64": { ...entry(), url: `https://github.com/nxxxsooo/codex-minus/releases/download/v${version}/CodexMinus_${version}_arm64-setup.exe` },
} });

test("selects only a newer Electron artifact for this exact platform and tag", () => {
  assert.deepEqual(selectElectronUpdate(JSON.stringify(manifest()), "0.4.18", "darwin", "arm64"), { version, platform: "darwin-aarch64", ...entry() });
  assert.equal(selectElectronUpdate(JSON.stringify(manifest()), "0.4.18", "win32", "arm64")?.platform, "windows-aarch64");
  assert.equal(selectElectronUpdate(JSON.stringify(manifest()), version, "darwin", "arm64"), null);
  assert.throws(() => selectElectronUpdate(JSON.stringify(manifest()), "0.4.20", "darwin", "arm64"), /UpdateDowngrade/);
});

test("rejects old Tauri feeds, malformed payloads and unsupported architectures", () => {
  assert.throws(() => selectElectronUpdate(JSON.stringify({ ...manifest(), runtime: "tauri" }), "0.4.18", "darwin", "arm64"), /UnsupportedUpdateRuntime/);
  assert.throws(() => selectElectronUpdate("{".repeat(70_000), "0.4.18", "darwin", "arm64"), /InvalidUpdateManifest/);
  assert.throws(() => selectElectronUpdate("not-json", "0.4.18", "darwin", "arm64"), /InvalidUpdateManifest/);
  assert.throws(() => selectElectronUpdate(JSON.stringify(manifest()), "0.4.18", "win32", "ia32"), /UnsupportedUpdateTarget/);
  assert.throws(() => selectElectronUpdate(JSON.stringify(manifest()), "0.4.18", "linux", "x64"), /UnsupportedUpdateTarget/);
});

test("rejects retargeted URLs, signature gaps, invalid bounds and unexpected keys", () => {
  for (const patch of [
    { url: macUrl.replace("github.com", "updates.example.com") },
    { url: macUrl.replace("/v0.4.19/", "/v0.4.18/") },
    { url: macUrl.replace(".app.tar.gz", "-setup.exe") },
    { url: macUrl + "?download=1" },
    { signature: "" },
    { signature: Buffer.from("not a minisign signature").toString("base64") },
    { size: -1 },
    { size: 0 },
    { size: Number.MAX_SAFE_INTEGER },
    { sha256: "garbage" },
    { other: true },
  ]) {
    const input = manifest();
    input.platforms["darwin-aarch64"] = { ...entry(), ...patch };
    assert.throws(() => selectElectronUpdate(JSON.stringify(input), "0.4.18", "darwin", "arm64"), /InvalidUpdateManifest/, JSON.stringify(patch));
  }
  assert.throws(() => selectElectronUpdate(JSON.stringify({ ...manifest(), extra: "ignore-me" }), "0.4.18", "darwin", "arm64"), /InvalidUpdateManifest/);
});
