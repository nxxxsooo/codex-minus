import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

// The provider mutators moved out of the shell; the wiring they must keep did not.
const source = readFileSync(new URL("./relay-settings.ts", import.meta.url), "utf8");
const shell = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

/// One function's own text.
///
/// Read to its closing brace rather than to whatever function happens to follow, so moving a
/// neighbour out of the shell cannot make this test read the wrong body or fail to find one.
function functionSource(name: string): string {
  const start = source.indexOf(`function ${name}(`);
  assert.ok(start >= 0, `${name} must remain statically auditable`);
  const end = source.indexOf("\n}\n", start);
  assert.ok(end > start, `${name} must end at a top-level brace`);
  return source.slice(start, end);
}

describe("provider config draft wiring", () => {
  it("keeps create blank and passes one explicit target through generation and patching", () => {
    const create = functionSource("createRelayProfile");
    const generate = functionSource("withGeneratedRelayFiles");
    const patch = functionSource("applyRelayProfilePatchToFiles");

    assert.match(create, /return createNewRelayProfileDraft\(\{ id, contextSelection \}\)/);
    assert.doesNotMatch(create, /withGeneratedRelayFiles/);
    assert.match(generate, /applyProviderConfigPatch\(profile, \{\}, contract\)/);
    assert.match(patch, /withGeneratedRelayFiles\(next, options\.target\)/);
    assert.match(patch, /applyProviderConfigPatch\(next, patch, options\.target\)/);
    assert.match(
      patch,
      /options\.target\.source === "existing"[\s\S]*?providerConfigPatchRequiresBackendTransform\(patch\)[\s\S]*?return \{ \.\.\.profile, authContents: "" \}/,
    );
    assert.doesNotMatch(source + shell, /ensureCodexProviderDefaults/);
  });

  it("decides the contract by provenance plus the explicit transient target", () => {
    const target = functionSource("providerConfigTargetContract");
    assert.match(target, /if \(!brandNew\) return \{ target: "preserveExisting", source: "existing" \}/);
    assert.match(target, /target: profile\.transientTarget \?\? "nativePriority", source: "brand-new-empty"/);
    // The retired four-target picker branched on live mode fields; the only extra input allowed
    // beyond provenance is the explicit one-time creation target the user chose.
    assert.doesNotMatch(target, /profile\.(relayMode|officialMixApiKey|protocol)/);
  });
});
