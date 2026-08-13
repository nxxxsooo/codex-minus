import assert from "node:assert";
import { describe, it } from "node:test";

import {
  applyProviderConfigPatch,
  buildRelayConfigToml,
  withGeneratedRelayConfig,
} from "./provider-config-draft.ts";
import { createNewRelayProfileDraft } from "./provider-onboarding.ts";

const nativeTarget = { target: "nativePriority", source: "brand-new-empty" } as const;
const existingTarget = { target: "preserveExisting", source: "existing" } as const;

describe("provider config draft targets", () => {
  it("keeps a brand-new native-priority config empty until the final required input is present", () => {
    const empty = createNewRelayProfileDraft({ id: "relay-new", contextSelection: {} });
    assert.equal(withGeneratedRelayConfig(empty, nativeTarget).configContents, "");

    const modelOnly = applyProviderConfigPatch(empty, { model: "gpt-5.5" }, nativeTarget);
    assert.equal(modelOnly.model, "gpt-5.5");
    assert.equal(modelOnly.configContents, "");

    const withUrl = applyProviderConfigPatch(
      modelOnly,
      { baseUrl: "https://relay.example/v1" },
      nativeTarget,
    );
    assert.equal(withUrl.baseUrl, "https://relay.example/v1");
    assert.equal(withUrl.configContents, "");

    const complete = applyProviderConfigPatch(withUrl, { apiKey: "provider-key" }, nativeTarget);
    assert.equal(complete.configContents, buildRelayConfigToml(complete, nativeTarget));
    assert.match(complete.configContents, /name = "OpenAI"/);
    assert.match(complete.configContents, /requires_openai_auth = false/);
    assert.match(complete.configContents, /experimental_bearer_token = "provider-key"/);
    assert.match(
      complete.configContents,
      /http_headers = \{ "x-openai-actor-authorization" = "local-image-extension" \}/,
    );
  });

  it("edits existing provider values without reconstructing its native-priority table", () => {
    const existing = `# preserve root comment
model = "gpt-old"
model_provider = "relay"

[model_providers.relay]
name = "OpenAI"
base_url = "https://old.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "old-key"
http_headers = { "x-openai-actor-authorization" = "local-image-extension", "x-keep" = "yes" }
custom_flag = "keep"
`;
    const profile = {
      ...createNewRelayProfileDraft({ id: "saved", contextSelection: {} }),
      transientTarget: undefined,
      model: "gpt-old",
      baseUrl: "https://old.example/v1",
      upstreamBaseUrl: "https://old.example/v1",
      apiKey: "old-key",
      configContents: existing,
    };

    assert.equal(
      buildRelayConfigToml(profile, { target: "pureOAuth", source: "brand-new-empty" }),
      existing,
      "even a mislabeled synchronous contract must not destructively rewrite existing TOML",
    );

    const edited = applyProviderConfigPatch(profile, {
      model: "gpt-next",
      baseUrl: "https://next.example/v1",
      apiKey: "next-key",
      contextWindow: "272000",
      autoCompactLimit: "240000",
    }, existingTarget);

    assert.match(edited.configContents, /^# preserve root comment/m);
    assert.match(edited.configContents, /model = "gpt-next"/);
    assert.match(edited.configContents, /base_url = "https:\/\/next\.example\/v1"/);
    assert.match(edited.configContents, /experimental_bearer_token = "next-key"/);
    assert.match(edited.configContents, /model_context_window = 272000/);
    assert.match(edited.configContents, /model_auto_compact_token_limit = 240000/);
    assert.match(edited.configContents, /name = "OpenAI"/);
    assert.match(edited.configContents, /requires_openai_auth = false/);
    assert.doesNotMatch(edited.configContents, /requires_openai_auth = true/);
    assert.match(
      edited.configContents,
      /http_headers = \{ "x-openai-actor-authorization" = "local-image-extension", "x-keep" = "yes" \}/,
    );
    assert.match(edited.configContents, /custom_flag = "keep"/);
  });

  it("keeps the complete native contract on a Base-URL-only existing edit", () => {
    const existing = `model = "gpt-5.5"
model_provider = "RelayOne"

[model_providers.RelayOne]
name = "OpenAI"
base_url = "https://before.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "provider-key"
custom_provider_field = "keep-provider"
http_headers = { "x-openai-actor-authorization" = "local-image-extension", "x-unowned" = "keep-header" }
`;
    const profile = {
      ...createNewRelayProfileDraft({ id: "base-only", contextSelection: {} }),
      transientTarget: undefined,
      model: "gpt-5.5",
      baseUrl: "https://before.example/v1",
      upstreamBaseUrl: "https://before.example/v1",
      apiKey: "provider-key",
      configContents: existing,
    };

    const edited = applyProviderConfigPatch(
      profile,
      { baseUrl: "https://after.example/v1" },
      existingTarget,
    );

    assert.match(edited.configContents, /base_url = "https:\/\/after\.example\/v1"/);
    assert.match(edited.configContents, /requires_openai_auth = false/);
    assert.doesNotMatch(edited.configContents, /requires_openai_auth = true/);
    assert.match(
      edited.configContents,
      /"x-openai-actor-authorization" = "local-image-extension"/,
    );
    assert.match(edited.configContents, /custom_provider_field = "keep-provider"/);
    assert.match(edited.configContents, /"x-unowned" = "keep-header"/);
  });

  it("never introduces global feature ownership into a new canonical provider", () => {
    const profile = {
      ...createNewRelayProfileDraft({ id: "no-global-features", contextSelection: {} }),
      model: "gpt-5.5",
      baseUrl: "https://relay.example/v1",
      apiKey: "provider-key",
    };
    const config = buildRelayConfigToml(profile, nativeTarget);

    assert.doesNotMatch(config, /^\s*\[features\]\s*$/m);
    assert.doesNotMatch(config, /^\s*goals\s*=/m);
  });

  it("requires every builder call to choose an explicit non-native compatibility target", () => {
    const profile = {
      ...createNewRelayProfileDraft({ id: "compat", contextSelection: {} }),
      model: "compat-model",
      baseUrl: "https://compat.example/v1",
      apiKey: "compat-key",
    };
    const compatibility = buildRelayConfigToml(profile, {
      target: "compatibility",
      source: "brand-new-empty",
    });
    const pureApi = buildRelayConfigToml(profile, {
      target: "pureApi",
      source: "brand-new-empty",
    });

    assert.match(compatibility, /requires_openai_auth = true/);
    assert.doesNotMatch(compatibility, /x-openai-actor-authorization/);
    assert.match(pureApi, /requires_openai_auth = false/);
    assert.doesNotMatch(pureApi, /x-openai-actor-authorization/);
  });
});
