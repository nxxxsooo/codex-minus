import {
  closestCenter,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { invoke } from "@tauri-apps/api/core";
import { } from "@tauri-apps/api/event";
import { } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check as checkForAppUpdate, type Update as AvailableAppUpdate } from "@tauri-apps/plugin-updater";
import {
  appUpdateBanner,
  appUpdatePhaseAfterEvent,
  type AppUpdatePhase,
} from "./app-update";
import {
  ArrowLeft,
  Archive,
  ArchiveRestore,
  Bell,
  CheckCircle2,
  Copy,
  Download,
  Edit3,
  GripVertical,
  Info,
  KeyRound,
  Languages,
  MessageCircle,
  Moon,
  Plus,
  RefreshCw,
  RotateCcw,
  Save,
  ShieldCheck,
  ShieldAlert,
  Stethoscope,
  Sun,
  TestTube,
  Trash2,
  TriangleAlert,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";

import { Badge as UiBadge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { knownRelayModel } from "./known-relay-models";
import {
  catalogActionRequiredLabel,
  customDisplayNameFollowsSlug,
  addCatalogCandidate,
  adoptionPreviewSummary,
  appModelLabel,
  catalogCandidateSlugs,
  catalogModeForOverlay,
  catalogModePresentation,
  catalogRestoreLosses,
  defaultCatalogMode,
  emptyOfficialOverride,
  officialModelIsVisible,
  officialVisibilityOverride,
  restoreCatalogList,
  type CatalogOverlayDraft,
  externalVersionRequiresAcceptance,
  managedContextConflictKeys,
  catalogRestartGuidance,
  providerManagedContextConflictKeys,
  providerEvidenceState,
  validateCatalogDraft,
} from "./model-catalog-ui";
import {
  catalogEditingAvailability,
  catalogProfileDraft,
  updateCatalogProfileDraft,
} from "./catalog-profile-draft";
import {
  buildProviderMutationInvocation,
  catalogDraftAvailability,
  managedCatalogCapable,
  providerDeleteAvailable,
  providerCommitFailureNotice,
  providerCommitFailureShouldReconcileForm,
  registerProviderCommit,
  settleProviderCommit,
  type ProviderCommitResponseDisposition,
  type ProfileCatalogDraft,
  type ProviderCommitUiState,
  type ProviderMutationKind,
} from "./provider-commit";
import { LiveConfigPanel } from "./relay-config-panels";
import {
  createNewRelayProfileDraft,
  PRO_MODEL_SLUGS,
  validateNewProviderDraft,
  type NewProviderTransientTarget,
} from "./provider-onboarding";
import {
  applyProviderDetailInspection,
  beginProviderDetailEdit,
  beginProviderDetailInspection,
  beginProviderDetailLegacyIdUpgrade,
  beginProviderDetailNativePriorityUpgrade,
  cancelProviderDetailLegacyProviderIdResolution,
  cancelProviderDetailTransition,
  confirmProviderDetailTransition,
  createProviderDetailDraftState,
  endProviderDetailSession,
  refreshProviderDetailCatalogDraftState,
  replaceProviderDetailCatalogDraft,
  replaceProviderDetailProfile,
  resolveProviderDetailLegacyProviderId,
  settleProviderDetailTransform,
  settleProviderDetailTransformError,
  type ProviderDetailDraftState,
  type ProviderDetailInspectionMetadata,
  type ProviderDetailStep,
  type ProviderDetailTransformInvocation,
  type ProviderDetailTransformResponse,
} from "./provider-detail-draft-state";
import {
  deriveProviderNativeCapabilityView,
  providerTransitionDecisionForStructuredPatch,
} from "./provider-native-capability-view";
import { getLanguage, t, tf, toggleLanguage } from "@/i18n";
import type {
  AdoptionPreviewResult,
  AggregateRelayProfile,
  ArchiveMaintenanceResult,
  ArchivePreviewResult,
  BackendSettings,
  CatalogMode,
  CodexContextEntries,
  CodexContextEntry,
  CommandResult,
  ContextKind,
  CustomCatalogModel,
  DeleteLocalSessionResult,
  EnvConflictsResult,
  ExtractRelayCommonConfigResult,
  ImageOverlayFitMode,
  LocalSession,
  LocalSessionsResult,
  ModelCatalogStatusResult,
  OfficialCatalogOverride,
  OfficialModelSummary,
  ProfileCatalogSummary,
  ProviderCommitResult,
  ProviderCompatibilityResult,
  ProviderDoctorResult,
  ProviderNativeCapabilityInspectionResult,
  ReasoningLevel,
  RelayAggregateConfig,
  RelayAggregateStrategy,
  RelayContextSelection,
  RelayFilesResult,
  RelayMode,
  RelayProfile,
  RelayProfileModelsResult,
  RelayProfileTestResult,
  RelayProtocol,
  RelayResult,
  RemoveEnvConflictsResult,
  Route,
  SessionLifecycleOperationResult,
  SessionLifecycleSettingsResult,
  SettingsResult,
  Status,
  Theme,
} from "./backend-types";
import {
  normalizeSettings,
  normalizeRelayProfile,
  activeRelayProfile,
  providerConfigTargetContract,
  deriveRelayProfileFromFiles,
  applyRelayProfilePatchToFiles,
  relayProfileSwitchValidation,
  syncLegacyRelayFields,
  updateRelayProfile,
  createRelayProfile,
  createAggregateRelayProfile,
  addRelayProfile,
  duplicateRelayProfile,
  reorderRelayProfiles,
  removeRelayProfile,
  isAggregateRelayProfile,
  normalizeAggregateRelayProfile,
  normalizeAggregateConfig,
  aggregateMemberCandidates,
  clampAggregateWeight,
  aggregateRelayProfileValidation,
  AGGREGATE_STRATEGIES,
  defaultSettings,
} from "./relay-settings";
import {
  splitContextConfigText,
  contextSelectionForAllEntries,
} from "./codex-context-entries";
import {
  CONTEXT_KIND_TABLES,
  tomlTablePathFromLine,
  contextHeaderFromLine,
  joinTomlSectionsRootFirst,
  normalizeDuplicateTomlTables,
  tomlKey,
  ensureTrailingNewline,
  rootTomlStringValue,
  codexModelFromConfig,
  codexBaseUrlFromConfig,
  codexExperimentalBearerTokenFromConfig,
  codexTopLevelIntFromConfig,
} from "./codex-toml";

const routes: Array<{ id: Route; label: string; icon: LucideIcon; badge?: string }> = [
  { id: "relay", label: t("供应商配置"), icon: KeyRound },
  { id: "sessions", label: t("会话管理"), icon: MessageCircle },
];

export function App() {
  const [theme, setTheme] = useState<Theme>(() => loadInitialTheme());
  const [route, setRoute] = useState<Route>(() => loadInitialRoute());
  const [notice, setNotice] = useState<{ title: string; message: string; detail?: string | null; status?: Status } | null>(null);
  const [confirmDialog, setConfirmDialog] = useState<{
    title: string;
    message: string;
    confirmText: string;
    cancelText: string;
    resolve: (confirmed: boolean) => void;
  } | null>(null);
  const [settings, setSettings] = useState<SettingsResult | null>(null);
  const [, setRelay] = useState<RelayResult | null>(null);
  const [relayFiles, setRelayFiles] = useState<RelayFilesResult | null>(null);
  const [modelCatalog, setModelCatalog] = useState<ModelCatalogStatusResult | null>(null);
  const [modelCatalogLoading, setModelCatalogLoading] = useState(false);
  const [envConflicts, setEnvConflicts] = useState<EnvConflictsResult | null>(null);
  const [localSessions, setLocalSessions] = useState<LocalSessionsResult | null>(null);
  const [sessionArchiveView, setSessionArchiveView] = useState(false);
  const [sessionLifecycle, setSessionLifecycle] = useState<SessionLifecycleSettingsResult | null>(null);
  const [archivePreview, setArchivePreview] = useState<ArchivePreviewResult | null>(null);
  const [archiveMaintenance, setArchiveMaintenance] = useState<ArchiveMaintenanceResult | null>(null);
  const [archiveMaintenanceRunning, setArchiveMaintenanceRunning] = useState(false);
  const [providerCompatibility, setProviderCompatibility] = useState<ProviderCompatibilityResult | null>(null);
  const [providerCompatibilityLoading, setProviderCompatibilityLoading] = useState(false);
  const [settingsForm, setSettingsForm] = useState<BackendSettings>({ ...defaultSettings });
  const [relaySwitching, setRelaySwitching] = useState(false);
  const providerCommitState = useRef<ProviderCommitUiState<SettingsResult>>({
    latestRevision: 0,
    baseline: null,
  });

  const call = <T,>(command: string, args?: Record<string, unknown>) => invoke<T>(command, args);

  const logDiagnostic = (event: string, detail: Record<string, unknown> = {}) => {
    void invoke("write_diagnostic_event", { event, detail }).catch(() => {});
  };

  const run = async <T,>(task: () => Promise<T>): Promise<T | null> => {
    try {
      return await task();
    } catch (error) {
      showErrorNotice(t("调用失败"), error);
      return null;
    }
  };

  const refreshSettings = async (silent = false) => {
    const result = await run(() => call<SettingsResult>("load_settings"));
    if (!result) return null;
    // A read that failed answers with default settings and no fingerprint. Adopting that as the
    // compare-and-swap baseline would replace the real profiles on screen and hide the reason
    // until the next save reported a missing fingerprint, so keep the old baseline and say why.
    if (!result.provider_fingerprint) {
      showNotice(t("设置已加载"), result.message, "failed");
      return null;
    }
    const normalized = normalizeSettings(result.settings);
    const baseline = { ...result, settings: normalized };
    providerCommitState.current = { ...providerCommitState.current, baseline };
    setSettings(baseline);
    setSettingsForm(normalized);
    if (!silent) showResultNotice(t("设置已加载"), result, { silentSuccess: true });
    return normalized;
  };

  const refreshRelay = async (silent = false) => {
    const result = await run(() => call<RelayResult>("relay_status"));
    if (result) {
      setRelay(result);
      if (!silent) showResultNotice(t("登录状态"), result, { silentSuccess: true });
    }
  };

  const refreshRelayFiles = async (silent = false) => {
    const result = await run(() => call<RelayFilesResult>("read_relay_files"));
    if (result) {
      setRelayFiles(result);
      if (!silent) showResultNotice(t("配置文件"), result, { silentSuccess: true });
    }
    return result;
  };

  const refreshModelCatalog = async (silent = false) => {
    if (modelCatalogLoading) return null;
    setModelCatalogLoading(true);
    try {
      const result = await run(() => call<ModelCatalogStatusResult>("model_catalog_status"));
      if (result) {
        setModelCatalog(result);
        if (!silent && !isSuccessStatus(result.status)) showNotice(t("模型目录"), result.message, result.status);
      }
      return result;
    } finally {
      setModelCatalogLoading(false);
    }
  };

  const adoptExternalModelCatalog = async (
    profileId: string,
    commit = false,
    preview?: AdoptionPreviewResult,
    acceptVersionMismatch = false,
    confirmContextCleanup = false,
  ) => {
    const result = await run(() =>
      call<AdoptionPreviewResult>("adopt_external_model_catalog", {
        request: {
          profileId,
          commit,
          expectedSourceHash: preview?.sourceHash ?? null,
          expectedTargetClientVersion: preview?.targetClientVersion ?? null,
          expectedVersionStatus: preview?.versionStatus ?? null,
          acceptVersionMismatch,
          confirmContextCleanup,
        },
      }),
    );
    if (result && commit && isSuccessStatus(result.status)) await refreshModelCatalog(true);
    if (result && (!isSuccessStatus(result.status) || commit)) showNotice(t("采用外部目录"), result.message, result.status);
    return result;
  };

  const refreshEnvConflicts = async (silent = false) => {
    const result = await run(() => call<EnvConflictsResult>("check_env_conflicts"));
    if (result) {
      setEnvConflicts(result);
      if (!silent || !isSuccessStatus(result.status)) showResultNotice(t("环境变量检测"), result, { silentSuccess: true });
    }
    return result;
  };

  const removeEnvConflicts = async (names: string[]) => {
    const uniqueNames = Array.from(new Set(names.map((name) => name.trim()).filter(Boolean)));
    if (!uniqueNames.length) return;
    if (!window.confirm(tf("删除这些环境变量？\n\n{0}\n\n删除前会写入备份。", [uniqueNames.join("\n")]))) return;
    const result = await run(() => call<RemoveEnvConflictsResult>("remove_env_conflicts", { request: { names: uniqueNames } }));
    if (result) {
      setEnvConflicts({
        status: result.status,
        message: result.message,
        conflicts: result.remaining,
      });
      showNotice(t("环境变量清理"), result.message, result.status);
    }
  };

  const refreshLocalSessions = async (
    silent = false,
    archived = sessionArchiveView,
    cursor?: string,
  ) => {
    const result = await run(() =>
      call<LocalSessionsResult>("list_local_sessions", {
        request: { archived, cursor, pageSize: SESSION_LIST_PAGE_SIZE },
      }),
    );
    if (result) {
      setSessionArchiveView(archived);
      setLocalSessions((current) =>
        cursor && current?.archived === archived
          ? { ...result, sessions: [...current.sessions, ...result.sessions] }
          : result,
      );
      if (!silent || !isSuccessStatus(result.status)) showResultNotice(t("会话管理"), result, { silentSuccess: true });
    }
    return result;
  };

  const refreshSessionLifecycle = async (silent = false) => {
    const result = await run(() => call<SessionLifecycleSettingsResult>("load_session_lifecycle_settings"));
    if (result) {
      setSessionLifecycle(result);
      if (!silent && !isSuccessStatus(result.status)) showNotice(t("自动归档"), result.message, result.status);
    }
    return result;
  };

  const refreshArchivePreview = async (retentionDays?: number, silent = false) => {
    const result = await run(() =>
      call<ArchivePreviewResult>("preview_session_archive", {
        request: { retentionDays: retentionDays ?? sessionLifecycle?.retentionDays ?? 30 },
      }),
    );
    if (result) {
      setArchivePreview(result);
      if (!silent && !isSuccessStatus(result.status)) showNotice(t("归档预览"), result.message, result.status);
    }
    return result;
  };

  const saveSessionLifecycle = async (patch: Partial<SessionLifecycleSettingsResult>, silent = false) => {
    const result = await run(() =>
      call<SessionLifecycleSettingsResult>("save_session_lifecycle_settings", {
        settings: {
          archiveEnabled: patch.archiveEnabled ?? sessionLifecycle?.archiveEnabled ?? false,
          firstRunReviewed: patch.firstRunReviewed ?? sessionLifecycle?.firstRunReviewed ?? false,
          retentionDays: patch.retentionDays ?? sessionLifecycle?.retentionDays ?? 30,
          lastCompletedAtMs: patch.lastCompletedAtMs ?? sessionLifecycle?.lastCompletedAtMs ?? null,
          autoAdaptProviderOnSwitch:
            patch.autoAdaptProviderOnSwitch ?? sessionLifecycle?.autoAdaptProviderOnSwitch ?? true,
        },
      }),
    );
    if (result) {
      setSessionLifecycle(result);
      if (!silent || !isSuccessStatus(result.status)) showNotice(t("会话设置"), result.message, result.status);
    }
    return result;
  };

  const enableSessionArchiving = async (retentionDays: number) => {
    const preview = await refreshArchivePreview(retentionDays, true);
    if (!preview) return;
    if (!preview.capability.available) {
      showNotice(t("原生归档不可用"), preview.capability.message, "failed");
      return;
    }
    const confirmed = window.confirm(
      tf("将自动归档 {0} 天未活动的会话。当前有 {1} 个候选，会移动到：\n{2}\n\n归档可恢复，不会释放磁盘空间。是否启用？", [
        retentionDays,
        preview.candidateCount,
        preview.destination,
      ]),
    );
    if (!confirmed) return;
    const saved = await saveSessionLifecycle({ archiveEnabled: true, firstRunReviewed: true, retentionDays });
    if (saved && isSuccessStatus(saved.status)) await runArchiveMaintenance();
  };

  const runArchiveMaintenance = async (force = false) => {
    if (archiveMaintenanceRunning) return null;
    setArchiveMaintenanceRunning(true);
    try {
      const result = await run(() => call<ArchiveMaintenanceResult>("run_session_archive_maintenance", { force }));
      if (result) {
        setArchiveMaintenance(result);
        await Promise.all([refreshSessionLifecycle(true), refreshLocalSessions(true, sessionArchiveView)]);
        if (!isSuccessStatus(result.status) && result.status !== "not_checked") {
          showNotice(t("自动归档"), result.message, result.status);
        }
      }
      return result;
    } finally {
      setArchiveMaintenanceRunning(false);
    }
  };

  const archiveOrRestoreSession = async (session: LocalSession, archived: boolean) => {
    if (archived && !window.confirm(tf("归档会话「{0}」？归档后可随时恢复。", [session.title || session.id]))) return;
    const result = await run(() =>
      call<SessionLifecycleOperationResult>(archived ? "archive_local_session" : "restore_local_session", {
        request: { sessionId: session.id },
      }),
    );
    if (result) {
      await Promise.all([refreshLocalSessions(true, sessionArchiveView), refreshProviderCompatibility(true)]);
      const mismatch = !archived && result.providerMismatch
        ? tf(" 会话原 provider 为 {0}，当前为 {1}；由于上游 active-only 写入尚不可用，已保留原标记。", [result.sessionProvider, result.currentProvider])
        : "";
      showNotice(archived ? t("归档会话") : t("恢复会话"), `${result.message}${mismatch}`, result.status);
    }
  };

  const refreshProviderCompatibility = async (silent = false) => {
    if (providerCompatibilityLoading) return null;
    setProviderCompatibilityLoading(true);
    try {
      const result = await run(() => call<ProviderCompatibilityResult>("scan_provider_compatibility"));
      if (result) {
        setProviderCompatibility(result);
        if (!silent && !isSuccessStatus(result.status)) showNotice(t("供应商兼容性"), result.message, result.status);
      }
      return result;
    } finally {
      setProviderCompatibilityLoading(false);
    }
  };

  const adaptActiveSessions = async (scan = providerCompatibility) => {
    if (!scan?.adaptationAvailable) return;
    const result = await run(() =>
      call<ProviderCompatibilityResult>("adapt_active_sessions_to_current_provider", {
        scanGeneration: scan.scanGeneration,
      }),
    );
    if (result) {
      setProviderCompatibility(result);
      showNotice(t("适配活动会话"), result.message, result.status);
    }
  };

  // The automatic pass never blocks the switch: it runs after the switch reports, scans fresh so
  // the compare-and-swap acts on what it just counted, and reports through its own notice.
  const adaptSessionsAfterSwitch = async () => {
    const scan = await refreshProviderCompatibility(true);
    if (sessionLifecycle?.autoAdaptProviderOnSwitch === false) return;
    if (scan?.mismatchCount) await adaptActiveSessions(scan);
  };

  const requestDeleteLocalSession = (session: LocalSession) =>
    call<DeleteLocalSessionResult>("delete_local_session", {
      request: { sessionId: session.id, title: session.title, dbPath: session.dbPath },
    });

  const confirmSessionDelete = (title: string, message: string) =>
    new Promise<boolean>((resolve) => {
      setConfirmDialog({
        title,
        message,
        confirmText: t("确认删除"),
        cancelText: t("取消"),
        resolve,
      });
    });

  const deleteLocalSession = async (session: LocalSession) => {
    const title = session.title || session.id;
    const confirmed = await confirmSessionDelete(t("删除会话"), tf("删除会话“{0}”？此操作会删除本地数据库记录和 rollout 文件，并创建备份。", [title]));
    if (!confirmed) return;
    const result = await run(() => requestDeleteLocalSession(session));
    if (result) {
      showResultNotice(t("会话删除"), result);
      await refreshLocalSessions(true);
    }
  };

  const deleteLocalSessions = async (sessions: LocalSession[]) => {
    const uniqueSessions = Array.from(new Map(sessions.map((session) => [session.id, session])).values());
    if (!uniqueSessions.length) {
      showNotice(t("批量删除会话"), t("请先选择要删除的会话。"), "failed");
      return;
    }
    const preview = uniqueSessions
      .slice(0, 6)
      .map((session) => `- ${truncateSessionDeletePreview(session.title || session.id)}`)
      .join("\n");
    const extraCount = uniqueSessions.length > 6 ? tf("\n...以及另外 {0} 个会话", [uniqueSessions.length - 6]) : "";
    const confirmed = await confirmSessionDelete(
      t("批量删除会话"),
      tf("删除选中的 {0} 个会话？此操作会删除本地数据库记录和 rollout 文件，并为每个会话创建备份。\n\n{1}{2}", [uniqueSessions.length, preview, extraCount]),
    );
    if (!confirmed) return;

    let succeeded = 0;
    const failed: string[] = [];
    for (const session of uniqueSessions) {
      const result = await run(() => requestDeleteLocalSession(session));
      if (result && isSuccessStatus(result.status)) {
        succeeded += 1;
      } else {
        failed.push(session.title || session.id);
      }
    }

    if (failed.length) {
      showNotice(
        t("批量删除会话"),
        tf("已删除 {0} 个，失败 {1} 个：{2}", [succeeded, failed.length, failed.slice(0, 3).map(truncateSessionDeletePreview).join(t("、"))]),
        succeeded ? "ok" : "failed",
      );
    } else {
      showNotice(t("批量删除会话"), tf("已删除 {0} 个会话。", [succeeded]), "ok");
    }
    await refreshLocalSessions(true);
  };

  const navigate = async (next: Route) => {
    setRoute(next);
    if (next === "relay") {
      await Promise.all([
        refreshSettings(true),
        refreshRelay(true),
        refreshRelayFiles(true),
        refreshEnvConflicts(true),
        refreshModelCatalog(true),
      ]);
    }
    if (next === "sessions") {
      await Promise.all([
        refreshSettings(true),
        refreshLocalSessions(true, sessionArchiveView),
        refreshSessionLifecycle(true).then((lifecycle) => refreshArchivePreview(lifecycle?.retentionDays, true)),
        refreshProviderCompatibility(true),
      ]);
    }
  };

  const saveSettings = async () => {
    const next = normalizeSettings(settingsForm);
    const result = await run(() => call<SettingsResult>("save_settings", { settings: next }));
    if (result) {
      const normalized = normalizeSettings(result.settings);
      const baseline = { ...result, settings: normalized };
      providerCommitState.current = { ...providerCommitState.current, baseline };
      setSettings(baseline);
      setSettingsForm(normalized);
      showNotice(t("设置保存"), result.message, result.status);
    }
  };

  const saveSettingsValue = async (next: BackendSettings, silent = true) => {
    const normalized = normalizeSettings(next);
    setSettingsForm(normalized);
    const result = await run(() => call<SettingsResult>("save_settings", { settings: normalized }));
    if (result) {
      const normalized = normalizeSettings(result.settings);
      const baseline = { ...result, settings: normalized };
      providerCommitState.current = { ...providerCommitState.current, baseline };
      setSettings(baseline);
      setSettingsForm(normalized);
      if (!silent || !isSuccessStatus(result.status)) showNotice(t("设置保存"), result.message, result.status);
    }
    return !!result && isSuccessStatus(result.status);
  };

  const extractRelayCommonConfig = async (configContents: string) => {
    const result = await run(() =>
      call<ExtractRelayCommonConfigResult>("extract_relay_common_config", {
        request: { configContents },
      }),
    );
    if (result) showResultNotice(t("通用配置文件"), result);
    return result && isSuccessStatus(result.status) ? result : null;
  };

  const testRelayProfile = async (profile: RelayProfile) => {
    const result = await run(() => call<RelayProfileTestResult>("test_relay_profile", { profile }));
    if (result) showNotice(t("供应商测试"), result.message, result.status);
  };

  const diagnoseRelayProfile = async (profile: RelayProfile) => {
    const result = await run(() => call<ProviderDoctorResult>("diagnose_relay_profile", { profile }));
    if (result) showNotice("Provider Doctor", result.message, result.status);
    return result ?? null;
  };

  const inspectProviderNativeCapabilities = async (profileId: string) => {
    try {
      const result = await call<ProviderNativeCapabilityInspectionResult>(
        "inspect_provider_native_capabilities",
        { request: { profileId } },
      );
      if (!isSuccessStatus(result.status)) return null;
      return result.inspections.find((inspection) => inspection.profileId === profileId) ?? null;
    } catch {
      return null;
    }
  };

  const transformProviderNativeCapability = async (
    invocation: ProviderDetailTransformInvocation<RelayProfile>,
  ) => {
    const response = await call<ProviderDetailTransformResponse<RelayProfile>>(
      invocation.command,
      { request: invocation.request },
    );
    // The wire format omits an empty string rather than sending `""`, so a profile whose model,
    // Base URL, or Key lives only inside its TOML comes back missing those fields. The editor's
    // type says they are present, and the next save read one of them straight into `.trim()`.
    return response.draft?.profile
      ? { ...response, draft: { ...response.draft, profile: normalizeRelayProfile(response.draft.profile) } }
      : response;
  };

  const fetchRelayProfileModels = async (profile: RelayProfile) => {
    const result = await run(() => call<RelayProfileModelsResult>("fetch_relay_profile_models", { profile }));
    if (result) showNotice(t("供应商支持的模型"), result.message, result.status);
    if (result && isSuccessStatus(result.status)) await refreshModelCatalog(true);
    return result && isSuccessStatus(result.status) ? result.models : null;
  };

  const catalogDraftForProfile = (profile: RelayProfile): ProfileCatalogDraft | null => {
    const summary = modelCatalog?.profiles.find((item) => item.profileId === profile.id) ?? null;
    if (!summary) return null;
    return catalogProfileDraft({
      profileId: profile.id,
      fallbackMode: defaultCatalogMode(profile.relayMode, profile.officialMixApiKey) as CatalogMode,
      summary,
    });
  };

  const persistedCatalogDrafts = () =>
    (modelCatalog?.profiles ?? []).map((summary) => catalogProfileDraft({
      profileId: summary.profileId,
      fallbackMode: summary.mode,
      summary,
    }));

  // Reads the committed generation back without extending the caller's pending state: the
  // transaction has already landed, so awaiting a slow status read would leave a save that
  // succeeded looking unfinished. Each refresh reports its own failure through `run`.
  const refreshAfterCommit = () => {
    void refreshRelay(true);
    void refreshRelayFiles(true);
    void refreshModelCatalog(true);
  };

  const submitProviderCommit = async (invocation: ReturnType<typeof buildProviderMutationInvocation>) => {
    const revision = invocation.request.draftRevision;
    providerCommitState.current = registerProviderCommit(providerCommitState.current, revision);
    const reconcileTopologyFailure = async (disposition: ProviderCommitResponseDisposition) => {
      if (!providerCommitFailureShouldReconcileForm(invocation.request.focusedProfileId, disposition)) return;
      const baseline = providerCommitState.current.baseline;
      if (baseline) {
        setSettings(baseline);
        setSettingsForm(normalizeSettings(baseline.settings));
      } else {
        await refreshSettings(true);
      }
    };
    let result: ProviderCommitResult;
    try {
      result = await call<ProviderCommitResult>(invocation.command, { request: invocation.request });
    } catch (error) {
      const settled = settleProviderCommit(providerCommitState.current, revision, false, null);
      providerCommitState.current = settled.state;
      if (settled.disposition === "report") {
        await reconcileTopologyFailure(settled.disposition);
        showErrorNotice(t("调用失败"), error);
      }
      return false;
    }
    const succeeded = isSuccessStatus(result.status) && !!result.settings;
    const selectedSettings = result.settings ? normalizeSettings(result.settings) : null;
    const priorBaseline = providerCommitState.current.baseline ?? settings;
    const nextBaseline = succeeded && selectedSettings
      ? {
          status: result.status,
          message: result.message,
          settings: selectedSettings,
          settings_path: priorBaseline?.settings_path ?? "",
          user_scripts: priorBaseline?.user_scripts ?? {},
          provider_fingerprint: result.providerFingerprint,
        }
      : null;
    const settled = settleProviderCommit(
      providerCommitState.current,
      result.draftRevision,
      succeeded,
      nextBaseline,
    );
    providerCommitState.current = settled.state;
    if (settled.disposition === "ignore") return false;
    if (settled.disposition === "report") {
      await reconcileTopologyFailure(settled.disposition);
      const failure = providerCommitFailureNotice(result.message, result.errorCode, result.reason);
      showNotice(t("保存供应商"), failure.sentence, result.status, failure.detail);
      return false;
    }
    if (!nextBaseline || !selectedSettings) return false;
    setSettings(nextBaseline);
    if (settled.disposition === "adopt-baseline") {
      refreshAfterCommit();
      return false;
    }
    setSettingsForm(selectedSettings);
    refreshAfterCommit();
    return true;
  };

  // The compare-and-swap baseline is the settings generation a save is written against. A save
  // pressed before the first read landed — or after one failed — has none, and the click used to
  // die on an internal condition the user could do nothing with. It must not simply commit
  // either: without a baseline the form on screen holds placeholder settings, so writing it back
  // would publish an empty provider list. Read the real generation, then ask for the edit again.
  const providerCommitBaseline = async (): Promise<SettingsResult | null> => {
    const current = providerCommitState.current.baseline ?? settings;
    if (current?.provider_fingerprint) return current;
    const reloaded = await refreshSettings(true);
    showNotice(
      t("保存供应商"),
      reloaded
        ? t("刚刚才读到供应商设置，页面已更新为当前内容，请确认后重新保存。")
        : t("尚未读取到供应商设置，请点击右上角刷新后重试。"),
      "failed",
    );
    return null;
  };

  const providerCommitCommon = (
    baseline: SettingsResult,
    next: BackendSettings,
    confirmContextCleanup = false,
  ) => {
    return {
      settings: normalizeSettings(next),
      persistedSettings: normalizeSettings(baseline.settings),
      catalogDrafts: persistedCatalogDrafts(),
      previousActiveRelayId: baseline.settings.activeRelayId,
      confirmContextCleanup,
      draftRevision: providerCommitState.current.latestRevision + 1,
      expectedProviderFingerprint: baseline.provider_fingerprint,
    };
  };

  const commitProviderTopology = async (
    next: BackendSettings,
    kind: Exclude<ProviderMutationKind, "detailSave" | "setCurrent">,
    copySourceProfileId?: string,
  ) => {
    try {
      const baseline = await providerCommitBaseline();
      if (!baseline) return false;
      const common = providerCommitCommon(baseline, next);
      const invocation = kind === "copy"
        ? buildProviderMutationInvocation({ ...common, kind, copySourceProfileId: copySourceProfileId ?? "" })
        : buildProviderMutationInvocation({ ...common, kind });
      return await submitProviderCommit(invocation);
    } catch (error) {
      showErrorNotice(t("保存供应商"), error);
      return false;
    }
  };

  const commitProviderDetail = async (
    next: BackendSettings,
    focusedProfileId: string,
    catalogDraft: ProfileCatalogDraft | null,
    focusedProfileWasPersisted: boolean,
    kind: "detailSave" | "setCurrent",
    confirmContextCleanup = false,
  ) => {
    try {
      const baseline = await providerCommitBaseline();
      if (!baseline) return false;
      const common = providerCommitCommon(baseline, next, confirmContextCleanup);
      const invocation = buildProviderMutationInvocation({
        ...common,
        kind,
        focusedProfileId,
        focusedProfileWasPersisted,
        catalogDrafts: catalogDraft ? [catalogDraft] : [],
      });
      return await submitProviderCommit(invocation);
    } catch (error) {
      showErrorNotice(t("保存供应商"), error);
      return false;
    }
  };

  const switchRelayProfile = async (
    next: BackendSettings,
    previousActiveRelayId = settingsForm.activeRelayId,
    catalogDraftOverride?: ProfileCatalogDraft,
  ) => {
    if (relaySwitching) {
      showNotice(t("供应商切换中"), t("上一次切换还没有完成，请稍后再试。"), "failed");
      return;
    }
    const switchSettings = normalizeSettings(next);
    if (!switchSettings.relayProfilesEnabled) {
      showNotice(t("供应商配置已关闭"), t("当前不会写入 Codex live 配置。打开供应商配置总开关后再切换。"), "failed");
      return;
    }
    const targetBeforeSnapshot = activeRelayProfile(switchSettings);
    logDiagnostic("switchRelayProfile.start", {
      currentRelayId: settingsForm.activeRelayId,
      targetRelayId: switchSettings.activeRelayId,
      targetRelayName: targetBeforeSnapshot.name,
      targetRelayMode: targetBeforeSnapshot.relayMode,
    });
    const selectedBeforeSave = activeRelayProfile(switchSettings);
    const validationError = relayProfileSwitchValidation(selectedBeforeSave);
    if (validationError) {
      logDiagnostic("switchRelayProfile.validation_failed", {
        targetRelayId: selectedBeforeSave.id,
        targetRelayName: selectedBeforeSave.name,
        error: validationError,
      });
      showNotice(t("供应商配置可能不正确"), validationError, "failed");
      return;
    }
    const selectedAfterSave = activeRelayProfile(switchSettings);
    const selectedCatalog = modelCatalog?.profiles.find((item) => item.profileId === selectedAfterSave.id);
    const selectedCatalogMode = catalogDraftOverride?.mode ?? selectedCatalog?.mode;
    const contextConflicts = selectedCatalogMode && managedCatalogMode(selectedCatalogMode)
      ? providerManagedContextConflictKeys(selectedAfterSave, relayFiles?.configContents ?? "")
      : [];
    const confirmContextCleanup = contextConflicts.length
      ? window.confirm(tf("切换到托管目录将移除这些全局上下文设置：\n\n{0}", [contextConflicts.join("\n")]))
      : false;
    if (contextConflicts.length && !confirmContextCleanup) return;

    logDiagnostic("switchRelayProfile.apply_start", {
      targetRelayId: selectedAfterSave.id,
      targetRelayName: selectedAfterSave.name,
      previousActiveRelayId,
    });
    setRelaySwitching(true);
    try {
      const selectedCatalogDraft = isAggregateRelayProfile(selectedAfterSave)
        || selectedAfterSave.protocol === "chatCompletions"
        ? null
        : catalogDraftOverride ?? catalogDraftForProfile(selectedAfterSave);
      if (!isAggregateRelayProfile(selectedAfterSave)
        && selectedAfterSave.protocol !== "chatCompletions"
        && !selectedCatalogDraft) {
        showNotice(t("模型目录不可用"), t("当前供应商的完整模型目录状态尚未加载，请刷新后重试。"), "failed");
        return;
      }
      const committed = await commitProviderDetail(
        switchSettings,
        selectedAfterSave.id,
        selectedCatalogDraft,
        true,
        "setCurrent",
        confirmContextCleanup,
      );
      if (!committed) {
        logDiagnostic("switchRelayProfile.apply_no_result", {
          targetRelayId: selectedAfterSave.id,
        });
        return;
      }
      logDiagnostic("switchRelayProfile.ok", {
        targetRelayId: selectedAfterSave.id,
        launchMode: switchSettings.launchMode,
        status: "ok",
      });
      await adaptSessionsAfterSwitch();
    } finally {
      setRelaySwitching(false);
    }
  };

  const openExternalUrl = async (url: string) => {
    const result = await run(() => call<CommandResult<Record<string, unknown>>>("open_external_url", { url }));
    if (result) {
      showResultNotice(t("打开链接"), result, { silentSuccess: true });
    }
  };

  const showNotice = (title: string, message: string, status?: Status, detail?: string | null) => {
    setNotice({ title, message: t(message), detail: detail ?? null, status });
  };

  // A raw thrown error is diagnostic, not a sentence: lead with plain language, keep the raw
  // text copyable behind 详情.
  const showErrorNotice = (title: string, error: unknown) => {
    showNotice(title, t("调用后端失败，请重试一次；原始错误在详情里。"), "failed", stringifyError(error));
  };

  const showResultNotice = (
    title: string,
    result: Pick<CommandResult<unknown>, "message" | "status">,
    options: { silentSuccess?: boolean } = {},
  ) => {
    if (options.silentSuccess && isSuccessStatus(result.status)) return;
    showNotice(title, result.message, result.status);
  };

  // In-app update: one silent check at startup. A failed check (offline, endpoint
  // unreachable) is silence, not a notice; an explicit install failure reports normally.
  const [appUpdate, setAppUpdate] = useState<{ update: AvailableAppUpdate; phase: AppUpdatePhase } | null>(null);
  useEffect(() => {
    let cancelled = false;
    checkForAppUpdate()
      .then((update) => {
        if (cancelled || !update) return;
        setAppUpdate({ update, phase: { kind: "available", version: update.version } });
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);
  const installAppUpdate = async () => {
    if (!appUpdate || appUpdate.phase.kind === "downloading" || appUpdate.phase.kind === "installing") return;
    const { update } = appUpdate;
    // Acknowledge the click at once — the first real download event is a network round-trip away.
    setAppUpdate({ update, phase: { kind: "downloading", version: update.version, received: 0, total: null } });
    try {
      await update.downloadAndInstall((event) => {
        setAppUpdate((current) => (current ? { update, phase: appUpdatePhaseAfterEvent(current.phase, event) } : current));
      });
      await relaunch();
    } catch (error) {
      setAppUpdate({ update, phase: { kind: "failed", version: update.version } });
      showErrorNotice(t("应用更新"), error);
    }
  };

  useEffect(() => {
    void Promise.all([
      refreshSettings(true),
      refreshRelay(true),
      refreshRelayFiles(true),
      refreshEnvConflicts(true),
      refreshModelCatalog(true),
    ]);
    const scheduleMaintenance = () => {
      void refreshSessionLifecycle(true).then((result) => {
        if (result?.archiveEnabled) void runArchiveMaintenance();
      });
    };
    const maintenanceTimer = window.setTimeout(scheduleMaintenance, 1500);
    const maintenanceInterval = window.setInterval(scheduleMaintenance, 15 * 60 * 1000);
    return () => {
      window.clearTimeout(maintenanceTimer);
      window.clearInterval(maintenanceInterval);
    };
  }, []);

  useEffect(() => {
    if (getLanguage() === "en") {
      void invoke("update_tray_labels", {
        showLabel: "Show window",
        quitLabel: "Quit",
        windowTitle: "Codex Minus",
      });
    }
  }, []);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    document.documentElement.classList.toggle("light", theme === "light");
    window.localStorage.setItem("codex-plus-theme", theme);
  }, [theme]);

  const actions = useMemo(
    () => ({
      refreshCurrent: () => navigate(route),
      saveSettings,
      saveSettingsValue,
      refreshSettings,
      refreshRelay,
      refreshRelayFiles,
      refreshModelCatalog,
      adoptExternalModelCatalog,
      refreshEnvConflicts,
      removeEnvConflicts,
      refreshLocalSessions,
      deleteLocalSession,
      deleteLocalSessions,
      refreshSessionLifecycle,
      refreshArchivePreview,
      saveSessionLifecycle,
      enableSessionArchiving,
      runArchiveMaintenance,
      archiveOrRestoreSession,
      refreshProviderCompatibility,
      adaptActiveSessions,
      openExternalUrl,
      extractRelayCommonConfig,
      testRelayProfile,
      diagnoseRelayProfile,
      inspectProviderNativeCapabilities,
      transformProviderNativeCapability,
      fetchRelayProfileModels,
      commitProviderTopology,
      commitProviderDetail,
      switchRelayProfile,
      relaySwitching,
      showMessage: async (title: string, message: string, status?: Status, detail?: string | null) => showNotice(title, message, status, detail),
      toggleTheme: () => setTheme((current) => (current === "dark" ? "light" : "dark")),
    }),
    [
      route,
      settingsForm,
      settings,
      theme,
      relayFiles,
      localSessions,
      envConflicts,
      modelCatalogLoading,
      relaySwitching,
      sessionArchiveView,
      sessionLifecycle,
      archiveMaintenanceRunning,
      providerCompatibility,
      providerCompatibilityLoading,
    ],
  );

  return (
    <div className={`shell ${theme}`}>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-copy">
            <div className="brand-title-row">
              <div className="brand-title">Codex Minus</div>
            </div>
            <div className="brand-subtitle">{t("管理控制台")}</div>
          </div>
        </div>
        <nav className="nav">
          {routes.map((item) => {
            const Icon = item.icon;
            return (
            <button
              className={`nav-item ${route === item.id ? "active" : ""}`}
              key={item.id}
              onClick={() => void navigate(item.id)}
              title={item.label}
              type="button"
            >
              <span className="nav-icon">
                <Icon className="h-4 w-4" aria-hidden="true" />
              </span>
              <span className="nav-label">{item.label}</span>
              {item.badge ? <span className="nav-badge">{item.badge}</span> : null}
            </button>
          );
          })}
        </nav>
        {appUpdate ? (
          <div className="update-banner" data-app-update-banner="true">
            <span>{tf(appUpdateBanner(appUpdate.phase).text.key, appUpdateBanner(appUpdate.phase).text.args)}</span>
            {appUpdateBanner(appUpdate.phase).action ? (
              <Button onClick={() => void installAppUpdate()} size="sm" variant="outline">
                {t(appUpdateBanner(appUpdate.phase).action ?? "")}
              </Button>
            ) : null}
          </div>
        ) : null}
      </aside>
      <main className="workspace">
        <header className="topbar">
          <div>
            <h1>{routeTitle(route)}</h1>
            <p>{routeSubtitle(route)}</p>
          </div>
          <div className="topbar-actions">
            <Button
              onClick={() => toggleLanguage()}
              size="icon"
              title={getLanguage() === "en" ? t("切换到中文") : t("切换到英文")}
              variant="outline"
            >
              <Languages className="h-4 w-4" />
            </Button>
            <Button
              onClick={actions.toggleTheme}
              size="icon"
              title={theme === "dark" ? t("切换到浅色") : t("切换到深色")}
              variant="outline"
            >
              {theme === "dark" ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            </Button>
            <Button onClick={() => void actions.refreshCurrent()} size="icon" title={t("刷新当前页面")} variant="outline">
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </header>
        <section className="screen">
          <div className={route === "relay" ? undefined : "hidden"}>
            <RelayScreen
              settings={settings}
              relayFiles={relayFiles}
              modelCatalog={modelCatalog}
              modelCatalogLoading={modelCatalogLoading}
              envConflicts={envConflicts}
              form={settingsForm}
              onFormChange={setSettingsForm}
              actions={actions}
            />
          </div>
          <div className={route === "sessions" ? undefined : "hidden"}>
            <SessionsScreen
              sessions={localSessions}
              archiveView={sessionArchiveView}
              lifecycle={sessionLifecycle}
              archivePreview={archivePreview}
              archiveMaintenance={archiveMaintenance}
              archiveMaintenanceRunning={archiveMaintenanceRunning}
              providerCompatibility={providerCompatibility}
              providerCompatibilityLoading={providerCompatibilityLoading}
              actions={actions}
            />
          </div>
        </section>
      </main>
      {notice ? (
        <NoticeDialog
          key={`${notice.title}-${notice.message}-${notice.status ?? ""}`}
          notice={notice}
          onClose={() => setNotice(null)}
        />
      ) : null}
      {confirmDialog ? (
        <ConfirmDialog
          confirm={confirmDialog}
          onCancel={() => {
            confirmDialog.resolve(false);
            setConfirmDialog(null);
          }}
          onConfirm={() => {
            confirmDialog.resolve(true);
            setConfirmDialog(null);
          }}
        />
      ) : null}
    </div>
  );
}

type Actions = {
  refreshCurrent: () => Promise<void>;
  saveSettings: () => Promise<void>;
  saveSettingsValue: (settings: BackendSettings, silent?: boolean) => Promise<boolean>;
  refreshSettings: (silent?: boolean) => Promise<BackendSettings | null>;
  refreshRelay: () => Promise<void>;
  refreshRelayFiles: () => Promise<RelayFilesResult | null>;
  refreshModelCatalog: (silent?: boolean) => Promise<ModelCatalogStatusResult | null>;
  adoptExternalModelCatalog: (profileId: string, commit?: boolean, preview?: AdoptionPreviewResult, acceptVersionMismatch?: boolean, confirmContextCleanup?: boolean) => Promise<AdoptionPreviewResult | null>;
  refreshEnvConflicts: (silent?: boolean) => Promise<EnvConflictsResult | null>;
  removeEnvConflicts: (names: string[]) => Promise<void>;
  refreshLocalSessions: (silent?: boolean, archived?: boolean, cursor?: string) => Promise<LocalSessionsResult | null>;
  refreshSessionLifecycle: (silent?: boolean) => Promise<SessionLifecycleSettingsResult | null>;
  refreshArchivePreview: (retentionDays?: number, silent?: boolean) => Promise<ArchivePreviewResult | null>;
  saveSessionLifecycle: (patch: Partial<SessionLifecycleSettingsResult>, silent?: boolean) => Promise<SessionLifecycleSettingsResult | null>;
  enableSessionArchiving: (retentionDays: number) => Promise<void>;
  runArchiveMaintenance: (force?: boolean) => Promise<ArchiveMaintenanceResult | null>;
  archiveOrRestoreSession: (session: LocalSession, archived: boolean) => Promise<void>;
  refreshProviderCompatibility: (silent?: boolean) => Promise<ProviderCompatibilityResult | null>;
  adaptActiveSessions: () => Promise<void>;
  deleteLocalSession: (session: LocalSession) => Promise<void>;
  deleteLocalSessions: (sessions: LocalSession[]) => Promise<void>;
  openExternalUrl: (url: string) => Promise<void>;
  extractRelayCommonConfig: (configContents: string) => Promise<ExtractRelayCommonConfigResult | null>;
  testRelayProfile: (profile: RelayProfile) => Promise<void>;
  diagnoseRelayProfile: (profile: RelayProfile) => Promise<ProviderDoctorResult | null>;
  inspectProviderNativeCapabilities: (profileId: string) => Promise<ProviderDetailInspectionMetadata | null>;
  transformProviderNativeCapability: (invocation: ProviderDetailTransformInvocation<RelayProfile>) => Promise<ProviderDetailTransformResponse<RelayProfile>>;
  fetchRelayProfileModels: (profile: RelayProfile) => Promise<string[] | null>;
  commitProviderTopology: (settings: BackendSettings, kind: Exclude<ProviderMutationKind, "detailSave" | "setCurrent">, copySourceProfileId?: string) => Promise<boolean>;
  commitProviderDetail: (settings: BackendSettings, focusedProfileId: string, catalogDraft: ProfileCatalogDraft | null, focusedProfileWasPersisted: boolean, kind: "detailSave" | "setCurrent", confirmContextCleanup?: boolean) => Promise<boolean>;
  switchRelayProfile: (settings: BackendSettings, previousActiveRelayId?: string, catalogDraftOverride?: ProfileCatalogDraft) => Promise<void>;
  relaySwitching: boolean;
  showMessage: (title: string, message: string, status?: Status, detail?: string | null) => Promise<void>;
  toggleTheme: () => void;
};

function RelayScreen({
  settings: _settings,
  relayFiles,
  modelCatalog,
  modelCatalogLoading,
  envConflicts,
  form,
  onFormChange,
  actions,
}: {
  settings: SettingsResult | null;
  relayFiles: RelayFilesResult | null;
  modelCatalog: ModelCatalogStatusResult | null;
  modelCatalogLoading: boolean;
  envConflicts: EnvConflictsResult | null;
  form: BackendSettings;
  onFormChange: (value: BackendSettings) => void;
  actions: Actions;
}) {
  const normalized = normalizeSettings(form);
  const [detailProfileId, setDetailProfileId] = useState<string | null>(null);
  const [newProfileDraft, setNewProfileDraft] = useState<RelayProfile | null>(null);
  const detailProfile = newProfileDraft || (detailProfileId
    ? normalized.relayProfiles.find((profile) => profile.id === detailProfileId) || null
    : null);
  const isNewProfile = !!newProfileDraft;
  const catalogProfile = detailProfile
    ? modelCatalog?.profiles.find((item) => item.profileId === detailProfile.id) ?? null
    : null;
  const saveRelaySettings = async (
    next: BackendSettings,
    kind: Exclude<ProviderMutationKind, "detailSave" | "setCurrent">,
    copySourceProfileId?: string,
  ) => {
    return actions.commitProviderTopology(next, kind, copySourceProfileId);
  };
  const createNewAggregateProfile = () => {
    const draft = createAggregateRelayProfile(normalized);
    setDetailProfileId(null);
    setNewProfileDraft(draft);
    if (!normalizeAggregateConfig(draft.aggregate, aggregateMemberCandidates(normalized, draft.id)).members.length) {
      void actions.showMessage(
        t("添加聚合供应商"),
        t("已打开聚合供应商详情；请先添加或完善至少 1 个普通 API 供应商的 Base URL / Key，再勾选为成员。"),
        "failed",
      );
    }
  };
  const editRelayProfile = async (profileId: string) => {
    setNewProfileDraft(null);
    setDetailProfileId(
      normalized.relayProfiles.some((item) => item.id === profileId) ? profileId : null,
    );
  };
  useEffect(() => {
    if (!newProfileDraft && detailProfileId && !normalized.relayProfiles.some((profile) => profile.id === detailProfileId)) {
      setDetailProfileId(null);
    }
  }, [detailProfileId, newProfileDraft, normalized.relayProfiles]);
  useEffect(() => {
    if (!newProfileDraft && detailProfileId === normalized.activeRelayId) {
      void actions.refreshRelayFiles();
    }
  }, [detailProfileId, newProfileDraft, normalized.activeRelayId]);

  if (detailProfile) {
    return (
      <RelayProfileDetail
        profile={detailProfile}
        relayFiles={relayFiles}
        modelCatalog={modelCatalog}
        catalogProfile={catalogProfile}
        form={normalized}
        isNew={isNewProfile}
        onBack={() => {
          setNewProfileDraft(null);
          setDetailProfileId(null);
        }}
        onSaved={() => {
          setNewProfileDraft(null);
          setDetailProfileId(null);
        }}
        actions={actions}
      />
    );
  }

  return (
    <>
      <Panel>
        <CardHead title={t("供应商列表")} detail={tf("{0} 个供应商配置；可拖动排序，点编辑进入详情", [normalized.relayProfiles.length])} />
        <CardContent>
          <EnvConflictNotice envConflicts={envConflicts} actions={actions} />
          <label className="switch-row relay-master-switch">
            <input
              checked={normalized.relayProfilesEnabled}
              onChange={(event) => {
                const next = { ...normalized, relayProfilesEnabled: event.currentTarget.checked };
                void saveRelaySettings(next, "enablement");
              }}
              type="checkbox"
            />
            <span>
              <strong>{t("启用供应商配置切换")}</strong>
              <small>{t("关闭后本工具不会在手动切换时写入 Codex 的 live provider 配置；auth.json 始终由官方客户端管理。")}</small>
            </span>
            <ToggleVisual />
          </label>
          <div className="relay-add-row">
            <Button
              variant="secondary"
              onClick={() => {
                setNewProfileDraft(createRelayProfile(normalized));
                setDetailProfileId(null);
              }}
            >
              <Plus className="h-4 w-4" />
              {t("添加供应商")}
            </Button>
            <Button
              variant="secondary"
              onClick={createNewAggregateProfile}
            >
              <Plus className="h-4 w-4" />
              {t("添加聚合供应商")}
            </Button>
          </div>
          <RelayProfileList
            form={normalized}
            onEdit={(profileId) => void editRelayProfile(profileId)}
            onFormChange={saveRelaySettings}
            disabled={!normalized.relayProfilesEnabled || actions.relaySwitching}
            actions={actions}
          />
        </CardContent>
      </Panel>
    </>
  );
}

function CatalogProfileEditor({
  catalog,
  draft,
  onDraftChange,
  profile,
  onProfileEdit,
  summary,
  isNew = false,
  actions,
}: {
  catalog: ModelCatalogStatusResult | null;
  draft: ProfileCatalogDraft;
  onDraftChange: (draft: ProfileCatalogDraft) => void;
  profile: RelayProfile;
  onProfileEdit: (patch: Partial<RelayProfile>) => void;
  summary: ProfileCatalogSummary | null;
  isNew?: boolean;
  actions: Actions;
}) {
  const { mode, modeExplicit, upstreamTopology, overlay } = draft;
  const editingAvailability = catalogEditingAvailability(isNew || !!summary?.managedAvailable);

  if (!isNew && !summary?.managedAvailable) {
    return (
      <section className="catalog-profile-editor unavailable">
        <div className="catalog-editor-head">
          <div>
            <strong>{t("模型目录")}</strong>
            <span>{summary?.actionRequired ? t(catalogActionRequiredLabel(summary.actionRequired)) : t("此供应商模式不支持托管模型目录。")}</span>
          </div>
          <UiBadge variant="outline">{t("不可用")}</UiBadge>
        </div>
      </section>
    );
  }

  const officialModels = catalog?.officialModels ?? [];
  const presentation = catalogModePresentation({
    selectedMode: mode,
    persistedMode: summary?.mode ?? null,
    generatedPath: summary?.generatedPath ?? null,
    externalPointer: summary?.externalPointer ?? null,
    restartRequired: summary?.restartRequired ?? false,
    customModelCount: overlay.custom.length,
  });
  // The rows this table shows, and the only official models the generated catalog will offer.
  const shownOfficial = officialModels.filter((model) => officialModelIsVisible(overlay, model));
  const draftError = validateCatalogDraft(
    overlay,
    mode,
    codexModelFromConfig(profile.configContents) || profile.model,
    shownOfficial.map((model) => model.slug),
  );
  // The model Codex starts on is one of these rows. It used to be a free-text field elsewhere in
  // the editor, which let a profile name a model its own catalog did not contain and gave the user
  // two places to look for one answer.
  const selectedModel = codexModelFromConfig(profile.configContents) || profile.model;
  const startupModelGroup = `startup-model-${draft.profileId}`;
  const selectStartupModel = (slug: string) => {
    if (slug) onProfileEdit({ model: slug });
  };
  // Every overlay edit goes through here, so a value the user types can never land in a mode that
  // would leave it dormant: asking for a different context window is asking for a managed catalog.
  const applyOverlay = (nextOverlay: CatalogOverlayDraft) => {
    const nextMode = catalogModeForOverlay(mode, nextOverlay);
    onDraftChange(updateCatalogProfileDraft(draft, nextMode === mode
      ? { overlay: nextOverlay }
      : { overlay: nextOverlay, mode: nextMode, modeExplicit: true }));
  };
  const setOfficialOverride = (slug: string, patch: Partial<OfficialCatalogOverride>) => {
    const next = { ...(overlay.official[slug] ?? emptyOfficialOverride()), ...patch };
    const official = { ...overlay.official };
    if (Object.values(next).every((item) => item === null)) delete official[slug];
    else official[slug] = next;
    applyOverlay({ ...overlay, official });
  };
  const updateCustom = (index: number, patch: Partial<CustomCatalogModel>) => {
    applyOverlay({
      ...overlay,
      custom: overlay.custom.map((item, itemIndex) => (itemIndex === index ? { ...item, ...patch } : item)),
    });
  };
  // The startup model is a row in this table, not a name typed somewhere else, so renaming or
  // deleting the row that is selected has to carry the selection with it. Otherwise the profile
  // would keep starting Codex on a model this catalog no longer contains.
  const renameCustom = (index: number, slug: string) => {
    const previous = overlay.custom[index]?.slug ?? "";
    const follows = customDisplayNameFollowsSlug(overlay.custom[index]?.displayName ?? "", previous);
    updateCustom(index, { slug, ...(follows ? { displayName: slug } : {}) });
    if (previous && selectedModel === previous) onProfileEdit({ model: slug });
  };
  const removeCustom = (index: number) => {
    const removed = overlay.custom[index]?.slug ?? "";
    applyOverlay({ ...overlay, custom: overlay.custom.filter((_, itemIndex) => itemIndex !== index) });
    if (removed && selectedModel === removed) onProfileEdit({ model: "" });
  };
  // Deleting an official row is a visibility wish, not a deletion: the bundled baseline still
  // carries the model, and the row has to be recoverable from the candidate strip below.
  const hideOfficial = (model: OfficialModelSummary) => {
    setOfficialOverride(model.slug, { visible: officialVisibilityOverride(model.visible, false) });
    if (selectedModel === model.slug) onProfileEdit({ model: "" });
  };
  const addCustom = (slug = "") => {
    if (slug) {
      const official = officialModels.find((model) => model.slug === slug);
      if (official) setOfficialOverride(slug, { visible: officialVisibilityOverride(official.visible, true) });
      else applyOverlay(addCatalogCandidate(overlay, slug));
      return;
    }
    applyOverlay({ ...overlay, custom: [...overlay.custom, {
      slug: "",
      displayName: "",
      description: "",
      contextWindow: 272000,
      effectiveContextWindowPercent: 100,
      visible: true,
      order: overlay.custom.length,
      supportedReasoningLevels: [],
      defaultReasoningLevel: null,
      supportedTools: [],
      toolCapabilities: null,
      templateProvenance: "user-created",
    }] });
  };
  // A list an older version mangled is repaired in one action, rather than by remembering which
  // models a Pro account routes and rebuilding them by hand.
  const proSlugs: readonly string[] = PRO_MODEL_SLUGS;
  const restoreProList = () => {
    const losses = catalogRestoreLosses({ overlay, officialModels, wanted: proSlugs });
    if (losses.length && !window.confirm(tf("还原为 Pro 列表会移除这些模型：\n\n{0}", [losses.join("\n")]))) return;
    applyOverlay(restoreCatalogList({ overlay, officialModels, wanted: proSlugs }));
    if (!proSlugs.includes(selectedModel)) onProfileEdit({ model: proSlugs[0] });
  };
  const candidates = catalogCandidateSlugs({
    overlay,
    officialModels,
    providerCandidates: summary?.customCandidates ?? [],
  });
  return (
    <section className="catalog-profile-editor">
      <div className="catalog-editor-head">
        <div>
          <strong>{t("模型")}</strong>
          <span>{t("这里就是 Codex 里能选到的模型；上下文留空表示用官方默认。")}</span>
        </div>
        <div className="catalog-editor-actions">
          {presentation.restart ? <UiBadge variant="secondary">{t("需重启 Codex")}</UiBadge> : null}
          <Button disabled={!editingAvailability.editable} onClick={restoreProList} size="sm" variant="outline">
            <RotateCcw className="h-4 w-4" />{t("还原 Pro 列表")}
          </Button>
        </div>
      </div>
      {summary?.actionRequired || draftError ? <div className="catalog-inline-error">{summary?.actionRequired ? t(catalogActionRequiredLabel(summary.actionRequired)) : catalogDraftErrorLabel(draftError)}</div> : null}
      {summary?.mode === "native-official" && mode !== "native-official" ? (
        <div className="hint-line">
          <Info className="h-4 w-4" />
          <span>{t("改过这份列表，所以保存会为这个供应商生成一份模型目录，之后需要完整退出并重开 Codex 才生效。")}</span>
        </div>
      ) : null}
      <fieldset className="catalog-editor-readonly" disabled={!editingAvailability.editable}>
        <div className="catalog-model-list">
          <div className="catalog-model-row catalog-model-row-head">
            <span>{t("启动")}</span><span>{t("模型")}</span><span>{t("上下文")}</span><span />
          </div>
          {shownOfficial.map((model) => (
            <div className="catalog-model-row" key={model.slug}>
              <input
                checked={selectedModel === model.slug}
                name={startupModelGroup}
                onChange={() => selectStartupModel(model.slug)}
                title={t("设为启动模型")}
                type="radio"
              />
              <span className="catalog-model-name">
                <strong>{appModelLabel(model.displayName)}</strong>
                <small>{model.slug}</small>
              </span>
              <Input
                inputMode="numeric"
                value={overlay.official[model.slug]?.contextWindow ?? ""}
                onChange={(event) => setOfficialOverride(model.slug, { contextWindow: positiveNumberOrNull(event.currentTarget.value) })}
                placeholder={model.contextWindow ? String(model.contextWindow) : t("默认")}
              />
              <Button onClick={() => hideOfficial(model)} size="icon" title={t("删除模型")} variant="ghost"><Trash2 className="h-4 w-4" /></Button>
            </div>
          ))}
          {overlay.custom.map((model, index) => (
            <div className="catalog-model-row" key={`custom-${index}`}>
              <input
                checked={!!model.slug && selectedModel === model.slug}
                disabled={!model.slug}
                name={startupModelGroup}
                onChange={() => selectStartupModel(model.slug)}
                title={t("设为启动模型")}
                type="radio"
              />
              <span className="catalog-model-name catalog-model-name-inputs">
                <Input
                  value={model.displayName}
                  onChange={(event) => updateCustom(index, { displayName: event.currentTarget.value })}
                  placeholder={t("显示名")}
                />
                <Input
                  value={model.slug}
                  onChange={(event) => renameCustom(index, event.currentTarget.value)}
                  placeholder="model-id"
                />
              </span>
              <Input
                inputMode="numeric"
                value={model.contextWindow}
                onChange={(event) => updateCustom(index, { contextWindow: positiveNumberOrDefault(event.currentTarget.value, 272000) })}
              />
              <Button onClick={() => removeCustom(index)} size="icon" title={t("删除模型")} variant="ghost"><Trash2 className="h-4 w-4" /></Button>
            </div>
          ))}
          <div className="catalog-list-foot">
            {candidates.length ? (
              <div className="catalog-candidates">
                {candidates.map((slug) => (
                  <button key={slug} onClick={() => addCustom(slug)} title={knownRelayModel(slug)?.displayName ?? slug} type="button"><Plus className="h-3 w-3" />{slug}</button>
                ))}
              </div>
            ) : <span />}
            <Button onClick={() => addCustom()} size="sm" variant="secondary"><Plus className="h-4 w-4" />{t("添加模型")}</Button>
          </div>
        </div>
      </fieldset>
    </section>
  );
}

function EnvConflictNotice({
  envConflicts,
  actions,
}: {
  envConflicts: EnvConflictsResult | null;
  actions: Actions;
}) {
  const conflicts = envConflicts?.conflicts ?? [];
  if (!conflicts.length) return null;
  const names = Array.from(new Set(conflicts.map((conflict) => conflict.name))).sort();
  return (
    <div className="env-conflict-notice">
      <div className="env-conflict-icon">
        <ShieldAlert className="h-4 w-4" />
      </div>
      <div className="env-conflict-body">
        <strong>{t("检测到 OPENAI 环境变量")}</strong>
        <p>{t("这些变量可能覆盖当前供应商的 provider 路由；CODEX_HOME 不会被清理。")}</p>
        <div className="env-conflict-tags">
          {conflicts.map((conflict) => (
            <span key={`${conflict.source}-${conflict.name}`}>
              {conflict.name}
              <small>{envConflictSourceLabel(conflict.source)}</small>
            </span>
          ))}
        </div>
      </div>
      <div className="env-conflict-actions">
        <Button onClick={() => void actions.removeEnvConflicts(names)} size="sm">
          <Trash2 className="h-4 w-4" />
          {t("删除")}
        </Button>
        <Button onClick={() => void actions.refreshEnvConflicts(false)} size="sm" variant="secondary">
          <RefreshCw className="h-4 w-4" />
          {t("检测")}
        </Button>
      </div>
    </div>
  );
}

function envConflictSourceLabel(source: string): string {
  if (source === "process") return t("当前进程");
  if (source === "user") return t("用户环境");
  return source || t("环境变量");
}

const SESSION_LIST_PAGE_SIZE = 100;

function SessionsScreen({
  sessions,
  archiveView,
  lifecycle,
  archivePreview,
  archiveMaintenance,
  archiveMaintenanceRunning,
  providerCompatibility,
  providerCompatibilityLoading,
  actions,
}: {
  sessions: LocalSessionsResult | null;
  archiveView: boolean;
  lifecycle: SessionLifecycleSettingsResult | null;
  archivePreview: ArchivePreviewResult | null;
  archiveMaintenance: ArchiveMaintenanceResult | null;
  archiveMaintenanceRunning: boolean;
  providerCompatibility: ProviderCompatibilityResult | null;
  providerCompatibilityLoading: boolean;
  actions: Actions;
}) {
  const items = sessions?.sessions ?? [];
  const activeCount = sessions?.activeCount ?? 0;
  const archivedCount = sessions?.archivedCount ?? 0;
  const [retentionDays, setRetentionDays] = useState(lifecycle?.retentionDays ?? 30);
  const [selectedSessionIds, setSelectedSessionIds] = useState<Set<string>>(() => new Set());
  const [selectionMode, setSelectionMode] = useState(false);
  const [bulkDeleting, setBulkDeleting] = useState(false);
  const selectedSessions = useMemo(() => items.filter((session) => selectedSessionIds.has(session.id)), [items, selectedSessionIds]);
  const selectedCount = selectedSessions.length;
  const allSelected = items.length > 0 && selectedCount === items.length;

  useEffect(() => setRetentionDays(lifecycle?.retentionDays ?? 30), [lifecycle?.retentionDays]);

  useEffect(() => {
    const itemIds = new Set(items.map((session) => session.id));
    setSelectedSessionIds((current) => {
      const next = new Set(Array.from(current).filter((id) => itemIds.has(id)));
      return next.size === current.size ? current : next;
    });
  }, [items]);

  useEffect(() => {
    setSelectedSessionIds(new Set());
    setSelectionMode(false);
  }, [archiveView]);

  const toggleSessionSelection = (sessionId: string, checked: boolean) => {
    setSelectedSessionIds((current) => {
      const next = new Set(current);
      if (checked) {
        next.add(sessionId);
      } else {
        next.delete(sessionId);
      }
      return next;
    });
  };

  const selectAllSessions = () => setSelectedSessionIds(new Set(items.map((session) => session.id)));

  const clearSelectedSessions = () => setSelectedSessionIds(new Set());

  const deleteSelectedSessions = async () => {
    setBulkDeleting(true);
    try {
      await actions.deleteLocalSessions(selectedSessions);
    } finally {
      setBulkDeleting(false);
    }
  };

  const toggleArchivePolicy = async (enabled: boolean) => {
    if (enabled) {
      await actions.enableSessionArchiving(retentionDays);
      return;
    }
    await actions.saveSessionLifecycle({ archiveEnabled: false, retentionDays });
  };

  // The days input saves itself like the toggle does; the preview follows the saved value.
  useEffect(() => {
    if (!lifecycle || retentionDays === lifecycle.retentionDays) return;
    const timer = window.setTimeout(() => {
      void actions.saveSessionLifecycle({ retentionDays }, true).then((saved) => {
        if (saved) void actions.refreshArchivePreview(retentionDays, true);
      });
    }, 800);
    return () => window.clearTimeout(timer);
  }, [retentionDays, lifecycle, actions]);

  return (
    <>
      <Panel>
        <CardHead title={t("会话生命周期")} detail={t("活动会话与原生归档")} />
        <CardContent>
          <div className="metric-list">
            <Metric label={t("活动会话")} value={tf("{0} 个", [activeCount])} />
            <Metric label={t("已归档")} value={tf("{0} 个", [archivedCount])} />
            <Metric label={t("自动归档")} value={lifecycle?.archiveEnabled ? t("已启用") : t("未启用")} />
            <Metric label={t("上次检查")} value={lifecycle?.lastCompletedAtMs ? formatTime(lifecycle.lastCompletedAtMs) : t("尚未执行")} />
            <Metric label={t("数据库")} value={sessions?.dbPath ?? "~/.codex/sqlite/*.db"} />
          </div>
          <div className="session-policy-row">
            <label className="feature-toggle">
              <input
                checked={lifecycle?.archiveEnabled ?? false}
                onChange={(event) => void toggleArchivePolicy(event.currentTarget.checked)}
                type="checkbox"
              />
              <span>
                <strong>{t("定期归档旧会话")}</strong>
                <small>{tf("超过 {0} 天未活动", [retentionDays])}</small>
              </span>
              <span className="toggle-switch-visual" aria-hidden="true"><span className="toggle-switch-thumb" /></span>
            </label>
            <Field label={t("保留天数")}>
              <Input
                max={3650}
                min={1}
                onChange={(event) => setRetentionDays(Math.max(1, Math.min(3650, Number(event.currentTarget.value) || 1)))}
                type="number"
                value={retentionDays}
              />
            </Field>
          </div>
          <Toolbar>
            <Button disabled={!lifecycle?.archiveEnabled || archiveMaintenanceRunning} onClick={() => void actions.runArchiveMaintenance(true)}>
              <RefreshCw className="h-4 w-4" />
              {archiveMaintenanceRunning ? t("检查中…") : t("立即检查")}
            </Button>
          </Toolbar>
          {archivePreview ? (
            <div className="hint-line">
              <Info className="h-4 w-4" />
              <span>{tf("截止 {0}，候选 {1} 个；位置：{2}。{3}", [formatTime(archivePreview.cutoffAtMs), archivePreview.candidateCount, archivePreview.destination, t(archivePreview.capability.message)])}</span>
            </div>
          ) : null}
          {archiveMaintenance ? (
            <div className="hint-line" data-tone={archiveMaintenance.deferred || archiveMaintenance.failedCount ? "warn" : undefined}>
              {archiveMaintenance.deferred || archiveMaintenance.failedCount ? <TriangleAlert className="h-4 w-4" /> : archiveMaintenance.due ? <CheckCircle2 className="h-4 w-4" /> : <Info className="h-4 w-4" />}
              <span>
                {archiveMaintenance.due && !archiveMaintenance.deferred
                  ? tf("候选 {0}，已归档 {1}，跳过 {2}，失败 {3}。", [archiveMaintenance.candidateCount, archiveMaintenance.archivedCount, archiveMaintenance.skippedCount, archiveMaintenance.failedCount])
                  : t(archiveMaintenance.message)}
              </span>
            </div>
          ) : null}
        </CardContent>
      </Panel>
      <Panel>
        <CardHead title={t("供应商兼容性")} detail={providerCompatibility?.currentProvider ?? t("读取当前配置")} />
        <CardContent>
          <div className="metric-list">
            <Metric label={t("当前 provider")} value={providerCompatibility?.currentProvider ?? t("未检查")} />
            <Metric label={t("活动会话")} value={tf("{0} 个", [providerCompatibility?.activeCount ?? 0])} />
            <Metric label={t("需要适配")} value={tf("{0} 个", [providerCompatibility?.mismatchCount ?? 0])} />
          </div>
          <Toolbar>
            <Button disabled={providerCompatibilityLoading} onClick={() => void actions.refreshProviderCompatibility()} variant="outline">
              <RefreshCw className="h-4 w-4" />
              {providerCompatibilityLoading ? t("检查中…") : t("重新检查")}
            </Button>
            <Button
              disabled={!providerCompatibility?.adaptationAvailable || !providerCompatibility.mismatchCount}
              onClick={() => void actions.adaptActiveSessions()}
            >
              <RefreshCw className="h-4 w-4" />
              {t("适配到当前 provider")}
            </Button>
          </Toolbar>
          <label className="feature-toggle">
            <input
              checked={lifecycle?.autoAdaptProviderOnSwitch ?? true}
              onChange={(event) => void actions.saveSessionLifecycle({ autoAdaptProviderOnSwitch: event.currentTarget.checked })}
              type="checkbox"
            />
            <span>
              <strong>{t("切换供应商后自动适配")}</strong>
              <small>{t("仅活动会话，写前自动备份；归档不受影响")}</small>
            </span>
            <span className="toggle-switch-visual" aria-hidden="true"><span className="toggle-switch-thumb" /></span>
          </label>
          {providerCompatibility ? (
            <div className="hint-line">
              <Info className="h-4 w-4" />
              <span>{providerCompatibility.mismatchCount ? t(providerCompatibility.adaptationMessage) : t("活动会话的 provider 已兼容当前配置。")}</span>
            </div>
          ) : null}
        </CardContent>
      </Panel>
      <Panel>
        <CardHead title={t("本地会话")} detail={items.length ? t("按更新时间倒序显示") : t("当前列表为空")} />
        <CardContent>
          <div className="session-view-tabs segmented" role="tablist">
            <button className={!archiveView ? "active" : ""} onClick={() => void actions.refreshLocalSessions(true, false)} role="tab" type="button">
              {t("活动")} <small>{activeCount}</small>
            </button>
            <button className={archiveView ? "active" : ""} onClick={() => void actions.refreshLocalSessions(true, true)} role="tab" type="button">
              {t("已归档")} <small>{archivedCount}</small>
            </button>
          </div>
          {items.length ? (
            <>
              <div className="session-list-toolbar">
                <span className="session-selection-summary">
                  {selectionMode ? `${t("已选择")} ${selectedCount} / ${items.length} ${t("个会话")}` : null}
                </span>
                <div className="session-selection-actions">
                  {selectionMode ? (
                    <>
                      <Button disabled={allSelected || bulkDeleting} onClick={selectAllSessions} size="sm" variant="outline">
                        {t("全选当前列表")}
                      </Button>
                      <Button disabled={!selectedCount || bulkDeleting} onClick={clearSelectedSessions} size="sm" variant="outline">
                        {t("清空选择")}
                      </Button>
                      <Button disabled={!selectedCount || bulkDeleting} onClick={() => void deleteSelectedSessions()} size="sm" variant="outline">
                        <Trash2 className="h-4 w-4" />
                        {bulkDeleting ? t("正在删除…") : t("删除已选")}
                      </Button>
                      <Button disabled={bulkDeleting} onClick={() => { setSelectionMode(false); clearSelectedSessions(); }} size="sm" variant="ghost">
                        {t("取消")}
                      </Button>
                    </>
                  ) : (
                    <Button onClick={() => setSelectionMode(true)} size="sm" variant="outline">
                      {t("多选")}
                    </Button>
                  )}
                </div>
              </div>
              <div className="session-list">
                {items.map((session) => {
                  const selected = selectedSessionIds.has(session.id);
                  return (
                    <div className="session-row" data-selection-mode={selectionMode} data-selected={selected} key={session.id}>
                      {selectionMode ? (
                        <label className="session-select" title={t("选择会话")}>
                          <input
                            aria-label={tf("选择会话 {0}", [session.title || session.id])}
                            checked={selected}
                            onChange={(event) => toggleSessionSelection(session.id, event.currentTarget.checked)}
                            type="checkbox"
                          />
                        </label>
                      ) : null}
                      <div className="session-main">
                        <strong>{session.title || t("未命名会话")}</strong>
                        <span>{session.id}</span>
                        <small>{session.cwd || t("未记录项目路径")}</small>
                      </div>
                      <div className="session-meta">
                        <Badge status={session.archived ? "archived" : "ok"} />
                        <span>{session.modelProvider || t("provider 未记录")}</span>
                        <span>{formatTime(session.updatedAtMs ?? 0)}</span>
                      </div>
                      <div className="session-row-actions">
                        <Button variant="outline" onClick={() => void actions.archiveOrRestoreSession(session, !archiveView)}>
                          {archiveView ? <ArchiveRestore className="h-4 w-4" /> : <Archive className="h-4 w-4" />}
                          {archiveView ? t("恢复") : t("归档")}
                        </Button>
                        <Button className="session-delete-button" variant="outline" onClick={() => void actions.deleteLocalSession(session)}>
                          <Trash2 className="h-4 w-4" />
                          {t("删除")}
                        </Button>
                      </div>
                    </div>
                  );
                })}
              </div>
              {sessions?.nextCursor ? (
                <Toolbar>
                  <Button variant="outline" onClick={() => void actions.refreshLocalSessions(true, archiveView, sessions.nextCursor ?? undefined)}>
                    {tf("显示更多（已显示 {0} 个）", [items.length])}
                  </Button>
                </Toolbar>
              ) : null}
            </>
          ) : (
            <div className="empty">{archiveView ? t("没有已归档会话。") : t("没有活动会话。")}</div>
          )}
        </CardContent>
      </Panel>
    </>
  );
}

function RelayProfileList({
  form,
  onFormChange,
  onEdit,
  disabled = false,
  actions,
}: {
  form: BackendSettings;
  onFormChange: (value: BackendSettings, kind: "reorder" | "copy" | "delete" | "aggregateCleanup", copySourceProfileId?: string) => void;
  onEdit: (id: string) => void;
  disabled?: boolean;
  actions: Actions;
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );
  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const next = reorderRelayProfiles(form, String(active.id), String(over.id));
    if (next !== form) onFormChange(next, "reorder");
  };
  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
      <SortableContext items={form.relayProfiles.map((profile) => profile.id)} strategy={verticalListSortingStrategy}>
        <div className="relay-profile-list">
          {form.relayProfiles.map((profile, index) => (
            <SortableRelayProfileCard
              actions={actions}
              form={form}
              index={index}
              key={profile.id}
              onEdit={onEdit}
              onFormChange={onFormChange}
              disabled={disabled}
              profile={profile}
            />
          ))}
        </div>
      </SortableContext>
    </DndContext>
  );
}

function SortableRelayProfileCard({
  form,
  profile,
  index,
  onFormChange,
  onEdit,
  disabled = false,
  actions,
}: {
  form: BackendSettings;
  profile: RelayProfile;
  index: number;
  onFormChange: (value: BackendSettings, kind: "reorder" | "copy" | "delete" | "aggregateCleanup", copySourceProfileId?: string) => void;
  onEdit: (id: string) => void;
  disabled?: boolean;
  actions: Actions;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: profile.id });
  const active = profile.id === form.activeRelayId;
  const deleteAvailable = providerDeleteAvailable(profile.id, form.activeRelayId, form.relayProfiles.length);
  const style: CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      className={`relay-profile-card ${active ? "active" : ""} ${isDragging ? "dragging" : ""}`}
      data-relay-profile-id={profile.id}
      key={profile.id}
      onKeyDown={(event) => {
        if (event.key === "Enter") onEdit(profile.id);
      }}
      ref={setNodeRef}
      style={style}
      tabIndex={0}
    >
      <button
        aria-label={t("拖动排序")}
        className="relay-drag"
        title={t("拖动排序")}
        type="button"
        {...attributes}
        {...listeners}
      >
        <GripVertical className="h-4 w-4" />
      </button>
      <span className="relay-index" title={profile.name || t("未命名供应商")}>
        {providerInitial(profile.name)}
      </span>
      <span className="relay-summary">
        <strong>{profile.name || t("未命名供应商")}</strong>
        <small>{relayModeLabel(profile.relayMode)} · {relayProtocolLabel(profile.protocol)} · {relayProfileConfigBrief(profile)}</small>
      </span>
      <span className="relay-card-actions">
        <Button
          className={`relay-use-button ${active ? "active" : ""}`}
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            if (disabled) return;
            const previousActiveRelayId = form.activeRelayId;
            const next = syncLegacyRelayFields({ ...form, activeRelayId: profile.id });
            void actions.switchRelayProfile(next, previousActiveRelayId);
          }}
          size="sm"
          title={disabled ? t("供应商切换不可用") : active ? t("当前正在使用") : t("设为当前")}
          variant={active ? "secondary" : "outline"}
        >
          <CheckCircle2 className="h-4 w-4" />
          {active ? t("使用中") : t("使用")}
        </Button>
        <span className="relay-card-extra">
          <Button
            disabled={isAggregateRelayProfile(profile)}
            onClick={(event) => {
              event.stopPropagation();
              if (isAggregateRelayProfile(profile)) return;
              void actions.testRelayProfile(profile);
            }}
            size="icon"
            title={isAggregateRelayProfile(profile) ? t("聚合供应商会在真实对话中轮转成员，请测试成员供应商") : t("发送 hi 测试")}
            variant="ghost"
          >
            <TestTube className="h-4 w-4" />
          </Button>
          <Button
            onClick={(event) => {
              event.stopPropagation();
              onEdit(profile.id);
            }}
            size="icon"
            title={t("编辑")}
            variant="ghost"
          >
            <Edit3 className="h-4 w-4" />
          </Button>
          <Button
            onClick={(event) => {
              event.stopPropagation();
              onFormChange(duplicateRelayProfile(form, profile.id), "copy", profile.id);
            }}
            size="icon"
            title={t("复制")}
            variant="ghost"
          >
            <Copy className="h-4 w-4" />
          </Button>
          <Button
            disabled={!deleteAvailable}
            onClick={(event) => {
              event.stopPropagation();
              onFormChange(removeRelayProfile(form, profile.id), "delete");
            }}
            size="icon"
            title={active ? t("当前供应商不能直接删除，请先切换到其他供应商。") : t("删除供应商")}
            variant="ghost"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </span>
      </span>
    </div>
  );
}

