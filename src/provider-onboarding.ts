export type NewProviderFieldErrors = Partial<
  Record<"baseUrl" | "apiKey" | "model", "required">
>;

export type NewProviderTransientTarget = "nativePriority";
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
    model: "" as const,
    baseUrl: "" as const,
    upstreamBaseUrl: "" as const,
    apiKey: "" as const,
    protocol: "responses" as const,
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
    modelList: "" as const,
    modelWindows: "" as const,
    userAgent: "" as const,
    aggregate: null,
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
  return {
    target: profile.transientTarget,
    status: "materialized",
    missingFields: [],
    configContents: `model = "${model}"
model_provider = "custom"

[model_providers.custom]
name = "OpenAI"
base_url = "${baseUrl}"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "${apiKey}"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
`,
  };
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

function tomlString(value: string): string {
  return JSON.stringify(value).slice(1, -1);
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
