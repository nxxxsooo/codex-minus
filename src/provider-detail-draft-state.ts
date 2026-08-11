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
  type ProviderDraftTransformConfirmation,
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
  sessionToken: symbol;
  lifecycle: "active" | "closed";
  profile: P;
  catalogDraft: ProfileCatalogDraft | null;
  latestTransformRevision: number;
  pendingTransformRevision: number | null;
  pendingTransition: ProviderDetailTransitionIntent | null;
  pendingConfirmation: ProviderDetailPendingConfirmation | null;
  inspection: ProviderDetailInspectionMetadata | null;
  preview: ProviderDetailTransformPreview | null;
  blockers: string[];
  rawConfigContents: string | null;
};

export type ProviderDetailTransitionIntent = {
  patch: Partial<ProviderConfigRoutableProfile>;
  target: ProviderConfigTargetContract;
  transition: ProviderDraftTransition;
};

export type ProviderDetailPendingConfirmation = ProviderDetailTransitionIntent & {
  requiredConfirmation: ProviderDraftTransformConfirmation;
};

export type ProviderDetailTransformInvocation<P extends ProviderDetailProfile> = {
  command: "transform_provider_native_capability_draft";
  request: ProviderDraftTransformRequest<P>;
};

export type ProviderDetailEffect<P extends ProviderDetailProfile> =
  | {
      kind: "transform";
      invocation: ProviderDetailTransformInvocation<P>;
      correlation: ProviderDetailTransformCorrelation;
    }
  | { kind: "commit"; invocation: ProviderCommitInvocation };

export type ProviderDetailTransformCorrelation = {
  sessionToken: symbol;
  profileId: string;
  revision: number;
};

export type ProviderDetailInspectionCorrelation = ProviderDetailTransformCorrelation;

export type ProviderDetailStep<P extends ProviderDetailProfile> = {
  state: ProviderDetailDraftState<P>;
  effects: ProviderDetailEffect<P>[];
};

export function createProviderDetailDraftState<P extends ProviderDetailProfile>(input: {
  profile: P;
  catalogDraft: ProfileCatalogDraft | null;
}): ProviderDetailDraftState<P> {
  if (input.catalogDraft && input.catalogDraft.profileId !== input.profile.id) {
    throw new Error("Provider detail catalog draft belongs to another profile.");
  }
  return {
    sessionToken: Symbol("provider-detail-session"),
    lifecycle: "active",
    profile: input.profile,
    catalogDraft: input.catalogDraft,
    latestTransformRevision: 0,
    pendingTransformRevision: null,
    pendingTransition: null,
    pendingConfirmation: null,
    inspection: null,
    preview: null,
    blockers: [],
    rawConfigContents: null,
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
  if (state.rawConfigContents !== null) {
    throw new Error("Verify the raw provider config draft before editing structured fields.");
  }
  const revision = state.latestTransformRevision + 1;
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
        latestTransformRevision: revision,
        pendingTransformRevision: null,
        pendingTransition: null,
        pendingConfirmation: null,
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
      latestTransformRevision: revision,
      pendingTransformRevision: revision,
      pendingTransition: input.transition
        ? {
            patch: { ...input.patch },
            target: input.target,
            transition: {
              ...input.transition,
              confirmations: [...input.transition.confirmations],
            },
          }
        : null,
      pendingConfirmation: null,
      inspection: null,
      preview: null,
      blockers: [],
    },
    effects: [{
      kind: "transform",
      invocation: routed,
      correlation: {
        sessionToken: state.sessionToken,
        profileId: state.profile.id,
        revision,
      },
    }],
  };
}

export function beginProviderDetailInspection<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
): ProviderDetailInspectionCorrelation {
  assertActive(state);
  return {
    sessionToken: state.sessionToken,
    profileId: state.profile.id,
    revision: state.latestTransformRevision,
  };
}

