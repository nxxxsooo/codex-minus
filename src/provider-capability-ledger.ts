export type ProviderContractEvidence =
  | "ready"
  | "upgradeAvailable"
  | "conflicting"
  | "invalid"
  | "notApplicable"
  | "unknown";

export type ProviderOAuthSessionEvidence =
  | "signedIn"
  | "signedOut"
  | "expired"
  | "unknown";

export type ProviderLocalPlanEvidence = "free" | "paid" | "other" | "unknown";

export type ProviderActorMarkerEvidence =
  | "eligible"
  | "ineligible"
  | "notApplicable"
  | "unknown";

export type ProviderCatalogModelEvidence =
  | "supported"
  | "missingMetadata"
  | "stale"
  | "unknown";

export type ProviderUpstreamEvidence =
  | "textReachable"
  | "imagePermissionVerified"
  | "denied"
  | "unknown";

export type ProviderRuntimeEvidence =
  | "restartRequired"
  | "newTaskRequired"
  | "adopted"
  | "unknown";

export type ProviderRouteKind =
  | "nativePriorityMixed"
  | "keyOnlyPureApi"
  | "legacyCompatibility";

export type ProviderImagePlanEvidence =
  | "verifiedFreePlanBlocked"
  | "verifiedNoFreePlanBlock"
  | "unknown";

export type ProviderCapabilityLedgerInput = {
  providerContract: ProviderContractEvidence;
  oauthSession: ProviderOAuthSessionEvidence;
  localPlan: ProviderLocalPlanEvidence;
  actorMarker: ProviderActorMarkerEvidence;
  catalogModel: ProviderCatalogModelEvidence;
  upstream: ProviderUpstreamEvidence;
  runtime: ProviderRuntimeEvidence;
  routeKind: ProviderRouteKind;
  imagePlanEvidence: ProviderImagePlanEvidence;
};

export type ProviderCapabilityLedger = {
  provider: { state: ProviderContractEvidence };
  oauth: {
    session: ProviderOAuthSessionEvidence;
    activationGate: "satisfied" | "blocked" | "unknown";
    mayActivate: boolean;
  };
  plan: {
    observed: ProviderLocalPlanEvidence;
    provesCapabilitySuccess: false;
  };
  actor: {
    state: ProviderActorMarkerEvidence;
    provesEligibilityOnly: true;
  };
  catalogModel: { state: ProviderCatalogModelEvidence };
  upstream: { state: ProviderUpstreamEvidence };
  runtime: { state: ProviderRuntimeEvidence };
  route: { label: "nativePriorityMixed" | "pureApi" | "compatibility" };
  image: {
    planGate: "blocked" | "notBlocked" | "unknown";
    status: "blocked" | "unknown";
  };
  inactiveSave: "allowed" | "allowedActionRequired";
};

export function buildProviderCapabilityLedger(
  input: ProviderCapabilityLedgerInput,
): ProviderCapabilityLedger {
  const signedIn = input.oauthSession === "signedIn";
  const oauthActivationGate = signedIn
    ? "satisfied"
    : input.oauthSession === "signedOut" || input.oauthSession === "expired"
      ? "blocked"
      : "unknown";
  const imagePlanGate = input.localPlan === "free"
    ? input.imagePlanEvidence === "verifiedFreePlanBlocked"
      ? "blocked"
      : input.imagePlanEvidence === "verifiedNoFreePlanBlock"
        ? "notBlocked"
        : "unknown"
    : "unknown";
  const routeLabel = input.routeKind === "keyOnlyPureApi"
    ? "pureApi"
    : input.routeKind === "legacyCompatibility"
      ? "compatibility"
      : "nativePriorityMixed";

  return {
    provider: { state: input.providerContract },
    oauth: {
      session: input.oauthSession,
      activationGate: oauthActivationGate,
      mayActivate: signedIn,
    },
    plan: {
      observed: input.localPlan,
      provesCapabilitySuccess: false,
    },
    actor: {
      state: input.actorMarker,
      provesEligibilityOnly: true,
    },
    catalogModel: { state: input.catalogModel },
    upstream: { state: input.upstream },
    runtime: { state: input.runtime },
    route: { label: routeLabel },
    image: {
      planGate: imagePlanGate,
      status: imagePlanGate === "blocked" || input.upstream === "denied"
        ? "blocked"
        : "unknown",
    },
    inactiveSave: signedIn ? "allowed" : "allowedActionRequired",
  };
}
