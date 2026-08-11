import assert from "node:assert";
import { describe, it } from "node:test";

import {
  addCatalogCandidate,
  adoptionPreviewSummary,
  catalogDiffSummary,
  catalogModeChangeDecision,
  catalogModeDraftController,
  catalogModePresentation,
  catalogRefreshGate,
  defaultCatalogMode,
  externalVersionRequiresAcceptance,
  managedContextConflictKeys,
  profileCatalogFlags,
  providerEvidenceState,
  validateCatalogDraft,
  type CatalogModeValue,
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

  it("requires confirmation before native mode abandons custom or external ownership", () => {
    assert.equal(catalogModeChangeDecision("official-plus-custom", "native-official", null, 7), "confirm-discard-custom");
    assert.equal(catalogModeChangeDecision("custom-only", "native-official", null, 1), "confirm-discard-custom");
    assert.equal(catalogModeChangeDecision("external", "native-official", "models_372k.json", 0), "confirm-discard-external");
    assert.equal(catalogModeChangeDecision("official-plus-custom", "custom-only", null, 7), "select");
    assert.equal(catalogModeChangeDecision("native-official", "native-official", null, 7), "select");
  });

  it("native presentation hides stale managed state and reports dormant custom models", () => {
    assert.deepEqual(catalogModePresentation({
      selectedMode: "native-official",
      persistedMode: "native-official",
      generatedPath: "model-catalogs/stale.json",
      externalPointer: null,
      restartRequired: true,
      customModelCount: 7,
    }), {
      source: "native",
      pendingSource: null,
      path: null,
      restart: false,
      dormantCustomCount: 7,
      pendingDormantCustomCount: 0,
      pathUnavailable: null,
    });
  });

  it("keeps a native draft unsaved until its managed or external catalog is persisted", () => {
    for (const persistedMode of ["official-plus-custom", "external"] as const) {
      assert.deepEqual(catalogModePresentation({
        selectedMode: "native-official",
        persistedMode,
        generatedPath: "model-catalogs/current.json",
        externalPointer: "models/external.json",
        restartRequired: true,
        customModelCount: 7,
      }), {
        source: "unsaved",
        pendingSource: "native",
        path: null,
        restart: false,
        dormantCustomCount: 0,
        pendingDormantCustomCount: 7,
        pathUnavailable: null,
      });
    }
  });

  it("describes every unsaved catalog draft by the mode that Save will activate", () => {
    const cases = [
      ["native-official", "native"],
      ["official-plus-custom", "managed"],
      ["custom-only", "managed"],
      ["external", "external"],
    ] as const;
    for (const [selectedMode, expectedPendingSource] of cases) {
      const presentation = catalogModePresentation({
        selectedMode,
        persistedMode: selectedMode === "native-official" ? "official-plus-custom" : "native-official",
        generatedPath: "model-catalogs/current.json",
        externalPointer: "models/external.json",
        restartRequired: true,
        customModelCount: 2,
      });
      assert.equal(
        (presentation as typeof presentation & { pendingSource?: string }).pendingSource,
        expectedPendingSource,
      );
    }
  });

  it("managed presentation exposes only the matching persisted generation", () => {
    assert.deepEqual(catalogModePresentation({
      selectedMode: "official-plus-custom",
      persistedMode: "official-plus-custom",
      generatedPath: "model-catalogs/current.json",
      externalPointer: null,
      restartRequired: true,
      customModelCount: 7,
    }), {
      source: "managed",
      pendingSource: null,
      path: "model-catalogs/current.json",
      restart: true,
      dormantCustomCount: 0,
      pendingDormantCustomCount: 0,
      pathUnavailable: null,
    });
    assert.equal(catalogModePresentation({
      selectedMode: "custom-only",
      persistedMode: "official-plus-custom",
      generatedPath: "model-catalogs/current.json",
      externalPointer: null,
      restartRequired: true,
      customModelCount: 7,
    }).source, "unsaved");
  });

  it("marks persisted catalog modes with missing paths for explicit UI copy", () => {
    assert.equal(catalogModePresentation({
      selectedMode: "official-plus-custom",
      persistedMode: "official-plus-custom",
      generatedPath: null,
      externalPointer: null,
      restartRequired: false,
      customModelCount: 0,
    }).pathUnavailable, "managed");
    assert.equal(catalogModePresentation({
      selectedMode: "external",
      persistedMode: "external",
      generatedPath: null,
      externalPointer: null,
      restartRequired: false,
      customModelCount: 0,
    }).pathUnavailable, "external");
  });

  it("keeps cancel, confirm, and restore catalog controls draft-only", () => {
    let selectedMode: CatalogModeValue = "official-plus-custom";
    let modeExplicit = false;
    const updateDraftMode = (nextMode: CatalogModeValue) => {
      selectedMode = nextMode;
      modeExplicit = true;
    };
    const cancelled = catalogModeDraftController({
      currentMode: selectedMode,
      externalPointer: null,
      customModelCount: 7,
      confirmDiscard: () => false,
      actions: { updateDraftMode },
    });
    assert.equal(cancelled.requestMode("native-official"), false);
    assert.equal(selectedMode, "official-plus-custom");
    assert.equal(modeExplicit, false);

    const confirmed = catalogModeDraftController({
      currentMode: selectedMode,
      externalPointer: null,
      customModelCount: 7,
      confirmDiscard: () => true,
      actions: { updateDraftMode },
    });
    assert.equal(confirmed.requestMode("native-official"), true);
    assert.equal(selectedMode, "native-official");
    assert.equal(modeExplicit, true);

    confirmed.restoreOfficialPlusCustom();
    assert.equal(selectedMode, "official-plus-custom");
    assert.equal(modeExplicit, true);
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
