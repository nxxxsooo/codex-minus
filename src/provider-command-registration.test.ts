import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const source = readFileSync(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
const nativeCapabilitySource = readFileSync(
  new URL("../src-tauri/src/provider_native_capability.rs", import.meta.url),
  "utf8",
);
const handler = source.match(/\.invoke_handler\(tauri::generate_handler!\[([\s\S]*?)\]\)/)?.[1];

const allowedNativeCatalogApis = new Set([
  "catalog_state_path",
  "default_catalog_mode_for_profile",
  "persisted_catalog_mode_from_path",
  "read_only_catalog_modes_from_path",
]);

function nativeCatalogAuthorityViolations(candidate: string): string[] {
  const violations: string[] = [];
  const catalogImports = candidate.match(/^\s*(?:pub\s+)?use\s+[^;]*model_catalog[^;]*;/gm) ?? [];
  if (catalogImports.some((entry) => entry.trim() !== "pub use crate::model_catalog::CatalogMode;")) {
    violations.push("catalog-authority-import");
  }

  const disallowedQualifiedApis = [...candidate.matchAll(/crate::model_catalog::([a-z_]+)/g)]
    .map((match) => match[1])
    .filter((name, index, names) => names.indexOf(name) === index)
    .filter((name) => !allowedNativeCatalogApis.has(name));
  const localAuthoritySymbol = /\b(?:[A-Z][A-Z0-9_]*(?:CATALOG|BASELINE|UPDATE_CHANNEL)[A-Z0-9_]*|(?:refresh|compose|materialize|update|write)_[a-z0-9_]*catalog[a-z0-9_]*)\b/;
  if (disallowedQualifiedApis.length > 0 || localAuthoritySymbol.test(candidate)) {
    violations.push("catalog-authority-symbol");
  }
  if (/\binclude_(?:str|bytes)!\s*\(/.test(candidate)) {
    violations.push("catalog-bundled-source");
  }
  if (/\b(?:std::fs::write|fs::write|File::create|OpenOptions)\b/.test(candidate)) {
    violations.push("catalog-write-surface");
  }
  return violations;
}

describe("provider command registration boundary", () => {
  it("exposes only the unified provider commit and explicit external-adoption write paths", () => {
    assert.ok(handler, "the Tauri invoke handler must remain statically auditable");
    assert.match(handler, /commands::commit_provider_detail/);
    assert.match(handler, /model_catalog::adopt_external_model_catalog/);
    for (const bypass of [
      "commands::save_relay_file",
      "commands::backfill_relay_profile_from_live",
      "commands::switch_relay_profile",
      "commands::save_active_relay_profile",
      "commands::apply_relay_injection",
      "commands::apply_pure_api_injection",
      "commands::clear_relay_injection",
      "model_catalog::save_profile_catalog",
    ]) {
      assert.doesNotMatch(handler, new RegExp(bypass.replace("::", "::")));
    }
  });

  it("keeps catalog refresh ownership out of the native-capability command surface", () => {
    assert.ok(handler, "the Tauri invoke handler must remain statically auditable");
    const nativeCommands = [...handler.matchAll(/provider_native_capability::([a-z_]+)/g)]
      .map((match) => match[1]);
    assert.deepEqual(nativeCommands, [
      "inspect_provider_native_capabilities",
      "transform_provider_native_capability_draft",
    ]);
    assert.match(handler, /model_catalog::refresh_official_model_catalog/);

    assert.deepEqual(nativeCatalogAuthorityViolations(nativeCapabilitySource), []);
  });

  it("rejects imported, aliased, or locally bundled catalog authority", () => {
    const competingAuthority = `
      use crate::model_catalog::refresh_official_model_catalog as refresh;
      const OFFICIAL_CATALOG_BASELINE: &str = include_str!("official-models.json");
      fn refresh_native_catalog() {
        std::fs::write("models.json", OFFICIAL_CATALOG_BASELINE).unwrap();
        let _ = refresh();
      }
    `;

    assert.deepEqual(nativeCatalogAuthorityViolations(competingAuthority), [
      "catalog-authority-import",
      "catalog-authority-symbol",
      "catalog-bundled-source",
      "catalog-write-surface",
    ]);
  });
});
