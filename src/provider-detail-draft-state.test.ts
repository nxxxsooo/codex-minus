import assert from "node:assert";
import { describe, it } from "node:test";

import {
  applyProviderDetailInspection,
  beginProviderDetailInspection,
  beginProviderDetailEdit,
  beginProviderDetailRawConfigEdit,
  buildProviderDetailCommitEffect,
  cancelProviderDetailTransition,
  confirmProviderDetailTransition,
  createProviderDetailDraftState,
  endProviderDetailSession,
  replaceProviderDetailCatalogDraft,
  replaceProviderDetailProfile,
  settleProviderDetailTransform,
  settleProviderDetailTransformError,
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
    aggregateRelayProfiles: [],
    activeRelayId: "relay-old",
    activeAggregateRelayId: "",
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

    const rawEdited = beginProviderDetailRawConfigEdit(current.state, {
      configContents: config.replace("keep-provider", "newer-raw"),
      catalogMode: "official-plus-custom",
    });
    const oldAfterRaw = applyProviderDetailInspection(
      rawEdited.state,
      reopenedCorrelation,
      inspection,
    );
    assert.equal(oldAfterRaw.disposition, "stale");
    assert.equal(oldAfterRaw.state.inspection, null);
  });

  it("routes raw provider TOML edits through one revisioned backend inspection", () => {
    const state = draftState();
    const changedConfig = config.replace("keep-provider", "raw-user-edit");
    const step = beginProviderDetailRawConfigEdit(state, {
      configContents: changedConfig,
      catalogMode: "official-plus-custom",
    });
    assert.equal(step.state.profile.configContents, config);
    assert.equal(step.state.rawConfigContents, changedConfig);
    assert.equal(step.state.latestTransformRevision, 1);
    assert.equal(step.state.pendingTransformRevision, 1);
    assert.equal(step.effects.length, 1);
    assert.equal(step.effects[0].kind, "transform");
    if (step.effects[0].kind !== "transform") return;
    assert.equal(step.effects[0].invocation.request.action, "validateRawEdit");
    assert.equal(step.effects[0].invocation.request.sourceConfigContents, config);
    assert.equal(step.effects[0].invocation.request.draftRevision, 1);
    assert.equal(step.effects[0].invocation.request.profile.configContents, changedConfig);
    assert.equal(step.effects[0].invocation.request.profile.authContents, "");

    const verified = settleProviderDetailTransform(step.state, transformCorrelation(step), {
      draftRevision: 1,
      status: "ready",
      draft: {
        profile: { ...profile(), configContents: changedConfig },
        structuredApiKey: "provider-key",
        catalogMode: "official-plus-custom",
      },
      blockers: [],
      inspection,
      preview,
    });
    assert.equal(verified.disposition, "applied");
    assert.equal(verified.state.profile.configContents, changedConfig);
    assert.equal(verified.state.rawConfigContents, null);

    const malformed = beginProviderDetailRawConfigEdit(step.state, {
      configContents: `broken = "sk-provider-sentinel`,
      catalogMode: "official-plus-custom",
    });
    assert.equal(malformed.state.profile.configContents, config);
    const blocked = settleProviderDetailTransform(
      malformed.state,
      transformCorrelation(malformed),
      {
        draftRevision: 2,
        status: "blocked",
        draft: {
          profile: { ...profile(), configContents: `broken = "sk-provider-sentinel` },
          structuredApiKey: "provider-key",
          catalogMode: "official-plus-custom",
        },
        blockers: ["malformedToml"],
        inspection,
        preview,
      },
    );
    assert.equal(blocked.disposition, "notApplied");
    assert.equal(blocked.state.profile.configContents, config);
    assert.equal(blocked.state.rawConfigContents, `broken = "sk-provider-sentinel`);
    const failed = settleProviderDetailTransformError(
      malformed.state,
      transformCorrelation(malformed),
    );
    assert.equal(failed.disposition, "error");
    assert.equal(failed.report, true);
    assert.equal(failed.state.rawConfigContents, `broken = "sk-provider-sentinel`);
    assert.equal(JSON.stringify(failed.effects).includes("sk-provider-sentinel"), false);
    assert.throws(
      () => buildProviderDetailCommitEffect(failed.state, {
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
      /unverified raw provider config/i,
    );

    const closed = endProviderDetailSession(malformed.state, "cancel");
    const late = settleProviderDetailTransform(closed.state, transformCorrelation(malformed), {
      draftRevision: 2,
      status: "ready",
      draft: {
        profile: { ...profile(), configContents: `broken = "sk-provider-sentinel` },
        structuredApiKey: "provider-key",
        catalogMode: "official-plus-custom",
      },
      blockers: [],
      inspection,
      preview,
    });
    assert.equal(late.disposition, "stale");
    assert.deepEqual(late.effects, []);
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
