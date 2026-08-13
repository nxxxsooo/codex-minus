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
      'className="relay-field-base-url"',
      'className="relay-field-key"',
      "Provider Doctor",
    ]) {
      assert.ok(editor.includes(kept), `the editor lost ${kept}`);
    }
    // The startup model is a row in the model table now, so a second place to name one would be
    // two answers to one question.
    assert.ok(
      !editor.includes('className="relay-field-config-model"'),
      "a free-text startup model is back beside the table that already selects one",
    );
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

  it("upgrades an old profile through the save itself, with no separate control", () => {
    const detail = appSource.match(
      /function RelayProfileDetail[\s\S]*?(?=\nfunction RelayProfileEditor)/,
    )?.[0] ?? "";
    assert.ok(detail.includes("saveDraft"), "the detail body was located");
    const save = detail.match(/const saveDraft = async \(\) => \{[\s\S]*?\n  \};/)?.[0] ?? "";
    assert.ok(save.length > 0, "saveDraft was located");
    // Removing the upgrade button without this would leave an old profile permanently unupgradable.
    assert.match(save, /beginProviderDetailNativePriorityUpgrade\(detailStateRef\.current\)/);
    assert.match(save, /upgradeAction === "upgrade" \|\| upgradeAction === "replaceActorHeader"/);
    assert.match(save, /beginProviderDetailLegacyIdUpgrade\(detailStateRef\.current\)/);
    assert.doesNotMatch(detail, /升级为原生能力优先|处理旧供应商 ID|替换自定义 Actor 标记/);
  });

  it("offers one model table with one context field and one startup choice per row", () => {
    const catalogEditor = appSource.match(
      /function CatalogProfileEditor\([\s\S]*?(?=\nfunction EnvConflictNotice)/,
    )?.[0] ?? "";
    assert.ok(catalogEditor.includes("catalog-model-list"), "the catalog editor was located");
    // Official and custom models are one list: they answer the same question for the user.
    assert.ok(!catalogEditor.includes("catalog-official-list"), "the official table split off again");
    assert.ok(!catalogEditor.includes("catalog-custom-list"), "the custom table split off again");
    assert.match(catalogEditor, /type="radio"/);
    assert.match(catalogEditor, /const selectedModel = codexModelFromConfig\(profile\.configContents\) \|\| profile\.model;/);
    // Raising Sol from 272k to 372k must be one number, not a hunt through a wide table.
    assert.match(catalogEditor, /contextWindow: positiveNumberOrNull\(event\.currentTarget\.value\)/);
    assert.match(catalogEditor, /placeholder=\{model\.contextWindow \? String\(model\.contextWindow\) : t\("默认"\)\}/);
    for (const [gone, why] of [
      ["CatalogModeControls", "the mode is not a user choice in this flow"],
      ["上游拓扑", "topology is derived, not chosen"],
      ["预览并采用", "external adoption is not part of the simple flow"],
      ["刷新供应商证据", "provider evidence is not a user-facing surface"],
      ["effectiveContextWindowPercent: boundedPercentOrNull", "percent is not a knob"],
      ["supportedReasoningLevels: parseReasoningLevels", "reasoning levels come from the baseline"],
      ["清除覆盖", "one field needs no per-row reset control"],
    ] as const) {
      assert.ok(!catalogEditor.includes(gone), `${gone} is back — ${why}`);
    }
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

describe("a profile whose empty fields never reached the wire", () => {
  it("normalizes a backend profile before the editor edits it", () => {
    // serde omits an empty string instead of sending "", so a profile whose model, Base URL, or
    // Key lives only inside its TOML comes back without those keys.
    const transform = appSource.match(
      /const transformProviderNativeCapability = async[\s\S]*?\n  \};/,
    )?.[0] ?? "";
    assert.ok(transform.length > 0, "the transform boundary was located");
    assert.match(transform, /normalizeRelayProfile\(response\.draft\.profile\)/);
  });

  it("reads a possibly-absent model the way every other field here is read", () => {
    const derive = appSource.match(
      /function deriveRelayProfileFromFiles[\s\S]*?\n\}/,
    )?.[0] ?? "";
    assert.ok(derive.length > 0, "the derivation was located");
    assert.doesNotMatch(derive, /profile\.model\.trim\(\)/);
    assert.match(derive, /\(profile\.model \|\| ""\)\.trim\(\)/);
  });
});

describe("one model table answers every model question", () => {
  const catalogEditor = appSource.match(
    /function CatalogProfileEditor\([\s\S]*?(?=\nfunction EnvConflictNotice)/,
  )?.[0] ?? "";

  it("was read from the real editor", () => {
    assert.ok(catalogEditor.includes("catalog-model-list"), "the model table was located");
  });

  it("carries the startup selection when the selected row is renamed or deleted", () => {
    // The selection names a model by slug, so a row that changes its slug or disappears would
    // otherwise leave the profile starting Codex on a model its catalog no longer contains.
    const rename = catalogEditor.match(/const renameCustom = [\s\S]*?\n  \};/)?.[0] ?? "";
    const remove = catalogEditor.match(/const removeCustom = [\s\S]*?\n  \};/)?.[0] ?? "";
    assert.ok(rename.length > 0 && remove.length > 0, "both row actions were located");
    assert.match(rename, /if \(previous && selectedModel === previous\) onProfileEdit\(\{ model: slug \}\)/);
    assert.match(remove, /if \(removed && selectedModel === removed\) onProfileEdit\(\{ model: "" \}\)/);
  });

  it("never selects a row that has no slug yet", () => {
    assert.match(catalogEditor, /const selectStartupModel = \(slug: string\) => \{\s*if \(slug\) onProfileEdit/);
    assert.match(catalogEditor, /disabled=\{!model\.slug\}/);
  });

  it("leaves no second place to name a model", () => {
    for (const [gone, why] of [
      ["供应商测试模型", "a provider is tested with the model it starts on"],
      ["modelWindowRows", "the removed per-profile window rows had no editor left"],
      ["serializeModelWindowRows", "the removed per-profile window rows had no editor left"],
    ] as const) {
      assert.ok(!appSource.includes(gone), `${gone} is back — ${why}`);
    }
  });
});
