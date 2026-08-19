import { tomlString } from "./codex-toml.ts";

export type NewProviderFieldErrors = Partial<
  Record<"baseUrl" | "apiKey" | "model", "required">
>;

export type NewProviderTransientTarget = "nativePriority" | "pureApi";
export type NewProviderRequiredField = "baseUrl" | "apiKey" | "model";
export type NewProviderMaterializationStatus =
  | "incomplete"
  | "materialized"
  | "backend-transform-required";

export type NewProviderMaterialization = {
  target: NewProviderTransientTarget;
  status: NewProviderMaterializationStatus;
  missingFields: NewProviderRequiredField[];
  configContents: string;
};

type NewProviderMaterializationInput = {
  transientTarget: NewProviderTransientTarget;
  model: string;
  baseUrl: string;
  apiKey: string;
  configContents: string;
};

/// Models a ChatGPT Pro account can route through a native-priority provider.
///
/// Shipped in the app rather than discovered, so a brand-new provider is usable before any
/// upstream call. The first entry is the prefilled default and must be a slug the official
/// bundled catalog carries: a default the catalog cannot represent fails the first save.
export const PRO_MODEL_SLUGS = [
  "gpt-5.6-terra",
  "gpt-5.6-luna",
  "gpt-5.6-sol",
  "gpt-5.5",
  "gpt-5.3-codex-spark",
] as const;

/// Slugs the official bundled catalog hides. Kept beside the shipped list so a retired model is
/// caught by a test rather than shipped to a user whose catalog can no longer represent it.
export const RETIRED_MODEL_SLUGS = ["gpt-5.4", "gpt-5.4-mini"] as const;

/// Provider table identifier a brand-new draft is created with.
///
/// Distinct from Codex's reserved built-in lowercase `openai`, which the pinned core rewrites and
/// upstream Codex ignores in favour of its own entry. Sessions record this identifier, so an
/// upgrade preserves whatever a profile already has instead of adopting this value.
export const NEW_PROVIDER_ID = "OpenAI";

export function createNewRelayProfileDraft<TContext>({
  id,
  contextSelection,
}: {
  id: string;
  contextSelection: TContext;
}) {
  return {
    id,
    name: "" as const,
    model: PRO_MODEL_SLUGS[0] as string,
    baseUrl: "" as const,
    apiKey: "" as const,
    relayMode: "official" as const,
    officialMixApiKey: true as const,
    transientTarget: "nativePriority" as const,
    testModel: "" as const,
    configContents: "" as const,
    authContents: "" as const,
    useCommonConfig: true as const,
    contextSelection,
    contextSelectionInitialized: true as const,
    contextWindow: "" as const,
    autoCompactLimit: "" as const,
    modelList: PRO_MODEL_SLUGS.join("\n"),
    modelWindows: "" as const,
    userAgent: "" as const,
  };
}

export function materializeNewProviderConfig(
  profile: NewProviderMaterializationInput,
): NewProviderMaterialization {
  const missingFields = requiredNewProviderFields(profile);
  if (profile.configContents.trim()) {
    return {
      target: profile.transientTarget,
      status: "backend-transform-required",
      missingFields,
      configContents: profile.configContents,
    };
  }
  if (missingFields.length) {
    return {
      target: profile.transientTarget,
      status: "incomplete",
      missingFields,
      configContents: "",
    };
  }

  const model = tomlString(profile.model.trim());
  const baseUrl = tomlString(profile.baseUrl.trim());
  const apiKey = tomlString(profile.apiKey.trim());
  // The pure-API target is the explicit no-login contract: `requires_openai_auth = false`, no
  // actor-authorization header, provider bearer only. It never claims native-capability priority;
  // the target CLI runs on the bearer alone.
  const contract = profile.transientTarget === "pureApi"
    ? `requires_openai_auth = false
experimental_bearer_token = "${apiKey}"
`
    : `requires_openai_auth = true
experimental_bearer_token = "${apiKey}"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
`;
  return {
    target: profile.transientTarget,
    status: "materialized",
    missingFields: [],
    configContents: `model = "${model}"
model_provider = "${NEW_PROVIDER_ID}"

[model_providers.${NEW_PROVIDER_ID}]
name = "OpenAI"
base_url = "${baseUrl}"
wire_api = "responses"
${contract}`,
  };
}

/// The one-time target choice on the new-provider page. The mixed native-priority target stays
/// the default; pure API is the explicit path for a user who cannot sign in to ChatGPT. Selecting
/// a target patches the draft's structured mode fields together with the transient target so the
/// brand-new materializer emits the matching contract.
export const NEW_PROVIDER_TARGET_OPTIONS: ReadonlyArray<{
  value: NewProviderTransientTarget;
  label: string;
  hint: string;
}> = [
  {
    value: "nativePriority",
    label: "官方登录＋混入 API Key（默认）",
    hint: "需要已登录的 ChatGPT 客户端；保留官方登录体验。",
  },
  {
    value: "pureApi",
    label: "纯 API（无需官方登录）",
    hint: "无法登录 ChatGPT 时选这个；只用中转 Key 请求，不声明官方登录派生的原生能力。",
  },
];

export function newProviderTargetPatch(target: NewProviderTransientTarget): {
  transientTarget: NewProviderTransientTarget;
  relayMode: "official" | "pureApi";
  officialMixApiKey: boolean;
} {
  return target === "pureApi"
    ? { transientTarget: "pureApi", relayMode: "pureApi", officialMixApiKey: false }
    : { transientTarget: "nativePriority", relayMode: "official", officialMixApiKey: true };
}

function requiredNewProviderFields(profile: {
  baseUrl: string;
  apiKey: string;
  model: string;
}): NewProviderRequiredField[] {
  const missing: NewProviderRequiredField[] = [];
  if (!profile.baseUrl.trim()) missing.push("baseUrl");
  if (!profile.apiKey.trim()) missing.push("apiKey");
  if (!profile.model.trim()) missing.push("model");
  return missing;
}

export function validateNewProviderDraft(profile: {
  baseUrl: string;
  apiKey: string;
  model: string;
}): NewProviderFieldErrors {
  const errors: NewProviderFieldErrors = {};
  for (const field of requiredNewProviderFields(profile)) errors[field] = "required";
  return errors;
}

export const OFFICIAL_AUTH_GUIDE_URL = "https://developers.openai.com/codex/auth";

export function officialLoginGuide(input: {
  isNew: boolean;
  authenticated: boolean;
}): { visible: boolean; url: string } {
  return {
    visible: input.isNew && !input.authenticated,
    url: OFFICIAL_AUTH_GUIDE_URL,
  };
}
