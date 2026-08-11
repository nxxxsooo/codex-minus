import assert from "node:assert";
import { describe, it } from "node:test";

import {
  OFFICIAL_AUTH_GUIDE_URL,
  createNewRelayProfileDraft,
  materializeNewProviderConfig,
  officialLoginGuide,
  validateNewProviderDraft,
} from "./provider-onboarding.ts";

describe("provider onboarding", () => {
  it("creates only the official auth mixed Responses draft", () => {
    const contextSelection = { mcpServers: ["memory"], skills: [], plugins: [] };
    const draft = createNewRelayProfileDraft({ id: "relay-new", contextSelection });

    assert.equal(draft.id, "relay-new");
    assert.equal(draft.name, "");
    assert.equal(draft.baseUrl, "");
    assert.equal(draft.upstreamBaseUrl, "");
    assert.equal(draft.apiKey, "");
    assert.equal(draft.model, "");
    assert.equal(draft.relayMode, "official");
    assert.equal(draft.officialMixApiKey, true);
    assert.equal(draft.protocol, "responses");
    assert.equal(draft.transientTarget, "nativePriority");
    assert.strictEqual(draft.contextSelection, contextSelection);
    assert.equal(draft.authContents, "");

    assert.deepEqual(materializeNewProviderConfig(draft), {
      target: "nativePriority",
      status: "incomplete",
      missingFields: ["baseUrl", "apiKey", "model"],
      configContents: "",
    });
  });

  it("materializes the exact native-priority TOML only when every new-provider input is complete", () => {
    const empty = createNewRelayProfileDraft({ id: "relay-new", contextSelection: {} });
    const complete = {
      ...empty,
      model: "gpt-5.5",
      baseUrl: "https://relay.example/v1",
      upstreamBaseUrl: "https://relay.example/v1",
      apiKey: "provider-key",
    };

    assert.deepEqual(materializeNewProviderConfig({ ...complete, apiKey: " " }), {
      target: "nativePriority",
      status: "incomplete",
      missingFields: ["apiKey"],
      configContents: "",
    });
    assert.deepEqual(materializeNewProviderConfig(complete), {
      target: "nativePriority",
      status: "materialized",
      missingFields: [],
      configContents: `model = "gpt-5.5"
model_provider = "custom"

[model_providers.custom]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "provider-key"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
`,
    });
  });

  it("refuses to synchronously rewrite any existing provider TOML", () => {
    const empty = createNewRelayProfileDraft({ id: "relay-new", contextSelection: {} });
    const existing = `# preserve this exact draft
model_provider = "legacy"
[model_providers.legacy]
name = "Custom"
http_headers = { "x-owned" = "keep" }
`;

    assert.deepEqual(materializeNewProviderConfig({
      ...empty,
      model: "gpt-5.5",
      baseUrl: "https://relay.example/v1",
      apiKey: "provider-key",
      configContents: existing,
    }), {
      target: "nativePriority",
      status: "backend-transform-required",
      missingFields: [],
      configContents: existing,
    });
  });

  it("reports each required first-save field", () => {
    assert.deepEqual(
      validateNewProviderDraft({ baseUrl: " ", apiKey: "", model: "\n" }),
      { baseUrl: "required", apiKey: "required", model: "required" },
    );
    assert.deepEqual(
      validateNewProviderDraft({ baseUrl: "https://relay.example/v1", apiKey: "sk-test", model: "gpt-5.5" }),
      {},
    );
  });

  it("shows the official login guide only for an unauthenticated new draft", () => {
    assert.deepEqual(
      officialLoginGuide({ isNew: true, authenticated: false }),
      { visible: true, url: OFFICIAL_AUTH_GUIDE_URL },
    );
    assert.equal(officialLoginGuide({ isNew: true, authenticated: true }).visible, false);
    assert.equal(officialLoginGuide({ isNew: false, authenticated: false }).visible, false);
  });
});
