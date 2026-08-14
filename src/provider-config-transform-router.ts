import {
  applyProviderConfigPatch,
  type ProviderConfigProfile,
  type ProviderConfigTargetContract,
} from "./provider-config-draft.ts";
import type { CatalogModeValue } from "./model-catalog-ui.ts";

export type ProviderCatalogMode = CatalogModeValue;

export type ProviderDraftTransformAction =
  | "inspect"
  | "enableNativePriority"
  | "exitPureApi"
  | "exitLegacyCompatibility"
  | "exitPureOAuth"
  | "exitChatCompletions";

export type ProviderDraftTransformConfirmation =
  | "replaceActorHeader"
  | "useStructuredKey"
  | "useProviderBearer"
  | "confirmDestructivePureOAuth"
  | "confirmCapabilityLoss";

export type ProviderDraftTransition = {
  action: ProviderDraftTransformAction;
  confirmations: ProviderDraftTransformConfirmation[];
  replacementProviderId?: string;
};

export type ProviderConfigRoutableProfile = ProviderConfigProfile & {
  relayMode: string;
  officialMixApiKey: boolean;
  authContents?: string;
};

export type ProviderDraftTransformRequest<P extends ProviderConfigRoutableProfile> = {
  draftRevision: number;
  profile: P;
  catalogMode: ProviderCatalogMode;
  action: ProviderDraftTransformAction;
  confirmations: ProviderDraftTransformConfirmation[];
  replacementProviderId?: string;
  sourceConfigContents?: string;
};

export type RoutedProviderConfigEdit<P extends ProviderConfigRoutableProfile> =
  | { kind: "synchronous"; profile: P }
  | {
      kind: "backendTransform";
      command: "transform_provider_native_capability_draft";
      request: ProviderDraftTransformRequest<P>;
    };

type RouteProviderConfigDraftEditInput<P extends ProviderConfigRoutableProfile> = {
  profile: P;
  patch: Partial<ProviderConfigRoutableProfile>;
  target: ProviderConfigTargetContract;
  catalogMode?: ProviderCatalogMode;
  draftRevision?: number;
  transition?: ProviderDraftTransition;
};

const BACKEND_TRANSFORM_FIELDS = ["relayMode", "officialMixApiKey", "protocol"] as const;

export function providerConfigPatchRequiresBackendTransform(
  patch: Partial<ProviderConfigRoutableProfile>,
): boolean {
  return BACKEND_TRANSFORM_FIELDS.some((field) => field in patch);
}

/**
 * Routes provider draft edits without parsing TOML in the browser.
 *
 * The only actor-authorized synchronous materialization is delegated to the
 * brand-new-empty builder. Existing actor/auth/mode transitions are represented
 * as a revisioned request for the Rust toml_edit transformer.
 */
export function routeProviderConfigDraftEdit<P extends ProviderConfigRoutableProfile>(
  input: RouteProviderConfigDraftEditInput<P>,
): RoutedProviderConfigEdit<P> {
  if (input.profile.authContents?.trim() || "authContents" in input.patch) {
    throw new Error("Provider auth contents are forbidden in draft transforms.");
  }
  if ("configContents" in input.patch) {
    throw new Error("Raw provider TOML changes require a dedicated backend transform.");
  }
  if (input.target.source === "brand-new-empty") {
    if (input.profile.configContents.trim()) {
      throw new Error("The brand-new empty target cannot be used for existing provider TOML.");
    }
    if (input.transition) {
      throw new Error("A brand-new empty draft must use the synchronous target builder.");
    }
    return {
      kind: "synchronous",
      profile: applyProviderConfigPatch(input.profile, input.patch, input.target),
    };
  }

  const hasBackendOwnedPatch = providerConfigPatchRequiresBackendTransform(input.patch);
  if (hasBackendOwnedPatch && !input.transition) {
    throw new Error("Existing provider mode and auth changes require a revisioned backend transform.");
  }
  if (input.transition) {
    const unsupportedFields = Object.keys(input.patch).filter(
      (field) => !BACKEND_TRANSFORM_FIELDS.includes(field as typeof BACKEND_TRANSFORM_FIELDS[number]),
    );
    if (unsupportedFields.length) {
      throw new Error("Apply ordinary structured edits before requesting a revisioned backend transform.");
    }
    if (input.catalogMode === undefined || input.draftRevision === undefined) {
      throw new Error("A revisioned backend transform requires catalog mode and draft revision.");
    }
    return {
      kind: "backendTransform",
      command: "transform_provider_native_capability_draft",
      request: {
        draftRevision: input.draftRevision,
        profile: input.profile,
        catalogMode: input.catalogMode,
        action: input.transition.action,
        confirmations: [...input.transition.confirmations],
        ...(input.transition.replacementProviderId
          ? { replacementProviderId: input.transition.replacementProviderId }
          : {}),
      },
    };
  }

  return {
    kind: "synchronous",
    profile: applyProviderConfigPatch(input.profile, input.patch, input.target),
  };
}

export type ProviderDraftTransformResponse<P extends ProviderConfigRoutableProfile> = {
  draftRevision: number;
  status: "ready" | "blocked" | "confirmationRequired";
  draft: {
    profile: P;
    structuredApiKey: string;
    catalogMode: ProviderCatalogMode;
  };
  blockers: string[];
};

export type AppliedProviderTransformResponse<P extends ProviderConfigRoutableProfile> =
  | { kind: "stale" }
  | { kind: "notApplied"; status: "blocked" | "confirmationRequired"; blockers: string[] }
  | { kind: "applied"; profile: P; catalogMode: ProviderCatalogMode };

export function applyProviderTransformResponse<P extends ProviderConfigRoutableProfile>(
  currentDraftRevision: number,
  response: ProviderDraftTransformResponse<P>,
): AppliedProviderTransformResponse<P> {
  if (response.draftRevision !== currentDraftRevision) return { kind: "stale" };
  if (response.status !== "ready") {
    return {
      kind: "notApplied",
      status: response.status,
      blockers: [...response.blockers],
    };
  }
  return {
    kind: "applied",
    profile: {
      ...response.draft.profile,
      apiKey: response.draft.structuredApiKey,
      authContents: "",
    },
    catalogMode: response.draft.catalogMode,
  };
}
