import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const source = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const relayFileEditorCall = source.match(/<RelayFileEditors[\s\S]*?\/>/)?.[0] ?? "";

describe("provider detail draft wiring", () => {
  it("keeps transform correlation local while sending only the invocation to Tauri", () => {
    assert.match(source, /createProviderDetailDraftState/);
    assert.match(source, /beginProviderDetailEdit/);
    assert.match(source, /transformProviderNativeCapability\(effect\.invocation\)/);
    assert.match(source, /settleProviderDetailTransform\([^;]*effect\.correlation/s);
    assert.match(source, /settleProviderDetailTransformError\([^;]*effect\.correlation/s);
    assert.doesNotMatch(source, /transformProviderNativeCapability\(effect\)/);
  });

  it("loads response-only inspection and closes the session without a provider commit", () => {
    assert.match(source, /inspectProviderNativeCapabilities\(profile\.id\)/);
    assert.match(source, /applyProviderDetailInspection/);
    assert.match(source, /beginProviderDetailRawConfigEdit/);
    assert.match(relayFileEditorCall, /onProviderConfigChange=\{editProviderConfigDraft\}/);
    assert.doesNotMatch(relayFileEditorCall, /onProfileChange=\{replaceDraft\}/);
    assert.match(source, /endProviderDetailSession\([^;]*"navigate"/s);
    assert.doesNotMatch(source, /nativeCapabilityInspection\s*:/);
  });
});
