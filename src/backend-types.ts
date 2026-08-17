/// The shapes the Rust backend sends and receives.
///
/// Every one of these mirrors a `#[derive(Serialize)]` struct or a command argument in
/// `src-tauri/src/`. They live apart from the screens that render them so that a module holding a
/// rule about a provider, a catalog, or a session can name what it operates on without importing
/// the entire application shell.
///
/// serde omits an empty string rather than sending `""`, so a field typed `string` here can still
/// arrive absent. Normalize at the boundary before handing one of these to code that reads it.

import type { NewProviderTransientTarget } from "./provider-onboarding";
import type { ProviderDetailInspectionMetadata } from "./provider-detail-draft-state";

export type Status = "ok" | "failed" | "not_implemented" | "not_checked" | string;

export type CommandResult<T> = T & {
  status: Status;
  message: string;
};

export type BackendSettings = {
  codexAppPath: string;
  codexExtraArgs: string[];
  providerSyncEnabled: boolean;
  providerSyncSavedProviders: string[];
  providerSyncManualProviders: string[];
  providerSyncLastSelectedProvider: string;
  relayProfilesEnabled: boolean;
  enhancementsEnabled: boolean;
  computerUseGuardEnabled: boolean;
  codexAppPluginMarketplaceUnlock: boolean;
  codexAppPluginAutoExpand: boolean;
  codexAppModelWhitelistUnlock: boolean;
  codexAppSessionDelete: boolean;
  codexAppMarkdownExport: boolean;
  codexAppPasteFix: boolean;
  codexAppForceChineseLocale: boolean;
  codexAppFastStartup: boolean;
  codexAppProjectMove: boolean;
  codexAppThreadIdBadge: boolean;
  codexAppConversationView: boolean;
  codexAppThreadScrollRestore: boolean;
  codexAppZedRemoteOpen: boolean;
  zedRemoteOpenStrategy: ZedOpenStrategy;
  zedRemoteProjectRegistryEnabled: boolean;
  zedRemoteSyncToZedSettings: boolean;
  codexAppUpstreamWorktreeCreate: boolean;
  codexAppNativeMenuPlacement: boolean;
  codexAppNativeMenuLocalization: boolean;
  codexAppServiceTierControls: boolean;
  codexAppPetRealMouseLook: boolean;
  codexAppStepwiseEnabled: boolean;
  codexAppStepwiseDirectSend: boolean;
  codexAppStepwiseBaseUrl: string;
  codexAppStepwiseApiKey: string;
  codexAppStepwiseApiKeyEnv: string;
  codexAppStepwiseModel: string;
  codexAppStepwiseMaxItems: number;
  codexAppStepwiseMaxInputChars: number;
  codexAppStepwiseMaxOutputTokens: number;
  codexAppStepwiseTimeoutMs: number;
  codexAppImageOverlayEnabled: boolean;
  codexAppImageOverlayPath: string;
  codexAppImageOverlayOpacity: number;
  codexAppImageOverlayFitMode: ImageOverlayFitMode;
  codexGoalsEnabled: boolean;
  launchMode: LaunchMode;
  relayBaseUrl: string;
  relayApiKey: string;
  relayProfiles: RelayProfile[];
  relayCommonConfigContents: string;
  relayContextConfigContents: string;
  activeRelayId: string;
  relayTestModel: string;
};

type ZedOpenStrategy = "addToFocusedWorkspace" | "reuseWindow" | "newWindow" | "default";

type LaunchMode = "patch" | "relay";

export type ImageOverlayFitMode = "fill" | "fit" | "stretch" | "tile" | "center";

export type RelayProfile = {
  id: string;
  name: string;
  model: string;
  baseUrl: string;
  upstreamBaseUrl: string;
  apiKey: string;
  protocol: RelayProtocol;
  relayMode: RelayMode;
  officialMixApiKey: boolean;
  testModel: string;
  configContents: string;
  authContents: string;
  useCommonConfig: boolean;
  contextSelection: RelayContextSelection;
  contextSelectionInitialized: boolean;
  contextWindow: string;
  autoCompactLimit: string;
  modelList: string;
  modelWindows: string;
  userAgent: string;
  transientTarget?: NewProviderTransientTarget;
};

