// Move top-level helper functions out of src/App.tsx into a module, with their leading comments.
//
//   node scripts/move-helpers.mjs <target.ts> <header-file> name1 name2 ...
//
// Mechanical on purpose: it copies bytes and rewrites nothing, so a move cannot change behaviour.
// Unresolved references are left for `tsc` to report rather than guessed at here.

import { readFileSync, writeFileSync } from "node:fs";

const [target, headerFile, ...names] = process.argv.slice(2);
const appPath = "src/App.tsx";
const lines = readFileSync(appPath, "utf8").split("\n");

const found = new Map();
for (let i = 0; i < lines.length; i += 1) {
  const match = /^(?:export )?function ([A-Za-z_]\w*)/.exec(lines[i]);
  if (!match || !names.includes(match[1])) continue;
  let start = i;
  while (start > 0 && /^\s*(\/\/|\/\*|\*)/.test(lines[start - 1])) start -= 1;
  // A top-level function ends at the first `}` in column one. Counting braces instead would be
  // wrong: an escaped bracket inside a regex literal — `/^\s*\[[^\]]+\]\s*$/` — reads as an
  // unbalanced close and ends the function early, in the middle of its body.
  let end = i + 1;
  while (end < lines.length && lines[end] !== "}") end += 1;
  if (end >= lines.length) throw new Error(`unterminated function ${match[1]}`);
  found.set(match[1], { start, end });
}

const missing = names.filter((name) => !found.has(name));
if (missing.length) throw new Error(`not found in App.tsx: ${missing.join(", ")}`);

const moved = names.map((name) => {
  const { start, end } = found.get(name);
  const text = lines.slice(start, end + 1).join("\n");
  return text.startsWith("export ") ? text : `export ${text}`;
});
writeFileSync(target, `${readFileSync(headerFile, "utf8").trimEnd()}\n\n${moved.join("\n\n")}\n`);

const drop = new Set();
for (const { start, end } of found.values()) {
  for (let i = start; i <= end; i += 1) drop.add(i);
  if ((lines[end + 1] ?? "").trim() === "") drop.add(end + 1);
}
const rest = lines.filter((_, i) => !drop.has(i));

// Import back only what the shell still names, so the import list states the real coupling.
const body = rest.join("\n");
const used = names.filter((name) => new RegExp(`\\b${name}\\b`).test(body));
const specifier = `./${target.replace(/^src\//, "").replace(/\.ts$/, "")}`;
const anchor = '} from "./backend-types";\n';
const at = body.indexOf(anchor);
if (at < 0) throw new Error("could not find the backend-types import to anchor after");
const insert = `import {\n${used.map((n) => `  ${n},\n`).join("")}} from "${specifier}";\n`;
writeFileSync(appPath, body.slice(0, at + anchor.length) + insert + body.slice(at + anchor.length));

process.stdout.write(`${target}: ${names.length} moved, ${used.length} imported back\n`);
