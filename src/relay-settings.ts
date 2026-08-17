/// Reading, repairing, and editing the provider list.
///
/// Everything the backend sends passes through `normalizeSettings` first. That is not defensive
/// habit: serde omits an empty string rather than sending `""`, so a profile whose model or Base
/// URL lives only inside its TOML arrives with those fields absent, and code that trusted the
/// declared type read `undefined.trim()` and took the whole save down with it.
///
/// The mutators return a new settings object rather than editing one in place. A provider save is a
/// compare-and-swap against a fingerprint of what was read, so the value that was read has to stay
/// intact and comparable while its replacement is being built.

import { t, tf } from "./i18n.ts";

import type {
  BackendSettings,
  ImageOverlayFitMode,
  RelayContextSelection,
  RelayMode,
  RelayProfile,
  RelayProtocol,
} from "./backend-types.ts";
import { splitContextConfigText, contextSelectionForAllEntries } from "./codex-context-entries.ts";
import {
  codexBaseUrlFromConfig,
  codexExperimentalBearerTokenFromConfig,
  codexModelFromConfig,
  codexTopLevelIntFromConfig,
  joinTomlSectionsRootFirst,
  rootTomlStringValue,
} from "./codex-toml.ts";
import {
  applyProviderConfigPatch,
  CHAT_UPSTREAM_BASE_URL_KEY,
  PROTOCOL_PROXY_BASE_URL,
  providerConfigPatchRequiresBackendTransform,
  type ProviderConfigTargetContract,
} from "./provider-config-transform-router.ts";
import { createNewRelayProfileDraft } from "./provider-onboarding.ts";

const emptyContextSelection = (): RelayContextSelection => ({
  mcpServers: [],
  skills: [],
  plugins: [],
});

export const defaultSettings: BackendSettings = {
  codexAppPath: "",
  codexExtraArgs: [],
  providerSyncEnabled: false,
  providerSyncSavedProviders: [],
  providerSyncManualProviders: [],
  providerSyncLastSelectedProvider: "",
  relayProfilesEnabled: true,
  enhancementsEnabled: true,
  computerUseGuardEnabled: false,
  codexAppPluginMarketplaceUnlock: true,
  codexAppPluginAutoExpand: true,
  codexAppModelWhitelistUnlock: true,
  codexAppSessionDelete: true,
  codexAppMarkdownExport: true,
  codexAppPasteFix: false,
  codexAppForceChineseLocale: true,
  codexAppFastStartup: false,
  codexAppProjectMove: true,
  codexAppThreadIdBadge: false,
  codexAppConversationView: false,
  codexAppThreadScrollRestore: true,
  codexAppZedRemoteOpen: true,
  zedRemoteOpenStrategy: "addToFocusedWorkspace",
  zedRemoteProjectRegistryEnabled: true,
  zedRemoteSyncToZedSettings: false,
  codexAppUpstreamWorktreeCreate: true,
  codexAppNativeMenuPlacement: true,
  codexAppNativeMenuLocalization: true,
  codexAppServiceTierControls: false,
  codexAppPetRealMouseLook: false,
  codexAppStepwiseEnabled: false,
  codexAppStepwiseDirectSend: false,
  codexAppStepwiseBaseUrl: "",
  codexAppStepwiseApiKey: "",
  codexAppStepwiseApiKeyEnv: "CODEX_STEPWISE_API_KEY",
  codexAppStepwiseModel: "",
  codexAppStepwiseMaxItems: 6,
  codexAppStepwiseMaxInputChars: 6000,
  codexAppStepwiseMaxOutputTokens: 500,
  codexAppStepwiseTimeoutMs: 8000,
  codexAppImageOverlayEnabled: false,
  codexAppImageOverlayPath: "",
  codexAppImageOverlayOpacity: 35,
  codexAppImageOverlayFitMode: "fit",
  codexGoalsEnabled: false,
  launchMode: "patch",
  relayBaseUrl: "",
  relayApiKey: "",
  relayProfiles: [
    {
      id: "default",
      name: t("默认中转"),
      model: "",
      baseUrl: "",
      upstreamBaseUrl: "",
      apiKey: "",
      protocol: "responses",
      relayMode: "official",
      officialMixApiKey: false,
      testModel: "",
      configContents: "",
      authContents: "",
      useCommonConfig: true,
      contextSelection: emptyContextSelection(),
      contextSelectionInitialized: true,
      contextWindow: "",
      autoCompactLimit: "",
      modelList: "",
      modelWindows: "",
      userAgent: "",
    },
  ],
  relayCommonConfigContents: "",
  relayContextConfigContents: "",
  activeRelayId: "default",
  relayTestModel: "",
};

