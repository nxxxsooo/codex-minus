import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { promisify } from "node:util";
import { stageVerifiedMacApp } from "../desktop/update-install.mjs";

test("valid framework links remain inside a bundle under a symlinked staging ancestor", { skip: process.platform !== "darwin" }, async () => {
  const root = await mkdtemp(join(tmpdir(), "cm-archive-alias-"));
  try {
    const source = join(root, "source"), destination = join(root, "actual");
    await mkdir(join(source, "Codex Minus.app/framework/Versions/A"), { recursive: true });
    await writeFile(join(source, "Codex Minus.app/framework/Versions/A/file"), "content");
    await symlink("Versions/A", join(source, "Codex Minus.app/framework/Current"));
    await mkdir(destination);
    const alias = join(root, "alias");
    await symlink(destination, alias);
    const archive = join(root, "valid.tar.gz");
    await promisify(execFile)("/usr/bin/tar", ["--no-xattrs", "-czf", archive, "-C", source, "Codex Minus.app"], { env: { ...process.env, COPYFILE_DISABLE: "1" } });
    const candidate = await stageVerifiedMacApp(archive, alias, "0.5.0", { inspect: async () => ({ id: "fun.mjshao.codex-minus", version: "0.5.0" }) });
    assert.equal(await readFile(join(candidate, "framework/Current/file"), "utf8"), "content");
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("archive traversal and symlink-parent writes cannot escape private extraction", { skip: process.platform !== "darwin" }, async () => {
  const root = await mkdtemp(join(tmpdir(), "cm-archive-"));
  try {
    const outside = join(root, "outside");
    await mkdir(outside);
    await writeFile(join(outside, "sentinel"), "untouched");
    for (const kind of ["traversal", "symlink"]) {
      const archive = join(root, `${kind}.tar.gz`), stage = join(root, kind);
      await mkdir(stage);
      await promisify(execFile)("/usr/bin/python3", ["-c", `import io,sys,tarfile
archive,outside,kind=sys.argv[1:]
with tarfile.open(archive,'w:gz') as t:
 if kind=='symlink':
  link=tarfile.TarInfo('Codex Minus.app/escape');link.type=tarfile.SYMTYPE;link.linkname=outside;t.addfile(link)
 name='../outside/sentinel' if kind=='traversal' else 'Codex Minus.app/escape/sentinel'
 item=tarfile.TarInfo(name);item.size=7;t.addfile(item,io.BytesIO(b'changed'))
`, archive, outside, kind]);
      await assert.rejects(stageVerifiedMacApp(archive, stage, "0.5.0"));
      assert.equal(await readFile(join(outside, "sentinel"), "utf8"), "untouched");
    }
  } finally { await rm(root, { recursive: true, force: true }); }
});
