import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, readdir, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { replaceMacBundle, waitForParentExit } from "./update-install.mjs";

async function tree() {
  const root = await mkdtemp(join(tmpdir(), "codex-replace-test-"));
  const target = join(root, "Codex Minus.app");
  const candidate = join(root, "stage", "Codex Minus.app");
  await mkdir(target);
  await mkdir(candidate, { recursive: true });
  await writeFile(join(target, "marker"), "old");
  await writeFile(join(candidate, "marker"), "new");
  return { root, target, candidate };
}
const inspect = async path => ({ id: "fun.mjshao.codex-minus", version: (await readFile(join(path, "marker"), "utf8")) === "new" ? "0.4.19" : "0.4.18" });
// Filesystem port for cross-platform rollback tests. The actual atomic primitive is exercised
// by Rust's macOS CLI integration test and by the signed disposable-bundle acceptance script.
const swap = async (left, right) => {
  const temporary = left + ".exchange";
  await rename(left, temporary); await rename(right, left); await rename(temporary, right);
};

test("replaces a verified bundle and removes its prior backup only after post-check", async () => {
  const f = await tree();
  try {
    await replaceMacBundle(f.candidate, f.target, "0.4.19", { inspect, swap });
    assert.equal(await readFile(join(f.target, "marker"), "utf8"), "new");
    assert.deepEqual((await readdir(f.root)).sort(), ["Codex Minus.app", "stage"]);
  } finally { await rm(f.root, { recursive: true, force: true }); }
});

test("a failed replacement rolls back the exact previous bundle", async () => {
  const f = await tree();
  try {
    await assert.rejects(replaceMacBundle(f.candidate, f.target, "0.4.19", {
      inspect, swap, afterExchange: async () => {
        assert.equal(await readFile(join(f.target, "marker"), "utf8"), "new");
        throw new Error("fixture replacement fault");
      },
    }), /fixture replacement fault/);
    assert.equal(await readFile(join(f.target, "marker"), "utf8"), "old");
    assert.equal(await readFile(join(f.candidate, "marker"), "utf8"), "new");
  } finally { await rm(f.root, { recursive: true, force: true }); }
});

test("a rejected new bundle post-check still restores the valid previous bundle", async () => {
  const f = await tree();
  try {
    await assert.rejects(replaceMacBundle(f.candidate, f.target, "0.4.19", {
      swap,
      inspect: async path => {
        const identity = await inspect(path);
        if (path === f.target && identity.version === "0.4.19") throw new Error("post-check-signature-failure");
        return identity;
      },
    }), /post-check-signature-failure/);
    assert.equal(await readFile(join(f.target, "marker"), "utf8"), "old");
  } finally { await rm(f.root, { recursive: true, force: true }); }
});

test("a wrong identity or timeout cannot stage any replacement", async () => {
  const f = await tree();
  try {
    await assert.rejects(replaceMacBundle(f.candidate, f.target, "0.4.19", { inspect: async () => ({ id: "wrong.app", version: "0.4.19" }) }), /InvalidUpdateIdentity/);
    await assert.rejects(replaceMacBundle(f.candidate, f.target, "0.4.18", { inspect: async path => ({ id: "fun.mjshao.codex-minus", version: (await readFile(join(path, "marker"), "utf8")) === "new" ? "0.4.18" : "0.4.18" }) }), /UpdateDowngrade/);
    assert.equal(await readFile(join(f.target, "marker"), "utf8"), "old");
    await assert.rejects(waitForParentExit(123, { isAlive: () => true, timeoutMs: 20, pollMs: 5 }), /UpdateParentStillRunning/);
  } finally { await rm(f.root, { recursive: true, force: true }); }
});
