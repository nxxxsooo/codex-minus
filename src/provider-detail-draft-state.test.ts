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
    const state = createProviderDetailDraftState({ profile: profile(), catalogDraft });
    const step = beginProviderDetailEdit(state, {
      patch: { relayMode: "official", officialMixApiKey: true },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });

    assert.equal(step.state.latestRevision, 1);
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
    const state = createProviderDetailDraftState({ profile: profile(), catalogDraft });
    const pending = beginProviderDetailEdit(state, {
      patch: { relayMode: "official", officialMixApiKey: true },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });
    const edited = beginProviderDetailEdit(pending.state, {
      patch: { baseUrl: "https://newer.example/v1" },
      target: existingTarget,
    });
    assert.equal(edited.state.latestRevision, 2);
    assert.equal(edited.effects.length, 0);

    const stale = settleProviderDetailTransform(edited.state, {
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

    const staleError = settleProviderDetailTransformError(stale.state, 1);
    assert.equal(staleError.disposition, "stale");
    assert.equal(staleError.report, false);
    assert.equal(staleError.state.profile.baseUrl, "https://newer.example/v1");
  });

  it("adopts only the current ready backend draft and keeps inspection response-only", () => {
    const state = createProviderDetailDraftState({ profile: profile(), catalogDraft });
    const pending = beginProviderDetailEdit(state, {
      patch: { relayMode: "official", officialMixApiKey: true },
      target: existingTarget,
      transition: { action: "enableNativePriority", confirmations: [] },
    });
    const transformedConfig = config.replace("keep-provider", "backend-preserved");
    const settled = settleProviderDetailTransform(pending.state, {
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
    const state = createProviderDetailDraftState({ profile: profile(), catalogDraft });
    const pending = beginProviderDetailEdit(state, {
      patch: { officialMixApiKey: false },
      target: existingTarget,
      transition: { action: "exitPureOAuth", confirmations: [] },
    });
    const blocked = settleProviderDetailTransform(pending.state, {
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
        createProviderDetailDraftState({ profile: profile(), catalogDraft }),
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
        }),
        /closed provider detail/i,
      );
    }
  });

  it("builds complete Save and SetCurrent envelopes without response-only metadata", () => {
    const state = {
      ...createProviderDetailDraftState({ profile: profile(), catalogDraft }),
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
      });

      assert.equal(step.effects.length, 1);
      assert.equal(step.effects[0].kind, "commit");
      if (step.effects[0].kind !== "commit") continue;
      const request = step.effects[0].invocation.request;
      assert.equal(request.draftRevision, 1);
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
