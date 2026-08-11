import assert from "node:assert";
import { describe, it } from "node:test";

import {
  addCatalogCandidate,
  adoptionPreviewSummary,
  catalogDiffSummary,
  catalogRefreshGate,
  defaultCatalogMode,
  externalVersionRequiresAcceptance,
  managedContextConflictKeys,
  profileCatalogFlags,
  providerEvidenceState,
  validateCatalogDraft,
  type CatalogOverlayDraft,
} from "./model-catalog-ui.ts";

const emptyOverlay = (): CatalogOverlayDraft => ({ official: {}, custom: [] });

describe("model catalog UI state", () => {
  it("assigns mode defaults and preserves external ownership", () => {
    assert.equal(defaultCatalogMode("official", false), "native-official");
    assert.equal(defaultCatalogMode("official", true), "official-plus-custom");
    assert.equal(defaultCatalogMode("pureApi", false), "custom-only");
    assert.equal(defaultCatalogMode("pureApi", false, null, "server-side-composite"), "official-plus-custom");
    assert.equal(defaultCatalogMode("pureApi", false, "models/custom.json"), "external");
  });

  it("gates refresh on target capability, credentials, and loading", () => {
    assert.deepEqual(catalogRefreshGate({ refreshAvailable: true, credentialAction: null, loading: false }), { disabled: false, reason: null });
    assert.equal(catalogRefreshGate({ refreshAvailable: true, credentialAction: "sign-in", loading: false }).reason, "sign-in");
    assert.equal(catalogRefreshGate({ refreshAvailable: false, credentialAction: null, loading: false }).reason, "target-unavailable");
    assert.equal(catalogRefreshGate({ refreshAvailable: true, credentialAction: null, loading: true }).reason, "loading");
  });

  it("keeps provider evidence non-authoritative and imports candidates explicitly", () => {
    assert.equal(providerEvidenceState("official-a", ["official-a"]), "reported");
    assert.equal(providerEvidenceState("official-b", ["official-a"]), "not-reported");
    const imported = addCatalogCandidate(emptyOverlay(), "provider-only");
    assert.equal(imported.custom[0].slug, "provider-only");
    assert.equal(imported.custom[0].effectiveContextWindowPercent, 100);
    assert.strictEqual(addCatalogCandidate(imported, "provider-only"), imported);
  });

  it("validates overlays and default-model impact", () => {
    const overlay = addCatalogCandidate(emptyOverlay(), "custom-a");
    assert.equal(validateCatalogDraft(overlay, "custom-only", "custom-a", []), null);
    assert.equal(validateCatalogDraft(overlay, "custom-only", "missing", []), "invalid-default-model");
    const duplicate = { ...overlay, custom: [...overlay.custom, { ...overlay.custom[0] }] };
    assert.equal(validateCatalogDraft(duplicate, "custom-only", "", []), "duplicate-custom-slug");
    const invalidReasoning = { ...overlay, custom: [{ ...overlay.custom[0], supportedReasoningLevels: [{ effort: "low", description: "Low" }], defaultReasoningLevel: "high" }] };
    assert.equal(validateCatalogDraft(invalidReasoning, "custom-only", "custom-a", []), "invalid-reasoning-default");
  });

  it("presents update diffs, adoption conflicts, partial failures, and restart state", () => {
    assert.equal(catalogDiffSummary({ added: ["a"], updated: ["b", "c"], removed: [], collisions: ["d"] }), "1/2/0/1");
    assert.deepEqual(adoptionPreviewSummary({ officialOverrideCount: 2, customModels: [1], collisions: [] }), { adoptable: true, summary: "2/1/0" });
    assert.equal(adoptionPreviewSummary({ officialOverrideCount: 0, customModels: [], collisions: ["x"] }).adoptable, false);
    assert.deepEqual(profileCatalogFlags({ restartRequired: true, actionRequired: "invalid default" }), { restart: true, partialFailure: true });
  });

  it("detects managed-context conflicts and requires explicit external mismatch acceptance", () => {
    assert.deepEqual(
      managedContextConflictKeys('model = "m"\nmodel_context_window = 372000\nmodel_auto_compact_token_limit = 330000\n'),
      ["model_context_window", "model_auto_compact_token_limit"],
    );
    assert.deepEqual(managedContextConflictKeys('model = "m"\n'), []);
    assert.equal(externalVersionRequiresAcceptance("mismatch"), true);
    assert.equal(externalVersionRequiresAcceptance("match"), false);
    assert.equal(externalVersionRequiresAcceptance("unknown"), false);
  });
});
