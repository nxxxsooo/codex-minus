import { createHash, randomUUID } from "node:crypto";
import { chmod, mkdtemp, open, rm, writeFile } from "node:fs/promises";
import { basename, join } from "node:path";

// Main calls this only with an artifact selected from the strict Electron manifest. A candidate
// is never returned to the installer until size, digest and the pinned Rust signature gate pass.
export async function downloadVerifiedUpdate(update, { tempRoot, fetchArtifact, verifyArtifact, onProgress = () => {} }) {
  if (!update || !Number.isSafeInteger(update.size) || update.size < 1 || update.size > 2 * 1024 ** 3
    || typeof update.sha256 !== "string" || !/^[a-f0-9]{64}$/.test(update.sha256)) {
    throw new Error("InvalidUpdateManifest");
  }
  const stage = await mkdtemp(join(tempRoot, "codex-minus-update-"));
  await chmod(stage, 0o700);
  let accepted = false;
  try {
    const stageId = randomUUID();
    await writeFile(join(stage, ".codex-minus-update.json"), JSON.stringify({ id: stageId, version: update.version }), { flag: "wx", mode: 0o600 });
    const url = new URL(update.url);
    if (url.protocol !== "https:" || url.hostname !== "github.com") throw new Error("InvalidUpdateManifest");
    const artifact = join(stage, basename(url.pathname));
    const signature = join(stage, "candidate.sig");
    const response = await fetchArtifact(update.url, { signal: AbortSignal.timeout(120_000) });
    if (!response.ok || !response.body || response.url && new URL(response.url).protocol !== "https:") throw new Error("UpdateDownloadFailed");
    const expectedLength = response.headers.get("content-length");
    if (expectedLength && Number(expectedLength) > update.size) throw new Error("UpdateSizeMismatch");
    const hash = createHash("sha256");
    let received = 0;
    onProgress({ event: "Started", data: { contentLength: update.size } });
    const output = await open(artifact, "wx", 0o600);
    try {
      for await (const chunk of response.body) {
        received += chunk.byteLength;
        if (received > update.size) throw new Error("UpdateSizeMismatch");
        hash.update(chunk);
        // FileHandle.write may perform a short write. writeFile on the open handle drains it.
        await output.writeFile(chunk);
        onProgress({ event: "Progress", data: { chunkLength: chunk.byteLength } });
      }
      await output.sync();
    } finally { await output.close(); }
    if (received !== update.size) throw new Error("UpdateSizeMismatch");
    if (hash.digest("hex") !== update.sha256) throw new Error("UpdateDigestMismatch");
    await writeFile(signature, update.signature + "\n", { flag: "wx", mode: 0o600 });
    let verified;
    try { verified = await verifyArtifact(artifact, signature, update.sha256, update.size); }
    catch { throw new Error("UpdateSignatureRejected"); }
    if (verified !== true) throw new Error("UpdateSignatureRejected");
    onProgress({ event: "Finished" });
    accepted = true;
    return { directory: stage, stageId, artifact, signature, version: update.version, sha256: update.sha256, size: update.size };
  } finally {
    if (!accepted) await rm(stage, { recursive: true, force: true });
  }
}
