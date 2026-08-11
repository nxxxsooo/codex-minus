import type { CatalogModeValue, CatalogOverlayDraft } from "./model-catalog-ui.ts";
import type { CatalogUpstreamTopology, ProfileCatalogDraft } from "./provider-commit.ts";

type CatalogDraftSummary = {
  profileId: string;
  mode: CatalogModeValue;
  modeExplicit: boolean;
  upstreamTopology: CatalogUpstreamTopology;
  externalPointer: string | null;
  overlay: CatalogOverlayDraft;
  generatedPath?: unknown;
  actionRequired?: unknown;
};

export function catalogProfileDraft(input: {
  profileId: string;
  fallbackMode: CatalogModeValue;
  summary: CatalogDraftSummary | null;
}): ProfileCatalogDraft {
  return {
    profileId: input.profileId,
    mode: input.summary?.mode ?? input.fallbackMode,
    modeExplicit: input.summary?.modeExplicit ?? false,
    upstreamTopology: input.summary?.upstreamTopology ?? "direct",
    externalPointer: input.summary?.externalPointer ?? null,
    overlay: input.summary?.overlay ?? { official: {}, custom: [] },
  };
}

export function updateCatalogProfileDraft(
  draft: ProfileCatalogDraft,
  patch: Partial<Omit<ProfileCatalogDraft, "profileId">>,
): ProfileCatalogDraft {
  return { ...draft, ...patch, profileId: draft.profileId };
}

export function catalogEditingAvailability(combinedSaveAvailable: boolean): {
  editable: boolean;
  label: string;
} {
  return combinedSaveAvailable
    ? { editable: true, label: "可随统一保存提交" }
    : { editable: false, label: "统一保存接入后可编辑" };
}