export type RelayContextSelection = {
  mcpServers: string[];
  skills: string[];
  plugins: string[];
};

export type ContextKind = "mcp" | "skill" | "plugin";

export type CodexContextEntry = {
  id: string;
  kind: ContextKind;
  title: string;
  summary: string;
  tomlBody: string;
  enabled: boolean;
};

export type CodexContextEntries = {
  mcpServers: CodexContextEntry[];
  skills: CodexContextEntry[];
  plugins: CodexContextEntry[];
};

export type RelayProtocol = "responses" | "chatCompletions";

export type RelayMode = "official" | "mixedApi" | "pureApi";

type UserScriptInventory = {
  enabled?: boolean;
  scripts?: Array<{
    key: string;
    name: string;
    source: string;
    enabled: boolean;
    status: string;
    error: string;
    market_id?: string;
    version?: string;
    installed?: boolean;
    source_url?: string;
    homepage?: string;
  }>;
};

export type SettingsResult = CommandResult<{
  settings: BackendSettings;
  settings_path: string;
  user_scripts: UserScriptInventory;
  provider_fingerprint: string;
}>;

export type ProviderCommitResult = CommandResult<{
  settings: BackendSettings | null;
  draftRevision: number;
  providerFingerprint: string;
  restartRequired: boolean;
  errorCode: string | null;
  reason: string | null;
}>;

export type ProviderNativeCapabilityInspectionResult = CommandResult<{
  inspections: ProviderDetailInspectionMetadata[];
}>;

export type RelayResult = CommandResult<{
  authenticated: boolean;
  authSource: string;
  accountLabel: string | null;
  configPath: string;
  configured: boolean;
  requiresOpenaiAuth: boolean;
  hasBearerToken: boolean;
  backupPath: string | null;
}>;

export type RelayFilesResult = CommandResult<{
  configPath: string;
  authPath: string;
  configContents: string;
  authStatus: {
    authenticated: boolean;
    source: string;
    accountLabel: string | null;
    actionRequired: string | null;
  };
}>;

export type LocalSession = {
  id: string;
  title: string;
  cwd: string;
  modelProvider: string;
  archived: boolean;
  updatedAtMs: number | null;
  rolloutPath: string;
  dbPath: string;
};

export type LocalSessionsResult = CommandResult<{
  dbPath: string;
  dbPaths: string[];
  sessions: LocalSession[];
  activeCount: number;
  archivedCount: number;
  archived: boolean;
  nextCursor: string | null;
  pageSize: number;
  elapsedMs: number;
}>;

export type SessionLifecycleSettingsResult = CommandResult<{
  archiveEnabled: boolean;
  firstRunReviewed: boolean;
  retentionDays: number;
  lastCompletedAtMs: number | null;
  autoAdaptProviderOnSwitch: boolean;
}>;

export type ArchivePreviewResult = CommandResult<{
  retentionDays: number;
  cutoffAtMs: number;
  candidateCount: number;
  missingTimestampCount: number;
  destination: string;
  capability: {
    available: boolean;
    cliPath: string | null;
    message: string;
  };
}>;

export type ArchiveMaintenanceResult = CommandResult<{
  due: boolean;
  deferred: boolean;
  cutoffAtMs: number;
  candidateCount: number;
  archivedCount: number;
  skippedCount: number;
  failedCount: number;
  elapsedMs: number;
  lastCompletedAtMs: number | null;
}>;

export type SessionLifecycleOperationResult = CommandResult<{
  sessionId: string;
  archived: boolean;
  currentProvider: string;
  sessionProvider: string;
  providerMismatch: boolean;
}>;

export type ProviderCompatibilityResult = CommandResult<{
  currentProvider: string;
  activeCount: number;
  mismatchCount: number;
  missingProviderCount: number;
  scanGeneration: string;
  encryptedContentWarning: string | null;
  adaptationAvailable: boolean;
  adaptationMessage: string;
  scanElapsedMs: number;
  archivedRolloutsTraversed: number;
}>;

