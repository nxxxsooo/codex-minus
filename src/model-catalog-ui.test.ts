import assert from "node:assert";
import { describe, it } from "node:test";

import {
  catalogActionRequiredLabel,
  customDisplayNameFollowsSlug,
  addCatalogCandidate,
  adoptionPreviewSummary,
  catalogCandidateSlugs,
  catalogModeForOverlay,
  catalogOverlayIsEmpty,
  catalogRestoreLosses,
  officialModelIsVisible,
  officialVisibilityOverride,
  restoreCatalogList,
  appModelLabel,
  catalogDiffSummary,
  catalogModeChangeDecision,
  catalogModeDraftController,
  catalogModePresentation,
  catalogRefreshGate,
  catalogRestartGuidance,
  defaultCatalogMode,
  externalVersionRequiresAcceptance,
  managedContextConflictKeys,
  providerManagedContextConflictKeys,
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
    assert.deepEqual(
      providerManagedContextConflictKeys(
        { configContents: 'model = "m"\n', contextWindow: "272000", autoCompactLimit: "" },
        "model_auto_compact_token_limit = 240000\n",
      ),
      ["model_auto_compact_token_limit", "model_context_window"],
    );
    assert.equal(externalVersionRequiresAcceptance("mismatch"), true);
    assert.equal(externalVersionRequiresAcceptance("match"), false);
    assert.equal(externalVersionRequiresAcceptance("unknown"), false);
  });
});

describe("a context override cannot be typed into a mode that ignores it", () => {
  const override = (patch: Record<string, unknown>) => ({
    displayName: null,
    visible: null,
    contextWindow: null,
    effectiveContextWindowPercent: null,
    order: null,
    supportedReasoningLevels: null,
    defaultReasoningLevel: null,
    supportedTools: null,
    toolCapabilities: null,
    ...patch,
  });

  it("reads an all-null override as asking for nothing", () => {
    assert.equal(catalogOverlayIsEmpty(emptyOverlay()), true);
    assert.equal(catalogOverlayIsEmpty({ official: { "gpt-5.6-sol": override({}) }, custom: [] }), true);
    assert.equal(
      catalogOverlayIsEmpty({ official: { "gpt-5.6-sol": override({ contextWindow: 372000 }) }, custom: [] }),
      false,
    );
    assert.equal(catalogOverlayIsEmpty(addCatalogCandidate(emptyOverlay(), "custom-a")), false);
  });

  it("turns a native profile into a managed one the moment an override exists", () => {
    const asked = { official: { "gpt-5.6-sol": override({ contextWindow: 372000 }) }, custom: [] };
    // Native mode generates no catalog, so the number would have been stored and ignored.
    assert.equal(catalogModeForOverlay("native-official", asked), "official-plus-custom");
    assert.equal(catalogModeForOverlay("native-official", emptyOverlay()), "native-official");
  });

  it("never changes a mode the user already owns", () => {
    const asked = { official: { "gpt-5.6-sol": override({ contextWindow: 372000 }) }, custom: [] };
    for (const mode of ["official-plus-custom", "custom-only", "external"] as const) {
      assert.equal(catalogModeForOverlay(mode, asked), mode);
      assert.equal(catalogModeForOverlay(mode, emptyOverlay()), mode);
    }
  });
});

