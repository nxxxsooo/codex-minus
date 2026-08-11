import type {
  CatalogModeValue,
  CatalogOverlayDraft,
} from "./model-catalog-ui.ts";

export type ProviderCommitAction = "save" | "setCurrent";
export type CatalogUpstreamTopology = "direct" | "server-side-composite";

export type ProviderContextSelectionDraft = {
  mcpServers: string[];
  skills: string[];
  plugins: string[];
};

export type ProviderRelayProfileDraft = {
  id: string;
  name: string;
  model: string;
  baseUrl: string;
  upstreamBaseUrl: string;
  apiKey: string;
  protocol: string;
  relayMode: string;
  officialMixApiKey: boolean;
  testModel: string;
  configContents: string;
  authContents: string;
  useCommonConfig: boolean;
  contextSelection: ProviderContextSelectionDraft;
  contextSelectionInitialized: boolean;
  contextWindow: string;
  autoCompactLimit: string;
  modelInsertMode?: string;
  modelList: string;
  modelWindows: string;
  userAgent: string;
};

export type ProviderAggregateDraft = {
  id: string;
  name: string;
  strategy: string;
  members: Array<{ relayId: string; weight: number }>;
};

export type ProviderSettingsSource = {
  relayProfilesEnabled: boolean;
  relayProfiles: ProviderRelayProfileDraft[];
  aggregateRelayProfiles: ProviderAggregateDraft[];
  activeRelayId: string;
  activeAggregateRelayId: string;
  relayBaseUrl: string;
  relayApiKey: string;
  relayCommonConfigContents: string;
  relayContextConfigContents: string;
  relayTestModel: string;
};

export type ProviderOwnedTopologyDraft = ProviderSettingsSource;

export type ProfileCatalogDraft = {
  profileId: string;
  mode: CatalogModeValue;
  modeExplicit: boolean;
  upstreamTopology: CatalogUpstreamTopology;
  externalPointer: string | null;
  overlay: CatalogOverlayDraft;
};

export type ProviderCommitRequest = {
  topology: ProviderOwnedTopologyDraft;
  catalogDrafts: ProfileCatalogDraft[];
  focusedProfileId: string | null;
  action: ProviderCommitAction;
  previousActiveRelayId: string;
  confirmContextCleanup: boolean;
  draftRevision: number;
  expectedProviderFingerprint: string;
};

type CatalogDraftSource = ProfileCatalogDraft & Record<string, unknown>;

type CommonBuilderInput = {
  settings: ProviderSettingsSource & Record<string, unknown>;
  catalogDrafts: CatalogDraftSource[];
  action: ProviderCommitAction;
  previousActiveRelayId: string;
  confirmContextCleanup: boolean;
  draftRevision: number;
  expectedProviderFingerprint: string;
};

export function buildProviderDetailRequest(input: CommonBuilderInput & {
  focusedProfileId: string;
  focusedProfileWasPersisted: boolean;
}): ProviderCommitRequest {
  const topology = projectProviderOwnedTopology(input.settings);
  const catalogDrafts = input.catalogDrafts.map(projectCatalogDraft);
  if (!input.focusedProfileWasPersisted && !catalogDrafts.some((draft) => draft.profileId === input.focusedProfileId)) {
    const profile = topology.relayProfiles.find((candidate) => candidate.id === input.focusedProfileId);
    if (profile && implicitMixedCatalogEligible(profile)) {
      catalogDrafts.push(implicitMixedCatalogDraft(profile.id));
    }
  }
  return buildEnvelope(input, topology, catalogDrafts, input.focusedProfileId);
}

export function buildProviderTopologyRequest(input: CommonBuilderInput): ProviderCommitRequest {
  return buildEnvelope(
    input,
    projectProviderOwnedTopology(input.settings),
    input.catalogDrafts.map(projectCatalogDraft),
    null,
  );
}

export function projectProviderOwnedTopology(settings: ProviderSettingsSource): ProviderOwnedTopologyDraft {
  return {
    relayProfilesEnabled: settings.relayProfilesEnabled,
    relayProfiles: settings.relayProfiles.map(projectRelayProfile),
    aggregateRelayProfiles: settings.aggregateRelayProfiles.map((aggregate) => ({
      id: aggregate.id,
      name: aggregate.name,
      strategy: aggregate.strategy,
      members: aggregate.members.map((member) => ({
        relayId: member.relayId,
        weight: member.weight,
      })),
    })),
    activeRelayId: settings.activeRelayId,
    activeAggregateRelayId: settings.activeAggregateRelayId,
    relayBaseUrl: settings.relayBaseUrl,
    relayApiKey: settings.relayApiKey,
    relayCommonConfigContents: settings.relayCommonConfigContents,
    relayContextConfigContents: settings.relayContextConfigContents,
    relayTestModel: settings.relayTestModel,
  };
}

