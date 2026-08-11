import assert from "node:assert";
import { describe, it } from "node:test";

import type { CatalogOverlayDraft } from "./model-catalog-ui.ts";
import type { ProviderRelayProfileSource } from "./provider-commit.ts";

const commitModule = await import("./provider-commit.ts").catch(() => null);

const emptyOverlay = (): CatalogOverlayDraft => ({ official: {}, custom: [] });

const firstProfile: ProviderRelayProfileSource & Record<string, unknown> = {
  id: "relay-a",
  name: "Relay A",
  model: "gpt-5.4",
  baseUrl: "https://a.example/v1",
  upstreamBaseUrl: "https://a.example/v1",
  apiKey: "secret-a",
  protocol: "responses",
  relayMode: "official",
  officialMixApiKey: true,
  testModel: "gpt-5.4-mini",
  configContents: "model = \"gpt-5.4\"\n",
  authContents: "",
  useCommonConfig: true,
  contextSelection: { mcpServers: ["memory"], skills: ["planner"], plugins: [] },
  contextSelectionInitialized: true,
  contextWindow: "272000",
  autoCompactLimit: "240000",
  modelInsertMode: "patch",
  modelList: "gpt-5.4",
  modelWindows: "{}",
  userAgent: "relay-a-agent",
  nativeCapabilityInspection: { state: "ready", providerKey: "must-not-leak" },
};

const secondProfile: ProviderRelayProfileSource & Record<string, unknown> = {
  ...firstProfile,
  id: "relay-b",
  name: "Relay B",
  apiKey: "secret-b",
  configContents: "model = \"custom-b\"\n",
  model: "custom-b",
  modelList: "custom-b",
  nativeCapabilityInspection: { state: "upgradeAvailable" },
};

function settingsWith(profiles: Array<ProviderRelayProfileSource & Record<string, unknown>>) {
  return {
    relayProfilesEnabled: true,
    relayProfiles: profiles,
    aggregateRelayProfiles: [],
    activeRelayId: "relay-a",
    activeAggregateRelayId: "",
    relayBaseUrl: "https://a.example/v1",
    relayApiKey: "secret-a",
    relayCommonConfigContents: "# common\n",
    relayContextConfigContents: "# context\n",
    relayTestModel: "gpt-5.4-mini",
    enhancementsEnabled: false,
    codexAppPath: "/Applications/Unrelated.app",
  };
}

