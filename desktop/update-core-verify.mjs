import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

export async function verifyUpdateMetadata(binary, bytes, signature) {
  if (bytes.length > 64 * 1024 || signature.length > 4096) return false;
  const directory = await mkdtemp(join(tmpdir(), "codex-minus-feed-"));
  try {
    const artifact = join(directory, "manifest.json"), signed = join(directory, "manifest.sig");
    await writeFile(artifact, bytes, { mode: 0o600, flag: "wx" });
    await writeFile(signed, signature, { mode: 0o600, flag: "wx" });
    return await verifyWithCore(binary, artifact, signed, createHash("sha256").update(bytes).digest("hex"), bytes.length);
  } finally { await rm(directory, { recursive: true, force: true }); }
}

// The path comes only from main's packaged resource lookup. Renderer code never selects a core
// or stages an arbitrary file. No child output is collected, even when verification fails.
export function verifyWithCore(binary, artifact, signature, digest, size, { spawnCore = spawn, timeoutMs = 120_000 } = {}) {
  return new Promise((resolve, reject) => {
    let child;
    try { child = spawnCore(binary, ["--verify-update", artifact, signature, digest, String(size)], { stdio: "ignore", windowsHide: true }); }
    catch { reject(new Error("UpdateVerificationFailed")); return; }
    let settled = false;
    const finish = (value, error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (error) reject(new Error(error)); else resolve(value);
    };
    const timer = setTimeout(() => { child.kill?.("SIGKILL"); finish(false, "UpdateVerificationTimedOut"); }, timeoutMs);
    child.once("error", () => finish(false, "UpdateVerificationFailed"));
    child.once("close", code => finish(code === 0));
  });
}
