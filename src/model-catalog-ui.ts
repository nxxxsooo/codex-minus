export type CatalogModeValue = "native-official" | "official-plus-custom" | "custom-only" | "external";

export type CatalogOverlayDraft = {
  official: Record<string, { visible: boolean | null; contextWindow: number | null; order: number | null }>;
  custom: Array<{
    slug: string;
    displayName: string;
    contextWindow: number;
    visible: boolean;
    order: number;
    templateProvenance: string;
  }>;
};

export function defaultCatalogMode(
  relayMode: string,
  officialMixApiKey: boolean,
  externalPointer?: string | null,
): CatalogModeValue {
  if (externalPointer) return "external";
  if (relayMode === "pureApi") return "custom-only";
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
        visible: true,
        order: overlay.custom.length,
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
    if (seen.has(slug)) return "duplicate-custom-slug";
    if (!Number.isFinite(custom.contextWindow) || custom.contextWindow <= 0) return "invalid-context-window";
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
