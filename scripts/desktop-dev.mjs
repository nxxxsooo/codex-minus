import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { setTimeout as delay } from "node:timers/promises";

const require = createRequire(import.meta.url);
const children = new Set();
function start(program, args, options = {}) {
  const child = spawn(program, args, { stdio: "inherit", ...options });
  children.add(child); child.on("exit", () => children.delete(child));
  return child;
}
function stop() { for (const child of children) child.kill(); }
process.on("SIGINT", () => { stop(); process.exit(130); });
process.on("SIGTERM", () => { stop(); process.exit(143); });
process.on("exit", stop);

const build = start("cargo", ["build", "--manifest-path", "src-tauri/Cargo.toml"]);
const code = await new Promise((resolve, reject) => { build.once("exit", resolve); build.once("error", reject); });
if (code !== 0) process.exit(code ?? 1);
const vite = start(process.execPath, ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", "1420", "--strictPort"]);
let ready = false;
for (let i = 0; i < 100 && vite.exitCode === null; i++) {
  try { if ((await fetch("http://127.0.0.1:1420")).ok) { ready = true; break; } } catch {}
  await delay(100);
}
if (!ready || vite.exitCode !== null) { stop(); process.exit(1); }
const env = { ...process.env, CODEX_MINUS_DEV_URL: "http://127.0.0.1:1420" };
delete env.ELECTRON_RUN_AS_NODE;
const electron = start(require("electron"), ["."], { env });
electron.once("exit", (code) => { stop(); process.exit(code ?? 0); });