export function beginProviderDetailRawConfigEdit<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  input: {
    configContents: string;
    catalogMode: ProviderDraftTransformRequest<P>["catalogMode"];
  },
): ProviderDetailStep<P> {
  assertActive(state);
  const revision = state.latestTransformRevision + 1;
  const correlation = {
    sessionToken: state.sessionToken,
    profileId: state.profile.id,
    revision,
  };
  return {
    state: {
      ...state,
      latestTransformRevision: revision,
      pendingTransformRevision: revision,
      pendingTransition: null,
      pendingConfirmation: null,
      inspection: null,
      preview: null,
      blockers: [],
      rawConfigContents: input.configContents,
    },
    effects: [{
      kind: "transform",
      invocation: {
        command: "transform_provider_native_capability_draft",
        request: {
          draftRevision: revision,
          profile: {
            ...state.profile,
            configContents: input.configContents,
            authContents: "",
          },
          catalogMode: input.catalogMode,
          action: "validateRawEdit",
          confirmations: [],
          sourceConfigContents: state.profile.configContents,
        },
      },
      correlation,
    }],
  };
}

export type ProviderDetailTransformResponse<P extends ProviderDetailProfile> =
  ProviderDraftTransformResponse<P> & {
    inspection: ProviderDetailInspectionMetadata;
    preview: ProviderDetailTransformPreview;
  };

export function settleProviderDetailTransform<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  correlation: ProviderDetailTransformCorrelation,
  response: ProviderDetailTransformResponse<P>,
): ProviderDetailStep<P> & { disposition: "applied" | "notApplied" | "stale" } {
  if (
    state.lifecycle !== "active"
    || correlation.sessionToken !== state.sessionToken
    || correlation.profileId !== state.profile.id
    || correlation.revision !== response.draftRevision
    || response.draftRevision !== state.latestTransformRevision
    || response.draftRevision !== state.pendingTransformRevision
    || response.draft.profile.id !== state.profile.id
    || response.inspection.profileId !== state.profile.id
  ) {
    return { state, effects: [], disposition: "stale" };
  }
  const applied = applyProviderTransformResponse(state.latestTransformRevision, response);
  if (applied.kind === "stale") return { state, effects: [], disposition: "stale" };
  if (applied.kind === "notApplied") {
    const requiredConfirmation = applied.status === "confirmationRequired"
      ? exitConfirmationForAction(state.pendingTransition?.transition.action)
      : null;
    return {
      state: {
        ...state,
        pendingTransformRevision: null,
        pendingTransition: null,
        pendingConfirmation: requiredConfirmation && state.pendingTransition
          ? { ...state.pendingTransition, requiredConfirmation }
          : null,
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
      catalogDraft: applied.profile.protocol === "chatCompletions"
        ? null
        : state.catalogDraft
          ? { ...state.catalogDraft, mode: applied.catalogMode }
          : null,
      pendingTransformRevision: null,
      pendingTransition: null,
      pendingConfirmation: null,
      inspection: response.inspection,
      preview: response.preview,
      blockers: [],
      rawConfigContents: null,
    },
    effects: [],
    disposition: "applied",
  };
}

export function settleProviderDetailTransformError<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  correlation: ProviderDetailTransformCorrelation,
): ProviderDetailStep<P> & { disposition: "error" | "stale"; report: boolean } {
  if (
    state.lifecycle !== "active"
    || correlation.sessionToken !== state.sessionToken
    || correlation.profileId !== state.profile.id
    || correlation.revision !== state.latestTransformRevision
    || correlation.revision !== state.pendingTransformRevision
  ) {
    return { state, effects: [], disposition: "stale", report: false };
  }
  return {
    state: {
      ...state,
      pendingTransformRevision: null,
      pendingTransition: null,
      pendingConfirmation: null,
    },
    effects: [],
    disposition: "error",
    report: true,
  };
}

export function applyProviderDetailInspection<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  correlation: ProviderDetailInspectionCorrelation,
  inspection: ProviderDetailInspectionMetadata,
): ProviderDetailStep<P> & { disposition: "applied" | "stale" } {
  if (
    state.lifecycle !== "active"
    || correlation.sessionToken !== state.sessionToken
    || correlation.profileId !== state.profile.id
    || correlation.revision !== state.latestTransformRevision
    || inspection.profileId !== state.profile.id
  ) {
    return { state, effects: [], disposition: "stale" };
  }
  return {
    state: { ...state, inspection },
    effects: [],
    disposition: "applied",
  };
}

