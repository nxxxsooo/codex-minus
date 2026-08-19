import type { ProviderDraftTransition } from "./provider-config-transform-router.ts";

export type ProviderNativeCapabilityState =
  | "nativePriority"
  | "upgradeAvailable"
  | "degraded"
  | "compatibility"
  | "notApplicable"
  | "unknown";

export type ProviderNativeCapabilityInspectionViewSource = {
  profileId: string;
  state: string;
  fields: Array<{
    field?: unknown;
    outcome?: unknown;
    reason?: unknown;
    [key: string]: unknown;
  }>;
};

export type ProviderModePresentation =
  | "nativePriority"
  | "nativeOfficial"
  | "external"
  | "advancedCompatibility"
  | "unknown";

export type ProviderLocalAccountPlan = "free" | "paid" | "other" | "unknown";

export type ProviderNativeCapabilityView = {
  state: ProviderNativeCapabilityState;
  externalOwnership: boolean;
  upgradeAvailability:
    | "available"
    | "confirmationRequired"
    | "manualResolutionRequired"
    | "unavailable";
  upgradeAction:
    | "upgrade"
    | "replaceActorHeader"
    | "resolveLegacyProviderId"
    | null;
  officialAuthGate: "satisfied" | "signInRequired" | "unknown";
  localPlan: ProviderLocalAccountPlan;
  localPlanBlocksActivation: false;
  actorMarkerEligibility: "eligible" | "ineligible" | "notApplicable" | "unknown";
  providerRoutableCapabilityProof: "unverified";
};

const NATIVE_CAPABILITY_STATES = new Set<ProviderNativeCapabilityState>([
  "nativePriority",
  "upgradeAvailable",
  "degraded",
  "compatibility",
  "notApplicable",
  "unknown",
]);

export function deriveProviderModePresentation(
  inspection: ProviderNativeCapabilityInspectionViewSource | null,
): ProviderModePresentation {
  if (!inspection) return "unknown";
  const reasons = new Set(inspection.fields.map((entry) => entry.reason));
  if (reasons.has("externalCatalog")) return "external";
  if (reasons.has("pureOAuth")) return "nativeOfficial";
  const legacyCompatibilityContract = inspection.state === "upgradeAvailable"
    && reasons.has("providerNameMismatch")
    && reasons.has("openAiAuthRequired")
    && reasons.has("missingActorHeader");
  if (
    legacyCompatibilityContract
    || reasons.has("pureApi")
    || reasons.has("unsupportedRelayMode")
    || reasons.has("legacyProviderIdRequiresRename")
  ) return "advancedCompatibility";
  if (
    inspection.state === "nativePriority"
    || inspection.state === "upgradeAvailable"
    || inspection.state === "degraded"
  ) return "nativePriority";
  return "unknown";
}

export function deriveProviderNativeCapabilityView(input: {
  inspection: ProviderNativeCapabilityInspectionViewSource | null;
  officialAuth: {
    authenticated: boolean | null;
    localPlan: ProviderLocalAccountPlan;
  };
}): ProviderNativeCapabilityView {
  const state = NATIVE_CAPABILITY_STATES.has(
      input.inspection?.state as ProviderNativeCapabilityState,
    )
    ? input.inspection?.state as ProviderNativeCapabilityState
    : "unknown";
  const externalOwnership = input.inspection?.fields.some(
    (entry) => entry.reason === "externalCatalog",
  ) ?? false;
  const legacyProviderIdRequiresRename = input.inspection?.fields.some(
    (entry) => entry.reason === "legacyProviderIdRequiresRename",
  ) ?? false;
  const nonSatisfiedFields = input.inspection?.fields.filter(
    (entry) => entry.outcome !== "satisfied",
  ) ?? [];
  const actorHeaderIsOnlyConflict = nonSatisfiedFields.length === 1
    && nonSatisfiedFields[0].reason === "actorHeaderValueConflict";
  const legacyProviderIdIsOnlyConflict = nonSatisfiedFields.length === 1
    && nonSatisfiedFields[0].reason === "legacyProviderIdRequiresRename";
  const actorField = input.inspection?.fields.find(
    (entry) => entry.field === "actorHeader",
  );
  const actorMarkerEligibility = externalOwnership || state === "notApplicable"
    ? "notApplicable"
    : actorField?.outcome === "satisfied"
      ? "eligible"
      : actorField
        ? "ineligible"
        : "unknown";
  const upgradeAvailability = externalOwnership
    ? "unavailable" as const
    : legacyProviderIdIsOnlyConflict
      ? "manualResolutionRequired" as const
      : actorHeaderIsOnlyConflict
        ? "confirmationRequired" as const
        : !legacyProviderIdRequiresRename && state === "upgradeAvailable"
          ? "available" as const
          : "unavailable" as const;
  return {
    state,
    externalOwnership,
    upgradeAvailability,
    upgradeAction: upgradeAvailability === "available"
      ? "upgrade"
      : upgradeAvailability === "confirmationRequired"
        ? "replaceActorHeader"
        : upgradeAvailability === "manualResolutionRequired"
          ? "resolveLegacyProviderId"
          : null,
    officialAuthGate: input.officialAuth.authenticated === true
      ? "satisfied"
      : input.officialAuth.authenticated === false
        ? "signInRequired"
        : "unknown",
    localPlan: input.officialAuth.localPlan,
    localPlanBlocksActivation: false,
    actorMarkerEligibility,
    providerRoutableCapabilityProof: "unverified",
  };
}