describe("the model table shows the list Codex will show", () => {
  const override = (patch: Record<string, unknown>) => ({
    displayName: null,
    visible: null,
    contextWindow: null,
    effectiveContextWindowPercent: null,
    order: null,
    supportedReasoningLevels: null,
    defaultReasoningLevel: null,
    supportedTools: null,
    toolCapabilities: null,
    ...patch,
  });
  // Mirrors the bundled baseline: the retired 5.4 pair is carried but hidden.
  const officialModels = [
    { slug: "gpt-5.6-sol", visible: true },
    { slug: "gpt-5.6-terra", visible: true },
    { slug: "gpt-5.6-luna", visible: true },
    { slug: "gpt-5.5", visible: true },
    { slug: "gpt-5.4", visible: false },
    { slug: "gpt-5.4-mini", visible: false },
    { slug: "gpt-5.3-codex-spark", visible: true },
  ];
  const pro = ["gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.6-sol", "gpt-5.5", "gpt-5.3-codex-spark"];
  const visibleList = (overlay: CatalogOverlayDraft) => [
    ...officialModels.filter((model) => officialModelIsVisible(overlay, model)).map((model) => model.slug),
    ...overlay.custom.map((model) => model.slug),
  ];

  it("takes the baseline's answer until the user gives one", () => {
    // Codex hides the retired pair, so a table that listed every official entry offered two models
    // the picker does not have.
    assert.equal(officialModelIsVisible(emptyOverlay(), { slug: "gpt-5.4", visible: false }), false);
    assert.equal(officialModelIsVisible(emptyOverlay(), { slug: "gpt-5.3-codex-spark", visible: true }), true);
    const shown = { official: { "gpt-5.4": override({ visible: true }) }, custom: [] };
    assert.equal(officialModelIsVisible(shown, { slug: "gpt-5.4", visible: false }), true);
  });

  it("records a deletion as its difference from the baseline", () => {
    // Writing `false` for a model already hidden would leave an overlay that asks for nothing and
    // still promotes a native profile to a generated catalog.
    assert.equal(officialVisibilityOverride(true, false), false);
    assert.equal(officialVisibilityOverride(false, false), null);
    assert.equal(officialVisibilityOverride(false, true), true);
    assert.equal(officialVisibilityOverride(true, true), null);
  });

  it("turns a deletion into the managed catalog that can express it", () => {
    const deleted = { official: { "gpt-5.3-codex-spark": override({ visible: false }) }, custom: [] };
    assert.equal(catalogModeForOverlay("native-official", deleted), "official-plus-custom");
  });

  it("offers a deleted model back and never offers one the table already shows", () => {
    const deleted = { official: { "gpt-5.3-codex-spark": override({ visible: false }) }, custom: [] };
    const candidates = catalogCandidateSlugs({
      overlay: deleted,
      officialModels,
      // The provider reports slugs that collide with official rows; adding one as a custom model
      // would put two rows with the same slug into one generated catalog.
      providerCandidates: ["gpt-5.6-sol", "gpt-5.3-codex-spark", "gpt-4o-audio-preview"],
    });
    assert.ok(candidates.includes("gpt-5.3-codex-spark"), "a deleted model can be added back");
    assert.ok(candidates.includes("gpt-5.4"), "a model the baseline hides can be added");
    assert.ok(!candidates.includes("gpt-5.6-sol"), "a visible official row is not offered again");
    assert.deepEqual(
      candidates.filter((slug) => slug === "gpt-5.3-codex-spark"),
      ["gpt-5.3-codex-spark"],
      "each candidate is offered once",
    );
  });

  // Today's baseline lists exactly the Pro set, so a listed-but-unwanted official row — the case
  // the restore must hide — needs a hypothetical future baseline entry to exist at all.
  const officialModelsWithExtra = [...officialModels, { slug: "gpt-future-extra", visible: true }];

  it("restores the Pro list without forgetting the context windows already typed", () => {
    const before: CatalogOverlayDraft = {
      official: { "gpt-5.6-sol": override({ contextWindow: 372000 }) },
      custom: addCatalogCandidate(emptyOverlay(), "some-experiment").custom,
    };
    const after = restoreCatalogList({ overlay: before, officialModels: officialModelsWithExtra, wanted: pro });
    assert.deepEqual(visibleList(after).sort(), [...pro].sort());
    assert.equal(after.official["gpt-5.6-sol"].contextWindow, 372000);
    assert.equal(after.official["gpt-5.6-sol"].visible, null, "a Pro model keeps the baseline answer");
    assert.equal(after.official["gpt-future-extra"].visible, false);
    assert.equal(after.official["gpt-5.4"], undefined, "a model already hidden needs no override");
  });

  it("names every row the restore would take away", () => {
    const before: CatalogOverlayDraft = {
      official: {},
      custom: addCatalogCandidate(emptyOverlay(), "some-experiment").custom,
    };
    assert.deepEqual(
      catalogRestoreLosses({ overlay: before, officialModels: officialModelsWithExtra, wanted: pro }).sort(),
      ["gpt-future-extra", "some-experiment"],
    );
    const restored = restoreCatalogList({ overlay: before, officialModels: officialModelsWithExtra, wanted: pro });
    assert.deepEqual(catalogRestoreLosses({ overlay: restored, officialModels: officialModelsWithExtra, wanted: pro }), []);
  });

  it("refuses a startup model the user deleted, and a list with nothing left in it", () => {
    const deleted = { official: { "gpt-5.3-codex-spark": override({ visible: false }) }, custom: [] };
    const slugs = officialModels.filter((model) => model.visible).map((model) => model.slug);
    assert.equal(validateCatalogDraft(deleted, "official-plus-custom", "gpt-5.3-codex-spark", slugs), "invalid-default-model");
    assert.equal(validateCatalogDraft(deleted, "official-plus-custom", "gpt-5.5", slugs), null);
    // The generator refuses a catalog with no visible model; the editor says so before the save.
    assert.equal(validateCatalogDraft(emptyOverlay(), "official-plus-custom", "", []), "empty-catalog");
    assert.equal(validateCatalogDraft(emptyOverlay(), "custom-only", "", slugs), "empty-catalog");
    assert.equal(validateCatalogDraft(emptyOverlay(), "native-official", "", []), null);
  });
});

