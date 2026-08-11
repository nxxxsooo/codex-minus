import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const source = readFileSync(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
const handler = source.match(/\.invoke_handler\(tauri::generate_handler!\[([\s\S]*?)\]\)/)?.[1];

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
});