type ProviderMode = {
  relayMode: string;
  officialMixApiKey: boolean;
};

export function providerAccessModeHint(profile: { officialMixApiKey: boolean }): string {
  return profile.officialMixApiKey
    ? "官方登录＋混入 API Key＋Responses API"
    : "官方登录＋不写 API Key＋Responses API";
}

/// Whether the provider detail editor offers the explicit "切换到纯 API" exit for this profile.
/// Only an existing mixed Responses contract can exit: pure OAuth has no provider table to keep,
/// an already-pure-API profile has nothing to exit, and external catalog ownership keeps every
/// transform blocked until explicit adoption.
export function providerPureApiExitAvailable(
  profile: { relayMode: string; officialMixApiKey: boolean },
  view: { externalOwnership: boolean },
): boolean {
  return profile.relayMode === "official"
    && profile.officialMixApiKey
    && !view.externalOwnership;
}

/// Whether a draft is a pure-OAuth profile holding a nonblank structured key — the state that
/// must pass through the explicit native-priority enablement confirmation before it can persist.
export function providerPureOAuthKeyEnablementPending(profile: {
  relayMode: string;
  officialMixApiKey: boolean;
  apiKey: string;
}): boolean {
  return profile.relayMode === "official"
    && !profile.officialMixApiKey
    && profile.apiKey.trim().length > 0;
}

export type ProviderStructuredTransitionDecision =
  | { kind: "noChange" }
  | { kind: "requiresExplicitUpgrade" }
  | { kind: "transition"; transition: ProviderDraftTransition };

export function providerTransitionDecisionForStructuredPatch(
  current: ProviderMode,
  patch: Partial<ProviderMode>,
): ProviderStructuredTransitionDecision {
  const next = { ...current, ...patch };
  const changed = (
    ("relayMode" in patch && patch.relayMode !== current.relayMode)
    || ("officialMixApiKey" in patch
      && patch.officialMixApiKey !== current.officialMixApiKey)
  );
  if (!changed) return { kind: "noChange" };
  if ("relayMode" in patch || "officialMixApiKey" in patch) {
    if (next.relayMode === "pureApi") {
      return {
        kind: "transition",
        transition: { action: "exitPureApi", confirmations: [] },
      };
    }
    if (next.relayMode === "official" && !next.officialMixApiKey) {
      return {
        kind: "transition",
        transition: { action: "exitPureOAuth", confirmations: [] },
      };
    }
    // Entering the mixed contract (pure OAuth or pure API → official + key) is the explicit
    // native-priority enablement; the revisioned backend transform owns the table edit and the
    // enablement confirmation gates it before any persist.
    return {
      kind: "transition",
      transition: { action: "enableNativePriority", confirmations: [] },
    };
  }
  return { kind: "requiresExplicitUpgrade" };
}
