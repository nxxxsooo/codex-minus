import {
  ensureTrailingNewline,
  rootTomlStringValue,
  tomlSectionName,
  tomlString,
} from "./codex-toml.ts";
import type { CatalogModeValue } from "./model-catalog-ui.ts";
import {
  materializeNewProviderConfig,
  type NewProviderTransientTarget,
} from "./provider-onboarding.ts";

export const PROTOCOL_PROXY_BASE_URL = "http://127.0.0.1:57321/v1";

export const CHAT_UPSTREAM_BASE_URL_KEY = "codex_plus_chat_base_url";

/// The two provenances a provider config edit can have. A brand-new empty draft materializes the
/// canonical native-priority contract through `materializeNewProviderConfig` — the one generator —
/// and an existing config is only ever patched in place, never reconstructed. The retired
/// four-target picker chose between alternate generators here; every alternate became unreachable
/// once the editor stopped offering 接入模式, so the choice collapsed into provenance alone.
export type ProviderConfigTargetContract =
  | { target: "nativePriority"; source: "brand-new-empty" }
  | { target: "preserveExisting"; source: "existing" };

export type ProviderConfigProfile = {
  model: string;
  baseUrl: string;
  upstreamBaseUrl: string;
  apiKey: string;
  protocol: string;
  configContents: string;
  contextWindow: string;
  autoCompactLimit: string;
  transientTarget?: NewProviderTransientTarget;
};

function brandNewProviderConfig(profile: ProviderConfigProfile): string {
  return materializeNewProviderConfig({
    transientTarget: "nativePriority",
    model: profile.model,
    baseUrl: profile.baseUrl,
    apiKey: profile.apiKey,
    configContents: profile.configContents,
  }).configContents;
}

export function applyProviderConfigPatch<T extends ProviderConfigProfile>(
  profile: T,
  patch: Partial<ProviderConfigProfile>,
  contract: ProviderConfigTargetContract,
): T {
  let next = { ...profile, ...patch } as T;
  if (!next.configContents.trim() && contract.source === "brand-new-empty") {
    next = { ...next, configContents: brandNewProviderConfig(next) };
    if (!next.configContents.trim()) return next;
  }
  if (!next.configContents.trim()) return next;

  if ("model" in patch) {
    next.configContents = setRootTomlStringKey(
      next.configContents,
      "model",
      modelSlug(patch.model ?? ""),
    );
  }
  if ("apiKey" in patch) {
    next.configContents = setProviderStringKey(
      next.configContents,
      "experimental_bearer_token",
      patch.apiKey ?? "",
    );
  }
  if ("baseUrl" in patch) next.upstreamBaseUrl = patch.baseUrl ?? "";
  if ("upstreamBaseUrl" in patch) next.baseUrl = patch.upstreamBaseUrl ?? "";
  if ("baseUrl" in patch || "upstreamBaseUrl" in patch || "protocol" in patch) {
    const baseUrl = next.protocol === "chatCompletions"
      ? PROTOCOL_PROXY_BASE_URL
      : next.upstreamBaseUrl || next.baseUrl;
    next.configContents = setProviderStringKey(next.configContents, "base_url", baseUrl);
    next.configContents = removeRootTomlKey(next.configContents, CHAT_UPSTREAM_BASE_URL_KEY);
  }
  if ("contextWindow" in patch) {
    next.configContents = setRootTomlIntKey(
      next.configContents,
      "model_context_window",
      patch.contextWindow ?? "",
    );
  }
  if ("autoCompactLimit" in patch) {
    next.configContents = setRootTomlIntKey(
      next.configContents,
      "model_auto_compact_token_limit",
      patch.autoCompactLimit ?? "",
    );
  }
  return next;
}

function modelSlug(value: string): string {
  const trimmed = value.trim();
  const match = /^(.*?)\[(\d+(?:[KkMm])?)\]$/.exec(trimmed);
  return match ? match[1].trim() : trimmed;
}

