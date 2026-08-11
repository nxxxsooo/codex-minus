import assert from "node:assert";
import { describe, it } from "node:test";

import {
  applyProviderTransformResponse,
  routeProviderConfigDraftEdit,
} from "./provider-config-transform-router.ts";
import { createNewRelayProfileDraft } from "./provider-onboarding.ts";

const newTarget = { target: "nativePriority", source: "brand-new-empty" } as const;
const existingTarget = { target: "preserveExisting", source: "existing" } as const;

const existingConfig = `# keep-root
model = "gpt-old"
model_provider = "RelayOne"

[model_providers.RelayOne]
name = "OpenAI"
base_url = "https://old.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "old-key"
custom_provider_field = "keep-provider"
http_headers = { "x-openai-actor-authorization" = "local-image-extension", "x-unowned" = "keep-header" }
`;

function existingProfile() {
  return {
    ...createNewRelayProfileDraft({ id: "saved", contextSelection: {} }),
    transientTarget: undefined,
    relayMode: "official" as const,
    officialMixApiKey: true,
    protocol: "responses",
    model: "gpt-old",
    baseUrl: "https://old.example/v1",
    upstreamBaseUrl: "https://old.example/v1",
    apiKey: "old-key",
    configContents: existingConfig,
  };
}

describe("provider config transform router", () => {
  it("synchronously creates the actor contract only for one brand-new empty draft", () => {
    const blank = createNewRelayProfileDraft({ id: "new", contextSelection: {} });
    const routed = routeProviderConfigDraftEdit({
      profile: blank,
      patch: {
        model: "gpt-5.5",
        baseUrl: "https://relay.example/v1",
        apiKey: "provider-key",
      },
      target: newTarget,
    });

    assert.equal(routed.kind, "synchronous");
    if (routed.kind !== "synchronous") return;
    assert.match(routed.profile.configContents, /name = "OpenAI"/);
    assert.match(routed.profile.configContents, /requires_openai_auth = false/);
    assert.match(
      routed.profile.configContents,
      /http_headers = \{ "x-openai-actor-authorization" = "local-image-extension" \}/,
    );
  });

  it("routes every existing actor or mode transition to one revisioned backend request", () => {
    const cases = [
      {
        label: "add actor",
        patch: { relayMode: "official", officialMixApiKey: true, protocol: "responses" },
        transition: { action: "enableNativePriority" as const, confirmations: [] },
      },
      {
        label: "replace actor",
        patch: { relayMode: "official", officialMixApiKey: true },
        transition: {
          action: "enableNativePriority" as const,
          confirmations: ["replaceActorHeader" as const],
        },
      },
      {
        label: "remove actor for pure API",
        patch: { relayMode: "pureApi", officialMixApiKey: false },
        transition: {
          action: "exitPureApi" as const,
          confirmations: ["confirmCapabilityLoss" as const],
        },
      },
      {
        label: "remove provider for pure OAuth",
        patch: { relayMode: "official", officialMixApiKey: false },
        transition: {
          action: "exitPureOAuth" as const,
          confirmations: ["confirmDestructivePureOAuth" as const],
        },
      },
      {
        label: "replace actor auth with compatibility auth",
        patch: { relayMode: "official", officialMixApiKey: true, protocol: "responses" },
        transition: {
          action: "exitLegacyCompatibility" as const,
          confirmations: ["confirmCapabilityLoss" as const],
        },
      },
      {
        label: "exit to chat completions",
        patch: { protocol: "chatCompletions" },
        transition: {
          action: "exitChatCompletions" as const,
          confirmations: ["confirmCapabilityLoss" as const],
        },
      },
    ];

    for (const [index, entry] of cases.entries()) {
      const profile = existingProfile();
      const routed = routeProviderConfigDraftEdit({
        profile,
        patch: entry.patch,
        target: existingTarget,
        catalogMode: "officialPlusCustom",
        draftRevision: 70 + index,
        transition: entry.transition,
      });

      assert.equal(routed.kind, "backendTransform", entry.label);
      if (routed.kind !== "backendTransform") continue;
      assert.equal(routed.command, "transform_provider_native_capability_draft", entry.label);
      assert.equal(routed.request.draftRevision, 70 + index, entry.label);
      assert.equal(routed.request.action, entry.transition.action, entry.label);
      assert.deepEqual(routed.request.confirmations, entry.transition.confirmations, entry.label);
      assert.equal(routed.request.profile.configContents, existingConfig, entry.label);
    }
  });

  it("routes actor and bearer conflict decisions without inspecting TOML in TypeScript", () => {
    for (const confirmation of [
      "replaceActorHeader",
      "useStructuredKey",
      "useProviderBearer",
    ] as const) {
      const routed = routeProviderConfigDraftEdit({
        profile: existingProfile(),
        patch: {},
        target: existingTarget,
        catalogMode: "officialPlusCustom",
        draftRevision: 91,
        transition: {
          action: "enableNativePriority",
          confirmations: [confirmation],
        },
      });
      assert.equal(routed.kind, "backendTransform");
      if (routed.kind !== "backendTransform") continue;
      assert.deepEqual(routed.request.confirmations, [confirmation]);
      assert.equal(routed.request.profile.configContents, existingConfig);
    }
  });

  it("rejects an existing mode or auth patch that lacks an explicit backend transition", () => {
    for (const patch of [
      { relayMode: "pureApi" },
      { officialMixApiKey: false },
      { protocol: "chatCompletions" },
    ]) {
      assert.throws(
        () => routeProviderConfigDraftEdit({
          profile: existingProfile(),
          patch,
          target: existingTarget,
        }),
        /revisioned backend transform/i,
      );
    }
  });

  it("keeps ordinary existing structured edits synchronous without clobbering native fields", () => {
    const routed = routeProviderConfigDraftEdit({
      profile: existingProfile(),
      patch: {
        model: "gpt-next",
        baseUrl: "https://next.example/v1",
        apiKey: "next-key",
        contextWindow: "272000",
        autoCompactLimit: "240000",
      },
      target: existingTarget,
    });

    assert.equal(routed.kind, "synchronous");
    if (routed.kind !== "synchronous") return;
    assert.match(routed.profile.configContents, /model = "gpt-next"/);
    assert.match(routed.profile.configContents, /base_url = "https:\/\/next\.example\/v1"/);
    assert.match(routed.profile.configContents, /experimental_bearer_token = "next-key"/);
    assert.match(routed.profile.configContents, /model_context_window = 272000/);
    assert.match(routed.profile.configContents, /model_auto_compact_token_limit = 240000/);
    assert.match(routed.profile.configContents, /requires_openai_auth = false/);
    assert.match(routed.profile.configContents, /"x-openai-actor-authorization" = "local-image-extension"/);
    assert.match(routed.profile.configContents, /custom_provider_field = "keep-provider"/);
    assert.match(routed.profile.configContents, /"x-unowned" = "keep-header"/);
  });

  it("applies only the current ready backend response and preserves its exact transformed TOML", () => {
    const transformed = existingConfig
      .replace('custom_provider_field = "keep-provider"', 'custom_provider_field = "still-unowned"')
      .replace('"x-unowned" = "keep-header"', '"x-unowned" = "backend-preserved"');
    const response = {
      draftRevision: 99,
      status: "ready" as const,
      draft: {
        profile: { ...existingProfile(), configContents: transformed, apiKey: "backend-key" },
        structuredApiKey: "backend-key",
        catalogMode: "officialPlusCustom" as const,
      },
      blockers: [],
    };

    assert.equal(applyProviderTransformResponse(100, response).kind, "stale");
    const current = applyProviderTransformResponse(99, response);
    assert.equal(current.kind, "applied");
    if (current.kind !== "applied") return;
    assert.equal(current.profile.configContents, transformed);
    assert.equal(current.profile.apiKey, "backend-key");
    assert.match(current.profile.configContents, /custom_provider_field = "still-unowned"/);
    assert.match(current.profile.configContents, /"x-unowned" = "backend-preserved"/);
  });
});
