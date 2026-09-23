import assert from "node:assert/strict";
import { it } from "node:test";
import { providerImageGenerationEnabled } from "./provider-image-generation.ts";

const config = `model_provider = "my.relay"
[model_providers."my.relay"]
requires_openai_auth = false
`;

it("shows explicit per-provider enablement without guessing inherited live feature settings", () => {
  assert.equal(providerImageGenerationEnabled({ relayMode: "pureApi", configContents: config }), false);
  assert.equal(providerImageGenerationEnabled({ relayMode: "pureApi", configContents: config + "[features]\nimage_generation = true" }), true);
  for (const features of ["[features]\nimage_generation = false", "features.image_generation = false", "features = { other = true, image_generation = false }"]) {
    const contents = features.startsWith("[") ? config + features : features + "\n" + config;
    assert.equal(providerImageGenerationEnabled({ relayMode: "pureApi", configContents: contents }), false, features);
  }
  assert.equal(providerImageGenerationEnabled({ relayMode: "official", configContents: config }), false);
  assert.equal(providerImageGenerationEnabled({ relayMode: "pureApi", configContents: config.replace("= false", "= true") }), false);
  assert.equal(providerImageGenerationEnabled({ relayMode: "pureApi", configContents: config.replace('[model_providers."my.relay"]', '[model_providers.my.relay]') + "[features]\nimage_generation = true" }), false);
});
