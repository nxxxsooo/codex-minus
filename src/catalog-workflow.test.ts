import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";
import { catalogProfileDraft } from "./catalog-profile-draft.ts";
import { addCatalogCandidate, catalogCandidateSlugs, catalogCustomRows, catalogModelDisplayName, catalogOfficialRows, catalogOverlayForMode, emptyOfficialOverride, restoreCatalogList, validateCatalogDraft, type CatalogOverlayDraft } from "./model-catalog-ui.ts";
import { longContextState, setLongContext } from "./catalog-long-context.ts";
import { PRO_MODEL_SLUGS } from "./provider-onboarding.ts";

const asset = JSON.parse(readFileSync(new URL("../src-tauri/assets/official-model-catalog.json", import.meta.url), "utf8"));
const officialModels = (asset.models as Array<{ slug: string; display_name: string; visibility: string; context_window: number }>).map((row) => ({ slug: row.slug, displayName: row.display_name, visible: row.visibility === "list", contextWindow: row.context_window }));
const empty = (): CatalogOverlayDraft => ({ official: {}, custom: [] });
const visible = (overlay: CatalogOverlayDraft, mode: "official-plus-custom" | "custom-only") => [
  ...catalogOfficialRows(overlay, officialModels, mode),
  ...catalogCustomRows(overlay, officialModels, mode).map(({ model }) => model),
];