function projectRelayProfile(profile: ProviderRelayProfileDraft): ProviderRelayProfileDraft & { modelInsertMode: string } {
  return {
    id: profile.id,
    name: profile.name,
    model: profile.model,
    baseUrl: profile.baseUrl,
    upstreamBaseUrl: profile.upstreamBaseUrl,
    apiKey: profile.apiKey,
    protocol: profile.protocol,
    relayMode: profile.relayMode,
    officialMixApiKey: profile.officialMixApiKey,
    testModel: profile.testModel,
    configContents: profile.configContents,
    authContents: profile.authContents,
    useCommonConfig: profile.useCommonConfig,
    contextSelection: {
      mcpServers: [...profile.contextSelection.mcpServers],
      skills: [...profile.contextSelection.skills],
      plugins: [...profile.contextSelection.plugins],
    },
    contextSelectionInitialized: profile.contextSelectionInitialized,
    contextWindow: profile.contextWindow,
    autoCompactLimit: profile.autoCompactLimit,
    modelInsertMode: profile.modelInsertMode ?? "patch",
    modelList: profile.modelList,
    modelWindows: profile.modelWindows,
    userAgent: profile.userAgent,
  };
}

function projectCatalogDraft(draft: CatalogDraftSource): ProfileCatalogDraft {
  return {
    profileId: draft.profileId,
    mode: draft.mode,
    modeExplicit: draft.modeExplicit,
    upstreamTopology: draft.upstreamTopology,
    externalPointer: draft.externalPointer,
    overlay: {
      official: Object.fromEntries(Object.entries(draft.overlay.official).map(([slug, value]) => [slug, {
        displayName: value.displayName,
        visible: value.visible,
        contextWindow: value.contextWindow,
        effectiveContextWindowPercent: value.effectiveContextWindowPercent,
        order: value.order,
        supportedReasoningLevels: value.supportedReasoningLevels?.map((level) => ({
          effort: level.effort,
          description: level.description,
        })) ?? null,
        defaultReasoningLevel: value.defaultReasoningLevel,
        supportedTools: value.supportedTools ? [...value.supportedTools] : null,
        toolCapabilities: value.toolCapabilities,
      }])),
      custom: draft.overlay.custom.map((value) => ({
        slug: value.slug,
        displayName: value.displayName,
        contextWindow: value.contextWindow,
        effectiveContextWindowPercent: value.effectiveContextWindowPercent,
        visible: value.visible,
        order: value.order,
        supportedReasoningLevels: value.supportedReasoningLevels.map((level) => ({
          effort: level.effort,
          description: level.description,
        })),
        defaultReasoningLevel: value.defaultReasoningLevel,
        supportedTools: [...value.supportedTools],
        toolCapabilities: value.toolCapabilities,
        templateProvenance: value.templateProvenance,
      })),
    },
  };
}

function buildEnvelope(
  input: CommonBuilderInput,
  topology: ProviderOwnedTopologyDraft,
  catalogDrafts: ProfileCatalogDraft[],
  focusedProfileId: string | null,
): ProviderCommitRequest {
  return {
    topology,
    catalogDrafts,
    focusedProfileId,
    action: input.action,
    previousActiveRelayId: input.previousActiveRelayId,
    confirmContextCleanup: input.confirmContextCleanup,
    draftRevision: input.draftRevision,
    expectedProviderFingerprint: input.expectedProviderFingerprint,
  };
}

function implicitMixedCatalogEligible(profile: ProviderRelayProfileDraft): boolean {
  return profile.relayMode === "official"
    && profile.officialMixApiKey
    && profile.protocol === "responses"
    && !/^\s*model_catalog_json\s*=/m.test(profile.configContents);
}

function implicitMixedCatalogDraft(profileId: string): ProfileCatalogDraft {
  return {
    profileId,
    mode: "official-plus-custom",
    modeExplicit: false,
    upstreamTopology: "direct",
    externalPointer: null,
    overlay: { official: {}, custom: [] },
  };
}