export function normalizeSettings(settings: BackendSettings): BackendSettings {
  const splitCommon = splitContextConfigText(settings.relayCommonConfigContents || "");
  const relayCommonConfigContents = splitCommon.common;
  const relayContextConfigContents = joinTomlSectionsRootFirst([
    settings.relayContextConfigContents || "",
    splitCommon.context,
  ]);
  const defaultContextSelection = contextSelectionForAllEntries({
    ...settings,
    relayCommonConfigContents,
    relayContextConfigContents,
  });
  const profiles =
    settings.relayProfiles?.length
      ? settings.relayProfiles.map((profile) =>
          normalizeRelayProfile(profile, defaultContextSelection),
        )
      : [
          {
            id: settings.activeRelayId || "default",
            name: t("默认中转"),
            model: "",
            baseUrl: settings.relayBaseUrl || defaultSettings.relayBaseUrl,
            upstreamBaseUrl: settings.relayBaseUrl || defaultSettings.relayBaseUrl,
            apiKey: settings.relayApiKey || "",
            protocol: "responses" as RelayProtocol,
            relayMode: "official" as RelayMode,
            officialMixApiKey: false,
            testModel: "",
            configContents: "",
            authContents: "",
            useCommonConfig: true,
            contextSelection: defaultContextSelection,
            contextSelectionInitialized: true,
            contextWindow: "",
            autoCompactLimit: "",
            modelList: "",
            modelWindows: "",
            userAgent: "",
          },
        ];
  const activeRelayId = profiles.some((profile) => profile.id === settings.activeRelayId)
    ? settings.activeRelayId
    : profiles[0]?.id || "default";
  return syncLegacyRelayFields({
    ...defaultSettings,
    ...settings,
    relayProfilesEnabled: settings.relayProfilesEnabled !== false,
    computerUseGuardEnabled: settings.computerUseGuardEnabled === true,
    codexAppImageOverlayOpacity: clampNumber(settings.codexAppImageOverlayOpacity || 35, 1, 100),
    codexAppImageOverlayFitMode: normalizeImageOverlayFitMode(settings.codexAppImageOverlayFitMode),
    codexAppStepwiseMaxItems: clampNumber(settings.codexAppStepwiseMaxItems ?? 6, 0, 6),
    codexAppStepwiseMaxInputChars: clampNumber(settings.codexAppStepwiseMaxInputChars || 6000, 1000, 24000),
    codexAppStepwiseMaxOutputTokens: clampNumber(settings.codexAppStepwiseMaxOutputTokens || 500, 100, 4000),
    codexAppStepwiseTimeoutMs: clampNumber(settings.codexAppStepwiseTimeoutMs || 8000, 1000, 60000),
    relayCommonConfigContents,
    relayContextConfigContents,
    relayProfiles: profiles,
    activeRelayId,
    // There is no global test model any more; the next save retires whatever an older version left.
    relayTestModel: "",
  });
}

function clampNumber(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}

function normalizeImageOverlayFitMode(value: string | undefined): ImageOverlayFitMode {
  return value === "fill" || value === "fit" || value === "stretch" || value === "tile" || value === "center"
    ? value
    : "fit";
}

export function normalizeRelayProfile(profile: RelayProfile, defaultContextSelection = emptyContextSelection()): RelayProfile {
  const legacyMixedApi = profile.relayMode === "mixedApi";
  const relayMode = normalizeRelayMode(profile.relayMode);
  const officialMixApiKey = profile.officialMixApiKey === true || legacyMixedApi;
  let normalized: RelayProfile = {
    ...profile,
    model: profile.model || "",
    baseUrl: profile.baseUrl || defaultSettings.relayBaseUrl,
    upstreamBaseUrl: profile.upstreamBaseUrl || profile.baseUrl || "",
    apiKey: profile.apiKey || "",
    protocol: profile.protocol === "chatCompletions" ? "chatCompletions" : "responses",
    relayMode,
    officialMixApiKey,
    // A provider is tested with the model it starts on. Clearing a legacy per-profile test value
    // on save retires a value the backend would otherwise keep preferring over the startup model.
    testModel: "",
    configContents: relayMode === "official" && !officialMixApiKey ? "" : profile.configContents || "",
    authContents: "",
    useCommonConfig: profile.useCommonConfig !== false,
    contextSelection: profile.contextSelectionInitialized
      ? normalizeContextSelection(profile.contextSelection)
      : normalizeContextSelection(undefined, defaultContextSelection),
    contextSelectionInitialized: true,
    contextWindow: profile.contextWindow || "",
    autoCompactLimit: profile.autoCompactLimit || "",
    modelList: profile.modelList || "",
    modelWindows: profile.modelWindows || "",
    userAgent: profile.userAgent || "",
  };
  return relayProfileUsesLiveFiles(normalized) ? deriveRelayProfileFromFiles(normalized) : normalized;
}