describe("known relay model cards in the editor", () => {
  it("offers every preset that is not already a row, beside provider candidates", () => {
    const candidates = catalogCandidateSlugs({
      overlay: emptyOverlay(),
      officialModels: [],
      providerCandidates: [],
    });
    for (const slug of ["claude-fable-5", "claude-opus-5", "claude-sonnet-5", "claude-haiku-4-5-20251001"]) {
      assert.ok(candidates.includes(slug), `${slug} preset is offered`);
    }
  });

  it("stops offering a preset once its slug is a row, and dedups a provider report of it", () => {
    const withFable = addCatalogCandidate(emptyOverlay(), "claude-fable-5");
    const candidates = catalogCandidateSlugs({
      overlay: withFable,
      officialModels: [],
      providerCandidates: ["claude-fable-5", "claude-opus-5"],
    });
    assert.ok(!candidates.includes("claude-fable-5"), "an added preset is not offered again");
    assert.deepEqual(
      candidates.filter((slug) => slug === "claude-opus-5"),
      ["claude-opus-5"],
      "a provider report of a preset is one chip, not two",
    );
  });

  it("adds a preset as its complete card, not template defaults", () => {
    const overlay = addCatalogCandidate(emptyOverlay(), "claude-fable-5");
    assert.deepEqual(overlay.custom, [{
      slug: "claude-fable-5",
      displayName: "Fable 5",
      description: "Creative-writing model via the Sub2API Responses bridge.",
      contextWindow: 1_000_000,
      effectiveContextWindowPercent: 95,
      visible: true,
      order: 0,
      supportedReasoningLevels: [
        { effort: "low", description: "Fast responses with lighter reasoning" },
        { effort: "medium", description: "Balances speed and reasoning depth for everyday tasks" },
        { effort: "high", description: "Greater reasoning depth for complex problems" },
        { effort: "xhigh", description: "Extra high reasoning depth for complex problems" },
      ],
      defaultReasoningLevel: "medium",
      supportedTools: [],
      toolCapabilities: null,
      templateProvenance: "known-relay-model",
    }]);
  });

  it("keeps the plain default shape for a slug no card knows", () => {
    const overlay = addCatalogCandidate(emptyOverlay(), "some-provider-model");
    assert.equal(overlay.custom[0].displayName, "some-provider-model");
    assert.equal(overlay.custom[0].contextWindow, 272000);
    assert.equal(overlay.custom[0].templateProvenance, "provider-candidate");
    assert.deepEqual(overlay.custom[0].supportedReasoningLevels, []);
  });

  it("lets a display name follow the slug only until it is edited independently", () => {
    // A hand-typed row: the name mirrors the slug while untouched, so typing the slug names it.
    assert.equal(customDisplayNameFollowsSlug("", ""), true);
    assert.equal(customDisplayNameFollowsSlug("claude-fab", "claude-fab"), true);
    // A card's name, or one the user edited, must survive a slug correction.
    assert.equal(customDisplayNameFollowsSlug("Fable 5", "claude-fable-5"), false);
  });
});

describe("the readiness sentinel reaches the screen as a sentence", () => {
  it("maps the persisted code and passes real sentences through", () => {
    // `catalog-readiness-unavailable` is a stable persisted sentinel; rendering it raw put a bare
    // code on screen. Every other actionRequired value the backend writes is already a sentence.
    assert.notEqual(catalogActionRequiredLabel("catalog-readiness-unavailable"), "catalog-readiness-unavailable");
    assert.match(catalogActionRequiredLabel("catalog-readiness-unavailable"), /。$/);
    assert.equal(catalogActionRequiredLabel("该供应商依赖未提供的代理能力。"), "该供应商依赖未提供的代理能力。");
  });
});

describe("restart guidance", () => {
  it("says nothing until a committed generation requires a restart", () => {
    assert.deepEqual(catalogRestartGuidance(false), []);
  });

  it("states host relaunch, the new-task requirement, and the limits of the marker", () => {
    const guidance = catalogRestartGuidance(true);
    assert.equal(guidance.length, 4);
    assert.ok(guidance.every((line) => line.trim().length > 0));
    assert.ok(guidance.some((line) => line.includes("完整退出")));
    assert.ok(guidance.some((line) => line.includes("新建") && line.includes("任务")));
    assert.ok(guidance.some((line) => line.includes("不会")));
    assert.ok(guidance.some((line) => line.includes("未知")));
    assert.equal(new Set(guidance).size, guidance.length);
  });
})

describe("model labels match the Codex picker", () => {
  it("renders the stored catalog name the way the app shows it", () => {
    // Observed in the official client's picker against the same catalog entries.
    for (const [stored, shown] of [
      ["GPT-5.6-Sol", "5.6 Sol"],
      ["GPT-5.6-Terra", "5.6 Terra"],
      ["GPT-5.6-Luna", "5.6 Luna"],
      ["GPT-5.5", "5.5"],
      ["GPT-5.4", "5.4"],
      ["GPT-5.4-Mini", "5.4 Mini"],
      ["GPT-5.3-Codex-Spark", "5.3 Codex Spark"],
    ] as const) {
      assert.equal(appModelLabel(stored), shown);
    }
  });

  it("leaves a name it does not recognize alone", () => {
    assert.equal(appModelLabel("Codex Auto Review"), "Codex Auto Review");
    assert.equal(appModelLabel("deepseek-v4-pro"), "deepseek v4 pro");
    assert.equal(appModelLabel(""), "");
  });
});
