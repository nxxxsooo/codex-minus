import { KNOWN_RELAY_MODELS, knownRelayModel } from "./known-relay-models.ts";
import { SUPPLEMENTAL_OPENAI_MODELS } from "./supplemental-openai-models.ts";

export type CatalogModeValue = "native-official" | "official-plus-custom" | "custom-only" | "external";

export function catalogOfficialModels<T>(mode: CatalogModeValue, baseline: readonly T[]): readonly T[] {
  return mode === "native-official" || mode === "official-plus-custom" ? baseline : [];
}

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
  return overlay.official[model.slug]?.visible
    ?? overlay.custom.find((custom) => custom.slug === model.slug)?.visible
    ?? model.visible;
}

type CatalogModelMetadata = { slug: string; displayName?: string; contextWindow?: number | null };

/// Keep baseline metadata available for naming/prefill even when custom-only mode displays none
/// of its rows. Names come from catalog metadata, never from reformatting a request ID.
export function catalogModelDisplayName(slug: string, models: readonly CatalogModelMetadata[]): string {
  return models.find((model) => model.slug === slug)?.displayName
    ?? SUPPLEMENTAL_OPENAI_MODELS.find((model) => model.slug === slug)?.displayName
    ?? knownRelayModel(slug)?.displayName ?? slug;
}

/// Same-slug custom values compose underneath official overrides in the backend. Render that
/// result once, retaining the stored custom record and its independently edited metadata.
export function catalogOfficialRows<T extends CatalogModelMetadata & { visible: boolean }>(
  overlay: CatalogOverlayDraft, models: readonly T[], mode: CatalogModeValue,
): T[] {
  return catalogOfficialModels(mode, models).filter((model) => officialModelIsVisible(overlay, model))
    .map((model) => {
      const custom = overlay.custom.find((item) => item.slug === model.slug);
      const override = overlay.official[model.slug];
      return { ...model,
        displayName: override?.displayName ?? custom?.displayName ?? model.displayName,
        contextWindow: override?.contextWindow ?? custom?.contextWindow ?? model.contextWindow,
      };
    });
}

export function catalogCustomRows(
  overlay: CatalogOverlayDraft, models: readonly { slug: string }[], mode: CatalogModeValue,
) {
  if (mode === "external") return [];
  const official = new Set(catalogOfficialModels(mode, models).map((model) => model.slug));
  return overlay.custom.map((model, index) => ({ model, index }))
    .filter(({ model }) => model.visible && !official.has(model.slug))
    .sort((a, b) => a.model.order - b.model.order);
}

