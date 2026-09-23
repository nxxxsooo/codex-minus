import { spawn } from "node:child_process";
import { constants } from "node:fs";
import { readFile } from "node:fs/promises";
import { copyFile, cp, mkdir, writeFile } from "./update-fs.mjs";
import { basename, dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const source = dirname(fileURLToPath(import.meta.url));

export function awaitUpdateHelperReady(child, timeoutMs = 120_000) {
  return new Promise((resolve, reject) => {
    let text = "", settled = false;
    const fail = (code = "UpdateHandoffFailed") => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      child.kill();
      child.stdin?.destroy(); child.stdout?.destroy();
      reject(new Error(code));
    };
    const timer = setTimeout(() => fail("UpdateHandoffTimedOut"), timeoutMs);
    child.once("error", () => fail());
    child.once("exit", () => fail());
    child.stdout.on("data", bytes => {
      text += bytes.toString("utf8");
      if (text === "error:UpdateLocationNotWritable\n") { fail("UpdateLocationNotWritable"); return; }
      if (text.startsWith("error:") && text.length < 64 && !text.includes("\n")) return;
      if (text.length > 64 || !"ready\n".startsWith(text)) { fail(); return; }
      if (text !== "ready\n" || settled) return;
      settled = true; clearTimeout(timer);
      resolve(() => {
        if (child.exitCode !== null || child.killed) throw new Error("UpdateHandoffFailed");
        child.stdin.end("install\n");
        child.stdout.destroy();
        child.unref();
      });
    });
    child.stdin.on("error", () => fail());
  });
}

// Copy the complete runtime, not only its EXE: Electron needs its sibling DLLs / Frameworks.
// APFS cloning avoids duplicating the bytes locally; all paths survive bundle replacement.
export async function launchUpdateHelper(update, target, outcomePath, coreBinary, { executable = process.execPath } = {}) {
  const { directory: stage, mode } = update;
  if (mode !== "mac" && mode !== "windows") throw new Error("UnsupportedUpdateTarget");
  const helper = join(stage, "update-helper.mjs");
  // Sources live inside the current app.asar, so read those through Electron's virtual fs.
  for (const name of ["update-helper.mjs", "update-install.mjs", "update-core-verify.mjs", "update-fs.mjs"]) await writeFile(join(stage, name), await readFile(join(source, name)), { mode: 0o600 });
  const runtimeSource = mode === "mac" ? target : dirname(executable);
  const runtime = join(stage, "worker", mode === "mac" ? "Codex Minus.app" : "runtime");
  await mkdir(dirname(runtime), { recursive: true, mode: 0o700 });
  await cp(runtimeSource, runtime, { recursive: true, verbatimSymlinks: true, mode: constants.COPYFILE_FICLONE });
  const binary = join(runtime, relative(runtimeSource, executable));
  const core = join(stage, basename(coreBinary));
  await copyFile(coreBinary, core);
  const job = join(stage, "install-job.json");
  await writeFile(job, JSON.stringify({ ...update, parent: process.pid, target, outcomePath, core }), { mode: 0o600 });
  const child = spawn(binary, [helper, job], {
    detached: true, stdio: ["pipe", "pipe", "ignore"], windowsHide: true,
    env: { ...process.env, ELECTRON_RUN_AS_NODE: "1" },
  });
  return awaitUpdateHelperReady(child);
}
