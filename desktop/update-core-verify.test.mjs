import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { test } from "node:test";
import { verifyWithCore } from "./update-core-verify.mjs";

test("invokes only the managed Rust verifier, with no captured credential-bearing streams", async () => {
  let invocation;
  const spawnCore = (...args) => {
    invocation = args;
    const child = new EventEmitter();
    queueMicrotask(() => child.emit("close", 0));
    return child;
  };
  assert.equal(await verifyWithCore("/managed/core", "/private/candidate", "/private/candidate.sig", "a".repeat(64), 123, { spawnCore }), true);
  assert.deepEqual(invocation[1], ["--verify-update", "/private/candidate", "/private/candidate.sig", "a".repeat(64), "123"]);
  assert.deepEqual(invocation[2], { stdio: "ignore", windowsHide: true });
});

test("a failed or interrupted verifier cannot authorize installation", async () => {
  for (const outcome of [1, null]) {
    const spawnCore = () => { const child = new EventEmitter(); queueMicrotask(() => child.emit("close", outcome)); return child; };
    assert.equal(await verifyWithCore("/managed/core", "a", "b", "c", 4, { spawnCore }), false);
  }
  const spawnCore = () => { const child = new EventEmitter(); queueMicrotask(() => child.emit("error", new Error("private-path"))); return child; };
  await assert.rejects(verifyWithCore("/managed/core", "a", "b", "c", 4, { spawnCore }), /UpdateVerificationFailed/);
});
