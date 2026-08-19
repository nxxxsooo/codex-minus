import assert from "node:assert";
import fs from "node:fs";
import { describe, it } from "node:test";

import {
  NEW_PROVIDER_ID,
  NEW_PROVIDER_TARGET_OPTIONS,
  OFFICIAL_AUTH_GUIDE_URL,
  PRO_MODEL_SLUGS,
  RETIRED_MODEL_SLUGS,
  createNewRelayProfileDraft,
  materializeNewProviderConfig,
  newProviderTargetPatch,
  officialLoginGuide,
  validateNewProviderDraft,
} from "./provider-onboarding.ts";
import { EN_BACKEND, EN_PLAIN } from "./i18n-en.ts";

/// Slug -> is-listed, straight from the shipped asset. The bundled baseline is what a brand-new
/// profile's catalog can actually represent, so the Pro list is validated against it, not against
/// another hand-maintained mirror.
const bundledBaselineVisibility = new Map<string, boolean>(
  (JSON.parse(
    fs.readFileSync(new URL("../src-tauri/assets/official-model-catalog.json", import.meta.url), "utf8"),
  ) as { models: Array<{ slug: string; visibility: string }> }).models
    .map((model) => [model.slug, model.visibility !== "hide"]),
);

describe("provider onboarding", () => {
  it("creates only the official auth mixed Responses draft", () => {
    const contextSelection = { mcpServers: ["memory"], skills: [], plugins: [] };
    const draft = createNewRelayProfileDraft({ id: "relay-new", contextSelection });

    assert.equal(draft.id, "relay-new");
    assert.equal(draft.name, "");
    assert.equal(draft.baseUrl, "");
    assert.equal(Object.hasOwn(draft, "upstreamBaseUrl"), false);
    assert.equal(draft.apiKey, "");
    assert.equal(draft.model, PRO_MODEL_SLUGS[0]);
    assert.equal(draft.relayMode, "official");
    assert.equal(draft.officialMixApiKey, true);
    assert.equal(Object.hasOwn(draft, "protocol"), false);
    assert.equal(draft.transientTarget, "nativePriority");
    assert.strictEqual(draft.contextSelection, contextSelection);
    assert.equal(draft.authContents, "");

    assert.deepEqual(materializeNewProviderConfig(draft), {
      target: "nativePriority",
      status: "incomplete",
      missingFields: ["baseUrl", "apiKey"],
      configContents: "",
    });
  });

  it("materializes the exact native-priority TOML only when every new-provider input is complete", () => {
    const empty = createNewRelayProfileDraft({ id: "relay-new", contextSelection: {} });
    const complete = {
      ...empty,
      model: "gpt-5.5",
      baseUrl: "https://relay.example/v1",
      apiKey: "provider-key",
    };

    assert.deepEqual(materializeNewProviderConfig({ ...complete, apiKey: " " }), {
      target: "nativePriority",
      status: "incomplete",
      missingFields: ["apiKey"],
      configContents: "",
    });
    assert.equal(Object.hasOwn(complete, "protocol"), false);
    assert.equal(materializeNewProviderConfig(complete).configContents.match(/^wire_api = "responses"$/gm)?.length, 1);
    assert.deepEqual(materializeNewProviderConfig(complete), {
      target: "nativePriority",
      status: "materialized",
      missingFields: [],
      configContents: `model = "gpt-5.5"
model_provider = "OpenAI"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = true
experimental_bearer_token = "provider-key"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
`,
    });
  });

  it("materializes the exact pure-API TOML for the explicit no-login target", () => {
    const empty = createNewRelayProfileDraft({ id: "relay-new", contextSelection: {} });
    const pureApi = {
      ...empty,
      ...newProviderTargetPatch("pureApi"),
      model: "gpt-5.5",
      baseUrl: "https://relay.example/v1",
      apiKey: "provider-key",
    };

    assert.deepEqual(materializeNewProviderConfig(pureApi), {
      target: "pureApi",
      status: "materialized",
      missingFields: [],
      configContents: `model = "gpt-5.5"
model_provider = "OpenAI"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://relay.example/v1"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "provider-key"
`,
    });
  });

  it("offers the mixed default first and patches mode fields with the target choice", () => {
    assert.equal(NEW_PROVIDER_TARGET_OPTIONS[0].value, "nativePriority");
    assert.equal(NEW_PROVIDER_TARGET_OPTIONS.length, 2);
    assert.deepEqual(newProviderTargetPatch("nativePriority"), {
      transientTarget: "nativePriority",
      relayMode: "official",
      officialMixApiKey: true,
    });
    assert.deepEqual(newProviderTargetPatch("pureApi"), {
      transientTarget: "pureApi",
      relayMode: "pureApi",
      officialMixApiKey: false,
    });
    for (const option of NEW_PROVIDER_TARGET_OPTIONS) {
      assert.equal(typeof EN_PLAIN[option.label], "string", option.label);
      assert.equal(typeof EN_PLAIN[option.hint], "string", option.hint);
    }
  });

  it("names the new provider table without impersonating the built-in entry", () => {
    // Codex reserves the lowercase `openai` identifier for its own provider and ignores a
    // user-defined table of that name, which would silently drop the bearer and actor header.
    assert.equal(NEW_PROVIDER_ID, "OpenAI");
    assert.notEqual(NEW_PROVIDER_ID, "openai");
    const materialized = materializeNewProviderConfig({
      ...createNewRelayProfileDraft({ id: "relay-new", contextSelection: {} }),
      model: "gpt-5.5",
      baseUrl: "https://relay.example/v1",
      apiKey: "provider-key",
    });
    assert.match(materialized.configContents, /^model_provider = "OpenAI"$/m);
    assert.match(materialized.configContents, /^\[model_providers\.OpenAI\]$/m);
    assert.doesNotMatch(materialized.configContents, /model_providers\.openai\]/);
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

describe("built-in Pro model list", () => {
  it("ships exact English keys for both truthful reset notices", () => {
    assert.equal(
      EN_PLAIN["已丢弃旧版自动生成的模型列表，并恢复官方模型；至少一个启动模型已设为 5.6 Terra。请重启 Codex 后新建任务。"],
      "The legacy automatically generated model list was discarded and official models were restored; at least one startup model was set to 5.6 Terra. Restart Codex and start a new task.",
    );
    assert.equal(
      EN_PLAIN["已丢弃旧版自动生成的模型列表，并恢复官方模型；现有启动模型已保留。请重启 Codex 后新建任务。"],
      "The legacy automatically generated model list was discarded and official models were restored; existing startup models were preserved. Restart Codex and start a new task.",
    );
    assert.equal(
      EN_BACKEND["已丢弃旧版自动生成的模型列表并恢复官方模型；本次供应商更改尚未保存。请重启 Codex 后新建任务，再检查更新后的设置并重新保存。"],
      "The legacy automatically generated model list was discarded and official models were restored; this provider change was not saved. Restart Codex and start a new task, then review the updated settings and save again.",
    );
    assert.equal(
      EN_BACKEND["已丢弃旧版自动生成的模型列表并恢复官方模型；本次供应商更改尚未保存。页面已更新，请检查后重新保存。"],
      "The legacy automatically generated model list was discarded and official models were restored; this provider change was not saved. The page was updated; review it and save again.",
    );
  });

  it("pins the Terra default across the frontend list, backend reset, and shipped catalog", () => {
    const rust = fs.readFileSync(
      new URL("../src-tauri/src/legacy_model_reset.rs", import.meta.url),
      "utf8",
    );
    const catalog = JSON.parse(
      fs.readFileSync(new URL("../src-tauri/assets/official-model-catalog.json", import.meta.url), "utf8"),
    ) as { models: Array<{ slug: string; visibility: string }> };

    assert.equal(PRO_MODEL_SLUGS[0], "gpt-5.6-terra");
    assert.match(
      rust,
      /CANONICAL_MIXED_DEFAULT_MODEL:\s*&str\s*=\s*"gpt-5\.6-terra"/,
    );
    assert.equal(
      catalog.models.find((model) => model.slug === "gpt-5.6-terra")?.visibility,
      "list",
    );
  });

  it("prefills a new provider with the Pro list and a default that the official catalog carries", () => {
    const draft = createNewRelayProfileDraft({ id: "relay-x", contextSelection: null });
    const list = PRO_MODEL_SLUGS;
    assert.ok(draft);
    assert.ok(list);
    assert.ok(list.length >= 4);
    assert.equal(new Set(list).size, list.length);
    assert.equal(draft.modelList, list.join("\n"));
    assert.equal(draft.model, list[0]);
    // The default must be a slug the official bundled catalog carries, or the first save of a
    // brand-new provider fails catalog planning with an unrepresentable default model.
    assert.ok(bundledBaselineVisibility.get(list[0]) === true, `${list[0]} is not a listed baseline model`);
  });

  it("keeps the canonical native-priority target for a brand-new provider", () => {
    const draft = createNewRelayProfileDraft({ id: "relay-y", contextSelection: null });
    assert.equal(draft?.transientTarget, "nativePriority");
    assert.equal(draft?.relayMode, "official");
    assert.equal(draft?.officialMixApiKey, true);
    assert.equal(Object.hasOwn(draft, "protocol"), false);
  });
});

describe("Pro model list maintenance", () => {
  it("ships no slug the official bundled catalog hides", () => {
    const retired = new Set<string>(RETIRED_MODEL_SLUGS);
    const shipped = PRO_MODEL_SLUGS.filter((slug) => retired.has(slug));
    assert.deepEqual(shipped, [], `retired slugs are still shipped: ${shipped.join(", ")}`);
  });

  it("records the retired slugs it guards against", () => {
    assert.ok((RETIRED_MODEL_SLUGS as readonly string[]).includes("gpt-5.4"));
    assert.ok((RETIRED_MODEL_SLUGS as readonly string[]).includes("gpt-5.4-mini"));
  });

  it("ships only models the bundled baseline actually lists", () => {
    // Cross-referencing the two frontend lists is not enough: the asset once carried a listed
    // gpt-5.2 while the Pro list shipped gpt-5.3-codex-spark, and nothing failed until a user's
    // first save would have. The asset itself is the contract.
    for (const slug of PRO_MODEL_SLUGS) {
      assert.equal(
        bundledBaselineVisibility.get(slug),
        true,
        `${slug} is shipped in the Pro list but the bundled baseline does not list it`,
      );
    }
    for (const slug of RETIRED_MODEL_SLUGS) {
      assert.equal(
        bundledBaselineVisibility.get(slug),
        false,
        `${slug} is recorded as retired but the bundled baseline does not carry it hidden`,
      );
    }
  });
});
