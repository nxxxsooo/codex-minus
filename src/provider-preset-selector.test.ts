import assert from "node:assert";
import { describe, it } from "node:test";

import { createPresetPatch, PRESETS } from "./presets.ts";

describe("new provider presets", () => {
  it("keeps the OpenAI Responses preset on the transient mixed native-priority target", () => {
    const preset = PRESETS.find((candidate) => candidate.id === "openai");
    assert.ok(preset);

    const patch = createPresetPatch(preset);

    assert.equal(patch.relayMode, "official");
    assert.equal(patch.officialMixApiKey, true);
    assert.equal(patch.protocol, "responses");
    assert.equal(patch.baseUrl, "https://api.openai.com/v1");
  });

  it("keeps an incompatible third-party preset on its explicit pure-API path", () => {
    const preset = PRESETS.find((candidate) => candidate.id === "deepseek");
    assert.ok(preset);

    const patch = createPresetPatch(preset);

    assert.equal(patch.relayMode, "pureApi");
    assert.equal(patch.officialMixApiKey, false);
    assert.equal(patch.protocol, "chatCompletions");
  });
});