describe("the current catalog reaches both provider modes with real display names", () => {
  it("seeds only brand-new drafts with the current preset", () => {
    for (const mode of ["official-plus-custom", "custom-only"] as const) {
      const draft = catalogProfileDraft({ profileId: "new", fallbackMode: mode, summary: null, officialModels });
      assert.deepEqual(visible(draft.overlay, mode).map((row) => row.slug).sort(), [...PRO_MODEL_SLUGS].sort());
      assert.ok(visible(draft.overlay, mode).every((row) => row.displayName !== row.slug));
      assert.equal(validateCatalogDraft(draft.overlay, mode, "gpt-5.6-terra", catalogOfficialRows(draft.overlay, officialModels, mode).map((row) => row.slug)), null);
    }
    const overlay = addCatalogCandidate(empty(), "gpt-5.6-sol");
    overlay.custom[0].displayName = "My Sol";
    const existing = catalogProfileDraft({ profileId: "existing", fallbackMode: "custom-only", officialModels,
      summary: { profileId: "existing", mode: "custom-only", modeExplicit: true, upstreamTopology: "direct", externalPointer: null, overlay } });
    assert.strictEqual(existing.overlay, overlay, "opening a saved profile does not migrate its chosen models or names");
  });

  it("converts a new draft to pure API without losing its selected official model", () => {
    const mixed = catalogProfileDraft({ profileId: "new", fallbackMode: "official-plus-custom", summary: null, officialModels });
    mixed.overlay.official["gpt-6-astra"] = { ...emptyOfficialOverride(), contextWindow: 500000 };
    const pure = catalogOverlayForMode(mixed.overlay, mixed.mode, "custom-only", officialModels);
    assert.deepEqual(visible(pure, "custom-only").map((row) => row.slug).sort(), [...PRO_MODEL_SLUGS].sort());
    assert.equal(pure.custom.find((row) => row.slug === "gpt-6-astra")?.contextWindow, 500000);
    assert.equal(pure.custom.find((row) => row.slug === "gpt-5.6-terra")?.displayName, "GPT-5.6-Terra");
    assert.equal(validateCatalogDraft(pure, "custom-only", "gpt-5.6-terra", []), null);
  });

  it("restores official capabilities after a new provider returns from pure API to mixed mode", () => {
    const terra = asset.models.find((row: { slug: string }) => row.slug === "gpt-5.6-terra");
    assert.equal(terra.supported_reasoning_levels.length, 6, "the fixture must expose the capability at risk");
    const mixed = catalogProfileDraft({ profileId: "new", fallbackMode: "official-plus-custom", summary: null, officialModels });
    const pure = catalogOverlayForMode(mixed.overlay, "official-plus-custom", "custom-only", officialModels);
    assert.ok(pure.custom.some((row) => row.slug === "gpt-5.6-terra" && row.supportedReasoningLevels.length === 0));
    const back = catalogOverlayForMode(pure, "custom-only", "official-plus-custom", officialModels);
    assert.ok(!back.custom.some((row) => row.slug === "gpt-5.6-terra"), "generated custom shadows cannot strip official reasoning levels");
    assert.deepEqual(visible(back, "official-plus-custom").map((row) => row.slug).sort(), [...PRO_MODEL_SLUGS].sort());
    assert.ok(back.custom.some((row) => row.slug === "gpt-6-sol"), "supplemental models without a baseline remain custom");

    const changed = catalogOverlayForMode({ ...pure, custom: pure.custom.map((row) => row.slug === "gpt-6-astra"
      ? { ...row, displayName: "My Astra", contextWindow: 500_000 } : row) }, "custom-only", "official-plus-custom", officialModels);
    assert.equal(changed.official["gpt-6-astra"]?.displayName, "My Astra", "an edited transitional row retains its owned fields");
    assert.equal(changed.official["gpt-6-astra"]?.contextWindow, 500_000);
    assert.equal(changed.official["gpt-6-astra"]?.supportedReasoningLevels, null);

    const effortEdit = { ...pure, custom: pure.custom.map(row => row.slug === "gpt-5.6-terra"
      ? { ...row, supportedReasoningLevels: [{ effort: "high", description: "Chosen by user" }] } : row) };
    const withEffort = catalogOverlayForMode(effortEdit, "custom-only", "official-plus-custom", officialModels);
    assert.deepEqual(withEffort.official["gpt-5.6-terra"]?.supportedReasoningLevels, [{ effort: "high", description: "Chosen by user" }]);
  });

  it("carries edits to a transitional official row as field-scoped overrides", () => {
    const mixed = catalogProfileDraft({ profileId: "new", fallbackMode: "official-plus-custom", summary: null, officialModels });
    const pure = catalogOverlayForMode(mixed.overlay, "official-plus-custom", "custom-only", officialModels);
    const edited = { ...pure, custom: pure.custom.map(row => row.slug === "gpt-5.6-terra"
      ? { ...row, displayName: "My Terra", contextWindow: 420_000 } : row) };
    const back = catalogOverlayForMode(edited, "custom-only", "official-plus-custom", officialModels);
    assert.equal(back.custom.some(row => row.slug === "gpt-5.6-terra"), false);
    assert.equal(back.official["gpt-5.6-terra"]?.displayName, "My Terra");
    assert.equal(back.official["gpt-5.6-terra"]?.contextWindow, 420_000);
    assert.equal(back.official["gpt-5.6-terra"]?.supportedReasoningLevels, null, "other official capabilities remain inherited");

    const generated = addCatalogCandidate(mixed.overlay, "gpt-5.6-terra", officialModels);
    const userOwned = { ...generated, custom: generated.custom.map(row => row.slug === "gpt-5.6-terra"
      ? { ...row, displayName: "User-owned Terra", templateProvenance: "user-created" } : row) };
    const roundTrip = catalogOverlayForMode(userOwned, "official-plus-custom", "custom-only", officialModels);
    assert.equal(roundTrip.custom.find(row => row.slug === "gpt-5.6-terra")?.displayName, "User-owned Terra");
    assert.equal(catalogOverlayForMode(roundTrip, "custom-only", "official-plus-custom", officialModels).custom.find(row => row.slug === "gpt-5.6-terra")?.displayName, "User-owned Terra");
  });

  it("does not remove an official-slug candidate the user added explicitly in custom-only mode", () => {
    const chosen = addCatalogCandidate(empty(), "gpt-5.6-terra", officialModels);
    assert.equal(chosen.custom[0].templateProvenance, "provider-candidate");
    const mixed = catalogOverlayForMode(chosen, "custom-only", "official-plus-custom", officialModels);
    assert.equal(mixed.custom[0].slug, "gpt-5.6-terra");
    assert.equal(mixed.custom[0].templateProvenance, "provider-candidate");
  });

  it("keeps a pre-existing candidate when mixed is switched to pure API and back", () => {
    const mixed = catalogProfileDraft({ profileId: "new", fallbackMode: "official-plus-custom", summary: null, officialModels });
    const existing = addCatalogCandidate(mixed.overlay, "gpt-5.6-terra", officialModels);
    const row = existing.custom.find(model => model.slug === "gpt-5.6-terra")!;
    const chosen = { ...existing, custom: existing.custom.map(model => model === row
      ? { ...row, displayName: "My own Terra", contextWindow: 390_000 } : model) };
    const pure = catalogOverlayForMode(chosen, "official-plus-custom", "custom-only", officialModels);
    const back = catalogOverlayForMode(pure, "custom-only", "official-plus-custom", officialModels);
    const retained = back.custom.find(model => model.slug === "gpt-5.6-terra");
    assert.equal(retained?.templateProvenance, "provider-candidate");
    assert.equal(retained?.displayName, "My own Terra");
    assert.equal(retained?.contextWindow, 390_000);
  });

  it("restores the user's original official override after a mixed-to-pure-to-mixed draft round trip", () => {
    const mixed = catalogProfileDraft({ profileId: "new", fallbackMode: "official-plus-custom", summary: null, officialModels });
    mixed.overlay.official["gpt-5.6-terra"] = { ...emptyOfficialOverride(), displayName: "Chosen Terra", contextWindow: 480_000 };
    const pure = catalogOverlayForMode(mixed.overlay, "official-plus-custom", "custom-only", officialModels);
    const back = catalogOverlayForMode(pure, "custom-only", "official-plus-custom", officialModels);
    assert.equal(back.official["gpt-5.6-terra"].displayName, "Chosen Terra");
    assert.equal(back.official["gpt-5.6-terra"].contextWindow, 480_000);
    assert.equal(back.custom.some(row => row.slug === "gpt-5.6-terra"), false);
  });

  it("never rewrites a persisted custom-only overlay on an existing provider", () => {
    const saved = addCatalogCandidate(empty(), "gpt-5.6-terra", officialModels);
    assert.strictEqual(catalogOverlayForMode(saved, "custom-only", "official-plus-custom", officialModels, false), saved);
  });

  it("repairs ID-mirroring names only on explicit restore and preserves meaningful aliases/windows", () => {
    let overlay = addCatalogCandidate(empty(), "gpt-5.6-terra");
    overlay = addCatalogCandidate(overlay, "gpt-6-astra");
    overlay = addCatalogCandidate(overlay, "gpt-5.5");
    overlay.custom[1].displayName = "My Astra";
    overlay.custom[1].contextWindow = 500000;
    const restored = restoreCatalogList({ overlay, officialModels, wanted: PRO_MODEL_SLUGS, mode: "custom-only" });
    assert.equal(overlay.custom[0].displayName, "gpt-5.6-terra");
    assert.equal(restored.custom.find((row) => row.slug === "gpt-5.6-terra")?.displayName, "GPT-5.6-Terra");
    assert.equal(restored.custom.find((row) => row.slug === "gpt-6-astra")?.displayName, "My Astra");
    assert.equal(restored.custom.find((row) => row.slug === "gpt-6-astra")?.contextWindow, 500000);
    assert.deepEqual(restored.custom.map((row) => row.slug).sort(), [...PRO_MODEL_SLUGS].sort());
  });

  it("prefills new Sol/Luna from sourced cards and keeps their model IDs exact", () => {
    const sol = addCatalogCandidate(empty(), "gpt-6-sol", officialModels).custom[0];
    assert.equal(sol.displayName, "GPT-6 Sol");
    assert.equal(sol.templateProvenance, "models-dev-openai-2026-09-22");
    assert.ok(sol.supportedReasoningLevels.some((level) => level.effort === "max"));
    assert.equal(catalogModelDisplayName("gpt-6-luna", officialModels), "GPT-6 Luna");
    assert.equal(catalogModelDisplayName("provider-exact-id", officialModels), "provider-exact-id");
    assert.equal(addCatalogCandidate(empty(), "gpt-5.3-codex-spark", officialModels).custom[0].contextWindow, 128000);
  });

  it("merges a same-slug custom record once with official override precedence", () => {
    const overlay = addCatalogCandidate(empty(), "gpt-5.6-terra");
    overlay.custom[0].displayName = "My Terra";
    overlay.official["gpt-5.6-terra"] = { ...emptyOfficialOverride(), contextWindow: 500000 };
    const rows = visible(overlay, "official-plus-custom").filter((row) => row.slug === "gpt-5.6-terra");
    assert.equal(rows.length, 1);
    assert.equal(rows[0].displayName, "My Terra");
    assert.equal(rows[0].contextWindow, 500000);
    overlay.official["gpt-5.6-terra"].visible = false;
    assert.ok(!visible(overlay, "official-plus-custom").some((row) => row.slug === "gpt-5.6-terra"));
    assert.ok(catalogCandidateSlugs({ overlay, officialModels, providerCandidates: [] }).includes("gpt-5.6-terra"));
  });

  it("restores hidden custom models without duplicating or forgetting their metadata", () => {
    const overlay = addCatalogCandidate(empty(), "my-model");
    overlay.custom[0].visible = false;
    overlay.custom[0].contextWindow = 400000;
    assert.equal(visible(overlay, "custom-only").length, 0);
    const restored = addCatalogCandidate(overlay, "my-model");
    assert.equal(restored.custom.length, 1);
    assert.equal(restored.custom[0].visible, true);
    assert.equal(restored.custom[0].contextWindow, 400000);
  });

  it("does not invent official rows before the baseline arrives or change external ownership", () => {
    const overlay = empty();
    assert.strictEqual(restoreCatalogList({ overlay, officialModels: [], wanted: PRO_MODEL_SLUGS }), overlay);
    assert.strictEqual(restoreCatalogList({ overlay, officialModels, wanted: PRO_MODEL_SLUGS, mode: "external" }), overlay);
  });
});

