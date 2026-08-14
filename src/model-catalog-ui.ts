import { KNOWN_RELAY_MODELS, knownRelayModel } from "./known-relay-models.ts";

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
    description?: string;
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

export type OfficialCatalogOverrideDraft = CatalogOverlayDraft["official"][string];

export function emptyOfficialOverride(): OfficialCatalogOverrideDraft {
  return {
    displayName: null,
    visible: null,
    contextWindow: null,
    effectiveContextWindowPercent: null,
    order: null,
    supportedReasoningLevels: null,
    defaultReasoningLevel: null,
    supportedTools: null,
    toolCapabilities: null,
  };
}

/// Whether a model reaches the Codex picker: the overlay's answer where it has one, the bundled
/// baseline's otherwise.
///
/// The baseline already hides models Codex retired, so an editor that lists every official entry
/// shows models the picker will not — two lists that disagree about the same profile.
export function officialModelIsVisible(
  overlay: CatalogOverlayDraft,
  model: { slug: string; visible: boolean },
): boolean {
  return overlay.official[model.slug]?.visible ?? model.visible;
}

/// A visibility wish recorded as its difference from the baseline.
///
/// Storing `false` for a model the baseline already hides would leave a non-empty overlay that asks
/// for nothing, which promotes a native profile to a generated catalog for no reason.
export function officialVisibilityOverride(baselineVisible: boolean, wanted: boolean): boolean | null {
  return wanted === baselineVisible ? null : wanted;
}

/// While a display name has never been edited independently it simply mirrors the slug, so a
/// slug edit carries it along; the moment it differs — a preset card's name, or a user's edit —
/// a slug correction must not overwrite it.
export function customDisplayNameFollowsSlug(displayName: string, previousSlug: string): boolean {
  return displayName === previousSlug;
}

/// Slugs offered as one-click additions: everything known that the table is not already showing.
///
/// Hidden official models belong here — deleting one has to be undoable, and the row it came from
/// is gone. Known relay-model cards belong here too, whether or not the provider has reported
/// them. Slugs the table already carries do not: offering one would add a second row with the
/// same slug as an official model, which the generator reports as a collision.
export function catalogCandidateSlugs(input: {
  overlay: CatalogOverlayDraft;
  officialModels: readonly { slug: string; visible: boolean }[];
  providerCandidates: readonly string[];
}): string[] {
  const shown = new Set<string>();
  const hidden: string[] = [];
  for (const model of input.officialModels) {
    if (officialModelIsVisible(input.overlay, model)) shown.add(model.slug);
    else hidden.push(model.slug);
  }
  for (const custom of input.overlay.custom) {
    const slug = custom.slug.trim();
    if (slug) shown.add(slug);
  }
  const known = KNOWN_RELAY_MODELS.map((card) => card.slug);
  return [...new Set([...hidden, ...input.providerCandidates, ...known])].filter((slug) => !shown.has(slug));
}

/// The overlay whose visible list is exactly `wanted`.
///
/// Context windows already typed survive: repairing which models appear is not a statement about
/// how big they are.
export function restoreCatalogList(input: {
  overlay: CatalogOverlayDraft;
  officialModels: readonly { slug: string; visible: boolean }[];
  wanted: readonly string[];
}): CatalogOverlayDraft {
  const wanted = input.wanted.map((slug) => slug.trim()).filter(Boolean);
  const wantedSlugs = new Set(wanted);
  const official = Object.fromEntries(
    Object.entries(input.overlay.official).map(([slug, override]) => [slug, { ...override }]),
  );
  for (const model of input.officialModels) {
    const next = {
      ...(official[model.slug] ?? emptyOfficialOverride()),
      visible: officialVisibilityOverride(model.visible, wantedSlugs.has(model.slug)),
    };
    if (Object.values(next).every((field) => field === null)) delete official[model.slug];
    else official[model.slug] = next;
  }
  const officialSlugs = new Set(input.officialModels.map((model) => model.slug));
  let restored: CatalogOverlayDraft = {
    official,
    custom: input.overlay.custom.filter((item) => wantedSlugs.has(item.slug.trim())),
  };
  for (const slug of wanted) {
    if (officialSlugs.has(slug)) continue;
    restored = addCatalogCandidate(restored, slug);
  }
  return restored;
}

/// Rows the restore would take away, so the user is asked about them by name rather than after.
export function catalogRestoreLosses(input: {
  overlay: CatalogOverlayDraft;
  officialModels: readonly { slug: string; visible: boolean }[];
  wanted: readonly string[];
}): string[] {
  const wantedSlugs = new Set(input.wanted.map((slug) => slug.trim()).filter(Boolean));
  const losses = input.officialModels
    .filter((model) => officialModelIsVisible(input.overlay, model) && !wantedSlugs.has(model.slug))
    .map((model) => model.slug);
  for (const custom of input.overlay.custom) {
    const slug = custom.slug.trim();
    if (slug && !wantedSlugs.has(slug)) losses.push(slug);
  }
  return losses;
}

/// True when an overlay asks for nothing the official baseline does not already say.
export function catalogOverlayIsEmpty(overlay: CatalogOverlayDraft): boolean {
  if (overlay.custom.length > 0) return false;
  return Object.values(overlay.official).every((override) =>
    Object.values(override).every((field) => field === null)
  );
}

/// The mode an overlay actually needs.
///
/// Native mode generates no catalog and points at none, so an override typed there is stored and
/// then sits dormant — the number changes nothing, with no mode control left in the editor to
/// explain why. Typing one *is* the request for a managed catalog, so it becomes one. Clearing the
/// overlay again does not go back: a managed catalog that merely restates the official baseline is
/// harmless, while silently dropping ownership under the user is not.
export function catalogModeForOverlay(
  mode: CatalogModeValue,
  overlay: CatalogOverlayDraft,
): CatalogModeValue {
  if (mode !== "native-official") return mode;
  return catalogOverlayIsEmpty(overlay) ? mode : "official-plus-custom";
}

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
  // A slug the fleet has verified end to end arrives as a complete card, not template defaults.
  // The card only prefills this draft; the saved profile owns its copy from here on.
  const card = knownRelayModel(normalized);
  return {
    ...overlay,
    custom: [
      ...overlay.custom,
      card
        ? {
            slug: card.slug,
            displayName: card.displayName,
            description: card.description,
            contextWindow: card.contextWindow,
            effectiveContextWindowPercent: card.effectiveContextWindowPercent,
            visible: true,
            order: overlay.custom.length,
            supportedReasoningLevels: card.supportedReasoningLevels.map((level) => ({ ...level })),
            defaultReasoningLevel: card.defaultReasoningLevel,
            supportedTools: [],
            toolCapabilities: null,
            templateProvenance: "known-relay-model",
          }
        : {
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
  // A model the user deleted is not a model Codex can start on, even though the baseline still
  // carries it.
  const effective = new Set(mode === "official-plus-custom"
    ? officialSlugs.filter((slug) => overlay.official[slug]?.visible !== false)
    : []);
  overlay.custom.forEach((item) => effective.add(item.slug.trim()));
  // The generator refuses a catalog with nothing in it. Saying so here names the list the user is
  // looking at, rather than failing the whole transaction with a sentence about JSON.
  if (!effective.size) return "empty-catalog";
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

/// Maps the persisted readiness sentinel to a sentence. Every other value the backend writes
/// into actionRequired is already plain language, so only the code needs mapping; the caller
/// translates the result.
export function catalogActionRequiredLabel(action: string): string {
  return action === "catalog-readiness-unavailable"
    ? "模型目录当前无法生成；请检查启动模型是否仍在本版本的目录中。"
    : action;
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
