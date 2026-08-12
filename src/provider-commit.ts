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

export type ProviderRelayProfileSource = {
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

export type ProviderRelayProfileDraft = Omit<ProviderRelayProfileSource, "modelInsertMode"> & {
  modelInsertMode: string;
};

export type ProviderAggregateDraft = {
  id: string;
  name: string;
  strategy: string;
  members: Array<{ relayId: string; weight: number }>;
};

export type ProviderSettingsSource = {
  relayProfilesEnabled: boolean;
  relayProfiles: ProviderRelayProfileSource[];
  aggregateRelayProfiles: ProviderAggregateDraft[];
  activeRelayId: string;
  activeAggregateRelayId: string;
  relayBaseUrl: string;
  relayApiKey: string;
  relayCommonConfigContents: string;
  relayContextConfigContents: string;
  relayTestModel: string;
};

export type ProviderOwnedTopologyDraft = Omit<ProviderSettingsSource, "relayProfiles"> & {
  relayProfiles: ProviderRelayProfileDraft[];
};

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

export type ProviderMutationKind =
  | "enablement"
  | "reorder"
  | "copy"
  | "delete"
  | "aggregateCleanup"
  | "testModel"
  | "detailSave"
  | "setCurrent";

export type ProviderCommitInvocation = {
  command: "commit_provider_detail";
  request: ProviderCommitRequest;
};

export type CatalogDraftAvailability = "not-required" | "implicit" | "persisted" | "unavailable";
export type ProviderCommitResponseDisposition = "apply" | "adopt-baseline" | "report" | "ignore";
export type ProviderCommitUiState<T> = {
  latestRevision: number;
  baseline: T | null;
};

export function providerCommitResponseIsCurrent(responseRevision: number, latestRevision: number): boolean {
  return responseRevision === latestRevision;
}

export function providerCommitResponseDisposition(
  responseRevision: number,
  latestRevision: number,
  succeeded: boolean,
): ProviderCommitResponseDisposition {
  if (providerCommitResponseIsCurrent(responseRevision, latestRevision)) {
    return succeeded ? "apply" : "report";
  }
  return succeeded ? "adopt-baseline" : "ignore";
}

export function providerCommitFailureShouldReconcileForm(
  focusedProfileId: string | null,
  disposition: ProviderCommitResponseDisposition,
): boolean {
  return focusedProfileId === null && disposition === "report";
}

export function registerProviderCommit<T>(state: ProviderCommitUiState<T>, revision: number): ProviderCommitUiState<T> {
  if (revision !== state.latestRevision + 1) throw new Error("provider commit revision is not the next submitted revision");
  return { ...state, latestRevision: revision };
}

export function settleProviderCommit<T>(
  state: ProviderCommitUiState<T>,
  revision: number,
  succeeded: boolean,
  baseline: T | null,
): { state: ProviderCommitUiState<T>; disposition: ProviderCommitResponseDisposition } {
  const disposition = providerCommitResponseDisposition(revision, state.latestRevision, succeeded);
  return {
    state: succeeded && baseline ? { ...state, baseline } : state,
    disposition,
  };
}

export function catalogDraftAvailability(
  profileWasPersisted: boolean,
  catalogCapable: boolean,
  persistedSummaryAvailable: boolean,
): CatalogDraftAvailability {
  if (!catalogCapable) return "not-required";
  if (!profileWasPersisted) return "implicit";
  return persistedSummaryAvailable ? "persisted" : "unavailable";
}

export function providerDeleteAvailable(profileId: string, activeProfileId: string, profileCount: number): boolean {
  return profileCount > 1 && profileId !== activeProfileId;
}

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

type ProviderMutationInvocationInput = Omit<CommonBuilderInput, "action"> & {
  persistedSettings: ProviderSettingsSource & Record<string, unknown>;
  focusedProfileId?: string;
  focusedProfileWasPersisted?: boolean;
} & (
  | { kind: "copy"; copySourceProfileId: string }
  | { kind: Exclude<ProviderMutationKind, "copy">; copySourceProfileId?: never }
);

export function buildProviderMutationInvocation(input: ProviderMutationInvocationInput): ProviderCommitInvocation {
  const detail = input.kind === "detailSave" || input.kind === "setCurrent";
  if (detail) {
    if (!input.focusedProfileId) throw new Error("focused provider profile is required");
    return {
      command: "commit_provider_detail",
      request: buildProviderDetailRequest({
        ...input,
        action: input.kind === "setCurrent" ? "setCurrent" : "save",
        focusedProfileId: input.focusedProfileId,
        focusedProfileWasPersisted: input.focusedProfileWasPersisted === true,
      }),
    };
  }

  const persistedIds = new Set(input.persistedSettings.relayProfiles.map((profile) => profile.id));
  const suppliedById = new Map(input.catalogDrafts.map((draft) => [draft.profileId, draft] as const));
  const catalogDrafts: CatalogDraftSource[] = [];
  const newProfiles = input.settings.relayProfiles.filter((profile) => !persistedIds.has(profile.id));
  if (input.kind === "copy") {
    if (newProfiles.length !== 1) throw new Error("copy must add exactly one provider profile");
    const source = input.persistedSettings.relayProfiles.find(
      (candidate) => candidate.id === input.copySourceProfileId,
    );
    if (!source || !sameCopySignature(source, newProfiles[0])) {
      throw new Error("copied provider profile does not match its explicit source");
    }
    if (managedCatalogCapable(newProfiles[0])) {
      const sourceDraft = suppliedById.get(source.id);
      if (!sourceDraft) throw new Error("catalog-capable copy requires its source catalog draft");
      catalogDrafts.push({ ...sourceDraft, profileId: newProfiles[0].id });
    }
  } else if (newProfiles.length) {
    throw new Error("only the copy topology action may add a provider profile");
  }
  return {
    command: "commit_provider_detail",
    request: buildProviderTopologyRequest({
      ...input,
      action: "save",
      catalogDrafts,
    }),
  };
}

export function managedCatalogCapable(profile: ProviderRelayProfileSource): boolean {
  return profile.relayMode !== "aggregate" && profile.protocol !== "chatCompletions";
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

function projectRelayProfile(profile: ProviderRelayProfileSource): ProviderRelayProfileDraft {
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

function sameCopySignature(left: ProviderRelayProfileSource, right: ProviderRelayProfileSource): boolean {
  const leftDraft = projectRelayProfile(left);
  const rightDraft = projectRelayProfile(right);
  leftDraft.id = "";
  leftDraft.name = "";
  rightDraft.id = "";
  rightDraft.name = "";
  return JSON.stringify(leftDraft) === JSON.stringify(rightDraft);
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

/// Actionable hints for the backend's typed `ProviderCommitErrorCode`. The generic failure
/// message alone cannot tell a user which gate rejected the commit, and the transaction writes
/// no log entry on the failure path, so the discriminator has to reach the notice.
export const PROVIDER_COMMIT_FAILURE_HINTS: Record<string, string> = {
  inputUnavailable: "无法读取已保存的供应商设置。",
  officialAuthRequired: "需要当前有效的官方 ChatGPT 登录。",
  catalogScopeStale: "官方模型目录与当前目标客户端或账号范围不一致，请先刷新官方模型目录后重试。",
  staleState: "供应商设置在本次编辑期间被其他写入改变，请重新加载后再保存。",
  invalidDraft: "本次草稿未通过校验。",
  catalogUnavailable: "模型目录状态不可用或无法生成。",
  stagingRejected: "预写入校验拒绝了本次提交。",
  transactionFailed: "写入事务失败，已回滚到上一代。",
};

export function providerCommitFailureMessage(
  message: string,
  errorCode: string | null | undefined,
  translate: (text: string) => string = (text) => text,
): string {
  const code = (errorCode ?? "").trim();
  if (!code) return message;
  const hint = PROVIDER_COMMIT_FAILURE_HINTS[code];
  return hint ? `${message}${translate(hint)}（${code}）` : `${message}（${code}）`;
}
