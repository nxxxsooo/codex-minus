export type CatalogModeValue = "native-official" | "official-plus-custom" | "custom-only" | "external";

export type CatalogOverlayDraft = {
  official: Record<string, {
    displayName: string | null;
    visible: boolean | null;
    contextWindow: number | null;
    effectiveContextWindowPercent: number | null;
    order: number | null;
    supportedReasoningLevels: Array<{ effort: string; description: string }> | null;
    defaultReasoningLevel: string | null;
    supportedTools: string[] | null;
    toolCapabilities: Record<string, unknown> | null;
  }>;
  custom: Array<{
    slug: string;
    displayName: string;
    contextWindow: number;
    effectiveContextWindowPercent: number;
    visible: boolean;
    order: number;
    supportedReasoningLevels: Array<{ effort: string; description: string }>;
    defaultReasoningLevel: string | null;
    supportedTools: string[];
    toolCapabilities: Record<string, unknown> | null;
    templateProvenance: string;
  }>;
};

export function catalogModeChangeDecision(
  currentMode: CatalogModeValue,
  requestedMode: CatalogModeValue,
  externalPointer: string | null,
  customModelCount: number,
): "select" | "confirm-discard-external" | "confirm-discard-custom" {
  if (requestedMode !== "native-official" || requestedMode === currentMode) return "select";
  if (externalPointer) return "confirm-discard-external";
  if (customModelCount > 0) return "confirm-discard-custom";
  return "select";
}

export function catalogModeDraftController(input: {
  currentMode: CatalogModeValue;
  externalPointer: string | null;
  customModelCount: number;
  confirmDiscard: (decision: "confirm-discard-external" | "confirm-discard-custom") => boolean;
  actions: {
    updateDraftMode: (mode: CatalogModeValue) => void;
  };
}): {
  requestMode: (requestedMode: CatalogModeValue) => boolean;
  restoreOfficialPlusCustom: () => void;
} {
  const updateDraftMode = (mode: CatalogModeValue) => input.actions.updateDraftMode(mode);
  return {
    requestMode(requestedMode) {
      const decision = catalogModeChangeDecision(
        input.currentMode,
        requestedMode,
        input.externalPointer,
        input.customModelCount,
      );
      if (decision !== "select" && !input.confirmDiscard(decision)) return false;
      updateDraftMode(requestedMode);
      return true;
    },
    restoreOfficialPlusCustom() {
      updateDraftMode("official-plus-custom");
    },
  };
}

export function catalogModePresentation(input: {
  selectedMode: CatalogModeValue;
  persistedMode: CatalogModeValue | null;
  generatedPath: string | null;
  externalPointer: string | null;
  restartRequired: boolean;
  customModelCount: number;
}): {
  source: "native" | "managed" | "external" | "unsaved";
  pendingSource: "native" | "managed" | "external" | null;
  path: string | null;
  restart: boolean;
  dormantCustomCount: number;
  pendingDormantCustomCount: number;
  pathUnavailable: "managed" | "external" | null;
} {
  if (input.selectedMode !== input.persistedMode) {
    return {
      source: "unsaved",
      pendingSource: input.selectedMode === "native-official"
        ? "native"
        : input.selectedMode === "external"
          ? "external"
          : "managed",
      path: null,
      restart: false,
      dormantCustomCount: 0,
      pendingDormantCustomCount: input.selectedMode === "native-official" ? input.customModelCount : 0,
      pathUnavailable: null,
    };
  }
  if (input.selectedMode === "native-official") {
    return {
      source: "native",
      pendingSource: null,
      path: null,
      restart: false,
      dormantCustomCount: input.customModelCount,
      pendingDormantCustomCount: 0,
      pathUnavailable: null,
    };
  }
  if (input.selectedMode === "external") {
    return {
      source: "external",
      pendingSource: null,
      path: input.externalPointer,
      restart: input.restartRequired,
      dormantCustomCount: 0,
      pendingDormantCustomCount: 0,
      pathUnavailable: input.externalPointer ? null : "external",
    };
  }
  return {
    source: "managed",
    pendingSource: null,
    path: input.generatedPath,
    restart: input.restartRequired,
    dormantCustomCount: 0,
    pendingDormantCustomCount: 0,
    pathUnavailable: input.generatedPath ? null : "managed",
  };
}

export function defaultCatalogMode(
  relayMode: string,
  officialMixApiKey: boolean,
  externalPointer?: string | null,
  upstreamTopology: "direct" | "server-side-composite" = "direct",
): CatalogModeValue {
  if (externalPointer) return "external";
  if (relayMode === "pureApi") return upstreamTopology === "server-side-composite" ? "official-plus-custom" : "custom-only";
  if (relayMode === "official" && !officialMixApiKey) return "native-official";
  return "official-plus-custom";
}

export function catalogRefreshGate(status: {
  refreshAvailable: boolean;
  credentialAction: string | null;
  loading: boolean;
}): { disabled: boolean; reason: string | null } {
  if (status.loading) return { disabled: true, reason: "loading" };
  if (status.credentialAction) return { disabled: true, reason: status.credentialAction };
  if (!status.refreshAvailable) return { disabled: true, reason: "target-unavailable" };
  return { disabled: false, reason: null };
}

