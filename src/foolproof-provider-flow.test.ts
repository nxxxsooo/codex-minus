import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

import { NEW_PROVIDER_ID, PRO_MODEL_SLUGS, createNewRelayProfileDraft } from "./provider-onboarding.ts";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const editor = appSource.match(
  /function RelayProfileEditor\([\s\S]*?\n\}\n\nfunction /,
)?.[0] ?? "";

describe("the provider editor offers one contract and no switches", () => {
  it("was read from the real source", () => {
    assert.ok(editor.includes("relay-field-base-url"), "the editor body was located");
    assert.ok(editor.includes("Provider Doctor"), "the editor body was located");
  });

  it("shows only the controls the seven-step flow needs", () => {
    for (const kept of [
      'className="relay-field-name"',
      'className="relay-field-config-model"',
      'className="relay-field-base-url"',
      'className="relay-field-key"',
      "Provider Doctor",
    ]) {
      assert.ok(editor.includes(kept), `the editor lost ${kept}`);
    }
  });

  it("offers no control that could produce a different contract", () => {
    for (const [removed, why] of [
      ["relay-field-protocol", "the contract is always Responses"],
      ["relay-field-user-agent", "the default User-Agent is the only value the flow needs"],
      ["relay-field-context-window", "the managed catalog owns per-model context"],
      ["relay-field-auto-compact", "the managed catalog owns per-model context"],
      ["混入 API KEY", "holding a key is what makes a profile mixed"],
      ["更多选项", "there is no advanced tier left to reveal"],
      ["ProviderPresetSelector", "presets encode alternate contracts"],
      ['<option value="pureApi">', "access mode is not a choice in this flow"],
    ] as const) {
      assert.ok(!editor.includes(removed), `${removed} is back — ${why}`);
    }
  });

  it("reveals the endpoint and key for every profile so an old one can be repaired", () => {
    assert.match(editor, /const showApiFields = true;/);
  });

  it("treats a supplied key as the mixed contract without a switch", () => {
    const update = editor.match(/const updateDraft = \(patch: Partial<RelayProfile>\) => \{[\s\S]*?\n  \};/)?.[0] ?? "";
    assert.ok(update.length > 0, "updateDraft was located");
    assert.match(update, /officialMixApiKey: true/);
    assert.doesNotMatch(
      update,
      /officialMixApiKey: false/,
      "clearing a key must not silently delete the provider table",
    );
  });

  it("prefills a new draft so only an endpoint and a key remain", () => {
    const draft = createNewRelayProfileDraft({ id: "relay-new", contextSelection: {} });
    assert.equal(draft.model, PRO_MODEL_SLUGS[0]);
    assert.equal(draft.modelList, PRO_MODEL_SLUGS.join("\n"));
    assert.equal(draft.baseUrl, "");
    assert.equal(draft.apiKey, "");
    assert.equal(NEW_PROVIDER_ID, "OpenAI");
  });
});
