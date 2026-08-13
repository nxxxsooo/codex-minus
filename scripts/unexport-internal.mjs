// Drop `export` from declarations in a module that nothing outside it names.
//
//   node scripts/unexport-internal.mjs src/codex-toml.ts
//
// A move carries a function's private helpers along with it. Left exported they read as a public
// surface the module does not actually offer, and knip reports every one of them.

import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const target = process.argv[2];
if (!target) throw new Error("usage: node scripts/unexport-internal.mjs <module.ts>");

const walk = (dir) =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });

const outside = walk("src")
  .filter((path) => path !== target && /\.tsx?$/.test(path))
  .map((path) => readFileSync(path, "utf8"))
  .join("\n");

let source = readFileSync(target, "utf8");
const dropped = [];
for (const [, name] of source.matchAll(/^export (?:function|const|type|interface) ([A-Za-z_]\w*)/gm)) {
  if (new RegExp(`\\b${name}\\b`).test(outside)) continue;
  source = source.replace(
    new RegExp(`^export ((?:function|const|type|interface) ${name}\\b)`, "m"),
    "$1",
  );
  dropped.push(name);
}
writeFileSync(target, source);
process.stdout.write(`${target}: un-exported ${dropped.length}${dropped.length ? ` (${dropped.join(", ")})` : ""}\n`);
