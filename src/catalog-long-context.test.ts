import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { longContextState, setLongContext } from "./catalog-long-context.ts";
import { addCatalogCandidate, emptyOfficialOverride, type CatalogOverlayDraft } from "./model-catalog-ui.ts";

const officialModels = ["gpt-6-astra", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.5", "gpt-5.3-codex-spark"]
  .map((slug) => ({ slug, visible: true, contextWindow: slug.endsWith("spark") ? 128000 : 272000 }));
const empty = (): CatalogOverlayDraft => ({ official: {}, custom: [] });
const mode = "official-plus-custom" as const;

describe("long context changes only the current provider's eligible visible models", () => {
  it("round trips the preset and preserves unrelated overrides and model capabilities", () => {
    const overlay = addCatalogCandidate(empty(), "claude-fable-5");
    overlay.official["gpt-6-astra"] = { ...emptyOfficialOverride(), displayName: "My Astra", order: 7 };
    overlay.official["gpt-5.5"] = { ...emptyOfficialOverride(), contextWindow: 400000 };
    const before = structuredClone(overlay);
    const enabled = setLongContext({ overlay, officialModels, mode }, true);
    assert.deepEqual(overlay, before, "the authoritative input is immutable");
    for (const slug of ["gpt-6-astra", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"]) {
      assert.equal(enabled.official[slug].contextWindow, 1050000);
    }
    assert.equal(enabled.official["gpt-5.5"].contextWindow, 400000);
    assert.equal(enabled.official["gpt-5.3-codex-spark"], undefined);
    assert.deepEqual(enabled.custom, before.custom);
    assert.deepEqual(longContextState({ overlay: enabled, officialModels, mode }), { available: true, checked: true, mixed: false });
    assert.deepEqual(setLongContext({ overlay: enabled, officialModels, mode }, false), before);
  });

  it("handles custom-only rows and same-slug official overrides without creating duplicates", () => {
    const overlay = addCatalogCandidate(empty(), "gpt-6-astra");
    overlay.official["gpt-6-astra"] = { ...emptyOfficialOverride(), contextWindow: 444000 };
    const enabled = setLongContext({ overlay, officialModels, mode }, true);
    assert.equal(enabled.custom.length, 1);
    assert.equal(enabled.custom[0].contextWindow, 1050000);
    assert.equal(enabled.official["gpt-6-astra"].contextWindow, 1050000);
    const pure = setLongContext({ overlay, officialModels, mode: "custom-only" }, true);
    assert.equal(pure.custom[0].contextWindow, 1050000);
    assert.deepEqual(pure.official, overlay.official, "dormant official overrides stay owned");
    const off = setLongContext({ overlay: pure, officialModels, mode: "custom-only" }, false);
    assert.equal(off.custom[0].contextWindow, 272000);
  });

  it("ignores hidden rows, external catalogs and post-toggle manual window edits", () => {
    const overlay = empty();
    overlay.official["gpt-5.6-luna"] = { ...emptyOfficialOverride(), visible: false, contextWindow: 128000 };
    assert.strictEqual(setLongContext({ overlay, officialModels, mode: "external" }, true), overlay);
    assert.deepEqual(longContextState({ overlay, officialModels, mode: "external" }), { available: false, checked: false, mixed: false });
    const enabled = setLongContext({ overlay, officialModels, mode }, true);
    assert.equal(enabled.official["gpt-5.6-luna"].contextWindow, 128000);
    enabled.official["gpt-6-astra"].contextWindow = 500000;
    assert.equal(longContextState({ overlay: enabled, officialModels, mode }).mixed, true);
    const off = setLongContext({ overlay: enabled, officialModels, mode }, false);
    assert.equal(off.official["gpt-6-astra"].contextWindow, 500000);
  });
});
