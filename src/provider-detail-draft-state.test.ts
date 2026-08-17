import assert from "node:assert";
import { describe, it } from "node:test";

import * as providerDetailStateApi from "./provider-detail-draft-state.ts";

import {
  applyProviderDetailInspection,
  beginProviderDetailInspection,
  beginProviderDetailEdit,
  beginProviderDetailNativePriorityUpgrade,
  buildProviderDetailCommitEffect,
  cancelProviderDetailTransition,
  confirmProviderDetailTransition,
  createProviderDetailDraftState,
  endProviderDetailSession,
  replaceProviderDetailCatalogDraft,
  replaceProviderDetailProfile,
  refreshProviderDetailCatalogDraftState,
  settleProviderDetailTransform,
  settleProviderDetailTransformError,
  type ProviderDetailStep,
} from "./provider-detail-draft-state.ts";

const existingTarget = { target: "preserveExisting", source: "existing" } as const;

const config = `model = "gpt-5.5"
model_provider = "RelayOne"

[model_providers.RelayOne]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "provider-key"
custom_provider_field = "keep-provider"
http_headers = { "x-openai-actor-authorization" = "local-image-extension", "x-unowned" = "keep-header" }
`;

function profile() {
  return {
    id: "relay-one",
    name: "Relay One",
    model: "gpt-5.5",
    baseUrl: "https://relay.example/v1",
    upstreamBaseUrl: "https://relay.example/v1",
    apiKey: "provider-key",
    protocol: "responses",
    relayMode: "official",
    officialMixApiKey: true,
    testModel: "gpt-5.5",
    configContents: config,
    authContents: "",
    useCommonConfig: true,
    contextSelection: { mcpServers: [], skills: [], plugins: [] },
    contextSelectionInitialized: true,
    contextWindow: "",
    autoCompactLimit: "",
    modelList: "gpt-5.5",
    modelWindows: "",
    userAgent: "",
  };
}

const catalogDraft = {
  profileId: "relay-one",
  mode: "official-plus-custom" as const,
  modeExplicit: true,
  upstreamTopology: "direct" as const,
  externalPointer: null,
  overlay: { official: {}, custom: [] },
};

function draftState(
  currentProfile = profile(),
  currentCatalog = catalogDraft,
) {
  return createProviderDetailDraftState({
    profile: currentProfile,
    catalogDraft: currentCatalog,
  });
}

function transformCorrelation(step: {
  effects: Array<{
    kind: string;
    correlation?: { sessionToken: symbol; profileId: string; revision: number };
  }>;
}) {
  const effect = step.effects[0];
  assert.equal(effect?.kind, "transform");
  assert.ok(effect.correlation);
  return effect.correlation;
}

function settings() {
  return {
    relayProfilesEnabled: true,
    relayProfiles: [profile()],
    activeRelayId: "relay-old",
    relayBaseUrl: "",
    relayApiKey: "",
    relayCommonConfigContents: "",
    relayContextConfigContents: "",
    relayTestModel: "gpt-5.5",
    unrelatedSetting: "preserve-outside-provider",
  };
}

const inspection = {
  profileId: "relay-one",
  state: "upgradeAvailable",
  fields: [{ field: "actorHeader", outcome: "missing", reason: "missingActorHeader" }],
};

const preview = {
  capabilityLoss: true,
  removesProviderTable: false,
  removedProviderId: null,
  removedProviderFields: [],
};

