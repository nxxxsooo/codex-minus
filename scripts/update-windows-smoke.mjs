// Native Windows acceptance for the copied worker and the real pinned signature gate. A forged
// candidate must fail before readiness, leave the running manager/install directory usable,
// and never launch an installer. Positive NSIS migration is exercised separately in CI.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { launchUpdateHelper } from "../desktop/update-handoff.mjs";

if (process.platform !== "win32" || !process.argv[2]) throw new Error("Native Windows install directory required");
const root = resolve(process.argv[2]);
const executable = join(root, "codex-minus.exe"), core = join(root, "resources/core/codex-minus-core.exe");
const before = createHash("sha256").update(await readFile(executable)).digest("hex");
const directory = await mkdtemp(join(tmpdir(), "codex-minus-update-"));
try {
  const bytes = Buffer.from("not-an-installer"), artifact = join(directory, "candidate.exe"), signature = join(directory, "candidate.sig");
  const outcome = join(directory, "outcome.json");
  await writeFile(artifact, bytes);
  await writeFile(signature, "forged-signature");
  await assert.rejects(launchUpdateHelper({ directory, artifact, signature, version: "0.5.0", size: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"), mode: "windows" }, executable, outcome, core, { executable }), /UpdateHandoffFailed/);
  assert.equal(JSON.parse(await readFile(outcome, "utf8")).code, "UpdateSignatureRejected");
  assert.equal(createHash("sha256").update(await readFile(executable)).digest("hex"), before);
  console.log(JSON.stringify({ copiedWindowsRuntimeExecuted: true, realSignatureGateRejected: true, managerPreserved: true, installerNeverStarted: true }));
} finally { await rm(directory, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 }); }