function setRootTomlStringKey(contents: string, key: string, value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return removeRootTomlKey(contents, key);
  return setRootTomlLine(contents, key, `${key} = "${tomlString(trimmed)}"`);
}

function setRootTomlIntKey(contents: string, key: string, value: string): string {
  const trimmed = value.replace(/[^\d]/g, "");
  if (!trimmed) return removeRootTomlKey(contents, key);
  return setRootTomlLine(contents, key, `${key} = ${trimmed}`);
}

function setRootTomlLine(contents: string, key: string, lineText: string): string {
  const lines = contents.split(/\r?\n/);
  const firstTable = lines.findIndex((line) => /^\s*\[[^\]]+\]\s*$/.test(line));
  const rootEnd = firstTable >= 0 ? firstTable : lines.length;
  for (let index = 0; index < rootEnd; index += 1) {
    if (new RegExp(`^\\s*${key}\\s*=`).test(lines[index])) {
      lines[index] = lineText;
      return ensureTrailingNewline(lines.join("\n").trimEnd());
    }
  }
  lines.splice(key === "model" ? 0 : rootEnd, 0, lineText);
  return ensureTrailingNewline(lines.join("\n").trimEnd());
}

function setProviderStringKey(contents: string, key: string, value: string): string {
  const provider = rootTomlStringValue(contents, "model_provider");
  if (!provider) return contents;
  const section = `model_providers.${provider}`;
  return value.trim()
    ? setTomlSectionRawKey(contents, section, key, `"${tomlString(value.trim())}"`)
    : removeTomlSectionKey(contents, section, key);
}

function setTomlSectionRawKey(
  contents: string,
  sectionName: string,
  key: string,
  value: string,
): string {
  const lines = contents.split(/\r?\n/);
  let sectionStart = -1;
  let sectionEnd = lines.length;
  for (let index = 0; index < lines.length; index += 1) {
    const section = tomlSectionName(lines[index]);
    if (section === null) continue;
    if (sectionStart >= 0) {
      sectionEnd = index;
      break;
    }
    if (section === sectionName) sectionStart = index;
  }
  if (sectionStart < 0) return contents;
  const replacement = `${key} = ${value}`;
  for (let index = sectionStart + 1; index < sectionEnd; index += 1) {
    if (new RegExp(`^\\s*${key}\\s*=`).test(lines[index])) {
      lines[index] = replacement;
      return ensureTrailingNewline(lines.join("\n").trimEnd());
    }
  }
  let insertAt = sectionEnd;
  while (insertAt > sectionStart + 1 && lines[insertAt - 1].trim() === "") insertAt -= 1;
  lines.splice(insertAt, 0, replacement);
  return ensureTrailingNewline(lines.join("\n").trimEnd());
}

function removeTomlSectionKey(contents: string, sectionName: string, key: string): string {
  const lines = contents.split(/\r?\n/);
  let sectionStart = -1;
  let sectionEnd = lines.length;
  for (let index = 0; index < lines.length; index += 1) {
    const section = tomlSectionName(lines[index]);
    if (section === null) continue;
    if (sectionStart >= 0) {
      sectionEnd = index;
      break;
    }
    if (section === sectionName) sectionStart = index;
  }
  if (sectionStart < 0) return contents;
  return ensureTrailingNewline(lines.filter((line, index) => (
    index <= sectionStart
    || index >= sectionEnd
    || !new RegExp(`^\\s*${key}\\s*=`).test(line)
  )).join("\n").trimEnd());
}

function removeRootTomlKey(contents: string, key: string): string {
  const lines: string[] = [];
  let inRoot = true;
  for (const line of contents.split(/\r?\n/)) {
    if (/^\s*\[[^\]]+\]\s*$/.test(line)) inRoot = false;
    if (inRoot && new RegExp(`^\\s*${key}\\s*=`).test(line)) continue;
    lines.push(line);
  }
  return ensureTrailingNewline(lines.join("\n").trimEnd());
}

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

/// Fields the browser must not rewrite inside existing TOML: mode and auth transitions delete or
/// create provider tables, which is the Rust transformer's job.
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
