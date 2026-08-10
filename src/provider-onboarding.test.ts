import assert from "node:assert";
import { describe, it } from "node:test";

import {
  OFFICIAL_AUTH_GUIDE_URL,
  createNewRelayProfileDraft,
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
    assert.strictEqual(draft.contextSelection, contextSelection);
    assert.equal(draft.authContents, "");
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
