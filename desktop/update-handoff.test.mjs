import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { test } from "node:test";
import { awaitUpdateHelperReady } from "./update-handoff.mjs";

test("a spawned helper that exits before readiness cannot make the manager quit", async () => {
  const child = spawn(process.execPath, ["-e", "process.exit(1)"], { stdio: ["pipe", "pipe", "ignore"] });
  await assert.rejects(awaitUpdateHelperReady(child, 1_000), /UpdateHandoffFailed/);
});

test("a helper must acknowledge readiness and receive an explicit commit", async () => {
  const child = spawn(process.execPath, ["-e", "process.stdout.write('ready\\n');process.stdin.once('data', d=>process.exit(d.toString()==='install\\n'?0:2))"], { stdio: ["pipe", "pipe", "ignore"] });
  const closed = new Promise(resolve => child.once("exit", resolve));
  const commit = await awaitUpdateHelperReady(child, 1_000);
  assert.equal(child.exitCode, null);
  commit();
  assert.equal(await closed, 0);
});

test("a silent helper is terminated without an installation commit", async () => {
  const child = spawn(process.execPath, ["-e", "setInterval(()=>{},1000)"], { stdio: ["pipe", "pipe", "ignore"] });
  await assert.rejects(awaitUpdateHelperReady(child, 100), /UpdateHandoffTimedOut/);
  assert.equal(child.killed, true);
});
