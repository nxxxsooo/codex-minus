import assert from "node:assert";
import { describe, it } from "node:test";

import {
  beginProviderDetailEdit,
  buildProviderDetailCommitEffect,
  createProviderDetailDraftState,
  endProviderDetailSession,
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
  sessionToken = "session-default",
  currentProfile = profile(),
  currentCatalog = catalogDraft,
) {
  return createProviderDetailDraftState({
    profile: currentProfile,
    catalogDraft: currentCatalog,
    sessionToken,
  });
}

function transformCorrelation(step: {
  effects: Array<{
    kind: string;
    correlation?: { sessionToken: string; profileId: string; revision: number };
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
    const stateA = draftState("session-a");
    const pendingA = beginProviderDetailEdit(stateA, {
      patch: { relayMode: "official", officialMixApiKey: true },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });
    const profileB = { ...profile(), id: "relay-two", name: "Relay Two" };
    const catalogB = { ...catalogDraft, profileId: "relay-two" };
    const pendingB = beginProviderDetailEdit(
      draftState("session-b", profileB, catalogB),
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
      draftState("session-a-reopened"),
      {
        patch: { relayMode: "official", officialMixApiKey: true },
        target: existingTarget,
        transition: { action: "enableNativePriority", confirmations: [] },
      },
    );
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

  it("closes, cancels, and navigates without producing persistence effects", () => {
    for (const reason of ["cancel", "close", "navigate"] as const) {
      const ended = endProviderDetailSession(
        draftState(`session-${reason}`),
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
        sessionToken: "catalog-mismatch",
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
