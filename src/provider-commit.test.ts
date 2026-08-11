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
  it("fails closed for stale responses, missing existing catalog state, and active deletion", () => {
    assert.ok(commitModule, "provider UI safety helpers must exist");
    assert.equal(commitModule.providerCommitResponseIsCurrent(8, 9), false);
    assert.equal(commitModule.providerCommitResponseIsCurrent(9, 9), true);
    assert.equal(commitModule.providerCommitResponseDisposition(8, 9, true), "adopt-baseline");
    assert.equal(commitModule.providerCommitResponseDisposition(8, 9, false), "ignore");
    assert.equal(commitModule.providerCommitResponseDisposition(9, 9, true), "apply");
    assert.equal(commitModule.providerCommitResponseDisposition(9, 9, false), "report");
    assert.equal(commitModule.catalogDraftAvailability(true, true, false), "unavailable");
    assert.equal(commitModule.catalogDraftAvailability(true, true, true), "persisted");
    assert.equal(commitModule.catalogDraftAvailability(false, true, false), "implicit");
    assert.equal(commitModule.catalogDraftAvailability(true, false, false), "not-required");
    assert.equal(commitModule.providerDeleteAvailable("relay-a", "relay-a", 2), false);
    assert.equal(commitModule.providerDeleteAvailable("relay-b", "relay-a", 2), true);
    assert.equal(commitModule.providerDeleteAvailable("relay-b", "relay-a", 1), false);
    assert.equal(commitModule.managedCatalogCapable(firstProfile), true);
    assert.equal(commitModule.managedCatalogCapable({ ...firstProfile, protocol: "chatCompletions" }), false);

    let state: { latestRevision: number; baseline: string | null } = {
      latestRevision: 0,
      baseline: "persisted-0",
    };
    state = commitModule.registerProviderCommit(state, 1);
    // A local builder failure for the next draft never registers revision 2.
    let settled = commitModule.settleProviderCommit(state, 1, true, "persisted-1");
    assert.equal(settled.disposition, "apply");
    assert.equal(settled.state.baseline, "persisted-1");

    state = commitModule.registerProviderCommit(settled.state, 2);
    state = commitModule.registerProviderCommit(state, 3);
    settled = commitModule.settleProviderCommit(state, 2, true, "persisted-2");
    assert.equal(settled.disposition, "adopt-baseline");
    assert.equal(settled.state.baseline, "persisted-2");
    settled = commitModule.settleProviderCommit(settled.state, 3, false, null);
    assert.equal(settled.disposition, "report");
    assert.equal(settled.state.baseline, "persisted-2");
    assert.equal(
      commitModule.providerCommitFailureShouldReconcileForm(null, settled.disposition),
      true,
    );
    assert.equal(
      commitModule.providerCommitFailureShouldReconcileForm("relay-a", settled.disposition),
      false,
    );
    assert.equal(commitModule.settleProviderCommit(settled.state, 2, false, null).disposition, "ignore");
    assert.equal(commitModule.providerCommitFailureShouldReconcileForm(null, "ignore"), false);
  });

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
      const invocation = kind === "copy"
        ? commitModule.buildProviderMutationInvocation({
            ...common,
            kind,
            copySourceProfileId: "relay-b",
            settings: settingsWith([firstProfile, secondProfile, { ...secondProfile, id: "relay-copy", name: "copy" }]),
            catalogDrafts: [{
              profileId: "relay-b",
              mode: "official-plus-custom",
              modeExplicit: false,
              upstreamTopology: "direct",
              externalPointer: null,
              overlay: emptyOverlay(),
            }],
          })
        : commitModule.buildProviderMutationInvocation({ ...common, kind });
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
      copySourceProfileId: "relay-b",
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

  it("requires an explicit copy source and selects its catalog when detail signatures collide", () => {
    assert.ok(commitModule, "provider commit request builders must exist");
    const shadow = { ...secondProfile, id: "relay-shadow", name: "Relay B shadow" };
    const copy = { ...secondProfile, id: "relay-copy", name: "Relay B copy" };
    const nativeDraft = {
      profileId: "relay-shadow",
      mode: "native-official" as const,
      modeExplicit: true,
      upstreamTopology: "direct" as const,
      externalPointer: null,
      overlay: emptyOverlay(),
    };
    const sourceDraft = {
      ...nativeDraft,
      profileId: "relay-b",
      mode: "custom-only" as const,
    };
    const invocation = commitModule.buildProviderMutationInvocation({
      kind: "copy",
      copySourceProfileId: "relay-b",
      settings: settingsWith([firstProfile, shadow, secondProfile, copy]),
      persistedSettings: settingsWith([firstProfile, shadow, secondProfile]),
      catalogDrafts: [nativeDraft, sourceDraft],
      previousActiveRelayId: "relay-a",
      confirmContextCleanup: false,
      draftRevision: 53,
      expectedProviderFingerprint: "sha256:ambiguous-copy",
    });

    assert.deepEqual(invocation.request.catalogDrafts, [{ ...sourceDraft, profileId: "relay-copy" }]);
  });

  it("emits no catalog draft for incapable copies and rejects a capable copy without source state", () => {
    assert.ok(commitModule, "provider commit request builders must exist");
    const aggregate = {
      ...firstProfile,
      id: "aggregate-a",
      name: "Aggregate A",
      relayMode: "aggregate",
      protocol: "responses",
    };
    const aggregateCopy = { ...aggregate, id: "aggregate-copy", name: "Aggregate A copy" };
    const incapable = commitModule.buildProviderMutationInvocation({
      kind: "copy",
      copySourceProfileId: "aggregate-a",
      settings: settingsWith([firstProfile, aggregate, aggregateCopy]),
      persistedSettings: settingsWith([firstProfile, aggregate]),
      catalogDrafts: [{
        profileId: "aggregate-a",
        mode: "native-official",
        modeExplicit: true,
        upstreamTopology: "direct",
        externalPointer: null,
        overlay: emptyOverlay(),
      }],
      previousActiveRelayId: "relay-a",
      confirmContextCleanup: false,
      draftRevision: 54,
      expectedProviderFingerprint: "sha256:aggregate-copy",
    });
    assert.deepEqual(incapable.request.catalogDrafts, []);

    assert.throws(() => commitModule.buildProviderMutationInvocation({
      kind: "copy",
      copySourceProfileId: "relay-b",
      settings: settingsWith([firstProfile, secondProfile, { ...secondProfile, id: "relay-copy", name: "copy" }]),
      persistedSettings: settingsWith([firstProfile, secondProfile]),
      catalogDrafts: [],
      previousActiveRelayId: "relay-a",
      confirmContextCleanup: false,
      draftRevision: 55,
      expectedProviderFingerprint: "sha256:missing-source-draft",
    }), /requires its source catalog draft/);
  });
});
