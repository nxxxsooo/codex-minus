import { execFile } from "node:child_process";
import { constants } from "node:fs";
import { cp, lstat, mkdtemp, readdir, realpath, rm } from "./update-fs.mjs";
import { basename, dirname, isAbsolute, join, relative } from "node:path";
import { promisify } from "node:util";

const execute = promisify(execFile);
const APP_NAME = "Codex Minus.app";
const APP_ID = "fun.mjshao.codex-minus";

export async function inspectMacBundle(bundle) {
  const plist = join(bundle, "Contents", "Info.plist");
  const read = async key => (await execute("/usr/bin/plutil", ["-extract", key, "raw", "-o", "-", plist], { timeout: 10_000 })).stdout.trim();
  const [id, version, executable] = await Promise.all([read("CFBundleIdentifier"), read("CFBundleShortVersionString"), read("CFBundleExecutable")]);
  if (executable !== "codex-minus") throw new Error("InvalidUpdateIdentity");
  for (const path of [join(bundle, "Contents/Resources/app.asar"), join(bundle, "Contents/Resources/core/codex-minus-core")]) {
    if (!(await lstat(path)).isFile()) throw new Error("InvalidUpdateIdentity");
  }
  await execute("/usr/bin/codesign", ["--verify", "--deep", "--strict", bundle], { timeout: 45_000 });
  return { id, version };
}

async function assertBundle(bundle, inspect, expectedVersion = null) {
  const metadata = await lstat(bundle);
  if (!metadata.isDirectory() || metadata.isSymbolicLink() || basename(bundle) !== APP_NAME) throw new Error("InvalidUpdateIdentity");
  const identity = await inspect(bundle);
  if (identity.id !== APP_ID || expectedVersion && identity.version !== expectedVersion) throw new Error("InvalidUpdateIdentity");
  return identity;
}

export async function prepareMacReplacement(candidate, target, version, { inspect = inspectMacBundle } = {}) {
  if (candidate === target || !/^[0-9]+\.[0-9]+\.[0-9]+$/.test(version)) throw new Error("InvalidUpdateIdentity");
  await assertBundle(candidate, inspect, version);
  const { version: previousVersion } = await assertBundle(target, inspect);
  const current = previousVersion.split(".").map(Number);
  const next = version.split(".").map(Number);
  const different = next.findIndex((part, index) => part !== current[index]);
  if (current.length !== 3 || current.some(part => !Number.isSafeInteger(part)) || different < 0 || next[different] < current[different]) throw new Error("UpdateDowngrade");
  // A sibling staging directory both preflights write access and guarantees same-volume exchange.
  let directory;
  try { directory = await mkdtemp(join(dirname(target), ".codex-minus-update-")); }
  catch { throw new Error("UpdateLocationNotWritable"); }
  const replacement = join(directory, APP_NAME);
  try {
    await cp(candidate, replacement, { recursive: true, verbatimSymlinks: true, mode: constants.COPYFILE_FICLONE });
    await assertBundle(replacement, inspect, version);
    return { directory, replacement, target, version, previousVersion };
  } catch (error) { await rm(directory, { recursive: true, force: true }); throw error; }
}

export async function commitMacReplacement(plan, { inspect = inspectMacBundle, swap, afterExchange = () => {} }) {
  const { directory, replacement, target, version, previousVersion } = plan;
  await assertBundle(target, inspect, previousVersion);
  await assertBundle(replacement, inspect, version);
  const previous = await lstat(target), next = await lstat(replacement);
  const sameFile = (left, right) => left.dev === right.dev && left.ino === right.ino;
  try {
    await swap(replacement, target);
    await afterExchange();
    await assertBundle(target, inspect, version);
  } catch (error) {
    try {
      // Reconcile syscall outcome by filesystem identity: the new bundle may be precisely what
      // failed its code-signature post-check, so requiring it to validate would prevent rollback.
      const installed = await lstat(target), retained = await lstat(replacement);
      if (sameFile(installed, next) && sameFile(retained, previous)) {
        await assertBundle(replacement, inspect, previousVersion);
        await swap(replacement, target);
      } else if (!sameFile(installed, previous) || !sameFile(retained, next)) throw new Error("UpdateRollbackFailed");
      await assertBundle(target, inspect, previousVersion);
    } catch { throw new Error("UpdateRollbackFailed"); }
    await rm(directory, { recursive: true, force: true });
    throw error;
  }
  // The old app remains recoverable until the new bundle passes its post-check. A cleanup error
  // leaves that backup behind, not a false installation failure after successful replacement.
  await rm(directory, { recursive: true, force: true }).catch(() => {});
}

export async function replaceMacBundle(candidate, target, version, options) {
  const plan = await prepareMacReplacement(candidate, target, version, options);
  return commitMacReplacement(plan, options);
}

export async function waitForParentExit(pid, { isAlive = parentAlive, timeoutMs = 30_000, pollMs = 100 } = {}) {
  if (!Number.isSafeInteger(pid) || pid <= 0) throw new Error("InvalidUpdateParent");
  const deadline = Date.now() + timeoutMs;
  while (isAlive(pid)) {
    if (Date.now() >= deadline) throw new Error("UpdateParentStillRunning");
    await new Promise(resolve => setTimeout(resolve, pollMs));
  }
}

function parentAlive(pid) {
  try { process.kill(pid, 0); return true; }
  catch (error) { return error.code !== "ESRCH"; }
}

async function assertLinksWithin(root, directory = root) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isSymbolicLink()) {
      const resolved = await realpath(path);
      const portion = relative(root, resolved);
      if (portion.startsWith("..") || isAbsolute(portion)) throw new Error("InvalidUpdateArchive");
    } else if (entry.isDirectory()) await assertLinksWithin(root, path);
  }
}

export async function stageVerifiedMacApp(archive, directory, version, { inspect = inspectMacBundle } = {}) {
  if (process.platform !== "darwin") throw new Error("UnsupportedUpdateTarget");
  const metadata = await lstat(archive);
  if (!metadata.isFile() || metadata.isSymbolicLink()) throw new Error("InvalidUpdateArchive");
  const { stdout } = await execute("/usr/bin/tar", ["-tzf", archive], { timeout: 120_000, maxBuffer: 16 * 1024 * 1024 });
  const entries = stdout.trimEnd().split("\n");
  if (!entries.length || entries.some(entry => !entry.startsWith(`${APP_NAME}/`) && entry !== APP_NAME
    || entry.split("/").some(part => part === ".." || part === ".") || entry.includes("\\"))) throw new Error("InvalidUpdateArchive");
  await execute("/usr/bin/tar", ["-xzf", archive, "-C", directory], { timeout: 120_000 });
  const candidate = join(directory, APP_NAME);
  await assertLinksWithin(await realpath(candidate));
  await assertBundle(candidate, inspect, version);
  return candidate;
}