describe("long context follows the effective rows across generations", () => {
  it("toggles current and retained 5.6 models, leaving unrelated or independently edited windows alone", () => {
    for (const mode of ["official-plus-custom", "custom-only"] as const) {
      const restored = restoreCatalogList({ overlay: empty(), officialModels, wanted: PRO_MODEL_SLUGS, mode });
      const overlay = addCatalogCandidate(restored, "claude-fable-5");
      const enabled = setLongContext({ overlay, officialModels, mode }, true);
      assert.ok(visible(enabled, mode).filter((row) => row.slug.startsWith("gpt-")).every((row) => row.contextWindow === 1050000));
      assert.equal(enabled.custom.find((row) => row.slug === "claude-fable-5")?.contextWindow, 1000000);
      assert.equal(longContextState({ overlay: enabled, officialModels, mode }).checked, true);
      const disabled = setLongContext({ overlay: enabled, officialModels, mode }, false);
      assert.deepEqual(disabled, overlay);
    }
    const legacy = addCatalogCandidate(empty(), "gpt-5.6-sol");
    assert.equal(setLongContext({ overlay: legacy, officialModels, mode: "custom-only" }, true).custom[0].contextWindow, 1050000);
    assert.strictEqual(setLongContext({ overlay: legacy, officialModels, mode: "external" }, true), legacy);
  });
});