export function replaceProviderDetailProfile<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  profile: P,
): ProviderDetailDraftState<P> {
  assertActive(state);
  if (profile.id !== state.profile.id) {
    throw new Error("Provider detail profile replacement belongs to another session.");
  }
  return {
    ...state,
    profile,
    latestTransformRevision: state.latestTransformRevision + 1,
    pendingTransformRevision: null,
    pendingTransition: null,
    pendingConfirmation: null,
    inspection: null,
    preview: null,
    blockers: [],
    rawConfigContents: null,
  };
}

export function replaceProviderDetailCatalogDraft<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  catalogDraft: ProfileCatalogDraft | null,
): ProviderDetailDraftState<P> {
  assertActive(state);
  if (catalogDraft && catalogDraft.profileId !== state.profile.id) {
    throw new Error("Provider detail catalog draft belongs to another profile.");
  }
  return {
    ...state,
    catalogDraft,
    latestTransformRevision: state.latestTransformRevision + 1,
    pendingTransformRevision: null,
    pendingTransition: null,
    pendingConfirmation: null,
    inspection: null,
    preview: null,
    blockers: [],
  };
}

export function endProviderDetailSession<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
  _reason: "cancel" | "close" | "navigate",
): ProviderDetailStep<P> {
  return {
    state: {
      ...state,
      lifecycle: "closed",
      pendingTransformRevision: null,
      pendingTransition: null,
      pendingConfirmation: null,
    },
    effects: [],
  };
}

export function confirmProviderDetailTransition<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
): ProviderDetailStep<P> {
  assertActive(state);
  const pending = state.pendingConfirmation;
  if (!pending) throw new Error("No provider transition confirmation is pending.");
  return beginProviderDetailEdit(
    {
      ...state,
      pendingConfirmation: null,
      preview: null,
      blockers: [],
    },
    {
      patch: pending.patch,
      target: pending.target,
      transition: {
        ...pending.transition,
        confirmations: Array.from(new Set([
          ...pending.transition.confirmations,
          pending.requiredConfirmation,
        ])),
      },
    },
  );
}

export function cancelProviderDetailTransition<P extends ProviderDetailProfile>(
  state: ProviderDetailDraftState<P>,
): ProviderDetailStep<P> {
  assertActive(state);
  return {
    state: {
      ...state,
      pendingConfirmation: null,
      preview: null,
      blockers: [],
    },
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
    draftRevision: number;
  },
): ProviderDetailStep<P> {
  if (state.lifecycle !== "active") {
    throw new Error("Cannot commit a closed provider detail session.");
  }
  if (state.pendingTransformRevision !== null) {
    throw new Error("Cannot commit while a provider draft transform is pending.");
  }
  if (state.rawConfigContents !== null) {
    throw new Error("Cannot commit an unverified raw provider config draft.");
  }
  if (state.pendingConfirmation !== null) {
    throw new Error("Cannot commit before confirming or cancelling the provider transition preview.");
  }
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
    draftRevision: input.draftRevision,
    expectedProviderFingerprint: input.expectedProviderFingerprint,
  });
  return {
    state,
    effects: [{ kind: "commit", invocation }],
  };
}

function assertActive<P extends ProviderDetailProfile>(state: ProviderDetailDraftState<P>) {
  if (state.lifecycle !== "active") throw new Error("Provider detail session is closed.");
}

function exitConfirmationForAction(
  action: ProviderDraftTransition["action"] | undefined,
): ProviderDraftTransformConfirmation | null {
  if (action === "exitPureOAuth") return "confirmDestructivePureOAuth";
  if (
    action === "exitPureApi"
    || action === "exitLegacyCompatibility"
    || action === "exitChatCompletions"
  ) return "confirmCapabilityLoss";
  return null;
}
