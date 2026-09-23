import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import { readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { selectElectronUpdate } from "../desktop/update-manifest.mjs";

export async function buildElectronManifest(directory, version) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) throw new Error("InvalidReleaseVersion");
  const artifacts = {
    "darwin-aarch64": `CodexMinus_${version}_aarch64.app.tar.gz`,
    "windows-x86_64": `CodexMinus_${version}_x64-setup.exe`,
    "windows-aarch64": `CodexMinus_${version}_arm64-setup.exe`,
  };
  const platforms = {};
  for (const [platform, name] of Object.entries(artifacts)) {
    const artifact = join(directory, name);
    const signatureFile = artifact + ".sig";
    const metadata = await stat(artifact);
    const signed = await stat(signatureFile);
    if (!metadata.isFile() || !signed.isFile() || !metadata.size || signed.size > 4096) throw new Error("InvalidReleaseArtifact");
    const signature = (await readFile(signatureFile, "utf8")).trim();
    const digest = createHash("sha256");
    for await (const chunk of createReadStream(artifact)) digest.update(chunk);
    platforms[platform] = {
      url: `https://github.com/nxxxsooo/codex-minus/releases/download/v${version}/${name}`,
      signature,
      size: metadata.size,
      sha256: digest.digest("hex"),
    };
  }
  const manifest = { schemaVersion: 1, runtime: "electron", version, platforms };
  selectElectronUpdate(JSON.stringify(manifest), "0.0.0", "darwin", "arm64");
  return manifest;
}

export function buildLegacyManifest(manifest) {
  selectElectronUpdate(JSON.stringify(manifest), "0.0.0", "darwin", "arm64");
  return {
    version: manifest.version,
    notes: "Upgrade Codex Minus to Electron. Existing provider, catalog and session data is retained.",
    platforms: Object.fromEntries(Object.entries(manifest.platforms).map(([platform, entry]) => [platform, { url: entry.url, signature: entry.signature }])),
  };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const [version, directory] = process.argv.slice(2);
  if (!version || !directory) throw new Error("Usage: electron-release-manifest.mjs <version> <asset-directory>");
  const manifest = await buildElectronManifest(directory, version);
  await writeFile(join(directory, "electron-latest.json"), JSON.stringify(manifest, null, 2) + "\n", { flag: "wx" });
  await writeFile(join(directory, "latest.json"), JSON.stringify(buildLegacyManifest(manifest), null, 2) + "\n", { flag: "wx" });
}
