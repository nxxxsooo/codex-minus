import { createRequire } from "node:module";

// Electron's patched fs treats app.asar as a virtual directory. Update/install operations must
// copy, inspect and remove the physical archive bytes, including in ELECTRON_RUN_AS_NODE helpers.
const require = createRequire(import.meta.url);
export const { copyFile, cp, lstat, mkdir, mkdtemp, readdir, realpath, rm, writeFile } =
  require(process.versions.electron ? "original-fs" : "node:fs").promises;
