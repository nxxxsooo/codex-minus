import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");
const tauriConfig = JSON.parse(read("../src-tauri/tauri.conf.json"));
const appSource = read("./App.tsx");
const libSource = read("../src-tauri/src/lib.rs");
const workflow = read("../.github/workflows/build.yml");

describe("product identity", () => {
  it("presents one product name across shipped surfaces", () => {
    assert.equal(tauriConfig.productName, "Codex Minus");
    assert.equal(tauriConfig.app.windows[0].title, "Codex Minus");
    assert.match(read("../index.html"), /<title>Codex Minus<\/title>/);
    assert.match(appSource, /windowTitle: "Codex Minus"/);
    assert.match(appSource, /<div className="brand-title">Codex Minus<\/div>/);
    assert.match(libSource, /\.title\("Codex Minus"\)/);
  });

  it("pins the binary name so the single-instance check keeps matching", () => {
    // Tauri defaults the executable name to productName; the Windows duplicate-launch guard
    // compares against a fixed name, so the display name must not decide the executable name.
    assert.equal(tauriConfig.mainBinaryName, "codex-minus");
    const guard = libSource.match(/eq_ignore_ascii_case\("([^"]+)"\)/)?.[1];
    assert.equal(guard, `${tauriConfig.mainBinaryName}.exe`);
  });

  it("keeps identity and on-disk contracts unchanged by the rename", () => {
    assert.equal(tauriConfig.identifier, "fun.mjshao.codex-minus");
    const catalog = read("../src-tauri/src/model_catalog.rs");
    assert.match(catalog, /GENERATED_PREFIX: &str = "codex-minus-"/);
    assert.match(read("../src-tauri/src/live_state.rs"), /\.codex-minus-/);
  });

  it("names release artifacts after the renamed bundle", () => {
    // The signing and zip steps reference the bundle path literally, so a missed rename here
    // fails the macOS build rather than producing a misnamed artifact.
    assert.match(workflow, /bundle\/macos\/Codex Minus\.app/);
    assert.ok(!workflow.includes("Codex-- Manager"), "the workflow still names the old bundle");
    assert.ok(!workflow.includes("Codex--Manager"), "the workflow still names the old artifact");
  });

  it("leaves no old product name in a shipped surface", () => {
    for (const path of ["./App.tsx", "./i18n-en.ts", "../src-tauri/src/commands.rs", "../index.html"]) {
      assert.ok(!read(path).includes("Codex--"), `${path} still says Codex--`);
    }
  });
});