describe("provider detail draft state", () => {
  it("starts an explicit upgrade as a revisioned draft transform and never as a commit", () => {
    const initial = draftState();
    const correlation = beginProviderDetailInspection(initial);
    const observed = applyProviderDetailInspection(initial, correlation, inspection);
    assert.equal(observed.disposition, "applied");
    assert.equal(observed.state.profile, initial.profile);
    assert.deepEqual(observed.effects, []);

    const upgrade = beginProviderDetailNativePriorityUpgrade(observed.state);
    assert.equal(upgrade.state.profile, initial.profile);
    assert.equal(upgrade.state.pendingTransformRevision, 1);
    assert.equal(upgrade.effects.length, 1);
    assert.equal(upgrade.effects[0].kind, "transform");
    if (upgrade.effects[0].kind !== "transform") return;
    assert.equal(upgrade.effects[0].invocation.request.action, "enableNativePriority");
    assert.deepEqual(upgrade.effects[0].invocation.request.confirmations, []);

    const closed = endProviderDetailSession(upgrade.state, "cancel");
    assert.deepEqual(closed.effects, []);
    assert.equal(closed.state.lifecycle, "closed");

    const chatState = draftState();
    const chatObserved = applyProviderDetailInspection(
      chatState,
      beginProviderDetailInspection(chatState),
      {
        profileId: "relay-one",
        state: "compatibility",
        fields: [{ field: "protocol", outcome: "mismatch", reason: "chatCompletions" }],
      },
    );
    assert.equal(chatObserved.disposition, "applied");
    const chatUpgrade = beginProviderDetailNativePriorityUpgrade(chatObserved.state);
    assert.equal(chatUpgrade.effects[0]?.kind, "transform");
    if (chatUpgrade.effects[0]?.kind === "transform") {
      assert.equal(chatUpgrade.effects[0].invocation.request.action, "enableNativePriority");
    }

    const legacyState = draftState();
    const legacyObserved = applyProviderDetailInspection(
      legacyState,
      beginProviderDetailInspection(legacyState),
      {
        profileId: "relay-one",
        state: "upgradeAvailable",
        fields: [{
          field: "providerSelection",
          outcome: "mismatch",
          reason: "legacyProviderIdRequiresRename",
        }],
      },
    );
    assert.equal(legacyObserved.disposition, "applied");
    assert.throws(
      () => beginProviderDetailNativePriorityUpgrade(legacyObserved.state),
      /not eligible/,
    );
  });

  it("requires an explicit second confirmation before replacing a custom actor header", () => {
    const actorState = draftState();
    const observed = applyProviderDetailInspection(
      actorState,
      beginProviderDetailInspection(actorState),
      {
        profileId: "relay-one",
        state: "degraded",
        fields: [
          { field: "relayMode", outcome: "satisfied", reason: "canonical" },
          { field: "actorHeader", outcome: "conflict", reason: "actorHeaderValueConflict" },
        ],
      },
    );
    assert.equal(observed.disposition, "applied");
    const first = beginProviderDetailNativePriorityUpgrade(observed.state);
    const blocked = settleProviderDetailTransform(first.state, transformCorrelation(first), {
      draftRevision: 1,
      status: "confirmationRequired",
      draft: {
        profile: profile(),
        structuredApiKey: "provider-key",
        catalogMode: "official-plus-custom",
      },
      blockers: ["actorHeaderValueConflict"],
      inspection: observed.state.inspection!,
      preview,
    });

    assert.equal(blocked.disposition, "notApplied");
    assert.equal(blocked.state.pendingConfirmation?.requiredConfirmation, "replaceActorHeader");
    assert.deepEqual(blocked.effects, []);
    const confirmed = confirmProviderDetailTransition(blocked.state);
    assert.equal(confirmed.effects[0]?.kind, "transform");
    if (confirmed.effects[0]?.kind === "transform") {
      assert.deepEqual(
        confirmed.effects[0].invocation.request.confirmations,
        ["replaceActorHeader"],
      );
    }
    const failedConfirmation = settleProviderDetailTransformError(
      confirmed.state,
      transformCorrelation(confirmed),
    );
    assert.equal(failedConfirmation.disposition, "error");
    assert.deepEqual(failedConfirmation.state.inspection, observed.state.inspection);
    const cancelled = cancelProviderDetailTransition(blocked.state);
    assert.deepEqual(cancelled.effects, []);
    assert.equal(cancelled.state.profile.configContents, actorState.profile.configContents);
  });

  it("keeps a legacy ID collision as a retryable draft-only resolution", () => {
    const legacyState = draftState();
    const pending = beginProviderDetailEdit(legacyState, {
      patch: {
        relayMode: "official",
        officialMixApiKey: true,
        protocol: "responses",
      },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });
    const blocked = settleProviderDetailTransform(pending.state, transformCorrelation(pending), {
      draftRevision: 1,
      status: "blocked",
      draft: {
        profile: profile(),
        structuredApiKey: "provider-key",
        catalogMode: "official-plus-custom",
      },
      blockers: ["replacementProviderIdRequired"],
      inspection: {
        profileId: "relay-one",
        state: "degraded",
        fields: [{
          field: "providerSelection",
          outcome: "mismatch",
          reason: "legacyProviderIdRequiresRename",
        }],
      },
      preview,
    });

    assert.equal(blocked.disposition, "notApplied");
    assert.equal(blocked.state.pendingLegacyProviderIdResolution?.reason, "required");
    assert.deepEqual(blocked.effects, []);
    assert.equal(blocked.state.profile.configContents, legacyState.profile.configContents);
    assert.throws(
      () => buildProviderDetailCommitEffect(blocked.state, {
        kind: "detailSave",
        settings: settings(),
        persistedSettings: settings(),
        catalogDrafts: [catalogDraft],
        focusedProfileWasPersisted: true,
        previousActiveRelayId: "relay-old",
        confirmContextCleanup: false,
        expectedProviderFingerprint: "fingerprint-old",
        draftRevision: 42,
      }),
      /legacy provider ID/i,
    );
  });

  it("retries a legacy collision with one validated replacement ID and cancels without effects", () => {
    const api = providerDetailStateApi as unknown as Record<string, unknown>;
    const beginLegacy = api.beginProviderDetailLegacyIdUpgrade;
    const resolveLegacy = api.resolveProviderDetailLegacyProviderId;
    const cancelLegacy = api.cancelProviderDetailLegacyProviderIdResolution;
    assert.equal(typeof beginLegacy, "function");
    assert.equal(typeof resolveLegacy, "function");
    assert.equal(typeof cancelLegacy, "function");
    if (
      typeof beginLegacy !== "function"
      || typeof resolveLegacy !== "function"
      || typeof cancelLegacy !== "function"
    ) return;

    const source = draftState();
    const observed = applyProviderDetailInspection(
      source,
      beginProviderDetailInspection(source),
      {
        profileId: "relay-one",
        state: "upgradeAvailable",
        fields: [{
          field: "providerSelection",
          outcome: "mismatch",
          reason: "legacyProviderIdRequiresRename",
        }],
      },
    ).state;
    const first = beginLegacy(observed) as ReturnType<typeof beginProviderDetailEdit>;
    assert.equal(first.effects[0]?.kind, "transform");
    if (first.effects[0]?.kind === "transform") {
      assert.equal(first.effects[0].invocation.request.replacementProviderId, undefined);
    }
    const blocked = settleProviderDetailTransform(first.state, transformCorrelation(first), {
      draftRevision: 1,
      status: "blocked",
      draft: {
        profile: profile(),
        structuredApiKey: "provider-key",
        catalogMode: "official-plus-custom",
      },
      blockers: ["replacementProviderIdRequired"],
      inspection: observed.inspection!,
      preview,
    });
    const retry = resolveLegacy(
      blocked.state,
      "RelayReplacement",
    ) as ReturnType<typeof beginProviderDetailEdit>;
    assert.equal(retry.effects[0]?.kind, "transform");
    if (retry.effects[0]?.kind === "transform") {
      assert.equal(
        retry.effects[0].invocation.request.replacementProviderId,
        "RelayReplacement",
      );
    }
    const failedRetry = settleProviderDetailTransformError(
      retry.state,
      transformCorrelation(retry),
    );
    assert.equal(failedRetry.disposition, "error");
    assert.deepEqual(failedRetry.state.inspection, observed.inspection);
    assert.equal(failedRetry.state.pendingLegacyProviderIdResolution?.reason, "unavailable");
    for (const invalid of ["", "openai", "CodexPlusPlus", "CodexPP"]) {
      assert.throws(
        () => resolveLegacy(blocked.state, invalid),
        /provider ID/i,
      );
    }
    const cancelled = cancelLegacy(blocked.state) as ProviderDetailStep<ReturnType<typeof profile>>;
    assert.deepEqual(cancelled.effects, []);
    assert.equal(cancelled.state.pendingLegacyProviderIdResolution, null);
    assert.equal(cancelled.state.profile.configContents, source.profile.configContents);

    const external = createProviderDetailDraftState({
      profile: profile(),
      catalogDraft: { ...catalogDraft, mode: "external" as const },
    });
    const externalObserved = applyProviderDetailInspection(
      external,
      beginProviderDetailInspection(external),
      {
        profileId: "relay-one",
        state: "notApplicable",
        fields: [
          { field: "catalog", outcome: "notApplicable", reason: "externalCatalog" },
          {
            field: "providerSelection",
            outcome: "mismatch",
            reason: "legacyProviderIdRequiresRename",
          },
        ],
      },
    ).state;
    assert.throws(() => beginLegacy(externalObserved), /not eligible/i);
  });

  it("binds catalog refresh re-inspection to the new response-only revision", () => {
    const loaded = draftState();
    const initial = applyProviderDetailInspection(
      loaded,
      beginProviderDetailInspection(loaded),
      inspection,
    ).state;
    const refreshed = refreshProviderDetailCatalogDraftState(initial, {
      ...catalogDraft,
      mode: "custom-only",
    }, initial.profile);

    assert.equal(refreshed.state.profile, initial.profile);
    assert.equal(refreshed.state.inspection, null);
    assert.equal(refreshed.state.latestTransformRevision, 1);
    assert.ok(refreshed.inspectionCorrelation);
    assert.equal(refreshed.inspectionCorrelation.profileId, "relay-one");
    assert.equal(refreshed.inspectionCorrelation.revision, 1);
    assert.equal(
      refreshed.inspectionCorrelation.sessionToken,
      refreshed.state.sessionToken,
    );
    const oldResponse = applyProviderDetailInspection(
      refreshed.state,
      beginProviderDetailInspection(initial),
      inspection,
    );
    assert.equal(oldResponse.disposition, "stale");
    const newResponse = applyProviderDetailInspection(
      refreshed.state,
      refreshed.inspectionCorrelation,
      inspection,
    );
    assert.equal(newResponse.disposition, "applied");
    assert.deepEqual(newResponse.effects, []);

    const dirty = replaceProviderDetailProfile(initial, {
      ...initial.profile,
      model: "locally-unsaved-model",
    });
    const dirtyRefresh = refreshProviderDetailCatalogDraftState(
      dirty,
      { ...catalogDraft, mode: "custom-only" },
      initial.profile,
    );
    assert.equal(dirtyRefresh.state.inspection, null);
    assert.equal(dirtyRefresh.inspectionCorrelation, null);
  });

  it("registers a backend transform only after building one exact revisioned request", () => {
    const state = draftState();
    const step = beginProviderDetailEdit(state, {
      patch: { relayMode: "official", officialMixApiKey: true },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });

    assert.equal(step.state.latestTransformRevision, 1);
    assert.equal(step.state.pendingTransformRevision, 1);
    assert.equal(step.effects.length, 1);
    assert.equal(step.effects[0].kind, "transform");
    if (step.effects[0].kind !== "transform") return;
    assert.equal(step.effects[0].invocation.command, "transform_provider_native_capability_draft");
    assert.equal(step.effects[0].invocation.request.draftRevision, 1);
    assert.equal(step.effects[0].invocation.request.catalogMode, "official-plus-custom");
    assert.equal(step.effects[0].invocation.request.profile.configContents, config);
  });

  it("drops a stale success and stale transport error after a newer ordinary edit", () => {
    const state = draftState();
    const pending = beginProviderDetailEdit(state, {
      patch: { relayMode: "official", officialMixApiKey: true },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });
    const edited = beginProviderDetailEdit(pending.state, {
      patch: { baseUrl: "https://newer.example/v1" },
      target: existingTarget,
    });
    assert.equal(edited.state.latestTransformRevision, 2);
    assert.equal(edited.effects.length, 0);

    const stale = settleProviderDetailTransform(edited.state, transformCorrelation(pending), {
      draftRevision: 1,
      status: "ready",
      draft: {
        profile: { ...profile(), baseUrl: "https://stale.example/v1" },
        structuredApiKey: "provider-key",
        catalogMode: "official-plus-custom",
      },
      blockers: [],
      inspection,
      preview,
    });
    assert.equal(stale.disposition, "stale");
    assert.equal(stale.state.profile.baseUrl, "https://newer.example/v1");

    const staleError = settleProviderDetailTransformError(
      stale.state,
      transformCorrelation(pending),
    );
    assert.equal(staleError.disposition, "stale");
    assert.equal(staleError.report, false);
    assert.equal(staleError.state.profile.baseUrl, "https://newer.example/v1");
  });

  it("rejects an old response across profile and same-profile reopened sessions", () => {
    const stateA = draftState();
    const pendingA = beginProviderDetailEdit(stateA, {
      patch: { relayMode: "official", officialMixApiKey: true },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });
    const profileB = { ...profile(), id: "relay-two", name: "Relay Two" };
    const catalogB = { ...catalogDraft, profileId: "relay-two" };
    const pendingB = beginProviderDetailEdit(
      draftState(profileB, catalogB),
      {
        patch: { relayMode: "official", officialMixApiKey: true },
        target: existingTarget,
        transition: { action: "enableNativePriority", confirmations: [] },
      },
    );
    const responseA = {
      draftRevision: 1,
      status: "ready" as const,
      draft: {
        profile: { ...profile(), configContents: config.replace("keep-provider", "from-a") },
        structuredApiKey: "provider-key-a",
        catalogMode: "official-plus-custom" as const,
      },
      blockers: [],
      inspection,
      preview,
    };

    const crossProfile = settleProviderDetailTransform(
      pendingB.state,
      transformCorrelation(pendingA),
      responseA,
    );
    assert.equal(crossProfile.disposition, "stale");
    assert.equal(crossProfile.state.profile.id, "relay-two");
    const crossProfileBlocked = settleProviderDetailTransform(
      pendingB.state,
      transformCorrelation(pendingA),
      {
        ...responseA,
        status: "confirmationRequired",
        blockers: ["actorHeaderValueConflict"],
      },
    );
    assert.equal(crossProfileBlocked.disposition, "stale");
    const crossProfileError = settleProviderDetailTransformError(
      pendingB.state,
      transformCorrelation(pendingA),
    );
    assert.equal(crossProfileError.disposition, "stale");
    assert.equal(crossProfileError.report, false);

    const reopened = beginProviderDetailEdit(
      draftState(),
      {
        patch: { relayMode: "official", officialMixApiKey: true },
        target: existingTarget,
        transition: { action: "enableNativePriority", confirmations: [] },
      },
    );
    assert.notEqual(reopened.state.sessionToken, pendingA.state.sessionToken);
    assert.equal(reopened.state.pendingTransformRevision, 1);
    const sameProfileOldSession = settleProviderDetailTransform(
      reopened.state,
      transformCorrelation(pendingA),
      responseA,
    );
    assert.equal(sameProfileOldSession.disposition, "stale");
    assert.equal(sameProfileOldSession.state.pendingTransformRevision, 1);
  });

  it("adopts only the current ready backend draft and keeps inspection response-only", () => {
    const state = draftState();
    const pending = beginProviderDetailEdit(state, {
      patch: { relayMode: "official", officialMixApiKey: true },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });
    const transformedConfig = config.replace("keep-provider", "backend-preserved");
    const settled = settleProviderDetailTransform(pending.state, transformCorrelation(pending), {
      draftRevision: 1,
      status: "ready",
      draft: {
        profile: { ...profile(), configContents: transformedConfig },
        structuredApiKey: "provider-key",
        catalogMode: "official-plus-custom",
      },
      blockers: [],
      inspection,
      preview: { ...preview, capabilityLoss: false },
    });

    assert.equal(settled.disposition, "applied");
    assert.equal(settled.state.pendingTransformRevision, null);
    assert.equal(settled.state.profile.configContents, transformedConfig);
    assert.deepEqual(settled.state.inspection, inspection);
    assert.equal("inspection" in settled.state.profile, false);
    assert.equal("preview" in settled.state.profile, false);
  });

  it("loads inspection only into the matching live detail session", () => {
    const first = draftState();
    const reopened = draftState();
    const firstCorrelation = beginProviderDetailInspection(first);
    const reopenedCorrelation = beginProviderDetailInspection(reopened);
    const stale = applyProviderDetailInspection(reopened, firstCorrelation, inspection);
    assert.equal(stale.disposition, "stale");
    assert.equal(stale.state.inspection, null);

    const current = applyProviderDetailInspection(
      reopened,
      reopenedCorrelation,
      inspection,
    );
    assert.equal(current.disposition, "applied");
    assert.deepEqual(current.state.inspection, inspection);
    assert.equal("inspection" in current.state.profile, false);

    const edited = beginProviderDetailEdit(current.state, {
      patch: { model: "gpt-5.6" },
      target: existingTarget,
    });
    const oldRevision = applyProviderDetailInspection(
      edited.state,
      reopenedCorrelation,
      inspection,
    );
    assert.equal(oldRevision.disposition, "stale");
    assert.equal(oldRevision.state.inspection, null);
  });

  it("invalidates pending transforms when the controlled profile changes locally", () => {
    const pending = beginProviderDetailEdit(draftState(), {
      patch: { relayMode: "official", officialMixApiKey: true },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });
    const replaced = replaceProviderDetailProfile(
      pending.state,
      { ...pending.state.profile, name: "New local name" },
    );
    assert.equal(replaced.latestTransformRevision, 2);
    assert.equal(replaced.pendingTransformRevision, null);
    assert.equal(replaced.profile.name, "New local name");
    assert.equal(
      settleProviderDetailTransformError(replaced, transformCorrelation(pending)).disposition,
      "stale",
    );
    assert.throws(
      () => replaceProviderDetailProfile(replaced, { ...replaced.profile, id: "other" }),
      /another session/i,
    );

    const catalog = replaceProviderDetailCatalogDraft(
      replaced,
      { ...catalogDraft, mode: "custom-only" },
    );
    assert.equal(catalog.catalogDraft?.mode, "custom-only");
    assert.equal(catalog.latestTransformRevision, 3);
    assert.equal(catalog.pendingTransformRevision, null);
    assert.throws(
      () => replaceProviderDetailCatalogDraft(catalog, { ...catalogDraft, profileId: "other" }),
      /another profile/i,
    );

    const transformPending = beginProviderDetailEdit(draftState(), {
      patch: { relayMode: "pureApi", officialMixApiKey: false },
      target: existingTarget,
      transition: { action: "exitPureApi", confirmations: [] },
    });
    const catalogChanged = replaceProviderDetailCatalogDraft(
      transformPending.state,
      { ...catalogDraft, mode: "custom-only" },
    );
    assert.equal(catalogChanged.latestTransformRevision, 2);
    assert.equal(catalogChanged.pendingTransformRevision, null);
    const staleCatalogResponse = settleProviderDetailTransform(
      catalogChanged,
      transformCorrelation(transformPending),
      {
        draftRevision: 1,
        status: "ready",
        draft: {
          profile: { ...profile(), relayMode: "pureApi", officialMixApiKey: false },
          structuredApiKey: "provider-key",
          catalogMode: "official-plus-custom",
        },
        blockers: [],
        inspection,
        preview,
      },
    );
    assert.equal(staleCatalogResponse.disposition, "stale");
    assert.equal(staleCatalogResponse.state.catalogDraft?.mode, "custom-only");
  });

  it("keeps preview and confirmation steps in memory without creating a commit effect", () => {
    const state = draftState();
    const pending = beginProviderDetailEdit(state, {
      patch: { officialMixApiKey: false },
      target: existingTarget,
      transition: { action: "exitPureOAuth", confirmations: [] },
    });
    const blocked = settleProviderDetailTransform(pending.state, transformCorrelation(pending), {
      draftRevision: 1,
      status: "confirmationRequired",
      draft: {
        profile: profile(),
        structuredApiKey: "provider-key",
        catalogMode: "official-plus-custom",
      },
      blockers: ["destructiveExitConfirmationRequired"],
      inspection,
      preview: { ...preview, removesProviderTable: true },
    });

    assert.equal(blocked.disposition, "notApplied");
    assert.equal(blocked.state.profile.configContents, config);
    assert.equal(blocked.state.preview?.removesProviderTable, true);
    assert.deepEqual(blocked.state.blockers, ["destructiveExitConfirmationRequired"]);

    const confirmed = beginProviderDetailEdit(blocked.state, {
      patch: { officialMixApiKey: false },
      target: existingTarget,
      transition: {
        action: "exitPureOAuth",
        confirmations: ["confirmDestructivePureOAuth"],
      },
    });
    assert.deepEqual(confirmed.effects.map((effect) => effect.kind), ["transform"]);
  });

  it("requires an explicit preview confirmation before compatibility exits can be committed", () => {
    for (const [action, patch, expectedConfirmation] of [
      ["exitChatCompletions", { protocol: "chatCompletions" }, "confirmCapabilityLoss"],
      ["exitPureApi", { relayMode: "pureApi", officialMixApiKey: false }, "confirmCapabilityLoss"],
      ["exitLegacyCompatibility", { relayMode: "official", officialMixApiKey: true }, "confirmCapabilityLoss"],
      ["exitPureOAuth", { relayMode: "official", officialMixApiKey: false }, "confirmDestructivePureOAuth"],
    ] as const) {
      const pending = beginProviderDetailEdit(draftState(), {
        patch,
        target: existingTarget,
        transition: { action, confirmations: [] },
      });
      const requested = settleProviderDetailTransform(
        pending.state,
        transformCorrelation(pending),
        {
          draftRevision: 1,
          status: "confirmationRequired",
          draft: {
            profile: profile(),
            structuredApiKey: "provider-key",
            catalogMode: "official-plus-custom",
          },
          blockers: [action === "exitPureOAuth"
            ? "destructiveExitConfirmationRequired"
            : "capabilityLossConfirmationRequired"],
          inspection,
          preview: {
            ...preview,
            removesProviderTable: action === "exitPureOAuth",
            removedProviderId: action === "exitPureOAuth" ? "RelayOne" : null,
            removedProviderFields: action === "exitPureOAuth"
              ? ["experimental_bearer_token", "custom_provider_field", "http_headers"]
              : [],
          },
        },
      );

      assert.equal(requested.disposition, "notApplied", action);
      assert.equal(requested.state.profile.configContents, config, action);
      assert.equal(requested.state.pendingConfirmation?.transition.action, action);
      assert.equal(requested.effects.length, 0, action);
      for (const kind of ["detailSave", "setCurrent"] as const) {
        assert.throws(
          () => buildProviderDetailCommitEffect(requested.state, {
            kind,
            settings: settings(),
            persistedSettings: settings(),
            catalogDrafts: [catalogDraft],
            focusedProfileWasPersisted: true,
            previousActiveRelayId: "relay-old",
            confirmContextCleanup: false,
            expectedProviderFingerprint: "fingerprint-old",
            draftRevision: 42,
          }),
          /confirm/i,
          `${action}/${kind}`,
        );
      }

      const confirmed = confirmProviderDetailTransition(requested.state);
      assert.deepEqual(confirmed.effects.map((effect) => effect.kind), ["transform"], action);
      assert.equal(confirmed.state.pendingConfirmation, null, action);
      const effect = confirmed.effects[0];
      assert.equal(effect.kind, "transform", action);
      if (effect.kind !== "transform") continue;
      assert.deepEqual(effect.invocation.request.confirmations, [expectedConfirmation], action);

      const cancelled = cancelProviderDetailTransition(requested.state);
      assert.equal(cancelled.state.pendingConfirmation, null, action);
      assert.equal(cancelled.state.profile.configContents, config, action);
      assert.deepEqual(cancelled.effects, [], action);
    }
  });

  it("commits a confirmed Chat Completions exit without a managed catalog draft", () => {
    const pending = beginProviderDetailEdit(draftState(), {
      patch: { protocol: "chatCompletions" },
      target: existingTarget,
      transition: { action: "exitChatCompletions", confirmations: [] },
    });
    const previewed = settleProviderDetailTransform(
      pending.state,
      transformCorrelation(pending),
      {
        draftRevision: 1,
        status: "confirmationRequired",
        draft: {
          profile: profile(),
          structuredApiKey: "provider-key",
          catalogMode: "official-plus-custom",
        },
        blockers: ["capabilityLossConfirmationRequired"],
        inspection,
        preview,
      },
    );
    const confirmed = confirmProviderDetailTransition(previewed.state);
    const transformedConfig = config.replace(
      'http_headers = { "x-openai-actor-authorization" = "local-image-extension", "x-unowned" = "keep-header" }',
      'http_headers = { "x-unowned" = "keep-header" }',
    );
    const ready = settleProviderDetailTransform(
      confirmed.state,
      transformCorrelation(confirmed),
      {
        draftRevision: 2,
        status: "ready",
        draft: {
          profile: {
            ...profile(),
            protocol: "chatCompletions",
            configContents: transformedConfig,
          },
          structuredApiKey: "provider-key",
          catalogMode: "official-plus-custom",
        },
        blockers: [],
        inspection,
        preview,
      },
    );
    assert.equal(ready.disposition, "applied");
    assert.equal(ready.state.profile.protocol, "chatCompletions");
    assert.equal(ready.state.catalogDraft, null);
    assert.equal(ready.state.profile.configContents, transformedConfig);

    for (const kind of ["detailSave", "setCurrent"] as const) {
      const commit = buildProviderDetailCommitEffect(ready.state, {
        kind,
        settings: settings(),
        persistedSettings: settings(),
        catalogDrafts: [catalogDraft],
        focusedProfileWasPersisted: true,
        previousActiveRelayId: "relay-old",
        confirmContextCleanup: false,
        expectedProviderFingerprint: "fingerprint-old",
        draftRevision: 43,
      });
      assert.equal(commit.effects[0].kind, "commit");
      if (commit.effects[0].kind !== "commit") continue;
      assert.deepEqual(commit.effects[0].invocation.request.catalogDrafts, [], kind);
      assert.equal(
        commit.effects[0].invocation.request.topology.relayProfiles[0].protocol,
        "chatCompletions",
        kind,
      );
    }
  });

  it("does not misclassify a key-conflict confirmation as an exit confirmation", () => {
    for (const [action, patch] of [
      ["exitPureOAuth", { relayMode: "official", officialMixApiKey: false }],
      ["exitChatCompletions", { protocol: "chatCompletions" }],
    ] as const) {
      const pending = beginProviderDetailEdit(draftState(), {
        patch,
        target: existingTarget,
        transition: { action, confirmations: [] },
      });
      const conflict = settleProviderDetailTransform(
        pending.state,
        transformCorrelation(pending),
        {
          draftRevision: 1,
          status: "confirmationRequired",
          draft: {
            profile: profile(),
            structuredApiKey: "provider-key",
            catalogMode: "official-plus-custom",
          },
          blockers: ["structuredKeyBearerConflict"],
          inspection,
          preview: {
            ...preview,
            removesProviderTable: action === "exitPureOAuth",
          },
        },
      );
      assert.equal(conflict.disposition, "notApplied", action);
      assert.equal(conflict.state.pendingConfirmation, null, action);
      assert.deepEqual(conflict.state.blockers, ["structuredKeyBearerConflict"], action);
      assert.throws(() => confirmProviderDetailTransition(conflict.state), /no provider transition/i);
      for (const kind of ["detailSave", "setCurrent"] as const) {
        assert.throws(
          () => buildProviderDetailCommitEffect(conflict.state, {
            kind,
            settings: settings(),
            persistedSettings: settings(),
            catalogDrafts: [catalogDraft],
            focusedProfileWasPersisted: true,
            previousActiveRelayId: "relay-old",
            confirmContextCleanup: false,
            expectedProviderFingerprint: "fingerprint-old",
            draftRevision: 44,
          }),
          /blocked/i,
          `${action}/${kind}`,
        );
      }
    }
  });

  it("closes, cancels, and navigates without producing persistence effects", () => {
    for (const reason of ["cancel", "close", "navigate"] as const) {
      const ended = endProviderDetailSession(
        draftState(),
        reason,
      );
      assert.equal(ended.state.lifecycle, "closed", reason);
      assert.deepEqual(ended.effects, [], reason);
      assert.throws(
        () => buildProviderDetailCommitEffect(ended.state, {
          kind: "detailSave",
          settings: settings(),
          persistedSettings: settings(),
          catalogDrafts: [catalogDraft],
          focusedProfileWasPersisted: true,
          previousActiveRelayId: "relay-old",
          confirmContextCleanup: false,
          expectedProviderFingerprint: "fingerprint-old",
          draftRevision: 41,
        }),
        /closed provider detail/i,
      );
    }
  });

  it("rejects catalog state owned by a different profile", () => {
    assert.throws(
      () => createProviderDetailDraftState({
        profile: profile(),
        catalogDraft: { ...catalogDraft, profileId: "relay-other" },
      }),
      /catalog draft belongs to another profile/i,
    );
  });

  it("builds complete Save and SetCurrent envelopes without response-only metadata", () => {
    const state = {
      ...draftState(),
      inspection,
      preview,
    };
    for (const kind of ["detailSave", "setCurrent"] as const) {
      const step = buildProviderDetailCommitEffect(state, {
        kind,
        settings: settings(),
        persistedSettings: settings(),
        catalogDrafts: [catalogDraft],
        focusedProfileWasPersisted: true,
        previousActiveRelayId: "relay-old",
        confirmContextCleanup: true,
        expectedProviderFingerprint: "fingerprint-old",
        draftRevision: 41,
      });

      assert.equal(step.effects.length, 1);
      assert.equal(step.effects[0].kind, "commit");
      if (step.effects[0].kind !== "commit") continue;
      const request = step.effects[0].invocation.request;
      assert.equal(request.draftRevision, 41);
      assert.equal(request.action, kind === "setCurrent" ? "setCurrent" : "save");
      assert.equal(request.focusedProfileId, "relay-one");
      assert.equal(request.previousActiveRelayId, "relay-old");
      assert.equal(request.confirmContextCleanup, true);
      assert.equal(request.expectedProviderFingerprint, "fingerprint-old");
      assert.equal(request.topology.activeRelayId, kind === "setCurrent" ? "relay-one" : "relay-old");
      assert.equal(request.topology.relayProfiles[0].configContents, config);
      assert.equal(request.catalogDrafts[0].profileId, "relay-one");
      assert.equal("inspection" in request.topology.relayProfiles[0], false);
      assert.equal("preview" in request.topology.relayProfiles[0], false);
      assert.equal(JSON.stringify(request).includes("upgradeAvailable"), false);
    }
  });
});
