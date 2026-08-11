import {
  buildProviderMutationInvocation,
  type ProfileCatalogDraft,
  type ProviderCommitInvocation,
  type ProviderRelayProfileSource,
  type ProviderSettingsSource,
} from "./provider-commit.ts";
import {
  applyProviderTransformResponse,
  routeProviderConfigDraftEdit,
  type ProviderConfigRoutableProfile,
  type ProviderDraftTransformRequest,
  type ProviderDraftTransformResponse,
  type ProviderDraftTransition,
} from "./provider-config-transform-router.ts";
import type { ProviderConfigTargetContract } from "./provider-config-draft.ts";

export type ProviderDetailProfile = ProviderRelayProfileSource & ProviderConfigRoutableProfile;

export type ProviderDetailInspectionMetadata = {
  profileId: string;
  state: string;
  fields: Array<Record<string, unknown>>;
};

export type ProviderDetailTransformPreview = {
  capabilityLoss: boolean;
  removesProviderTable: boolean;
  removedProviderId: string | null;
  removedProviderFields: string[];
};

export type ProviderDetailDraftState<P extends ProviderDetailProfile> = {
  lifecycle: "active" | "closed";
  profile: P;
  catalogDraft: ProfileCatalogDraft | null;
  latestRevision: number;
  pendingTransformRevision: number | null;
  inspection: ProviderDetailInspectionMetadata | null;
  preview: ProviderDetailTransformPreview | null;
  blockers: string[];
};

export type ProviderDetailTransformInvocation<P extends ProviderDetailProfile> = {
  command: "transform_provider_native_capability_draft";
  request: ProviderDraftTransformRequest<P>;
};

export type ProviderDetailEffect<P extends ProviderDetailProfile> =
  | { kind: "transform"; invocation: ProviderDetailTransformInvocation<P> }
  | { kind: "commit"; invocation: ProviderCommitInvocation };

export type ProviderDetailStep<P extends ProviderDetailProfile> = {
  state: ProviderDetailDraftState<P>;
  effects: ProviderDetailEffect<P>[];
};

export function createProviderDetailDraftState<P extends ProviderDetailProfile>(input: {
  profile: P;
  catalogDraft: ProfileCatalogDraft | null;
}): ProviderDetailDraftState<P> {
  return {
    lifecycle: "active",
    profile: input.profile,
    catalogDraft: input.catalogDraft,
    latestRevision: 0,
    pendingTransformRevision: null,
    inspection: null,
    preview: null,
    blockers: [],
  };
}

export function beginProviderDetailEdit<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  input: {
    patch: Partial<ProviderConfigRoutableProfile>;
    target: ProviderConfigTargetContract;
    transition?: ProviderDraftTransition;
  },
): ProviderDetailStep<P> {
  assertActive(state);
  const revision = state.latestRevision + 1;
  const routed = routeProviderConfigDraftEdit({
    profile: state.profile,
    patch: input.patch,
    target: input.target,
    ...(input.transition
      ? {
          transition: input.transition,
          catalogMode: state.catalogDraft?.mode,
          draftRevision: revision,
        }
      : {}),
  });
  if (routed.kind === "synchronous") {
    return {
      state: {
        ...state,
        profile: routed.profile,
        latestRevision: revision,
        pendingTransformRevision: null,
        inspection: null,
        preview: null,
        blockers: [],
      },
      effects: [],
    };
  }
  return {
    state: {
      ...state,
      latestRevision: revision,
      pendingTransformRevision: revision,
      inspection: null,
      preview: null,
      blockers: [],
    },
    effects: [{ kind: "transform", invocation: routed }],
  };
}

export type ProviderDetailTransformResponse<P extends ProviderDetailProfile> =
  ProviderDraftTransformResponse<P> & {
    inspection: ProviderDetailInspectionMetadata;
    preview: ProviderDetailTransformPreview;
  };

