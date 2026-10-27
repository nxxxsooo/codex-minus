// This source and the complete Electron runtime are copied outside the installed bundle.
import { spawn, execFile } from "node:child_process";
import { readFile, rename, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { promisify } from "node:util";
import { createInterface } from "node:readline";
import { prepareMacReplacement, commitMacReplacement, waitForParentExit } from "./update-install.mjs";
import { verifyWithCore } from "./update-core-verify.mjs";

const execute = promisify(execFile);
let job, plan, installerPid = null, committed = false;
const outcome = async (status, code) => {
  const path = job.outcomePath;
  const temporary = path + ".pending";
  await writeFile(temporary, JSON.stringify({ status, code, version: job.version,
    stage: job.directory, stageId: job.stageId, helperPid: process.pid, installerPid,
    backup: plan?.directory ?? null }) + "\n", { mode: 0o600 });
  await rename(temporary, path);
};
const start = async (binary, args, env = process.env) => {
  const child = spawn(binary, args, { detached: true, stdio: "ignore", windowsHide: false, env });
  await new Promise((resolve, reject) => { child.once("spawn", resolve); child.once("error", reject); });
  child.unref();
  return child.pid;
};

try {
  job = JSON.parse(await readFile(process.argv[2], "utf8"));
  if (!Number.isSafeInteger(job.parent) || job.parent < 1 || !["mac", "windows"].includes(job.mode)
    || job.mode !== (process.platform === "darwin" ? "mac" : process.platform === "win32" ? "windows" : null)) throw new Error("InvalidUpdateHandoff");
  if (!await verifyWithCore(job.core, job.artifact, job.signature, job.sha256, job.size)) throw new Error("UpdateSignatureRejected");
  if (job.mode === "mac") plan = await prepareMacReplacement(join(job.directory, "Codex Minus.app"), job.target, job.version);
  const input = createInterface({ input: process.stdin });
  const authorized = new Promise(resolve => {
    const timer = setTimeout(() => { resolve(false); input.close(); }, 30_000);
    input.once("line", line => { clearTimeout(timer); resolve(line === "install"); input.close(); });
    input.once("close", () => { clearTimeout(timer); resolve(false); });
  });
  process.stdout.write("ready\n");
  if (!await authorized) throw new Error("UpdateHandoffCancelled");
  process.stdin.destroy();
  committed = true;
  await outcome("pending", "UpdatePending");
  await waitForParentExit(job.parent);
  if (job.mode === "mac") {
    await commitMacReplacement(plan, {
      swap: (left, right) => execute(job.core, ["--swap-update-bundles", left, right], { timeout: 15_000, windowsHide: true }),
    });
    await outcome("installed", "UpdateInstalled");
    const env = { ...process.env };
    delete env.ELECTRON_RUN_AS_NODE;
    await start(join(job.target, "Contents/MacOS/codex-minus"), [], env);
  } else {
    // Record before spawn: a fast installer may open the new manager before returning to us.
    await outcome("handoff", "InstallerStarted");
    installerPid = await start(job.artifact, []);
    await outcome("handoff", "InstallerStarted");
  }
} catch (error) {
  const code = ["UpdateSignatureRejected", "UpdateLocationNotWritable", "UpdateParentStillRunning", "UpdateHandoffCancelled", "InvalidUpdateIdentity", "UpdateDowngrade", "UpdateRollbackFailed"]
    .find(value => error?.message === value) ?? "UpdateInstallationFailed";
  if (job?.outcomePath) await outcome("failed", code).catch(() => {});
  if (!committed) process.stdout.write(`error:${code}\n`);
  if (!committed && plan) await rm(plan.directory, { recursive: true, force: true }).catch(() => {});
  process.exitCode = 1;
}