describe("provider-owned commit request", () => {
  it("builds the literal first-save envelope and supplies an implicit mixed catalog draft", () => {
    assert.ok(commitModule, "provider commit request builders must exist");
    const request = commitModule.buildProviderDetailRequest({
      settings: settingsWith([firstProfile]),
      focusedProfileId: "relay-a",
      focusedProfileWasPersisted: false,
      catalogDrafts: [],
      action: "save",
      previousActiveRelayId: "",
      confirmContextCleanup: true,
      draftRevision: 7,
      expectedProviderFingerprint: "sha256:persisted-before-create",
    });

    assert.deepEqual(request, {
      topology: {
        relayProfilesEnabled: true,
        relayProfiles: [{
          id: "relay-a",
          name: "Relay A",
          model: "gpt-5.4",
          baseUrl: "https://a.example/v1",
          upstreamBaseUrl: "https://a.example/v1",
          apiKey: "secret-a",
          protocol: "responses",
          relayMode: "official",
          officialMixApiKey: true,
          testModel: "gpt-5.4-mini",
          configContents: "model = \"gpt-5.4\"\n",
          authContents: "",
          useCommonConfig: true,
          contextSelection: { mcpServers: ["memory"], skills: ["planner"], plugins: [] },
          contextSelectionInitialized: true,
          contextWindow: "272000",
          autoCompactLimit: "240000",
          modelInsertMode: "patch",
          modelList: "gpt-5.4",
          modelWindows: "{}",
          userAgent: "relay-a-agent",
        }],
        aggregateRelayProfiles: [],
        activeRelayId: "relay-a",
        activeAggregateRelayId: "",
        relayBaseUrl: "https://a.example/v1",
        relayApiKey: "secret-a",
        relayCommonConfigContents: "# common\n",
        relayContextConfigContents: "# context\n",
        relayTestModel: "gpt-5.4-mini",
      },
      catalogDrafts: [{
        profileId: "relay-a",
        mode: "official-plus-custom",
        modeExplicit: false,
        upstreamTopology: "direct",
        externalPointer: null,
        overlay: { official: {}, custom: [] },
      }],
      focusedProfileId: "relay-a",
      action: "save",
      previousActiveRelayId: "",
      confirmContextCleanup: true,
      draftRevision: 7,
      expectedProviderFingerprint: "sha256:persisted-before-create",
    });
  });

  it("keeps an existing profile's complete catalog draft and set-current correlation fields", () => {
    assert.ok(commitModule, "provider commit request builders must exist");
    const overlay = emptyOverlay();
    overlay.custom.push({
      slug: "custom-b",
      displayName: "Custom B",
      contextWindow: 128000,
      effectiveContextWindowPercent: 90,
      visible: true,
      order: 0,
      supportedReasoningLevels: [{ effort: "high", description: "Deep" }],
      defaultReasoningLevel: "high",
      supportedTools: ["web_search"],
      toolCapabilities: { web_search: true },
      templateProvenance: "user-authored",
    });
    const request = commitModule.buildProviderDetailRequest({
      settings: { ...settingsWith([firstProfile, secondProfile]), activeRelayId: "relay-b" },
      focusedProfileId: "relay-b",
      focusedProfileWasPersisted: true,
      catalogDrafts: [{
        profileId: "relay-b",
        mode: "custom-only",
        modeExplicit: true,
        upstreamTopology: "direct",
        externalPointer: null,
        overlay,
        providerEvidenceAtMs: 123,
        generatedPath: "/response/only.json",
      }],
      action: "setCurrent",
      previousActiveRelayId: "relay-a",
      confirmContextCleanup: false,
      draftRevision: 19,
      expectedProviderFingerprint: "sha256:existing",
    });

    assert.deepEqual(request.catalogDrafts, [{
      profileId: "relay-b",
      mode: "custom-only",
      modeExplicit: true,
      upstreamTopology: "direct",
      externalPointer: null,
      overlay: {
        official: {},
        custom: [{
          slug: "custom-b",
          displayName: "Custom B",
          contextWindow: 128000,
          effectiveContextWindowPercent: 90,
          visible: true,
          order: 0,
          supportedReasoningLevels: [{ effort: "high", description: "Deep" }],
          defaultReasoningLevel: "high",
          supportedTools: ["web_search"],
          toolCapabilities: { web_search: true },
          templateProvenance: "user-authored",
        }],
      },
    }]);
    assert.equal(request.focusedProfileId, "relay-b");
    assert.equal(request.action, "setCurrent");
    assert.equal(request.previousActiveRelayId, "relay-a");
    assert.equal(request.draftRevision, 19);
  });

  it("projects enable, reorder, copy, delete, aggregate cleanup, and test-model mutations literally", () => {
    assert.ok(commitModule, "provider commit request builders must exist");
    const aggregate = {
      id: "aggregate-main",
      name: "Aggregate",
      strategy: "failover",
      members: [{ relayId: "relay-b", weight: 2 }],
    };
    const request = commitModule.buildProviderTopologyRequest({
      settings: {
        ...settingsWith([secondProfile, { ...firstProfile, id: "relay-copy", name: "Relay A copy" }]),
        relayProfilesEnabled: false,
        aggregateRelayProfiles: [aggregate],
        activeRelayId: "relay-b",
        relayTestModel: "topology-test-model",
      },
      catalogDrafts: [{
        profileId: "relay-copy",
        mode: "official-plus-custom",
        modeExplicit: false,
        upstreamTopology: "direct",
        externalPointer: null,
        overlay: emptyOverlay(),
      }],
      action: "save",
      previousActiveRelayId: "relay-a",
      confirmContextCleanup: false,
      draftRevision: 23,
      expectedProviderFingerprint: "sha256:before-topology",
    });

    assert.deepEqual(request.topology.relayProfiles.map((profile: { id: string }) => profile.id), ["relay-b", "relay-copy"]);
    assert.equal(request.topology.relayProfilesEnabled, false);
    assert.equal(request.topology.relayTestModel, "topology-test-model");
    assert.equal(request.topology.relayProfiles[0].testModel, "gpt-5.4-mini");
    assert.deepEqual(request.topology.aggregateRelayProfiles, [{
      id: "aggregate-main",
      name: "Aggregate",
      strategy: "failover",
      members: [{ relayId: "relay-b", weight: 2 }],
    }]);
    assert.equal(request.focusedProfileId, null);
    assert.deepEqual(request.catalogDrafts, [{
      profileId: "relay-copy",
      mode: "official-plus-custom",
      modeExplicit: false,
      upstreamTopology: "direct",
      externalPointer: null,
      overlay: { official: {}, custom: [] },
    }]);
    assert.equal("enhancementsEnabled" in request.topology, false);
    assert.equal("nativeCapabilityInspection" in request.topology.relayProfiles[0], false);
  });

  it("defaults omitted source modelInsertMode but always emits it in the canonical request", () => {
    assert.ok(commitModule, "provider commit request builders must exist");
    const { modelInsertMode: _omitted, ...sourceWithoutInsertMode } = firstProfile;
    const request = commitModule.buildProviderTopologyRequest({
      settings: settingsWith([sourceWithoutInsertMode]),
      catalogDrafts: [],
      action: "save",
      previousActiveRelayId: "relay-a",
      confirmContextCleanup: false,
      draftRevision: 31,
      expectedProviderFingerprint: "sha256:source-default",
    });

    assert.equal(request.topology.relayProfiles[0].modelInsertMode, "patch");
    assert.equal(Object.hasOwn(request.topology.relayProfiles[0], "modelInsertMode"), true);
  });

  it("routes every provider-bearing UI mutation through one provider commit invocation", () => {
    assert.ok(commitModule, "provider commit request builders must exist");
    const common = {
      settings: settingsWith([firstProfile, secondProfile]),
      persistedSettings: settingsWith([firstProfile, secondProfile]),
      catalogDrafts: [],
      previousActiveRelayId: "relay-a",
      confirmContextCleanup: false,
      draftRevision: 51,
      expectedProviderFingerprint: "sha256:provider-ui",
    };
    for (const kind of ["enablement", "reorder", "copy", "delete", "aggregateCleanup", "testModel"] as const) {
      const invocation = commitModule.buildProviderMutationInvocation({ ...common, kind });
      assert.equal(invocation.command, "commit_provider_detail");
      assert.equal(invocation.request.focusedProfileId, null);
      assert.equal(invocation.request.action, "save");
    }
    const save = commitModule.buildProviderMutationInvocation({
      ...common,
      kind: "detailSave",
      focusedProfileId: "relay-b",
      focusedProfileWasPersisted: true,
    });
    assert.equal(save.request.focusedProfileId, "relay-b");
    assert.equal(save.request.action, "save");
    const setCurrent = commitModule.buildProviderMutationInvocation({
      ...common,
      kind: "setCurrent",
      focusedProfileId: "relay-b",
      focusedProfileWasPersisted: true,
    });
    assert.equal(setCurrent.request.focusedProfileId, "relay-b");
    assert.equal(setCurrent.request.action, "setCurrent");
  });

  it("derives a copied profile catalog draft from the matching persisted source", () => {
    assert.ok(commitModule, "provider commit request builders must exist");
    const copy = { ...secondProfile, id: "relay-copy", name: "Relay B copy" };
    const sourceDraft = {
      profileId: "relay-b",
      mode: "custom-only" as const,
      modeExplicit: true,
      upstreamTopology: "direct" as const,
      externalPointer: null,
      overlay: emptyOverlay(),
    };
    const invocation = commitModule.buildProviderMutationInvocation({
      kind: "copy",
      settings: settingsWith([firstProfile, secondProfile, copy]),
      persistedSettings: settingsWith([firstProfile, secondProfile]),
      catalogDrafts: [sourceDraft],
      previousActiveRelayId: "relay-a",
      confirmContextCleanup: false,
      draftRevision: 52,
      expectedProviderFingerprint: "sha256:copy-source",
    });

    assert.deepEqual(invocation.request.catalogDrafts, [{ ...sourceDraft, profileId: "relay-copy" }]);
  });
});
