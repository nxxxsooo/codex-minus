import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { buildElectronManifest, buildLegacyManifest } from "./electron-release-manifest.mjs";
import { selectElectronUpdate } from "../desktop/update-manifest.mjs";

const version = "0.4.19";
const names = [
  `CodexMinus_${version}_aarch64.app.tar.gz`,
  `CodexMinus_${version}_x64-setup.exe`,
  `CodexMinus_${version}_arm64-setup.exe`,
];
const signature = Buffer.from("untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1556193335\tfile:test\ny/rUw2y8/hOUYjZU71eHp/Wo1KZ40fGy2VJEDl34XMJM+TX48Ss/17u3IvIfbVR1FkZZSNCisQbuQY+bHwhEBg==\n").toString("base64");

test("manifest generated from final signed asset names is accepted by the Electron client", async () => {
  const root = await mkdtemp(join(tmpdir(), "codex-manifest-test-"));
  try {
    for (const name of names) {
      await writeFile(join(root, name), `fixture-${name}`);
      await writeFile(join(root, name + ".sig"), signature + "\n");
    }
    const manifest = await buildElectronManifest(root, version);
    assert.equal(manifest.runtime, "electron");
    assert.equal(manifest.schemaVersion, 1);
    const selected = selectElectronUpdate(JSON.stringify(manifest), "0.4.18", "darwin", "arm64");
    assert.equal(selected.size, Buffer.byteLength(`fixture-${names[0]}`));
    assert.equal(selected.signature, signature);
    const legacy = buildLegacyManifest(manifest);
    assert.equal(legacy.version, "0.4.19");
    assert.equal(legacy.platforms["windows-x86_64"].url, "https://github.com/nxxxsooo/codex-minus/releases/download/v0.4.19/CodexMinus_0.4.19_x64-setup.exe");
    assert.equal(legacy.platforms["darwin-aarch64"].signature, signature);
    assert.equal(Object.keys(legacy.platforms).length, 3);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("an unsigned or misnamed asset cannot produce an update feed", async () => {
  const root = await mkdtemp(join(tmpdir(), "codex-manifest-test-"));
  try {
    for (const name of names) await writeFile(join(root, name), "fixture");
    await assert.rejects(buildElectronManifest(root, version));
    await assert.rejects(buildElectronManifest(root, "0.4.19/escape"), /InvalidReleaseVersion/);
  } finally { await rm(root, { recursive: true, force: true }); }
});