function RelayProfileDetail({
  profile,
  relayFiles,
  modelCatalog,
  catalogProfile,
  form,
  isNew = false,
  onBack,
  onSaved,
  actions,
}: {
  profile: RelayProfile;
  relayFiles: RelayFilesResult | null;
  modelCatalog: ModelCatalogStatusResult | null;
  catalogProfile: ProfileCatalogSummary | null;
  form: BackendSettings;
  isNew?: boolean;
  onBack: () => void;
  onSaved?: () => void;
  actions: Actions;
}) {
  const [saving, setSaving] = useState(false);
  const savingRef = useRef(false);
  const [legacyReplacementProviderId, setLegacyReplacementProviderId] = useState("");
  const fallbackCatalogMode = defaultCatalogMode(
    profile.relayMode,
    profile.officialMixApiKey,
  ) as CatalogMode;
  const initialCatalogDraft = isNew || catalogProfile
    ? catalogProfileDraft({
        profileId: profile.id,
        fallbackMode: fallbackCatalogMode,
        summary: catalogProfile,
      })
    : null;
  const [detailState, setDetailState] = useState<ProviderDetailDraftState<RelayProfile>>(() =>
    createProviderDetailDraftState({ profile, catalogDraft: initialCatalogDraft }),
  );
  const detailStateRef = useRef(detailState);
  const authoritativeCapabilityProfile = isAggregateRelayProfile(profile)
    ? normalizeAggregateRelayProfile(profile, form)
    : deriveRelayProfileFromFiles({
        ...profile,
        configContents: profile.configContents,
        authContents: "",
      });
  const authoritativeCapabilityProfileRevision = JSON.stringify(
    authoritativeCapabilityProfile,
  );
  const catalogSummaryFingerprint = JSON.stringify({
    profileId: catalogProfile?.profileId ?? null,
    mode: catalogProfile?.mode ?? null,
    modeExplicit: catalogProfile?.modeExplicit ?? null,
    effectiveHash: catalogProfile?.effectiveHash ?? null,
    restartRequired: catalogProfile?.restartRequired ?? null,
    actionRequired: catalogProfile?.actionRequired ?? null,
  });
  const authoritativeCapabilityCatalogDraft = isNew || catalogProfile
    ? catalogProfileDraft({
        profileId: profile.id,
        fallbackMode: fallbackCatalogMode,
        summary: catalogProfile,
      })
    : null;
  const updateDetailState = (next: ProviderDetailDraftState<RelayProfile>) => {
    const current = detailStateRef.current;
    if (
      next.sessionToken !== current.sessionToken
      || next.latestTransformRevision !== current.latestTransformRevision
      || JSON.stringify(next.profile) !== JSON.stringify(current.profile)
      || JSON.stringify(next.catalogDraft) !== JSON.stringify(current.catalogDraft)
    ) {
    }
    detailStateRef.current = next;
    setDetailState(next);
  };
  const draft = detailState.profile;
  const catalogDraft = detailState.catalogDraft;
  const isActive = !isNew && profile.id === form.activeRelayId;
  const nativeCapabilityView = deriveProviderNativeCapabilityView({
    inspection: detailState.inspection,
    officialAuth: {
      authenticated: relayFiles?.authStatus.authenticated ?? null,
      localPlan: "unknown",
    },
  });
  useEffect(() => {
    const nextDraft = isAggregateRelayProfile(profile)
      ? normalizeAggregateRelayProfile(profile, form)
      : deriveRelayProfileFromFiles({
          ...profile,
          configContents: profile.configContents,
          authContents: "",
        });
    const nextCatalogDraft = isNew || catalogProfile
      ? catalogProfileDraft({
          profileId: profile.id,
          fallbackMode: defaultCatalogMode(profile.relayMode, profile.officialMixApiKey) as CatalogMode,
          summary: catalogProfile,
        })
      : null;
    const nextState = createProviderDetailDraftState({
      profile: nextDraft,
      catalogDraft: nextCatalogDraft,
    });
    updateDetailState(nextState);
    setLegacyReplacementProviderId("");
    let cancelled = false;
    if (!isNew && !isAggregateRelayProfile(nextDraft)) {
      const inspectionCorrelation = beginProviderDetailInspection(nextState);
      void actions.inspectProviderNativeCapabilities(profile.id).then((inspection) => {
        if (cancelled || !inspection) return;
        const applied = applyProviderDetailInspection(
          detailStateRef.current,
          inspectionCorrelation,
          inspection,
        );
        if (applied.disposition === "applied") updateDetailState(applied.state);
      });
    }
    return () => {
      cancelled = true;
      if (detailStateRef.current.sessionToken === nextState.sessionToken) {
        detailStateRef.current = endProviderDetailSession(detailStateRef.current, "close").state;
      }
    };
  }, [profile.id, authoritativeCapabilityProfileRevision]);
  useEffect(() => {
    let cancelled = false;
    const nextCatalogDraft = isNew || catalogProfile
      ? catalogProfileDraft({
          profileId: profile.id,
          fallbackMode: defaultCatalogMode(profile.relayMode, profile.officialMixApiKey) as CatalogMode,
          summary: catalogProfile,
        })
      : null;
    if (
      detailStateRef.current.lifecycle === "active"
      && detailStateRef.current.profile.id === profile.id
      && JSON.stringify(detailStateRef.current.catalogDraft) !== JSON.stringify(nextCatalogDraft)
    ) {
      const refreshed = refreshProviderDetailCatalogDraftState(
        detailStateRef.current,
        nextCatalogDraft,
        isAggregateRelayProfile(profile)
          ? normalizeAggregateRelayProfile(profile, form)
          : deriveRelayProfileFromFiles({
              ...profile,
              configContents: profile.configContents,
              authContents: "",
            }),
      );
      updateDetailState(refreshed.state);
      const inspectionCorrelation = refreshed.inspectionCorrelation;
      if (
        inspectionCorrelation
        && !isNew
        && !isAggregateRelayProfile(refreshed.state.profile)
      ) {
        void actions.inspectProviderNativeCapabilities(profile.id).then((inspection) => {
          if (cancelled || !inspection) return;
          const applied = applyProviderDetailInspection(
            detailStateRef.current,
            inspectionCorrelation,
            inspection,
          );
          if (applied.disposition === "applied") updateDetailState(applied.state);
        });
      }
    }
    return () => {
      cancelled = true;
    };
  }, [
    profile.id,
    isNew,
    authoritativeCapabilityProfileRevision,
    catalogSummaryFingerprint,
  ]);
  const replaceDraft = (next: RelayProfile) => {
    updateDetailState(replaceProviderDetailProfile(detailStateRef.current, next));
  };
  const dispatchProviderDetailStep = (step: ProviderDetailStep<RelayProfile>): Promise<boolean> => {
    updateDetailState(step.state);
    const effect = step.effects.find((candidate) => candidate.kind === "transform");
    if (!effect || effect.kind !== "transform") return Promise.resolve(true);
    return actions.transformProviderNativeCapability(effect.invocation).then((response) => {
      const settled = settleProviderDetailTransform(
        detailStateRef.current,
        effect.correlation,
        response,
      );
      if (settled.disposition === "stale") return false;
      updateDetailState(settled.state);
      if (settled.disposition === "notApplied") {
        if (settled.state.pendingLegacyProviderIdResolution) return false;
        if (
          response.status === "confirmationRequired"
          && settled.state.pendingConfirmation
        ) {
          const accepted = window.confirm(providerTransitionConfirmationMessage(settled.state));
          return dispatchProviderDetailStep(
            accepted
              ? confirmProviderDetailTransition(settled.state)
              : cancelProviderDetailTransition(settled.state),
          );
        }
        void actions.showMessage(
          t("供应商配置转换"),
          response.blockers.join("、") || t("当前供应商配置不能完成该转换。"),
          "failed",
        );
        return false;
      }
      return true;
    }).catch((error) => {
      const settled = settleProviderDetailTransformError(
        detailStateRef.current,
        effect.correlation,
      );
      if (settled.report) {
        updateDetailState(settled.state);
        void actions.showMessage(t("供应商配置转换"), t("调用后端失败，请重试一次；原始错误在详情里。"), "failed", stringifyError(error));
      }
      return false;
    });
  };
  const editDraft = (patch: Partial<RelayProfile>) => {
    let current = detailStateRef.current;
    const target = providerConfigTargetContract(
      { ...current.profile, ...patch },
      isNew && !current.profile.configContents.trim(),
    );
    let step;
    try {
      const hasTransitionFields = "relayMode" in patch
        || "officialMixApiKey" in patch
        || "protocol" in patch;
      const decision = target.source === "existing" && hasTransitionFields
        ? providerTransitionDecisionForStructuredPatch(current.profile, patch)
        : null;
      if (decision?.kind === "requiresExplicitUpgrade") {
        void actions.showMessage(
          t("原生能力优先"),
          t("此变更必须通过明确的升级预览操作完成。"),
          "failed",
        );
        return;
      }
      const transition = decision?.kind === "transition" ? decision.transition : undefined;
      let routedPatch = patch;
      if (decision) {
        const {
          relayMode,
          officialMixApiKey,
          protocol,
          ...ordinaryPatch
        } = patch;
        if (Object.keys(ordinaryPatch).length) {
          const ordinary = beginProviderDetailEdit(current, {
            patch: ordinaryPatch,
            target,
          });
          current = ordinary.state;
        }
        routedPatch = transition
          ? {
              ...(relayMode === undefined ? {} : { relayMode }),
              ...(officialMixApiKey === undefined ? {} : { officialMixApiKey }),
              ...(protocol === undefined ? {} : { protocol }),
            }
          : ordinaryPatch;
        if (!Object.keys(routedPatch).length) return;
      }
      step = beginProviderDetailEdit(current, {
        patch: routedPatch,
        target,
        transition,
      });
    } catch (error) {
      void actions.showMessage(t("供应商配置转换"), t("调用后端失败，请重试一次；原始错误在详情里。"), "failed", stringifyError(error));
      return;
    }
    void dispatchProviderDetailStep(step);
  };
  const retryLegacyProviderIdDraft = () => {
    try {
      dispatchProviderDetailStep(
        resolveProviderDetailLegacyProviderId(
          detailStateRef.current,
          legacyReplacementProviderId,
        ),
      );
    } catch (error) {
      void actions.showMessage(t("旧供应商 ID"), t("调用后端失败，请重试一次；原始错误在详情里。"), "failed", stringifyError(error));
    }
  };
  const cancelLegacyProviderIdDraft = () => {
    updateDetailState(
      cancelProviderDetailLegacyProviderIdResolution(detailStateRef.current).state,
    );
    setLegacyReplacementProviderId("");
  };
  const newProviderFieldErrors = isNew && !isAggregateRelayProfile(draft)
    ? validateNewProviderDraft(draft)
    : {};
  const validationError = isAggregateRelayProfile(draft)
    ? aggregateRelayProfileValidation(draft)
    : Object.keys(newProviderFieldErrors).length
      ? t("请填写所有必填字段。")
      : detailState.pendingTransformRevision !== null
        ? t("供应商配置转换中。")
        : detailState.pendingConfirmation !== null
          ? t("请先确认或取消供应商兼容模式转换。")
          : detailState.pendingLegacyProviderIdResolution !== null
            ? t("请先完成或取消旧供应商 ID 重命名。")
            : detailState.blockers.length
              ? t("供应商草稿被后端验证阻止，请处理提示后重试。")
              : null;
  const saveDraft = async () => {
    if (validationError || savingRef.current) return;
    savingRef.current = true;
    setSaving(true);
    try {
      // Opening a profile that predates this contract and pressing save upgrades it in place: the
      // save is the explicit action, so there is no separate upgrade control to find. A legacy
      // alias still needs a name from the user, so that one only opens its prompt.
      const upgradeAction = isNew ? null : nativeCapabilityView.upgradeAction;
      if (upgradeAction === "upgrade" || upgradeAction === "replaceActorHeader") {
        const upgraded = await dispatchProviderDetailStep(
          beginProviderDetailNativePriorityUpgrade(detailStateRef.current),
        );
        if (!upgraded) return;
      } else if (upgradeAction === "resolveLegacyProviderId") {
        await dispatchProviderDetailStep(
          beginProviderDetailLegacyIdUpgrade(detailStateRef.current),
        );
        return;
      }
      // Deriving the draft inside the guard keeps the `finally` reset reachable: a throw before it
      // would leave the button pending forever, and its click handler discards the rejection.
      const current = detailStateRef.current.profile;
      const normalizedDraft = isAggregateRelayProfile(current) ? normalizeAggregateRelayProfile(current, form) : deriveRelayProfileFromFiles(current);
      const next = isNew
        ? addRelayProfile(form, normalizedDraft)
        : updateRelayProfile(form, profile.id, normalizedDraft);
      const catalogCapable = !isAggregateRelayProfile(normalizedDraft)
        && normalizedDraft.protocol !== "chatCompletions";
      const catalogAvailability = catalogDraftAvailability(!isNew, catalogCapable, !!catalogProfile);
      if (catalogAvailability === "unavailable" || (catalogCapable && !catalogDraft)) {
        await actions.showMessage(
          t("模型目录不可用"),
          t("当前供应商的完整模型目录状态尚未加载，请刷新后重试。"),
          "failed",
        );
        return;
      }
      const managedCatalog = !isAggregateRelayProfile(normalizedDraft)
        && normalizedDraft.protocol !== "chatCompletions"
        && !!catalogDraft
        && managedCatalogMode(catalogDraft.mode);
      const contextConflicts = managedCatalog
        ? providerManagedContextConflictKeys(
            normalizedDraft,
            isActive ? relayFiles?.configContents ?? "" : "",
          )
        : [];
      const confirmContextCleanup = contextConflicts.length
        ? window.confirm(tf("保存托管目录将移除这些全局上下文设置：\n\n{0}", [contextConflicts.join("\n")]))
        : false;
      if (contextConflicts.length && !confirmContextCleanup) return;
      const saved = await actions.commitProviderDetail(
        next,
        normalizedDraft.id,
        isAggregateRelayProfile(normalizedDraft) || normalizedDraft.protocol === "chatCompletions"
          ? null
          : catalogDraft,
        !isNew,
        "detailSave",
        confirmContextCleanup,
      );
      if (!saved) return;
      await actions.showMessage(t("保存供应商"), t("供应商配置已保存。"), "ok");
      onSaved?.();
    } catch (error) {
      // The click handler discards this rejection, so an unreported throw would look like a save
      // that simply did nothing.
      await actions.showMessage(t("保存供应商"), t("调用后端失败，请重试一次；原始错误在详情里。"), "failed", stringifyError(error));
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  };
  const switchDraft = () => {
    if (
      isNew
      || !form.relayProfilesEnabled
      || detailState.pendingTransformRevision !== null
      || detailState.pendingConfirmation !== null
      || detailState.pendingLegacyProviderIdResolution !== null
      || detailState.blockers.length > 0
    ) return;
    const normalizedDraft = isAggregateRelayProfile(draft) ? normalizeAggregateRelayProfile(draft, form) : deriveRelayProfileFromFiles(draft);
    const previousActiveRelayId = form.activeRelayId;
    const next = syncLegacyRelayFields({
      ...form,
      relayProfiles: form.relayProfiles.map((item) => (item.id === profile.id ? normalizedDraft : item)),
      activeRelayId: profile.id,
    });
    void actions.switchRelayProfile(
      next,
      previousActiveRelayId,
      isAggregateRelayProfile(normalizedDraft) || normalizedDraft.protocol === "chatCompletions" || !catalogDraft
        ? undefined
        : catalogDraft,
    );
  };
  const navigateBack = () => {
    const ended = endProviderDetailSession(detailStateRef.current, "navigate");
    detailStateRef.current = ended.state;
    onBack();
  };
  return (
    <div className="relay-detail-page" key={profile.id}>
      <div className="relay-detail-sticky">
        <Toolbar>
          <Button onClick={navigateBack} variant="secondary">
            <ArrowLeft className="h-4 w-4" />
            {t("返回列表")}
          </Button>
          <Button
            aria-busy={saving}
            disabled={saving || !!validationError}
            onClick={() => void saveDraft()}
            title={validationError || (saving ? t("保存中") : t("保存"))}
          >
            {saving ? <RefreshCw className="h-4 w-4 spin" /> : <Save className="h-4 w-4" />}
            {saving ? t("保存中") : t("保存")}
          </Button>
        </Toolbar>
      </div>
      {detailState.pendingLegacyProviderIdResolution === null ? null : (
        <section className="catalog-profile-editor">
          <div className="catalog-editor-head">
            <div>
              <strong>{t("旧供应商 ID 需要一个新名字")}</strong>
              <span>{t("这个供应商用的是旧版名称，保存前请给它取一个未被占用的名字。")}</span>
            </div>
          </div>
          <div className="catalog-editor-actions">
            <Input
              value={legacyReplacementProviderId}
              onChange={(event) => setLegacyReplacementProviderId(event.currentTarget.value)}
              placeholder={t("未使用的供应商 ID")}
            />
            <Button type="button" size="sm" onClick={retryLegacyProviderIdDraft}>
              {t("重试重命名")}
            </Button>
            <Button
              type="button"
              size="sm"
              variant="secondary"
              onClick={cancelLegacyProviderIdDraft}
            >
              {t("取消")}
            </Button>
          </div>
          {detailState.preview?.renamedProviderFrom && detailState.preview.renamedProviderTo ? (
            <span>
              {tf("供应商 ID 将从 {0} 重命名为 {1}；当前仍是未保存草稿。", [
                detailState.preview.renamedProviderFrom,
                detailState.preview.renamedProviderTo,
              ])}
            </span>
          ) : null}
        </section>
      )}
        <RelayProfileEditor profile={draft} form={form} isNew={isNew} onProfileChange={replaceDraft} onProfileEdit={editDraft} onSwitch={switchDraft} actions={actions} catalogProfile={catalogProfile} draftCommitBlocked={detailState.pendingTransformRevision !== null || detailState.pendingConfirmation !== null || detailState.pendingLegacyProviderIdResolution !== null || detailState.blockers.length > 0} />
      {!managedCatalogCapable(draft) ? null : catalogDraft ? (
        <CatalogProfileEditor
          catalog={modelCatalog}
          draft={catalogDraft}
          onDraftChange={(next) => updateDetailState(replaceProviderDetailCatalogDraft(detailStateRef.current, next))}
          profile={draft}
          onProfileEdit={editDraft}
          summary={catalogProfile}
          isNew={isNew}
          actions={actions}
        />
      ) : (
        <section className="catalog-profile-editor unavailable">
          <div className="catalog-editor-head">
            <div>
              <strong>{t("模型目录")}</strong>
              <span>{t("当前供应商的完整模型目录状态尚未加载，请刷新后重试。")}</span>
            </div>
            <UiBadge variant="outline">{t("不可用")}</UiBadge>
          </div>
        </section>
      )}
      {isAggregateRelayProfile(draft) ? null : (
      <RelayLiveFilePanels
        authStatus={isActive ? relayFiles?.authStatus ?? null : null}
        liveConfigContents={relayFiles?.configContents ?? ""}
      />
      )}
    </div>
  );
}

function RelayProfileEditor({
  profile,
  form,
  isNew = false,
  onProfileChange,
  onProfileEdit,
  onSwitch,
  actions,
  catalogProfile,
  draftCommitBlocked = false,
}: {
  profile: RelayProfile;
  form: BackendSettings;
  isNew?: boolean;
  onProfileChange: (value: RelayProfile) => void;
  onProfileEdit?: (patch: Partial<RelayProfile>) => void;
  onSwitch: () => void;
  actions: Actions;
  catalogProfile: ProfileCatalogSummary | null;
  draftCommitBlocked?: boolean;
}) {
  const [doctorResult, setDoctorResult] = useState<ProviderDoctorResult | null>(null);
  const [doctorOpen, setDoctorOpen] = useState(false);
  const [doctorRunning, setDoctorRunning] = useState(false);
  const doctorRequestSequenceRef = useRef(0);
  const doctorSourceRevision = JSON.stringify(profile);
  const doctorSourceRevisionRef = useRef(doctorSourceRevision);
  doctorSourceRevisionRef.current = doctorSourceRevision;
  useEffect(() => {
    doctorRequestSequenceRef.current += 1;
    setDoctorResult(null);
    setDoctorOpen(false);
    setDoctorRunning(false);
    return () => {
      doctorRequestSequenceRef.current += 1;
    };
  }, [doctorSourceRevision]);
  if (isAggregateRelayProfile(profile)) {
    return (
      <AggregateRelayProfileEditor
        profile={profile}
        form={form}
        isNew={isNew}
        onProfileChange={onProfileChange}
      />
    );
  }

  const newProviderFieldErrors = isNew ? validateNewProviderDraft(profile) : {};
  // Every profile shows its endpoint and key, including one that has none yet: supplying them is
  // how a profile that predates this contract upgrades itself.
  const showApiFields = true;
  const updateDraft = (patch: Partial<RelayProfile>) => {
    // Holding a provider key is what makes an official profile mixed; there is no separate
    // switch. Clearing the key never flips it back, because leaving the mixed contract deletes a
    // provider table and stays an explicit, previewed action.
    const merged = { ...profile, ...patch };
    const resolved = merged.relayMode === "official"
      && !merged.officialMixApiKey
      && merged.apiKey.trim()
      ? { ...patch, officialMixApiKey: true }
      : patch;
    if (onProfileEdit) {
      onProfileEdit(resolved);
      return;
    }
    const target = providerConfigTargetContract({ ...profile, ...resolved }, isNew);
    onProfileChange(applyRelayProfilePatchToFiles(profile, resolved, {
      allowGenerateFiles: isNew,
      target,
    }));
  };
  const runProviderDoctor = async () => {
    const requestSourceFingerprint = doctorSourceRevisionRef.current;
    const requestSequence = doctorRequestSequenceRef.current + 1;
    doctorRequestSequenceRef.current = requestSequence;
    setDoctorOpen(true);
    setDoctorRunning(true);
    setDoctorResult(null);
    const probedProfile = deriveRelayProfileFromFiles(profile);
    const result = await actions.diagnoseRelayProfile(probedProfile);
    // Drop a result the draft has already moved past, so a slow probe cannot overwrite a newer one.
    if (
      requestSequence !== doctorRequestSequenceRef.current
      || requestSourceFingerprint !== doctorSourceRevisionRef.current
    ) return;
    setDoctorResult(result);
    setDoctorRunning(false);
  };
  return (
    <div className="relay-profile-editor">
      <div className="relay-editor-head">
        <div>
          <strong>{profile.name || t("未命名供应商")}</strong>
          <span>{relayProfileEditorStatus(profile, form, isNew)}</span>
        </div>
        {isNew ? null : (
          <Button
            disabled={!form.relayProfilesEnabled || actions.relaySwitching || draftCommitBlocked}
            onClick={onSwitch}
            title={!form.relayProfilesEnabled ? t("供应商配置总开关已关闭") : actions.relaySwitching ? t("供应商切换中") : draftCommitBlocked ? t("供应商配置尚未通过后端验证。") : undefined}
            variant={profile.id === form.activeRelayId ? "secondary" : "default"}
          >
            {actions.relaySwitching ? t("切换中") : profile.id === form.activeRelayId ? t("使用中") : t("设为当前")}
          </Button>
        )}
      </div>
      <div className="relay-fields">
        <Field className="relay-field-name" label={t("名称")}>
          <Input
            value={profile.name}
            onChange={(event) => updateDraft({ name: event.currentTarget.value })}
          />
        </Field>
        {profile.relayMode === "official" ? (
          <Field className="relay-field-mode" label={t("接入模式")}>
            <p className="field-hint">{t("官方登录＋混入 API Key＋Responses API")}</p>
          </Field>
        ) : null}
        {showApiFields ? (
          <div className="relay-api-fields">
            <Field className="relay-field-base-url" label="Base URL">
              <Input
                aria-describedby={newProviderFieldErrors.baseUrl ? "provider-base-url-error" : undefined}
                aria-invalid={newProviderFieldErrors.baseUrl ? true : undefined}
                value={profile.baseUrl}
                onChange={(event) => updateDraft({ baseUrl: event.currentTarget.value })}
                placeholder={t("填写中转服务 Base URL")}
              />
              {newProviderFieldErrors.baseUrl ? <p className="field-hint" id="provider-base-url-error" role="alert">{t("必填")}</p> : null}
            </Field>
            <Field className="relay-field-key" label="Key">
              <Input
                aria-describedby={newProviderFieldErrors.apiKey ? "provider-api-key-error" : undefined}
                aria-invalid={newProviderFieldErrors.apiKey ? true : undefined}
                type="password"
                value={profile.apiKey}
                onChange={(event) => updateDraft({ apiKey: event.currentTarget.value })}
                placeholder={t("输入中转服务的 API Key")}
              />
              {newProviderFieldErrors.apiKey ? <p className="field-hint" id="provider-api-key-error" role="alert">{t("必填")}</p> : null}
            </Field>
          </div>
        ) : null}
        {showApiFields ? (
          <div className="provider-doctor">
            <div className="provider-doctor-head">
              <div>
                <strong>Provider Doctor</strong>
                <span>{t("检查配置、供应商支持的模型和一次真实请求，定位供应商不可用原因。")}</span>
              </div>
              <Button onClick={() => void runProviderDoctor()} size="sm" type="button" variant="secondary">
                <Stethoscope className="h-4 w-4" />
                {t("诊断供应商")}
              </Button>
            </div>
            <span>{doctorResult?.summary ?? t("点击后会打开诊断弹框，按步骤检查供应商。")}</span>
          </div>
        ) : null}
      </div>
      {showApiFields && profile.protocol === "chatCompletions" ? (
        <div className="hint-line relay-protocol-hint">
          <MessageCircle className="h-4 w-4" />
          <span>{t("此上游依赖本地 127.0.0.1:57321 协议代理转成 Responses API；Codex Minus 不提供该代理，选择此协议后 Codex 将无法请求，请慎用。")}</span>
        </div>
      ) : null}
      <div className="hint-line relay-protocol-hint">
        <ShieldCheck className="h-4 w-4" />
        <span>{relayProfileModeHelp(profile)}</span>
      </div>
      {doctorOpen ? (
        <ProviderDoctorModal
          result={doctorResult}
          running={doctorRunning}
          onClose={() => {
            if (!doctorRunning) setDoctorOpen(false);
          }}
        />
      ) : null}
    </div>
  );
}

function providerTransitionConfirmationMessage(
  state: ProviderDetailDraftState<RelayProfile>,
): string {
  const pending = state.pendingConfirmation;
  if (!pending) return t("当前没有等待确认的供应商转换。");
  if (pending.requiredConfirmation === "replaceActorHeader") {
    return t("当前自定义 Actor 标记与原生能力优先标记冲突。确认后只替换冲突的 Actor 标记并更新草稿，仍需点击保存或设为当前才会生效。是否继续？");
  }
  if (pending.transition.action === "exitPureOAuth") {
    const providerId = state.preview?.removedProviderId || t("当前自定义供应商");
    const fields = state.preview?.removedProviderFields.join("、") || t("全部供应商字段");
    return tf(
      "切换到纯 OAuth 将删除自定义供应商 {0} 及其全部配置字段（{1}）。确认后只更新草稿，仍需点击保存或设为当前才会生效。是否继续？",
      [providerId, fields],
    );
  }
  return t("切换到兼容模式将失去原生能力优先配置。确认后只更新草稿，仍需点击保存或设为当前才会生效。是否继续？");
}

function providerNativeCapabilityStateLabel(state: string): string {
  switch (state) {
    case "nativePriority":
      return t("配置就绪");
    case "upgradeAvailable":
      return t("可升级");
    case "degraded":
      return t("需要处理");
    case "compatibility":
      return t("兼容模式");
    case "notApplicable":
      return t("不适用");
    default:
      return t("状态未知");
  }
}

function AggregateRelayProfileEditor({
  profile,
  form,
  isNew = false,
  onProfileChange,
}: {
  profile: RelayProfile;
  form: BackendSettings;
  isNew?: boolean;
  onProfileChange: (value: RelayProfile) => void;
}) {
  const candidates = aggregateMemberCandidates(form, profile.id);
  const aggregate = normalizeAggregateConfig(profile.aggregate, candidates);
  const memberIds = new Set(aggregate.members.map((member) => member.profileId));
  const updateAggregate = (nextAggregate: RelayAggregateConfig) => {
    onProfileChange(normalizeAggregateRelayProfile({ ...profile, aggregate: nextAggregate }, form));
  };
  const toggleMember = (profileId: string, checked: boolean) => {
    const members = checked
      ? [...aggregate.members, { profileId, weight: 1 }]
      : aggregate.members.filter((member) => member.profileId !== profileId);
    updateAggregate({ ...aggregate, members });
  };
  const updateWeight = (profileId: string, weight: number) => {
    updateAggregate({
      ...aggregate,
      members: aggregate.members.map((member) =>
        member.profileId === profileId ? { ...member, weight: clampAggregateWeight(weight) } : member,
      ),
    });
  };
  const totalWeight = aggregate.members.reduce((total, member) => total + clampAggregateWeight(member.weight), 0);

  return (
    <div className="relay-profile-editor aggregate-editor">
      <div className="relay-editor-head">
        <div>
          <strong>{profile.name || t("未命名聚合供应商")}</strong>
          <span>{t("本地成员轮转依赖已移除的 127.0.0.1:57321 代理，不能应用。")}</span>
        </div>
        <UiBadge variant="outline">{t("高级兼容路径")} · {t("本地聚合（不可用）")}</UiBadge>
      </div>
      <div className="relay-fields aggregate-fields">
        <Field className="relay-field-name" label={t("名称")}>
          <Input
            value={profile.name}
            onChange={(event) => onProfileChange({ ...profile, name: event.currentTarget.value })}
            placeholder={t("例如 主力聚合池")}
          />
        </Field>
        <Field className="relay-field-test-model" label={t("测试模型")}>
          <Input
            value={profile.testModel}
            onChange={(event) => onProfileChange({ ...profile, testModel: event.currentTarget.value })}
            placeholder={t("留空使用该聚合成员自己的模型")}
          />
        </Field>
        <Field className="aggregate-strategy-field" label={t("聚合策略")}>
          <select
            className="field-select"
            value={aggregate.strategy}
            onChange={(event) => updateAggregate({ ...aggregate, strategy: event.currentTarget.value as RelayAggregateStrategy })}
          >
            {aggregateStrategyOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </Field>
      </div>
      <div className="aggregate-strategy-grid">
        {aggregateStrategyOptions.map((option) => (
          <button
            className={`mode-option aggregate-strategy-option ${aggregate.strategy === option.value ? "active" : ""}`}
            key={option.value}
            onClick={() => updateAggregate({ ...aggregate, strategy: option.value })}
            type="button"
          >
            <strong>{option.label}</strong>
            <span>{option.description}</span>
          </button>
        ))}
      </div>
      <div className="aggregate-members">
        <div className="aggregate-members-head">
          <div>
            <strong>{t("成员供应商")}</strong>
            <span>{t("只能勾选已填写 Base URL / Key 的 API 供应商，聚合供应商不会作为成员。")}</span>
          </div>
          <UiBadge variant="outline">{aggregate.members.length} / {candidates.length}</UiBadge>
        </div>
        {candidates.length ? (
          <div className="aggregate-member-list">
            {candidates.map((candidate) => {
              const member = aggregate.members.find((item) => item.profileId === candidate.id);
              const checked = memberIds.has(candidate.id);
              return (
                <label className={`aggregate-member-row ${checked ? "selected" : ""}`} key={candidate.id}>
                  <input
                    checked={checked}
                    onChange={(event) => toggleMember(candidate.id, event.currentTarget.checked)}
                    type="checkbox"
                  />
                  <span className="aggregate-member-summary">
                    <strong>{candidate.name || t("未命名供应商")}</strong>
                    <small>{relayModeLabel(candidate.relayMode)} · {relayProtocolLabel(candidate.protocol)} · {relayProfileConfigBrief(candidate)}</small>
                  </span>
                  <span className="aggregate-weight-box">
                    <span>{t("权重")}</span>
                    <Input
                      disabled={!checked}
                      min={1}
                      onChange={(event) => updateWeight(candidate.id, Number.parseInt(event.currentTarget.value, 10))}
                      type="number"
                      value={String(member?.weight ?? 1)}
                    />
                  </span>
                </label>
              );
            })}
          </div>
        ) : (
          <div className="empty">{t("先添加至少 1 个已填写 Base URL / Key 的 API 供应商，再创建聚合供应商。")}</div>
        )}
      </div>
      <div className="relay-grid compact aggregate-preview">
        <Metric label={t("策略")} value={aggregateStrategyLabel(aggregate.strategy)} />
        <Metric label={t("成员数量")} value={tf("{0} 个", [aggregate.members.length])} />
        <Metric label={t("总权重")} value={`${totalWeight}`} />
        <Metric label={t("序列化字段")} value="aggregate.strategy / aggregate.members" />
      </div>
      <div className="hint-line relay-protocol-hint">
        <ShieldCheck className="h-4 w-4" />
        <span>{aggregateStrategyHelp(aggregate.strategy)}</span>
      </div>
    </div>
  );
}

function RelayLiveFilePanels({
  authStatus,
  liveConfigContents,
}: {
  authStatus: RelayFilesResult["authStatus"] | null;
  liveConfigContents: string;
}) {
  return (
    <div className="relay-file-grid">
      <LiveConfigPanel
        liveConfig={liveConfigContents}
        liveHelp={t("直接读取 Codex 当前文件；Manager 不保存副本，切换时只替换供应商字段。密钥以 ●● 遮蔽。")}
        liveTitle={t("实时 config.toml")}
        unavailableLiveMessage={t("当前 live config.toml 不可用")}
      />
      <div className="relay-file-panel">
        <div className="relay-file-head">
          <div>
            <strong>{t("官方认证")}</strong>
            <span>{t("登录与令牌刷新由官方 Codex/ChatGPT 客户端管理；供应商切换不会写入 auth.json。")}</span>
          </div>
        </div>
        <div className="relay-file-auth-status">
          <strong>{authStatus?.authenticated ? t("已登录") : t("需要登录")}</strong>
          <span>{authStatus?.accountLabel || authStatus?.actionRequired || t("仅活动供应商显示实时认证状态。")}</span>
        </div>
      </div>
    </div>
  );
}

function ProviderDoctorModal({
  result,
  running,
  onClose,
}: {
  result: ProviderDoctorResult | null;
  running: boolean;
  onClose: () => void;
}) {
  const steps = providerDoctorSteps(result, running);
  const doneCount = steps.filter((step) => step.state === "ok" || step.state === "warning" || step.state === "failed").length;
  const progress = Math.round((doneCount / steps.length) * 100);
  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true">
      <div className="modal-card provider-doctor-modal">
        <div className="modal-head">
          <div>
            <h2>Provider Doctor</h2>
            <p>{running ? t("正在诊断供应商，请稍候。") : result?.summary ?? t("诊断已完成。")}</p>
          </div>
          <UiBadge variant={result && !isSuccessStatus(result.status) ? "outline" : "secondary"}>
            {running ? t("诊断中") : result && !isSuccessStatus(result.status) ? t("异常") : t("完成")}
          </UiBadge>
        </div>
        <div className="provider-doctor-progress" aria-valuemin={0} aria-valuemax={100} aria-valuenow={progress} role="progressbar">
          <div style={{ width: `${progress}%` }} />
        </div>
        <div className="provider-doctor-step-list">
          {steps.map((step) => (
            <div className={`provider-doctor-step ${step.state}`} key={step.id}>
              <span className="provider-doctor-step-icon">
                {step.state === "running" ? (
                  <RefreshCw className="h-4 w-4" />
                ) : step.state === "ok" ? (
                  <CheckCircle2 className="h-4 w-4" />
                ) : step.state === "warning" ? (
                  <ShieldAlert className="h-4 w-4" />
                ) : step.state === "failed" ? (
                  <Info className="h-4 w-4" />
                ) : (
                  <span />
                )}
              </span>
              <div>
                <strong>{step.title}</strong>
                <small>{step.detail}</small>
              </div>
            </div>
          ))}
        </div>
        {result?.recommendation ? <p className="provider-doctor-recommendation">{result.recommendation}</p> : null}
        <div className="modal-actions">
          <Button disabled={running} onClick={onClose} variant="secondary">
            {running ? t("诊断中") : t("关闭")}
          </Button>
        </div>
      </div>
    </div>
  );
}

type ProviderDoctorStepState = "pending" | "running" | "ok" | "warning" | "failed";

function providerDoctorSteps(
  result: ProviderDoctorResult | null,
  running: boolean,
): Array<{ id: string; title: string; detail: string; state: ProviderDoctorStepState }> {
  const base = [
    { id: "config", title: t("配置完整性"), pending: t("等待检查 Base URL / API Key。") },
    { id: "models", title: t("供应商支持的模型"), pending: t("等待检查 /v1/models。") },
    { id: "request", title: t("真实请求"), pending: t("等待发送一次测试请求。") },
    { id: "recommendation", title: t("处理建议"), pending: t("等待生成建议。") },
  ];
  if (!result) {
    return base.map((step, index) => ({
      id: step.id,
      title: step.title,
      detail: index === 0 && running ? t("正在检查配置完整性…") : step.pending,
      state: index === 0 && running ? "running" : "pending",
    }));
  }
  const checks = new Map(result.checks.map((check) => [check.id, check]));
  return base.map((step) => {
    if (step.id === "recommendation") {
      return {
        id: step.id,
        title: step.title,
        detail: result.recommendation || step.pending,
        state: result.status === "failed" ? "warning" : "ok",
      };
    }
    const check = checks.get(step.id);
    if (!check) {
      return {
        id: step.id,
        title: step.title,
        detail: step.id === "models" || step.id === "request" ? t("该步骤未执行。") : step.pending,
        state: "pending",
      };
    }
    return {
      id: step.id,
      title: check.title || step.title,
      detail: check.detail,
      state: check.status === "ok" ? "ok" : check.status === "warning" ? "warning" : "failed",
    };
  });
}

function ToggleVisual() {
  return (
    <span aria-hidden="true" className="toggle-switch-visual">
      <span className="toggle-switch-thumb" />
    </span>
  );
}

function NoticeDialog({
  notice,
  onClose,
}: {
  notice: { title: string; message: string; detail?: string | null; status?: Status };
  onClose: () => void;
}) {
  const failed = notice.status === "failed";
  useEffect(() => {
    // A failure stays until dismissed: its 详情 exists to be expanded, read, and screenshotted,
    // none of which survives a 4-second toast.
    if (failed) return;
    const timer = window.setTimeout(onClose, 4200);
    return () => window.clearTimeout(timer);
  }, [failed]);

  return (
    <div className="toast-wrap" role="status" aria-live="polite">
      <div className={`toast-card ${failed ? "failed" : ""}`}>
        {failed ? null : <div className="toast-progress" />}
        <div className="toast-icon">
          {failed ? <Bell className="h-5 w-5" /> : <CheckCircle2 className="h-5 w-5" />}
        </div>
        <div className="toast-body">
          <h2>{notice.title}</h2>
          <p>{notice.message}</p>
          {notice.detail ? (
            <details className="toast-detail" data-notice-detail="true">
              <summary>{t("详情")}</summary>
              <pre>{notice.detail}</pre>
            </details>
          ) : null}
        </div>
        <button className="toast-close" onClick={onClose} type="button">×</button>
      </div>
    </div>
  );
}

function ConfirmDialog({
  confirm,
  onConfirm,
  onCancel,
}: {
  confirm: { title: string; message: string; confirmText: string; cancelText: string };
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true">
      <div className="modal-card">
        <div className="modal-head">
          <div>
            <h2>{confirm.title}</h2>
            <p className="modal-message">{confirm.message}</p>
          </div>
          <button className="toast-close" onClick={onCancel} type="button">×</button>
        </div>
        <Toolbar>
          <Button onClick={onConfirm}>
            <Trash2 className="h-4 w-4" />
            {confirm.confirmText}
          </Button>
          <Button onClick={onCancel} variant="secondary">{confirm.cancelText}</Button>
        </Toolbar>
      </div>
    </div>
  );
}

function Panel({ children, fill = false, className = "" }: { children: React.ReactNode; fill?: boolean; className?: string }) {
  return (
    <Card className={`panel ${fill ? "fill" : ""} ${className}`}>
      {children}
    </Card>
  );
}

function CardHead({ title, detail }: { title: string; detail: string }) {
  return (
    <CardHeader className="panel-head">
      <CardTitle>{title}</CardTitle>
      <CardDescription>{detail}</CardDescription>
    </CardHeader>
  );
}

function Toolbar({ children }: { children: React.ReactNode }) {
  return <div className="toolbar">{children}</div>;
}

function Field({ label, children, className = "" }: { label: string; children: React.ReactNode; className?: string }) {
  return (
    <Label className={`field ${className}`}>
      <span>{label}</span>
      {children}
    </Label>
  );
}

function Badge({ status }: { status: string }) {
  return <UiBadge className={statusClass(status)} variant="secondary">{statusLabel(status)}</UiBadge>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function routeTitle(route: Route) {
  return routes.find((item) => item.id === route)?.label ?? t("概览");
}

function routeSubtitle(route: Route) {
  const subtitles: Record<Route, string> = {
    relay: t("管理 API 供应商、协议、Key 与配置文件"),
    sessions: t("查看、删除和修复 Codex 本地会话"),
  };
  return subtitles[route];
}

// Labels are the shell's; the table names come from the module that reads the file.
const contextKindLabels: Record<ContextKind, string> = { mcp: "MCP", skill: "Skills", plugin: t("插件") };
const contextKindOptions = CONTEXT_KIND_TABLES.map((option) => ({
  ...option,
  label: contextKindLabels[option.kind],
}));

function relayProfileEditorStatus(profile: RelayProfile, form: BackendSettings, isNew: boolean) {
  if (isNew) return t("新建供应商需要先保存到列表");
  if (!form.relayProfilesEnabled) return t("供应商配置总开关已关闭；当前只保存配置，不写入 Codex live 文件");
  return profile.id === form.activeRelayId ? t("当前正在使用") : t("编辑后保存列表，再切换模式时会使用新配置");
}

function providerInitial(name: string) {
  const trimmed = (name || t("供应商")).trim();
  return Array.from(trimmed)[0]?.toUpperCase() || t("供");
}

function statusLabel(status: string) {
  const labels: Record<string, string> = {
    found: t("已找到"),
    missing: t("缺失"),
    installed: t("已安装"),
    ok: t("正常"),
    running: t("运行中"),
    failed: t("失败"),
    archived: t("已归档"),
    accepted: t("已受理"),
    not_checked: t("未检查"),
    not_implemented: t("未实现"),
    disabled: t("已禁用"),
    unknown: t("未知"),
  };
  return labels[status] ?? status;
}

function statusClass(status: string) {
  if (["found", "installed", "ok", "running"].includes(status)) return "good";
  if (["failed", "missing"].includes(status)) return "bad";
  return "warn";
}

function isSuccessStatus(status?: Status) {
  return status === "ok" || status === "accepted";
}

function truncateSessionDeletePreview(value: string) {
  const normalized = value.trim();
  return normalized.length > 20 ? `${normalized.slice(0, 20)}...` : normalized;
}

function relayProtocolLabel(protocol: RelayProtocol): string {
  return protocol === "chatCompletions" ? t("Chat Completions 转 Responses") : "Responses API";
}

function relayModeLabel(mode: RelayMode): string {
  if (mode === "aggregate") return t("聚合供应商");
  if (mode === "pureApi") return t("纯 API");
  return t("官方登录");
}

function relayProfileConfigBrief(profile: RelayProfile): string {
  if (isAggregateRelayProfile(profile)) {
    const aggregate = normalizeAggregateConfig(profile.aggregate, []);
    return tf("{0} · {1} 个成员", [aggregateStrategyLabel(aggregate.strategy), aggregate.members.length]);
  }
  if (profile.relayMode === "official") return profile.officialMixApiKey ? t("混入 API Key") : t("不写 API 文件");
  return profile.baseUrl || t("未填写 URL");
}

function relayProfileModeHelp(profile: RelayProfile): string {
  if (isAggregateRelayProfile(profile)) {
    return t("聚合供应商只保存成员和策略配置，成员来自已有 API 供应商；切为当前后会通过本地协议代理轮转请求。");
  }
  if (profile.relayMode === "official") {
    if (profile.officialMixApiKey) {
      return t("此供应商会保留官方登录模式，并把请求混入当前 API Key。");
    }
    return t("此供应商会切回官方登录模式，使用 ChatGPT 官方账号，不写入 API Key。");
  }
  if (profile.relayMode === "pureApi") {
    return t("此供应商只把 API Key 写入 owner-only 的 provider bearer 配置，并明确不要求 ChatGPT 认证。");
  }
  return t("此供应商会保留官方登录模式，并把请求混入当前 API Key。");
}

// Labels are the shell's; the set of valid values comes from the module that stores them.
const aggregateStrategyLabels: Array<{ value: RelayAggregateStrategy; label: string; description: string }> = [
  {
    value: "failover",
    label: t("失败切换"),
    description: t("按成员顺序请求，失败后切到下一个供应商。"),
  },
  {
    value: "conversationRoundRobin",
    label: t("按对话轮转"),
    description: t("同一对话保持一个成员，不同对话依次分配。"),
  },
  {
    value: "requestRoundRobin",
    label: t("按请求轮转"),
    description: t("每次请求按成员顺序切换，适合均匀摊请求量。"),
  },
  {
    value: "weightedRoundRobin",
    label: t("权重轮转"),
    description: t("按成员权重分配请求，权重越高承担越多。"),
  },
];
const aggregateStrategyOptions = AGGREGATE_STRATEGIES.map(
  (value) => aggregateStrategyLabels.find((option) => option.value === value)!,
);

function aggregateStrategyLabel(strategy: RelayAggregateStrategy): string {
  return aggregateStrategyOptions.find((option) => option.value === strategy)?.label ?? t("失败切换");
}

function aggregateStrategyHelp(strategy: RelayAggregateStrategy): string {
  if (strategy === "failover") return t("失败切换会保留成员顺序，优先使用第一个可用供应商。");
  if (strategy === "conversationRoundRobin") return t("按对话轮转会让同一对话尽量保持固定成员，降低上下文漂移。");
  if (strategy === "requestRoundRobin") return t("按请求轮转会逐请求切换成员，适合供应商能力接近的场景。");
  return t("权重轮转会读取每个成员的权重值，权重越高的成员获得更多请求。");
}

function formatTime(value: number) {
  if (!value) return "-";
  return new Date(value).toLocaleString("zh-CN");
}

function positiveNumberOrNull(value: string): number | null {
  const parsed = Number.parseInt(value.replace(/[^\d]/g, ""), 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

function boundedPercentOrNull(value: string): number | null {
  const parsed = positiveNumberOrNull(value);
  return parsed !== null && parsed <= 100 ? parsed : null;
}

function boundedPercentOrDefault(value: string, fallback: number): number {
  return boundedPercentOrNull(value) ?? fallback;
}

function parseCommaListOrNull(value: string): string[] | null {
  const items = [...new Set(value.split(",").map((item) => item.trim()).filter(Boolean))];
  return items.length ? items : null;
}

function parseReasoningLevels(value: string): ReasoningLevel[] | null {
  const efforts = parseCommaListOrNull(value);
  return efforts?.map((effort) => ({ effort, description: effort })) ?? null;
}

function reasoningEffortsText(levels: ReasoningLevel[]): string {
  return levels.map((level) => level.effort).join(",");
}

function managedCatalogMode(mode: CatalogMode): boolean {
  return mode === "official-plus-custom" || mode === "custom-only";
}

function integerOrNull(value: string): number | null {
  if (!value.trim()) return null;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

function positiveNumberOrDefault(value: string, fallback: number): number {
  return positiveNumberOrNull(value) ?? fallback;
}

function integerOrDefault(value: string, fallback: number): number {
  return integerOrNull(value) ?? fallback;
}

function catalogDraftErrorLabel(error: string | null): string {
  if (error === "empty-custom-slug") return t("自定义模型 slug 不能为空。");
  if (error === "empty-display-name") return t("自定义模型显示名不能为空。");
  if (error === "duplicate-custom-slug") return t("自定义模型 slug 不能重复。");
  if (error === "invalid-context-window") return t("上下文窗口必须是正整数。");
  if (error === "invalid-effective-percent") return t("有效上下文百分比必须为 1 到 100。");
  if (error === "invalid-reasoning-levels") return t("推理级别不能为空或重复。");
  if (error === "invalid-reasoning-default") return t("默认推理级别必须包含在支持列表中。");
  if (error === "invalid-default-model") return t("当前默认模型不在有效目录中，请先调整目录或默认模型。");
  if (error === "empty-catalog") return t("模型列表不能为空，至少保留一个模型。");
  return "";
}

function stringifyError(error: unknown) {
  if (error instanceof Error) return error.message;
  return String(error);
}

function loadInitialTheme(): Theme {
  if (typeof window === "undefined") return "dark";
  return window.localStorage.getItem("codex-plus-theme") === "light" ? "light" : "dark";
}

function loadInitialRoute(): Route {
  return "relay";
}