export function settleProviderDetailTransform<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  response: ProviderDetailTransformResponse<P>,
): ProviderDetailStep<P> & { disposition: "applied" | "notApplied" | "stale" } {
  if (
    state.lifecycle !== "active"
    || response.draftRevision !== state.latestRevision
    || response.draftRevision !== state.pendingTransformRevision
  ) {
    return { state, effects: [], disposition: "stale" };
  }
  const applied = applyProviderTransformResponse(state.latestRevision, response);
  if (applied.kind === "stale") return { state, effects: [], disposition: "stale" };
  if (applied.kind === "notApplied") {
    return {
      state: {
        ...state,
        pendingTransformRevision: null,
        inspection: response.inspection,
        preview: response.preview,
        blockers: [...response.blockers],
      },
      effects: [],
      disposition: "notApplied",
    };
  }
  return {
    state: {
      ...state,
      profile: applied.profile,
      catalogDraft: state.catalogDraft
        ? { ...state.catalogDraft, mode: applied.catalogMode }
        : null,
      pendingTransformRevision: null,
      inspection: response.inspection,
      preview: response.preview,
      blockers: [],
    },
    effects: [],
    disposition: "applied",
  };
}

export function settleProviderDetailTransformError<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  revision: number,
): ProviderDetailStep<P> & { disposition: "error" | "stale"; report: boolean } {
  if (
    state.lifecycle !== "active"
    || revision !== state.latestRevision
    || revision !== state.pendingTransformRevision
  ) {
    return { state, effects: [], disposition: "stale", report: false };
  }
  return {
    state: { ...state, pendingTransformRevision: null },
    effects: [],
    disposition: "error",
    report: true,
  };
}

export function endProviderDetailSession<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  _reason: "cancel" | "close" | "navigate",
): ProviderDetailStep<P> {
  return {
    state: { ...state, lifecycle: "closed", pendingTransformRevision: null },
    effects: [],
  };
}

export function buildProviderDetailCommitEffect<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  input: {
    kind: "detailSave" | "setCurrent";
    settings: ProviderSettingsSource & Record<string, unknown>;
    persistedSettings: ProviderSettingsSource & Record<string, unknown>;
    catalogDrafts: ProfileCatalogDraft[];
    focusedProfileWasPersisted: boolean;
    previousActiveRelayId: string;
    confirmContextCleanup: boolean;
    expectedProviderFingerprint: string;
  },
): ProviderDetailStep<P> {
  if (state.lifecycle !== "active") {
    throw new Error("Cannot commit a closed provider detail session.");
  }
  if (state.pendingTransformRevision !== null) {
    throw new Error("Cannot commit while a provider draft transform is pending.");
  }
  const revision = state.latestRevision + 1;
  const existingIndex = input.settings.relayProfiles.findIndex(
    (candidate) => candidate.id === state.profile.id,
  );
  const relayProfiles = [...input.settings.relayProfiles];
  if (existingIndex >= 0) relayProfiles[existingIndex] = state.profile;
  else relayProfiles.push(state.profile);
  const settings = {
    ...input.settings,
    relayProfiles,
    activeRelayId: input.kind === "setCurrent"
      ? state.profile.id
      : input.settings.activeRelayId,
  };
  const catalogDrafts = input.catalogDrafts.filter(
    (candidate) => candidate.profileId !== state.profile.id,
  );
  if (state.catalogDraft) catalogDrafts.push(state.catalogDraft);
  const invocation = buildProviderMutationInvocation({
    settings,
    persistedSettings: input.persistedSettings,
    catalogDrafts,
    kind: input.kind,
    focusedProfileId: state.profile.id,
    focusedProfileWasPersisted: input.focusedProfileWasPersisted,
    previousActiveRelayId: input.previousActiveRelayId,
    confirmContextCleanup: input.confirmContextCleanup,
    draftRevision: revision,
    expectedProviderFingerprint: input.expectedProviderFingerprint,
  });
  return {
    state: { ...state, latestRevision: revision },
    effects: [{ kind: "commit", invocation }],
  };
}

function assertActive<P extends ProviderDetailProfile>(state: ProviderDetailDraftState<P>) {
  if (state.lifecycle !== "active") throw new Error("Provider detail session is closed.");
}
