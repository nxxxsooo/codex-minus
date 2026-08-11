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
  renamedProviderFrom?: string | null;
  renamedProviderTo?: string | null;
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
  pendingLegacyProviderIdResolution: ProviderDetailLegacyProviderIdResolution | null;
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

export type ProviderDetailLegacyProviderIdResolution = ProviderDetailTransitionIntent & {
  reason: "required" | "invalid" | "unavailable";
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
    pendingLegacyProviderIdResolution: null,
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
        pendingLegacyProviderIdResolution: null,
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
      pendingLegacyProviderIdResolution: null,
      inspection: state.inspection,
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

export function beginProviderDetailNativePriorityUpgrade<
  P extends ProviderDetailProfile,
>(state: ProviderDetailDraftState<P>): ProviderDetailStep<P> {
  assertActive(state);
  const externalOwnership = state.inspection?.fields.some(
    (entry) => entry.reason === "externalCatalog",
  ) ?? false;
  const chatCompatibility = state.inspection?.state === "compatibility" && (
    state.inspection.fields.some((entry) => entry.reason === "chatCompletions")
  );
  const legacyProviderIdRequiresRename = state.inspection?.fields.some(
    (entry) => entry.reason === "legacyProviderIdRequiresRename",
  ) ?? false;
  const nonSatisfiedFields = state.inspection?.fields.filter(
    (entry) => entry.outcome !== "satisfied",
  ) ?? [];
  const actorHeaderIsOnlyConflict = nonSatisfiedFields.length === 1
    && nonSatisfiedFields[0].reason === "actorHeaderValueConflict";
  if (
    (
      state.inspection?.state !== "upgradeAvailable"
      && !chatCompatibility
      && !actorHeaderIsOnlyConflict
    )
    || externalOwnership
    || legacyProviderIdRequiresRename
  ) {
    throw new Error("This provider is not eligible for the ordinary native-priority upgrade.");
  }
  return beginProviderDetailEdit(state, {
    patch: {
      relayMode: "official",
      officialMixApiKey: true,
      protocol: "responses",
    },
    target: { target: "preserveExisting", source: "existing" },
    transition: { action: "enableNativePriority", confirmations: [] },
  });
}

export function beginProviderDetailLegacyIdUpgrade<
  P extends ProviderDetailProfile,
>(state: ProviderDetailDraftState<P>): ProviderDetailStep<P> {
  assertActive(state);
  const nonSatisfiedFields = state.inspection?.fields.filter(
    (entry) => entry.outcome !== "satisfied",
  ) ?? [];
  const legacyProviderIdIsOnlyConflict = nonSatisfiedFields.length === 1
    && nonSatisfiedFields[0].reason === "legacyProviderIdRequiresRename";
  const externalOwnership = state.inspection?.fields.some(
    (entry) => entry.reason === "externalCatalog",
  ) ?? false;
  if (!legacyProviderIdIsOnlyConflict || externalOwnership) {
    throw new Error("This provider is not eligible for a legacy provider ID upgrade.");
  }
  return beginProviderDetailEdit(state, {
    patch: {
      relayMode: "official",
      officialMixApiKey: true,
      protocol: "responses",
    },
    target: { target: "preserveExisting", source: "existing" },
    transition: { action: "enableNativePriority", confirmations: [] },
  });
}

export function resolveProviderDetailLegacyProviderId<
  P extends ProviderDetailProfile,
>(
  state: ProviderDetailDraftState<P>,
  replacementProviderId: string,
): ProviderDetailStep<P> {
  assertActive(state);
  const pending = state.pendingLegacyProviderIdResolution;
  if (!pending) throw new Error("No legacy provider ID resolution is pending.");
  const replacement = replacementProviderId.trim();
  if (
    !replacement
    || replacement === "openai"
    || replacement === "CodexPlusPlus"
    || replacement === "CodexPP"
  ) {
    throw new Error("Choose a non-empty, non-reserved replacement provider ID.");
  }
  return beginProviderDetailEdit(
    {
      ...state,
      pendingLegacyProviderIdResolution: null,
      preview: null,
      blockers: [],
    },
    {
      patch: pending.patch,
      target: pending.target,
      transition: {
        ...pending.transition,
        replacementProviderId: replacement,
      },
    },
  );
}

export function cancelProviderDetailLegacyProviderIdResolution<
  P extends ProviderDetailProfile,
>(state: ProviderDetailDraftState<P>): ProviderDetailStep<P> {
  assertActive(state);
  return {
    state: {
      ...state,
      pendingLegacyProviderIdResolution: null,
      preview: null,
      blockers: [],
    },
    effects: [],
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
      pendingLegacyProviderIdResolution: null,
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
      ? confirmationForResponse(
          state.pendingTransition?.transition.action,
          response.blockers,
        )
      : null;
    const legacyProviderIdResolution = state.pendingTransition
      ? legacyProviderIdResolutionForResponse(
          state.pendingTransition.transition.action,
          response.blockers,
        )
      : null;
    return {
      state: {
        ...state,
        pendingTransformRevision: null,
        pendingTransition: null,
        pendingConfirmation: requiredConfirmation && state.pendingTransition
          ? { ...state.pendingTransition, requiredConfirmation }
          : null,
        pendingLegacyProviderIdResolution:
          legacyProviderIdResolution && state.pendingTransition
            ? { ...state.pendingTransition, reason: legacyProviderIdResolution }
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
      pendingLegacyProviderIdResolution: null,
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
  const failedLegacyRetry = state.pendingTransition?.transition.action === "enableNativePriority"
    && !!state.pendingTransition.transition.replacementProviderId
    ? { ...state.pendingTransition, reason: "unavailable" as const }
    : null;
  return {
    state: {
      ...state,
      pendingTransformRevision: null,
      pendingTransition: null,
      pendingConfirmation: null,
      pendingLegacyProviderIdResolution: failedLegacyRetry,
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
    pendingLegacyProviderIdResolution: null,
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
    pendingLegacyProviderIdResolution: null,
    inspection: null,
    preview: null,
    blockers: [],
  };
}

export function refreshProviderDetailCatalogDraftState<
  P extends ProviderDetailProfile,
>(
  state: ProviderDetailDraftState<P>,
  catalogDraft: ProfileCatalogDraft | null,
  authoritativeProfile: P,
): {
  state: ProviderDetailDraftState<P>;
  inspectionCorrelation: ProviderDetailInspectionCorrelation | null;
} {
  if (authoritativeProfile.id !== state.profile.id) {
    throw new Error("Authoritative provider profile belongs to another detail session.");
  }
  const profileMatchesAuthoritative = JSON.stringify(state.profile)
    === JSON.stringify(authoritativeProfile);
  const refreshed = replaceProviderDetailCatalogDraft(state, catalogDraft);
  return {
    state: refreshed,
    inspectionCorrelation: profileMatchesAuthoritative
      ? beginProviderDetailInspection(refreshed)
      : null,
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
      pendingLegacyProviderIdResolution: null,
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
  if (state.pendingLegacyProviderIdResolution !== null) {
    throw new Error("Cannot commit before resolving or cancelling the legacy provider ID.");
  }
  if (state.blockers.length) {
    throw new Error("Cannot commit a provider draft blocked by backend validation.");
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

function confirmationForResponse(
  action: ProviderDraftTransition["action"] | undefined,
  blockers: string[],
): ProviderDraftTransformConfirmation | null {
  if (
    action === "enableNativePriority"
    && blockers.length === 1
    && blockers[0] === "actorHeaderValueConflict"
  ) return "replaceActorHeader";
  if (
    action === "exitPureOAuth"
    && blockers.length === 1
    && blockers[0] === "destructiveExitConfirmationRequired"
  ) return "confirmDestructivePureOAuth";
  if (
    blockers.length === 1
    && blockers[0] === "capabilityLossConfirmationRequired"
    && (
      action === "exitPureApi"
      || action === "exitLegacyCompatibility"
      || action === "exitChatCompletions"
    )
  ) return "confirmCapabilityLoss";
  return null;
}

function legacyProviderIdResolutionForResponse(
  action: ProviderDraftTransition["action"] | undefined,
  blockers: string[],
): ProviderDetailLegacyProviderIdResolution["reason"] | null {
  if (action !== "enableNativePriority" || blockers.length !== 1) return null;
  if (blockers[0] === "replacementProviderIdRequired") return "required";
  if (blockers[0] === "replacementProviderIdInvalid") return "invalid";
  if (blockers[0] === "replacementProviderIdUnavailable") return "unavailable";
  return null;
}