/// Switching a new draft to custom-only must carry its visible official rows into that mode;
/// baseline metadata still prefills names/windows, but the new mode has no official rows.
export function catalogOverlayForMode(
  overlay: CatalogOverlayDraft, mode: CatalogModeValue, nextMode: CatalogModeValue,
  models: readonly (CatalogModelMetadata & { visible: boolean })[],
  brandNew = true,
): CatalogOverlayDraft {
  if (!brandNew) return overlay;
  if (mode === "custom-only" && nextMode === "official-plus-custom") {
    const official = { ...overlay.official };
    const custom = overlay.custom.filter((row) => {
      const baseline = models.find((model) => model.slug === row.slug);
      // The transitional custom row is produced only for a new mixed draft switched to pure API.
      // Do not reinterpret a user-created or provider-candidate row as an official override.
      if (!baseline || row.templateProvenance !== "new-provider-official-mode-transition") return true;
      const next = {
        ...(official[row.slug] ?? emptyOfficialOverride()),
        displayName: row.displayName === (baseline.displayName ?? row.slug) ? null : row.displayName,
        visible: row.visible === baseline.visible ? null : row.visible,
        contextWindow: row.contextWindow === (baseline.contextWindow ?? 272_000) ? null : row.contextWindow,
        ...(row.effectiveContextWindowPercent === 100 ? {} : { effectiveContextWindowPercent: row.effectiveContextWindowPercent }),
        ...(row.supportedReasoningLevels.length ? { supportedReasoningLevels: row.supportedReasoningLevels } : {}),
        ...(row.defaultReasoningLevel ? { defaultReasoningLevel: row.defaultReasoningLevel } : {}),
        ...(row.supportedTools.length ? { supportedTools: row.supportedTools } : {}),
        ...(row.toolCapabilities ? { toolCapabilities: row.toolCapabilities } : {}),
      };
      if (Object.values(next).every((value) => value === null)) delete official[row.slug];
      else official[row.slug] = next;
      return false;
    });
    return { official, custom };
  }
  if (nextMode !== "custom-only" || mode === nextMode || mode === "external") return overlay;
  let next = overlay;
  for (const model of catalogOfficialRows(overlay, models, mode)) {
    const alreadyOwned = next.custom.some((row) => row.slug === model.slug);
    next = addCatalogCandidate(next, model.slug, [model]);
    next = { ...next, custom: next.custom.map((row) => row.slug === model.slug && !alreadyOwned
      ? { ...row, templateProvenance: "new-provider-official-mode-transition", displayName: model.displayName ?? row.displayName,
            contextWindow: model.contextWindow ?? row.contextWindow, visible: true }
      : row) };
  }
  return next;
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
/// them. Slugs the table already carries do not: adding them would create a redundant same-slug
/// custom override, rather than adding a model to the effective catalog.
export function catalogCandidateSlugs(input: {
  overlay: CatalogOverlayDraft;
  officialModels: readonly { slug: string; visible: boolean }[];
  providerCandidates: readonly string[];
  mode?: CatalogModeValue;
}): string[] {
  const shown = new Set<string>();
  const hidden: string[] = [];
  for (const model of input.officialModels) {
    if (input.mode !== "custom-only" && officialModelIsVisible(input.overlay, model)) shown.add(model.slug);
    else hidden.push(model.slug);
  }
  for (const custom of input.overlay.custom) {
    const slug = custom.slug.trim();
    if (!slug || (input.mode !== "custom-only" && input.officialModels.some((model) => model.slug === slug))) continue;
    if (custom.visible) shown.add(slug);
    else hidden.push(slug);
  }
  const known = [...SUPPLEMENTAL_OPENAI_MODELS, ...KNOWN_RELAY_MODELS].map((card) => card.slug);
  return [...new Set([...hidden, ...input.providerCandidates, ...known])].filter((slug) => !shown.has(slug));
}

/// The overlay whose visible list is exactly `wanted`.
///
/// Context windows already typed survive: repairing which models appear is not a statement about
/// how big they are.
export function restoreCatalogList(input: {
  overlay: CatalogOverlayDraft;
  officialModels: readonly (CatalogModelMetadata & { visible: boolean })[];
  wanted: readonly string[];
  mode?: CatalogModeValue;
}): CatalogOverlayDraft {
  // Missing official metadata must not turn an absent baseline into custom rows.
  if (!input.officialModels.length || input.mode === "external") return input.overlay;
  const wanted = input.wanted.map((slug) => slug.trim()).filter(Boolean);
  const wantedSlugs = new Set(wanted);
  const officialSlugs = new Set(input.officialModels.map((model) => model.slug));
  const custom = input.overlay.custom.filter((item) => wantedSlugs.has(item.slug.trim()) && !(
    input.mode !== "custom-only" && officialSlugs.has(item.slug) && item.templateProvenance === "provider-candidate"
    && item.displayName === item.slug && !item.description && item.contextWindow === 272000
    && item.effectiveContextWindowPercent === 100 && item.visible
    && !item.supportedReasoningLevels.length && item.defaultReasoningLevel === null
    && !item.supportedTools.length && item.toolCapabilities === null
  )).map((item) => ({ ...item, visible: true,
    // An explicit restore repairs an ID-mirroring default; an independently named row stays owned.
    displayName: item.displayName === item.slug ? catalogModelDisplayName(item.slug, input.officialModels) : item.displayName,
  }));
  const official = Object.fromEntries(
    Object.entries(input.overlay.official).map(([slug, override]) => [slug, { ...override }]),
  );
  for (const model of input.officialModels) {
    if (input.mode === "custom-only") continue;
    const next = {
      ...(official[model.slug] ?? emptyOfficialOverride()),
      visible: officialVisibilityOverride(custom.find((item) => item.slug === model.slug)?.visible ?? model.visible, wantedSlugs.has(model.slug)),
    };
    if (Object.values(next).every((field) => field === null)) delete official[model.slug];
    else official[model.slug] = next;
  }
  let restored: CatalogOverlayDraft = { official, custom };
  for (const slug of wanted) {
    if (input.mode !== "custom-only" && officialSlugs.has(slug)) continue;
    restored = addCatalogCandidate(restored, slug, input.officialModels);
  }
  return restored;
}

/// Rows the restore would take away, so the user is asked about them by name rather than after.
export function catalogRestoreLosses(input: {
  overlay: CatalogOverlayDraft;
  officialModels: readonly { slug: string; visible: boolean }[];
  wanted: readonly string[];
  mode?: CatalogModeValue;
}): string[] {
  const wantedSlugs = new Set(input.wanted.map((slug) => slug.trim()).filter(Boolean));
  const losses = input.officialModels
    .filter((model) => input.mode !== "custom-only" && officialModelIsVisible(input.overlay, model) && !wantedSlugs.has(model.slug))
    .map((model) => model.slug);
  for (const custom of input.overlay.custom) {
    const slug = custom.slug.trim();
    if (slug && !wantedSlugs.has(slug)) losses.push(slug);
  }
  return [...new Set(losses)];
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

/// Renders the display name owned by the catalog or the user.
///
/// The model slug is shown separately. Reformatting the display name would make a refreshed
/// catalog or a user-defined name look unlike the source that owns it.
export function appModelLabel(displayName: string): string {
  return displayName;
}

export function providerEvidenceState(slug: string, reportedSlugs: readonly string[]): "reported" | "not-reported" {
  return reportedSlugs.includes(slug) ? "reported" : "not-reported";
}

export function addCatalogCandidate(
  overlay: CatalogOverlayDraft,
  slug: string,
  officialModels: readonly CatalogModelMetadata[] = [],
): CatalogOverlayDraft {
  const normalized = slug.trim();
  if (!normalized) return overlay;
  const existing = overlay.custom.findIndex((item) => item.slug.trim() === normalized);
  if (existing !== -1) return overlay.custom[existing].visible ? overlay : {
    ...overlay, custom: overlay.custom.map((item, index) => index === existing ? { ...item, visible: true } : item),
  };
  // A slug the fleet has verified end to end arrives as a complete card, not template defaults.
  // The card only prefills this draft; the saved profile owns its copy from here on.
  const supplemental = SUPPLEMENTAL_OPENAI_MODELS.find((model) => model.slug === normalized);
  const card = supplemental ?? knownRelayModel(normalized);
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
            templateProvenance: supplemental ? "models-dev-openai-2026-09-22" : "known-relay-model",
          }
        : {
            slug: normalized,
            displayName: catalogModelDisplayName(normalized, officialModels),
            contextWindow: officialModels.find((model) => model.slug === normalized)?.contextWindow ?? 272000,
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
  overlay.custom.filter((item) => item.visible && (mode === "custom-only" || overlay.official[item.slug]?.visible !== false))
    .forEach((item) => effective.add(item.slug.trim()));
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
