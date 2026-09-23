import { readFile, rename, rmdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { lstat, readdir, realpath, rm } from "./update-fs.mjs";

function processAlive(pid) {
  if (!Number.isSafeInteger(pid) || pid <= 0) return false;
  try { process.kill(pid, 0); return true; } catch (error) { return error.code !== "ESRCH"; }
}

async function ownedStage(result, tempRoot) {
  if (typeof result.stage !== "string" || !/^codex-minus-update-[\w-]+$/.test(basename(result.stage))
    || typeof result.stageId !== "string" || !result.stageId || result.stageId.length > 80) return null;
  const metadata = await lstat(result.stage);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) return null;
  const stage = await realpath(result.stage);
  if (dirname(stage) !== await realpath(tempRoot)) return null;
  const path = join(stage, ".codex-minus-update.json");
  const marker = await lstat(path).catch(() => null);
  // If interrupted after removing the marker last, only an empty owned-name directory remains.
  if (!marker) return (await readdir(stage)).length === 0 ? stage : null;
  if (!marker.isFile() || marker.isSymbolicLink() || marker.size > 512) return null;
  const owner = JSON.parse(await readFile(path, "utf8"));
  return owner.id === result.stageId && owner.version === result.version ? stage : null;
}

function versionAtLeast(current, previous) {
  const a = current.split(".").map(Number), b = previous.split(".").map(Number);
  const index = a.findIndex((part, i) => part !== b[i]);
  return index < 0 || a[index] > b[index];
}

export async function consumeUpdateOutcomes(directory, currentVersion, options) {
  const notices = [];
  for (const name of await readdir(directory).catch(() => [])) {
    if (!/^[\w-]+\.json$/.test(name)) continue;
    const result = await consumeUpdateOutcome(join(directory, name), currentVersion, options);
    if (result) notices.push(result);
  }
  return notices.find(notice => notice.status === "unconfirmed") ?? notices.at(-1) ?? null;
}

export async function consumeUpdateOutcome(path, currentVersion, {
  tempRoot = tmpdir(), isAlive = processAlive, removeEntry = path => rm(path, { recursive: true, force: true }),
} = {}) {
  let result;
  try {
    const metadata = await lstat(path);
    if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size > 8192) return null;
    result = JSON.parse(await readFile(path, "utf8"));
    if (!result || !/^[0-9]+\.[0-9]+\.[0-9]+$/.test(result.version)) return null;
  } catch { return null; }
  const installed = versionAtLeast(currentVersion, result.version) && ["installed", "handoff"].includes(result.status);
  const notice = result.reported ? null : { status: installed ? "installed" : "unconfirmed" };
  // A live installer may still hold the worker/EXE. Keep a reported receipt for the next startup;
  // interrupted or failed rollback receipts retain their backup locations for recovery.
  if (installed && !isAlive(result.helperPid) && !isAlive(result.installerPid)) {
    const stage = await ownedStage(result, tempRoot).catch(() => null);
    if (stage) {
      try {
        // Delete payloads first. Ownership survives locked files, crashes and interrupted removal.
        for (const name of await readdir(stage)) {
          if (name !== ".codex-minus-update.json") await removeEntry(join(stage, name));
        }
        await rm(join(stage, ".codex-minus-update.json"), { force: true });
        await rmdir(stage);
        await rm(path, { force: true });
        return notice;
      } catch { /* Retry cleanup at the next launch if Windows still has a file open. */ }
    }
  }
  const pending = path + ".reported";
  await writeFile(pending, JSON.stringify({ ...result, reported: true }) + "\n", { mode: 0o600 });
  await rename(pending, path);
  return notice;
}
