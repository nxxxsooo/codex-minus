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

export type ProviderUpstreamEvidence = {
  textResponses: "reachable" | "fallbackReachable" | "denied" | "unknown";
  imageGeneration: "permissionVerified" | "denied" | "unknown";
};

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
  | { kind: "unknown" }
  | {
      kind: "verifiedTargetPolicy";
      policySource: "targetCliPolicy";
      targetVersion: string;
      capabilityPath: "providerRoutedImageActorMarker";
      freePlanRule: "blocked" | "notBlocked";
    };

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
    activationGate: "satisfied" | "blocked" | "unknown" | "notApplicable";
    inactiveSaveDisposition: "satisfied" | "actionRequired" | "notApplicable";
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
  upstream: ProviderUpstreamEvidence;
  runtime: { state: ProviderRuntimeEvidence };
  route: { label: "nativePriorityMixed" | "pureApi" | "compatibility" };
  image: {
    planGate: "blocked" | "notBlocked" | "unknown";
    status: "blocked" | "unknown";
    planEvidenceScope: {
      targetVersion: string;
      capabilityPath: "providerRoutedImageActorMarker";
    } | null;
  };
};

export function buildProviderCapabilityLedger(
  input: ProviderCapabilityLedgerInput,
): ProviderCapabilityLedger {
  const oauthApplicable = input.routeKind === "nativePriorityMixed";
  const signedIn = input.oauthSession === "signedIn";
  const oauthActivationGate = !oauthApplicable
    ? "notApplicable"
    : signedIn
      ? "satisfied"
      : input.oauthSession === "signedOut" || input.oauthSession === "expired"
        ? "blocked"
        : "unknown";
  const verifiedImagePlanScope = input.localPlan === "free"
    && input.routeKind === "nativePriorityMixed"
    && input.actorMarker === "eligible"
    && input.imagePlanEvidence.kind === "verifiedTargetPolicy"
    && input.imagePlanEvidence.policySource === "targetCliPolicy"
    && input.imagePlanEvidence.capabilityPath === "providerRoutedImageActorMarker"
    && input.imagePlanEvidence.targetVersion.trim().length > 0
    ? input.imagePlanEvidence
    : null;
  const imagePlanGate = verifiedImagePlanScope?.freePlanRule ?? "unknown";
  const routeLabel = providerRouteLabel(input.routeKind);

  return {
    provider: { state: input.providerContract },
    oauth: {
      session: input.oauthSession,
      activationGate: oauthActivationGate,
      inactiveSaveDisposition: !oauthApplicable
        ? "notApplicable"
        : signedIn
          ? "satisfied"
          : "actionRequired",
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
    upstream: { ...input.upstream },
    runtime: { state: input.runtime },
    route: { label: routeLabel },
    image: {
      planGate: imagePlanGate,
      status: imagePlanGate === "blocked" || input.upstream.imageGeneration === "denied"
        ? "blocked"
        : "unknown",
      planEvidenceScope: verifiedImagePlanScope
        ? {
            targetVersion: verifiedImagePlanScope.targetVersion,
            capabilityPath: verifiedImagePlanScope.capabilityPath,
          }
        : null,
    },
  };
}

function providerRouteLabel(
  routeKind: ProviderRouteKind,
): ProviderCapabilityLedger["route"]["label"] {
  switch (routeKind) {
    case "nativePriorityMixed":
      return "nativePriorityMixed";
    case "keyOnlyPureApi":
      return "pureApi";
    case "legacyCompatibility":
      return "compatibility";
    default:
      return assertNever(routeKind);
  }
}

function assertNever(value: never): never {
  throw new Error(`Unsupported provider route kind: ${String(value)}`);
}
