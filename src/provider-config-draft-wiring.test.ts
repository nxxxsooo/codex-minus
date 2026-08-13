import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const source = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

function functionSource(name: string, nextName: string): string {
  const start = source.indexOf(`function ${name}(`);
  const end = source.indexOf(`function ${nextName}(`, start + 1);
  assert.ok(start >= 0 && end > start, `${name} must remain statically auditable`);
  return source.slice(start, end);
}

describe("provider config draft wiring", () => {
  it("keeps create blank and passes one explicit target through generation and patching", () => {
    const create = functionSource("createRelayProfile", "createAggregateRelayProfile");
    const generate = functionSource("withGeneratedRelayFiles", "providerConfigTargetContract");
    const patch = functionSource("applyRelayProfilePatchToFiles", "codexModelFromConfig");

    assert.match(create, /return createNewRelayProfileDraft\(\{ id, contextSelection \}\)/);
    assert.doesNotMatch(create, /withGeneratedRelayFiles/);
    assert.match(generate, /withGeneratedRelayConfig\(profile, contract\)/);
    assert.match(patch, /withGeneratedRelayFiles\(next, options\.target\)/);
    assert.match(patch, /applyProviderConfigPatch\(next, patch, options\.target\)/);
    assert.match(
      patch,
      /options\.target\.source === "existing"[\s\S]*?providerConfigPatchRequiresBackendTransform\(patch\)[\s\S]*?return \{ \.\.\.profile, authContents: "" \}/,
    );
    assert.doesNotMatch(source, /ensureCodexProviderDefaults/);
  });

  it("uses preserveExisting for every saved profile edit", () => {
    const target = functionSource("providerConfigTargetContract", "deriveRelayProfileFromFiles");
    assert.match(target, /if \(!brandNew\) return \{ target: "preserveExisting", source: "existing" \}/);
    assert.match(target, /target: "nativePriority", source: "brand-new-empty"/);
  });
});
