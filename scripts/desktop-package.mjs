import { spawn } from "node:child_process";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { build, Platform, Arch } from "electron-builder";
import { inspectMacBundle } from "../desktop/update-install.mjs";

const platform = process.argv[2] ?? process.platform;
const arch = process.argv[3] ?? process.arch;
const targets = {
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
  "win32-arm64": "aarch64-pc-windows-msvc",
};
const target = targets[`${platform}-${arch}`];
if (!target || platform !== process.platform) throw new Error("Build on the target OS: desktop-package.mjs [darwin|win32] [arm64|x64]");
const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const command = (binary, args, env = process.env) => new Promise((resolve, reject) => {
  const child = spawn(binary, args, { cwd: root, stdio: "inherit", windowsHide: true, env });
  child.once("error", reject);
  child.once("exit", code => code === 0 ? resolve() : reject(new Error(`${binary} failed (${code})`)));
});
const config = JSON.parse(await readFile(join(root, "electron-builder.json"), "utf8"));
const { version } = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
await command("cargo", ["build", "--locked", "--release", "--manifest-path", "src-tauri/Cargo.toml", "--target", target]);
await command(process.execPath, ["node_modules/vite/bin/vite.js", "build"]);
const binary = platform === "win32" ? "codex-minus-core.exe" : "codex-minus-core";
const core = join(root, "src-tauri/target", target, "release", binary);
config.extraResources[0] = { from: core, to: `core/${binary}` };
// Never re-sign an in-place executable from a previously launched review bundle.
const unpacked = platform === "darwin" ? "mac-arm64" : arch === "x64" ? "win-unpacked" : "win-arm64-unpacked";
await rm(join(root, "dist-desktop", unpacked), { recursive: true, force: true });
// The builder concatenates resource arrays when an API config overlays electron-builder.json.
// Load exactly one complete config file, otherwise two core generations race to the same path.
const configFile = join(root, "dist-desktop", `.builder-config-${platform}-${arch}.json`);
await mkdir(join(root, "dist-desktop"), { recursive: true });
await writeFile(configFile, JSON.stringify(config));
await build({
  projectDir: root,
  targets: (platform === "darwin" ? Platform.MAC : Platform.WINDOWS).createTarget(platform === "darwin" ? ["dir"] : ["nsis"], Arch[arch]),
  publish: "never", config: configFile,
});
if (platform === "darwin") {
  const directory = join(root, "dist-desktop/mac-arm64");
  const app = join(directory, "Codex Minus.app");
  // electron-builder skips its signing hook for PRs. This identity is deliberately ad-hoc and
  // needs no private certificate; every review bundle must still have valid resource seals.
  if (config.mac.identity === "-") await command("/usr/bin/codesign", ["--force", "--deep", "--sign", "-", app]);
  if ((await inspectMacBundle(app)).version !== version) throw new Error("PackagedVersionMismatch");
  await command("/usr/bin/ditto", ["-c", "-k", "--sequesterRsrc", "--keepParent", app, join(root, `dist-desktop/CodexMinus_${version}_aarch64.app.zip`)]);
  // AppleDouble entries such as top-level ._Codex Minus.app break the legacy updater's
  // strip-first-component extraction. Code signatures live in ordinary bundle files.
  await command("/usr/bin/tar", ["--no-xattrs", "-czf", join(root, `dist-desktop/CodexMinus_${version}_aarch64.app.tar.gz`), "-C", directory, "Codex Minus.app"], { ...process.env, COPYFILE_DISABLE: "1" });
}
