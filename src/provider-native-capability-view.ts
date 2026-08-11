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

export type ProviderLocalAccountPlan = "free" | "paid" | "other" | "unknown";

export type ProviderNativeCapabilityView = {
  state: ProviderNativeCapabilityState;
  externalOwnership: boolean;
  upgradeAvailability: "available" | "unavailable";
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
  return {
    state,
    externalOwnership,
    upgradeAvailability: state === "upgradeAvailable" && !externalOwnership
      ? "available"
      : "unavailable",
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

type ProviderModeProtocol = {
  relayMode: string;
  officialMixApiKey: boolean;
  protocol: string;
};

export type ProviderStructuredTransitionDecision =
  | { kind: "noChange" }
  | { kind: "requiresExplicitUpgrade" }
  | { kind: "transition"; transition: ProviderDraftTransition };

export function providerTransitionDecisionForStructuredPatch(
  current: ProviderModeProtocol,
  patch: Partial<ProviderModeProtocol>,
): ProviderStructuredTransitionDecision {
  const next = { ...current, ...patch };
  const changed = (
    ("relayMode" in patch && patch.relayMode !== current.relayMode)
    || ("officialMixApiKey" in patch
      && patch.officialMixApiKey !== current.officialMixApiKey)
    || ("protocol" in patch && patch.protocol !== current.protocol)
  );
  if (!changed) return { kind: "noChange" };
  if (next.protocol === "chatCompletions") {
    return {
      kind: "transition",
      transition: { action: "exitChatCompletions", confirmations: [] },
    };
  }
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
  return { kind: "requiresExplicitUpgrade" };
}