export function activeRelayProfile(settings: BackendSettings): RelayProfile {
  return (
    settings.relayProfiles.find((profile) => profile.id === settings.activeRelayId) ||
    settings.relayProfiles[0] ||
    defaultSettings.relayProfiles[0]
  );
}

function normalizeRelayMode(mode: RelayMode | undefined): RelayMode {
  if (mode === "pureApi") return mode;
  return "official";
}

function normalizeContextSelection(
  selection?: Partial<RelayContextSelection>,
  fallback: RelayContextSelection = emptyContextSelection(),
): RelayContextSelection {
  if (!selection) {
    return {
      mcpServers: [...fallback.mcpServers],
      skills: [...fallback.skills],
      plugins: [...fallback.plugins],
    };
  }
  return {
    mcpServers: Array.isArray(selection?.mcpServers) ? selection.mcpServers.map(String) : [],
    skills: Array.isArray(selection?.skills) ? selection.skills.map(String) : [],
    plugins: Array.isArray(selection?.plugins) ? selection.plugins.map(String) : [],
  };
}

export function withGeneratedRelayFiles(
  profile: RelayProfile,
  contract: ProviderConfigTargetContract,
): RelayProfile {
  return {
    ...applyProviderConfigPatch(profile, {}, contract),
    authContents: "",
  };
}

export function providerConfigTargetContract(
  profile: RelayProfile,
  brandNew: boolean,
): ProviderConfigTargetContract {
  if (!brandNew) return { target: "preserveExisting", source: "existing" };
  // Every brand-new draft is native-priority by construction: `createNewRelayProfileDraft` fixes
  // mode, protocol, and key mixing, so provenance is the whole decision left to make. The profile
  // parameter stays because a contract is decided per profile, and callers should keep saying
  // which one they mean.
  return { target: "nativePriority", source: "brand-new-empty" };
}

export function deriveRelayProfileFromFiles(profile: RelayProfile): RelayProfile {
  const configContents = profile.configContents || "";
  const configBaseUrl = codexBaseUrlFromConfig(configContents);
  const chatUpstreamBaseUrl = rootTomlStringValue(configContents, CHAT_UPSTREAM_BASE_URL_KEY);
  const isProxyConfig = configBaseUrl === PROTOCOL_PROXY_BASE_URL;
  const upstreamBaseUrl = profile.upstreamBaseUrl || chatUpstreamBaseUrl || (configBaseUrl && !isProxyConfig ? configBaseUrl : profile.baseUrl || "");
  const configApiKey = codexExperimentalBearerTokenFromConfig(configContents);
  const configModel = codexModelFromConfig(configContents);
  // 如果用户输入了带后缀的模型名，优先保留在界面的「配置模型」字段中；
  // config.toml 里实际写的是剥离后缀的 slug（由 applyRelayProfilePatchToFiles 处理）。
  // An omitted field arrives as undefined, not "": read it the same way every other field here is.
  const declaredModel = (profile.model || "").trim();
  const model = /\[.+\]$/.test(declaredModel) ? declaredModel : configModel;
  return {
    ...profile,
    model,
    baseUrl: upstreamBaseUrl,
    upstreamBaseUrl,
    apiKey: configApiKey || profile.apiKey || "",
    contextWindow: codexTopLevelIntFromConfig(configContents, "model_context_window"),
    autoCompactLimit: codexTopLevelIntFromConfig(configContents, "model_auto_compact_token_limit"),
    configContents,
    authContents: "",
  };
}

export function applyRelayProfilePatchToFiles(
  profile: RelayProfile,
  patch: Partial<RelayProfile>,
  options: {
    allowGenerateFiles?: boolean;
    target: ProviderConfigTargetContract;
  },
): RelayProfile {
  let next: RelayProfile = { ...profile, ...patch };
  if (
    options.target.source === "existing"
    && providerConfigPatchRequiresBackendTransform(patch)
  ) {
    return { ...profile, authContents: "" };
  }
  const shouldHaveFiles =
    next.relayMode !== "official" || next.officialMixApiKey || next.configContents.trim();
  if (options.allowGenerateFiles && shouldHaveFiles && !next.configContents.trim()) {
    next = withGeneratedRelayFiles(next, options.target);
  }
  next = applyProviderConfigPatch(next, patch, options.target);
  if ("relayMode" in patch || "officialMixApiKey" in patch) {
    if (next.relayMode === "official" && !next.officialMixApiKey) {
      next.configContents = "";
      next.authContents = "";
    } else if (options.allowGenerateFiles && !next.configContents.trim()) {
      next = withGeneratedRelayFiles(next, options.target);
    }
  }

  next.authContents = "";
  if (!next.configContents.trim()) return next;
  return deriveRelayProfileFromFiles(next);
}