export type DeleteLocalSessionResult = CommandResult<{
  status: string;
  session_id: string;
  message: string;
  undo_token: string | null;
  backup_path: string | null;
}>;

export type ExtractRelayCommonConfigResult = CommandResult<{
  commonConfigContents: string;
  profileConfigContents: string;
}>;

export type RelayProfileTestResult = CommandResult<{
  httpStatus: number;
  endpoint: string;
  responsePreview: string;
  compatibilityFallbackUsed: boolean;
  initialHttpStatus: number | null;
}>;

export type RelayProfileModelsResult = CommandResult<{
  models: string[];
  endpoint: string;
}>;

export type CatalogMode = "native-official" | "official-plus-custom" | "custom-only" | "external";

type UpstreamTopology = "direct" | "server-side-composite";

export type ReasoningLevel = { effort: string; description: string };

export type OfficialCatalogOverride = {
  displayName: string | null;
  visible: boolean | null;
  contextWindow: number | null;
  effectiveContextWindowPercent: number | null;
  order: number | null;
  supportedReasoningLevels: ReasoningLevel[] | null;
  defaultReasoningLevel: string | null;
  supportedTools: string[] | null;
  toolCapabilities: Record<string, unknown> | null;
};

export type CustomCatalogModel = {
  slug: string;
  displayName: string;
  description?: string;
  contextWindow: number;
  effectiveContextWindowPercent: number;
  visible: boolean;
  order: number;
  supportedReasoningLevels: ReasoningLevel[];
  defaultReasoningLevel: string | null;
  supportedTools: string[];
  toolCapabilities: Record<string, unknown> | null;
  templateProvenance: string;
};

type CatalogOverlay = {
  official: Record<string, OfficialCatalogOverride>;
  custom: CustomCatalogModel[];
};

export type OfficialModelSummary = {
  slug: string;
  displayName: string;
  visible: boolean;
  contextWindow: number | null;
};

export type ProfileCatalogSummary = {
  profileId: string;
  mode: CatalogMode;
  modeExplicit: boolean;
  upstreamTopology: UpstreamTopology;
  managedAvailable: boolean;
  contextConflicts: string[];
  externalPointer: string | null;
  generatedPath: string | null;
  effectiveHash: string | null;
  restartRequired: boolean;
  actionRequired: string | null;
  officialOverrideCount: number;
  customCount: number;
  providerEvidenceAtMs: number | null;
  providerReportedCount: number;
  customCandidates: string[];
  providerReportedSlugs: string[];
  overlay: CatalogOverlay;
};

export type ModelCatalogStatusResult = CommandResult<{
  statePath: string;
  source: string;
  targetClientVersion: string | null;
  targetCliPath: string | null;
  visibleCount: number;
  totalCount: number;
  credentialAction: string | null;
  officialModels: OfficialModelSummary[];
  profiles: ProfileCatalogSummary[];
}>;

export type AdoptionPreviewResult = CommandResult<{
  profileId: string;
  sourcePath: string;
  officialOverrideCount: number;
  customModels: CustomCatalogModel[];
  collisions: string[];
  sourceHash: string;
  catalogClientVersion: string | null;
  targetClientVersion: string;
  versionStatus: "match" | "mismatch" | "unknown" | string;
  committed: boolean;
}>;

type ProviderDoctorCheck = {
  id: string;
  title: string;
  status: Status;
  detail: string;
};

export type ProviderDoctorResult = CommandResult<{
  profileName: string;
  model: string;
  summary: string;
  recommendation: string;
  checks: ProviderDoctorCheck[];
  compatibilityFallbackUsed: boolean;
  initialHttpStatus: number | null;
  requestHttpStatus: number | null;
}>;

type EnvConflict = {
  name: string;
  source: "process" | "user" | string;
  valuePresent: boolean;
};

export type EnvConflictsResult = CommandResult<{
  conflicts: EnvConflict[];
}>;

export type RemoveEnvConflictsResult = CommandResult<{
  removed: Array<{
    name: string;
    removedProcess: boolean;
    removedUser: boolean;
  }>;
  backupPath: string | null;
  remaining: EnvConflict[];
}>;

export type Route = "relay" | "sessions";

export type Theme = "dark" | "light";
