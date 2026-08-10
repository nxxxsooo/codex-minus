export type NewProviderFieldErrors = Partial<
  Record<"baseUrl" | "apiKey" | "model", "required">
>;

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

export function validateNewProviderDraft(profile: {
  baseUrl: string;
  apiKey: string;
  model: string;
}): NewProviderFieldErrors {
  const errors: NewProviderFieldErrors = {};
  if (!profile.baseUrl.trim()) errors.baseUrl = "required";
  if (!profile.apiKey.trim()) errors.apiKey = "required";
  if (!profile.model.trim()) errors.model = "required";
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