export function relayProfileSwitchValidation(profile: RelayProfile): string | null {
  if (profile.relayMode === "official" && !profile.officialMixApiKey) return null;
  if (!profile.configContents.trim()) {
    return tf("供应商「{0}」缺少独立 config.toml，已停止切换，避免继续显示上一套配置文件。请先在该供应商详情里保存 config.toml。", [profile.name || profile.id]);
  }
  return null;
}

function relayProfileUsesLiveFiles(profile: RelayProfile): boolean {
  return profile.relayMode !== "official" || profile.officialMixApiKey;
}

export function syncLegacyRelayFields(settings: BackendSettings): BackendSettings {
  const relayProfiles = settings.relayProfiles.map(deriveRelayProfileFromFiles);
  const active = activeRelayProfile({ ...settings, relayProfiles });
  return {
    ...settings,
    relayProfiles,
    activeRelayId: active.id,
    relayBaseUrl: active.baseUrl,
    relayApiKey: active.apiKey,
  };
}

export function updateRelayProfile(settings: BackendSettings, id: string, patch: Partial<RelayProfile>): BackendSettings {
  return syncLegacyRelayFields({
    ...settings,
    relayProfiles: settings.relayProfiles.map((profile) => {
      if (profile.id !== id) return profile;
      return deriveRelayProfileFromFiles({ ...profile, ...patch });
    }),
  });
}

export function createRelayProfile(settings: BackendSettings): RelayProfile {
  const id = `relay-${Date.now().toString(36)}`;
  const contextSelection = contextSelectionForAllEntries(settings);
  return createNewRelayProfileDraft({ id, contextSelection });
}

export function addRelayProfile(settings: BackendSettings, profile: RelayProfile): BackendSettings {
  const nextWithFiles = deriveRelayProfileFromFiles(
    profile.configContents.trim()
      ? profile
      : withGeneratedRelayFiles(profile, providerConfigTargetContract(profile, true)),
  );
  const activeId = settings.relayProfiles.some((item) => item.id === settings.activeRelayId)
    ? settings.activeRelayId
    : activeRelayProfile(settings).id;
  return syncLegacyRelayFields({
    ...settings,
    relayProfiles: [...settings.relayProfiles, nextWithFiles],
    activeRelayId: activeId,
  });
}

export function duplicateRelayProfile(settings: BackendSettings, id: string): BackendSettings {
  const sourceIndex = settings.relayProfiles.findIndex((profile) => profile.id === id);
  const source = settings.relayProfiles[sourceIndex] || activeRelayProfile(settings);
  const nextId = `relay-${Date.now().toString(36)}`;
  const next = {
    ...source,
    id: nextId,
    name: tf("{0} 副本", [source.name || t("未命名供应商")]),
  };
  const relayProfiles = [...settings.relayProfiles];
  relayProfiles.splice(sourceIndex >= 0 ? sourceIndex + 1 : relayProfiles.length, 0, next);
  return syncLegacyRelayFields({
    ...settings,
    relayProfiles,
  });
}

export function reorderRelayProfiles(settings: BackendSettings, sourceId: string, targetId: string): BackendSettings {
  if (sourceId === targetId) return settings;
  const sourceIndex = settings.relayProfiles.findIndex((profile) => profile.id === sourceId);
  const targetIndex = settings.relayProfiles.findIndex((profile) => profile.id === targetId);
  if (sourceIndex < 0 || targetIndex < 0) return settings;
  const relayProfiles = [...settings.relayProfiles];
  const [moved] = relayProfiles.splice(sourceIndex, 1);
  relayProfiles.splice(targetIndex, 0, moved);
  return syncLegacyRelayFields({
    ...settings,
    relayProfiles,
  });
}

export function removeRelayProfile(settings: BackendSettings, id: string): BackendSettings {
  const profiles = settings.relayProfiles.filter((profile) => profile.id !== id);
  return syncLegacyRelayFields({
    ...settings,
    relayProfiles: profiles.length ? profiles : defaultSettings.relayProfiles,
    activeRelayId: settings.activeRelayId === id ? profiles[0]?.id || "default" : settings.activeRelayId,
  });
}
