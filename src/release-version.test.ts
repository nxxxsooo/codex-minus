import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

/// The version lives in four places that no tool keeps in sync.
///
/// `package.json` names the npm package, `tauri.conf.json` names the installer and the app bundle,
/// `Cargo.toml` names the crate, and `Cargo.lock` records what was actually built. Bumping three of
/// them and missing the fourth produces installers whose filename disagrees with the app's own
/// About screen — and `Cargo.lock` in particular is easy to miss because nothing but `cargo` writes
/// it. Reading them here means the mismatch fails a test instead of shipping.
const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

const sources = {
  "package.json": JSON.parse(read("../package.json")).version as string,
  "src-tauri/tauri.conf.json": JSON.parse(read("../src-tauri/tauri.conf.json")).version as string,
  "src-tauri/Cargo.toml":
    read("../src-tauri/Cargo.toml").match(/^version = "([^"]+)"/m)?.[1] ?? "",
  "src-tauri/Cargo.lock":
    read("../src-tauri/Cargo.lock").match(
      /\nname = "codex-minus"\nversion = "([^"]+)"/,
    )?.[1] ?? "",
};

describe("every file that carries the version agrees", () => {
  it("found a version in each of them", () => {
    for (const [file, version] of Object.entries(sources)) {
      assert.match(version, /^\d+\.\d+\.\d+$/, `${file} carries no readable version`);
    }
  });

  it("carries one version, not four", () => {
    const [reference] = Object.values(sources);
    for (const [file, version] of Object.entries(sources)) {
      assert.equal(
        version,
        reference,
        `${file} says ${version} while package.json says ${reference} — run scripts/release.sh`,
      );
    }
  });
});
