import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readdir, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { downloadVerifiedUpdate } from "./update-download.mjs";

const bytes = Buffer.from("signed-disposable-artifact");
const sha256 = createHash("sha256").update(bytes).digest("hex");
const update = () => ({ version: "0.4.19", platform: "darwin-aarch64", url: "https://github.com/nxxxsooo/codex-minus/releases/download/v0.4.19/CodexMinus_0.4.19_aarch64.app.tar.gz", size: bytes.length, sha256, signature: "fixture-signature" });

test("retains a downloaded artifact only after its exact bytes reach the signature gate", async () => {
  const root = await mkdtemp(join(tmpdir(), "codex-update-test-"));
  const events = [];
  let verified = 0;
  try {
    const result = await downloadVerifiedUpdate(update(), {
      tempRoot: root,
      fetchArtifact: async () => new Response(bytes, { status: 200 }),
      verifyArtifact: async (artifact, signature, digest, size) => {
        verified++;
        assert.deepEqual(await readFile(artifact), bytes);
        assert.equal((await readFile(signature, "utf8")).trim(), "fixture-signature");
        assert.equal(digest, sha256);
        assert.equal(size, bytes.length);
        return true;
      },
      onProgress: event => events.push(event),
    });
    assert.equal(verified, 1);
    assert.deepEqual(events.map(event => event.event), ["Started", "Progress", "Finished"]);
    assert.deepEqual(await readFile(result.artifact), bytes);
    assert.equal((await readdir(root)).length, 1);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("rejects oversized and tampered downloads before verification and removes staging", async () => {
  const root = await mkdtemp(join(tmpdir(), "codex-update-test-"));
  let verified = 0;
  const ports = {
    tempRoot: root,
    fetchArtifact: async () => new Response(Buffer.concat([bytes, Buffer.from("extra")]), { status: 200 }),
    verifyArtifact: async () => { verified++; return true; },
  };
  try {
    await assert.rejects(downloadVerifiedUpdate(update(), ports), /UpdateSizeMismatch/);
    await assert.rejects(downloadVerifiedUpdate({ ...update(), size: bytes.length + 5 }, ports), /UpdateDigestMismatch/);
    assert.equal(verified, 0);
    assert.deepEqual(await readdir(root), []);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("a failed signature gate leaves no candidate available to an installer", async () => {
  const root = await mkdtemp(join(tmpdir(), "codex-update-test-"));
  try {
    await assert.rejects(downloadVerifiedUpdate(update(), {
      tempRoot: root,
      fetchArtifact: async () => new Response(bytes, { status: 200 }),
      verifyArtifact: async () => false,
    }), /UpdateSignatureRejected/);
    assert.deepEqual(await readdir(root), []);
  } finally { await rm(root, { recursive: true, force: true }); }
});