/// Renders a model the way the Codex model picker does.
///
/// The official catalog stores `GPT-5.6-Sol` while the picker shows `5.6 Sol`. Showing the stored
/// form here makes the same model look like two different models across the two windows.
export function appModelLabel(displayName: string): string {
  return displayName.replace(/^GPT-/i, "").replace(/-/g, " ").trim() || displayName;
}

export function providerEvidenceState(slug: string, reportedSlugs: readonly string[]): "reported" | "not-reported" {
  return reportedSlugs.includes(slug) ? "reported" : "not-reported";
}

export function addCatalogCandidate(
  overlay: CatalogOverlayDraft,
  slug: string,
): CatalogOverlayDraft {
  const normalized = slug.trim();
  if (!normalized || overlay.custom.some((item) => item.slug === normalized)) return overlay;
  return {
    ...overlay,
    custom: [
      ...overlay.custom,
      {
        slug: normalized,
        displayName: normalized,
        contextWindow: 272000,
        effectiveContextWindowPercent: 100,
        visible: true,
        order: overlay.custom.length,
        supportedReasoningLevels: [],
        defaultReasoningLevel: null,
        supportedTools: [],
        toolCapabilities: null,
        templateProvenance: "provider-candidate",
      },
    ],
  };
}

export function validateCatalogDraft(
  overlay: CatalogOverlayDraft,
  mode: CatalogModeValue,
  defaultModel: string,
  officialSlugs: readonly string[],
): string | null {
  if (mode === "native-official" || mode === "external") return null;
  const seen = new Set<string>();
  for (const custom of overlay.custom) {
    const slug = custom.slug.trim();
    if (!slug) return "empty-custom-slug";
    if (!custom.displayName.trim()) return "empty-display-name";
    if (seen.has(slug)) return "duplicate-custom-slug";
    if (!Number.isFinite(custom.contextWindow) || custom.contextWindow <= 0) return "invalid-context-window";
    if (!Number.isInteger(custom.effectiveContextWindowPercent) || custom.effectiveContextWindowPercent < 1 || custom.effectiveContextWindowPercent > 100) return "invalid-effective-percent";
    const efforts = custom.supportedReasoningLevels.map((level) => level.effort.trim());
    if (new Set(efforts).size !== efforts.length || efforts.some((effort) => !effort)) return "invalid-reasoning-levels";
    if (custom.defaultReasoningLevel && !efforts.includes(custom.defaultReasoningLevel)) return "invalid-reasoning-default";
    seen.add(slug);
  }
  const effective = new Set(mode === "official-plus-custom" ? officialSlugs : []);
  overlay.custom.forEach((item) => effective.add(item.slug.trim()));
  if (defaultModel.trim() && !effective.has(defaultModel.trim())) return "invalid-default-model";
  return null;
}

export function catalogDiffSummary(diff: {
  added: readonly string[];
  updated: readonly string[];
  removed: readonly string[];
  collisions: readonly string[];
}): string {
  return `${diff.added.length}/${diff.updated.length}/${diff.removed.length}/${diff.collisions.length}`;
}

export function adoptionPreviewSummary(preview: {
  officialOverrideCount: number;
  customModels: readonly unknown[];
  collisions: readonly unknown[];
}): { adoptable: boolean; summary: string } {
  return {
    adoptable: preview.collisions.length === 0,
    summary: `${preview.officialOverrideCount}/${preview.customModels.length}/${preview.collisions.length}`,
  };
}

export function profileCatalogFlags(profile: {
  restartRequired: boolean;
  actionRequired: string | null;
}): { restart: boolean; partialFailure: boolean } {
  return {
    restart: profile.restartRequired,
    partialFailure: !!profile.actionRequired,
  };
}

export function managedContextConflictKeys(config: string): string[] {
  return ["model_context_window", "model_auto_compact_token_limit"].filter((key) =>
    new RegExp(`^\\s*${key}\\s*=`, "m").test(config),
  );
}

export function providerManagedContextConflictKeys(
  profile: { configContents: string; contextWindow: string; autoCompactLimit: string },
  liveConfigContents = "",
): string[] {
  const conflicts = new Set([
    ...managedContextConflictKeys(profile.configContents),
    ...managedContextConflictKeys(liveConfigContents),
  ]);
  if (profile.contextWindow.trim()) conflicts.add("model_context_window");
  if (profile.autoCompactLimit.trim()) conflicts.add("model_auto_compact_token_limit");
  return [...conflicts];
}

export function externalVersionRequiresAcceptance(status: string): boolean {
  return status === "mismatch";
}

/// Complete guidance for a committed generation whose runtime contract changed. Codex Minus never
/// terminates or relaunches a host, keeps this bound to the single existing restart marker, and
/// has no trustworthy runtime observer, so it states the unknown adoption instead of clearing
/// the marker on its own.
export function catalogRestartGuidance(restartRequired: boolean): string[] {
  if (!restartRequired) return [];
  return [
    "完整退出并重新启动 Codex / Desktop / IDE 宿主；本工具不会替你结束或重启这些进程。",
    "重启后新建一个任务，本地扩展注册表才会按新的供应商与目录重建。",
    "重启并新建任务之前，现有任务仍在旧的注册表上运行。",
    "本工具没有可信的运行时观察器，不会自动清除该提示；运行时是否已采用保持未知。",
  ];
}
