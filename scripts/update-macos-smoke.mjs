// Headless native update acceptance. App trees, installer bytes and restart marker are disposable.
// Signature *orchestration* uses a fixture-only verifier executable; pinned-key crypto is covered
// by the Rust signature tests. No production bypass, key or runtime environment override exists.
import assert from "node:assert/strict";
import { execFile, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { constants } from "node:fs";
import { cp, mkdir, mkdtemp, readFile, rename, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { promisify } from "node:util";
import { pathToFileURL } from "node:url";
import { createPackage } from "@electron/asar";
import { inspectMacBundle, replaceMacBundle, stageVerifiedMacApp, prepareMacReplacement } from "../desktop/update-install.mjs";

if (process.platform !== "darwin") throw new Error("This acceptance requires native macOS");
const execute = promisify(execFile);
const require = createRequire(import.meta.url);
const repo = resolve(new URL("..", import.meta.url).pathname);
const core = join(repo, "src-tauri/target/debug/codex-minus-core");
const electron = require("electron");
const runtime = dirname(dirname(dirname(electron)));
await mkdir(join(homedir(), ".cache"), { recursive: true });
const root = await mkdtemp(join(homedir(), ".cache/cm-update-"));
const target = join(root, "installed/Codex Minus.app");
const candidate = join(root, "download/Codex Minus.app");
const marker = join(root, "restarted.json");
const outcome = join(root, "outcome.json");
const swap = (left, right) => execute(core, ["--swap-update-bundles", left, right], { timeout: 15_000 });
const shellQuote = value => `'${value.replaceAll("'", "'\\''")}'`;

async function makeApp(path, version) {
  await cp(runtime, path, { recursive: true, verbatimSymlinks: true, mode: constants.COPYFILE_FICLONE });
  await rename(join(path, "Contents/MacOS/Electron"), join(path, "Contents/MacOS/codex-minus"));
  const plist = join(path, "Contents/Info.plist");
  for (const [key, value] of Object.entries({ CFBundleIdentifier: "fun.mjshao.codex-minus", CFBundleExecutable: "codex-minus", CFBundleShortVersionString: version })) {
    await execute("/usr/bin/plutil", ["-replace", key, "-string", value, plist]);
  }
  const source = join(root, `source-${version}`);
  await mkdir(source);
  await writeFile(join(source, "package.json"), JSON.stringify({ name: "update-fixture", version, main: "main.cjs" }));
  await writeFile(join(source, "main.cjs"), `const {app}=require('electron');app.setActivationPolicy('prohibited');app.whenReady().then(()=>{require('fs').writeFileSync(process.env.CODEX_MINUS_UPDATE_TEST_MARKER,JSON.stringify({version:app.getVersion(),windows:require('electron').BrowserWindow.getAllWindows().length}));app.quit()});`);
  await createPackage(source, join(path, "Contents/Resources/app.asar"));
  await mkdir(join(path, "Contents/Resources/core"));
  await cp(core, join(path, "Contents/Resources/core/codex-minus-core"));
  await execute("/usr/bin/codesign", ["--force", "--deep", "--sign", "-", path], { timeout: 120_000 });
}

try {
  await makeApp(target, "0.4.18");
  await makeApp(candidate, "0.4.19");
  await assert.rejects(replaceMacBundle(candidate, target, "0.4.19", {
    swap, afterExchange: async () => {
      assert.equal((await inspectMacBundle(target)).version, "0.4.19");
      throw new Error("injected-post-exchange-failure");
    },
  }), /injected-post-exchange-failure/);
  assert.equal((await inspectMacBundle(target)).version, "0.4.18");
  // Simulate interruption immediately after the atomic syscall: both complete generations exist.
  const interrupted = await prepareMacReplacement(candidate, target, "0.4.19");
  await swap(interrupted.replacement, target);
  assert.equal((await inspectMacBundle(target)).version, "0.4.19");
  assert.equal((await inspectMacBundle(interrupted.replacement)).version, "0.4.18");
  await swap(interrupted.replacement, target);
  await rm(interrupted.directory, { recursive: true });

  const directory = dirname(candidate);
  const artifact = join(directory, "candidate.tar.gz"), signature = join(directory, "candidate.sig");
  await execute("/usr/bin/tar", ["-czf", artifact, "-C", directory, "Codex Minus.app"], { timeout: 120_000 });
  const bytes = await readFile(artifact);
  const sha256 = createHash("sha256").update(bytes).digest("hex");
  await writeFile(signature, "disposable-signature-fixture");
  const verifier = join(root, "fixture-verifier");
  await writeFile(verifier, `#!/bin/sh\nif [ "$1" = '--verify-update' ]; then\n  [ "$4" = '${sha256}' ] && [ "$(/usr/bin/shasum -a 256 "$2" | /usr/bin/cut -d ' ' -f 1)" = '${sha256}' ]\n  exit $?\nfi\nexec ${shellQuote(core)} "$@"\n`, { mode: 0o700 });
  const extracted = join(root, "extracted");
  await mkdir(extracted);
  await stageVerifiedMacApp(artifact, extracted, "0.4.19");
  await rm(extracted, { recursive: true });
  const update = { directory, artifact, signature, version: "0.4.19", sha256, size: bytes.length, mode: "mac" };
  const script = `import {launchUpdateHelper} from ${JSON.stringify(pathToFileURL(join(repo, "desktop/update-handoff.mjs")).href)};const commit=await launchUpdateHelper(${JSON.stringify(update)},${JSON.stringify(target)},${JSON.stringify(outcome)},${JSON.stringify(verifier)},{executable:${JSON.stringify(join(target, "Contents/MacOS/codex-minus"))}});commit();setTimeout(()=>process.exit(0),100);`;
  const parent = spawn(process.execPath, ["--input-type=module", "-e", script], {
    env: { ...process.env, CODEX_MINUS_UPDATE_TEST_MARKER: marker }, stdio: ["ignore", "ignore", "pipe"],
  });
  let errors = "";
  parent.stderr.on("data", data => { errors += data.toString(); });
  const exit = await new Promise(resolve => parent.once("exit", resolve));
  assert.equal(exit, 0, errors + await readFile(outcome, "utf8").catch(() => "no outcome"));
  const deadline = Date.now() + 90_000;
  let restarted;
  while (Date.now() < deadline) {
    try { restarted = JSON.parse(await readFile(marker, "utf8")); break; } catch {}
    const result = JSON.parse(await readFile(outcome, "utf8").catch(() => "null"));
    if (result?.status === "failed") throw new Error(`Native update failed: ${result.code}`);
    await delay(100);
  }
  assert.deepEqual(restarted, { version: "0.4.19", windows: 0 });
  assert.equal((await inspectMacBundle(target)).version, "0.4.19");
  assert.equal(JSON.parse(await readFile(outcome, "utf8")).status, "installed");
  console.log(JSON.stringify({ nativeExchange: true, verifiedRollback: true, interruptedPathIntact: true, copiedRuntimeReady: true, parentExitHandoff: true, managerRestarted: true, windowsOpened: 0, fixtureSignatureOnly: true }));
} finally { await rm(root, { recursive: true, force: true }); }
