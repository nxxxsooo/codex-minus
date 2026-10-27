import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { consumeUpdateOutcome, consumeUpdateOutcomes } from "./update-outcome.mjs";

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), "cm-outcome-"));
  const stage = await mkdtemp(join(root, "codex-minus-update-"));
  const path = join(root, "outcome.json");
  const result = { status: "installed", version: "0.5.0", stage, stageId: "owned-test-stage", helperPid: 100, installerPid: null };
  await writeFile(join(stage, ".codex-minus-update.json"), JSON.stringify({ id: result.stageId, version: result.version }));
  await writeFile(join(stage, "download"), "old-artifact");
  await writeFile(path, JSON.stringify(result));
  return { root, stage, path, result };
}

test("a confirmed update reclaims only its owned stage after helper exit", async () => {
  const f = await fixture();
  try {
    assert.deepEqual(await consumeUpdateOutcome(f.path, "0.5.0", { tempRoot: f.root, isAlive: () => false }), { status: "installed" });
    await assert.rejects(stat(f.stage), { code: "ENOENT" });
    await assert.rejects(stat(f.path), { code: "ENOENT" });
  } finally { await rm(f.root, { recursive: true, force: true }); }
});

test("installer activity defers cleanup and a later startup finishes without duplicate notice", async () => {
  const f = await fixture();
  try {
    await consumeUpdateOutcome(f.path, "0.5.0", { tempRoot: f.root, isAlive: () => true });
    assert.equal(await readFile(join(f.stage, "download"), "utf8"), "old-artifact");
    assert.equal(await consumeUpdateOutcome(f.path, "0.5.0", { tempRoot: f.root, isAlive: () => false }), null);
    await assert.rejects(stat(f.stage), { code: "ENOENT" });
  } finally { await rm(f.root, { recursive: true, force: true }); }
});

test("an unconfirmed rollback and a forged stage path retain recovery files", async () => {
  const f = await fixture();
  try {
    await writeFile(f.path, JSON.stringify({ ...f.result, status: "failed", code: "UpdateRollbackFailed" }));
    assert.deepEqual(await consumeUpdateOutcome(f.path, "0.4.18", { tempRoot: f.root, isAlive: () => false }), { status: "unconfirmed" });
    assert.equal(await readFile(join(f.stage, "download"), "utf8"), "old-artifact");
    const outside = join(f.root, "user-documents");
    await mkdir(outside);
    await writeFile(join(outside, "keep"), "user-owned");
    await writeFile(f.path, JSON.stringify({ ...f.result, stage: outside }));
    await consumeUpdateOutcome(f.path, "0.5.0", { tempRoot: f.root, isAlive: () => false });
    assert.equal(await readFile(join(outside, "keep"), "utf8"), "user-owned");
  } finally { await rm(f.root, { recursive: true, force: true }); }
});

test("a locked payload cannot remove the ownership marker needed for the next cleanup attempt", async () => {
  const f = await fixture();
  try {
    await consumeUpdateOutcome(f.path, "0.5.0", { tempRoot: f.root, isAlive: () => false,
      removeEntry: async () => { throw Object.assign(new Error("locked"), { code: "EBUSY" }); },
    });
    assert.equal(JSON.parse(await readFile(join(f.stage, ".codex-minus-update.json"), "utf8")).id, "owned-test-stage");
    assert.equal(await consumeUpdateOutcome(f.path, "0.5.0", { tempRoot: f.root, isAlive: () => false }), null);
    await assert.rejects(stat(f.stage), { code: "ENOENT" });
  } finally { await rm(f.root, { recursive: true, force: true }); }
});

test("per-attempt receipts reclaim a deferred older update after a later update", async () => {
  const f = await fixture();
  try {
    const receipts = join(f.root, "receipts");
    await mkdir(receipts);
    await writeFile(join(receipts, "first.json"), JSON.stringify({ ...f.result, reported: true }));
    const second = await mkdtemp(join(f.root, "codex-minus-update-"));
    await writeFile(join(second, ".codex-minus-update.json"), JSON.stringify({ id: "second", version: "0.5.1" }));
    await writeFile(join(receipts, "second.json"), JSON.stringify({ ...f.result, stage: second, stageId: "second", version: "0.5.1" }));
    assert.deepEqual(await consumeUpdateOutcomes(receipts, "0.5.1", { tempRoot: f.root, isAlive: () => false }), { status: "installed" });
    await assert.rejects(stat(f.stage), { code: "ENOENT" });
    await assert.rejects(stat(second), { code: "ENOENT" });
  } finally { await rm(f.root, { recursive: true, force: true }); }
});
