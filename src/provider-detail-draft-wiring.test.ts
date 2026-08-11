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
    assert.match(
      source,
      /settleProviderDetailTransformError\([\s\S]*?if \(!settled\.report\) return;\s*updateDetailState\(settled\.state\)/,
    );
    assert.doesNotMatch(source, /transformProviderNativeCapability\(effect\)/);
  });

  it("loads response-only inspection and closes the session without a provider commit", () => {
    assert.match(source, /inspectProviderNativeCapabilities\(profile\.id\)/);
    assert.match(source, /applyProviderDetailInspection/);
    assert.match(source, /beginProviderDetailRawConfigEdit/);
    assert.match(relayFileEditorCall, /onProviderConfigChange=\{editProviderConfigDraft\}/);
    assert.match(relayFileEditorCall, /providerReadOnly=\{isNew\}/);
    assert.doesNotMatch(relayFileEditorCall, /onProfileChange=\{replaceDraft\}/);
    assert.match(source, /draftCommitBlocked=\{detailState\.pendingTransformRevision !== null \|\| detailState\.rawConfigContents !== null \|\| detailState\.pendingConfirmation !== null \|\| detailState\.pendingLegacyProviderIdResolution !== null \|\| detailState\.blockers\.length > 0\}/);
    assert.match(source, /switchDraft[\s\S]*detailState\.rawConfigContents !== null[\s\S]*return;/);
    assert.match(source, /endProviderDetailSession\([^;]*"navigate"/s);
    assert.doesNotMatch(source, /nativeCapabilityInspection\s*:/);
  });

  it("wires explicit upgrade availability without letting ordinary controls synthesize enablement", () => {
    assert.match(source, /deriveProviderNativeCapabilityView/);
    assert.match(source, /beginProviderDetailNativePriorityUpgrade/);
    assert.match(source, /providerTransitionDecisionForStructuredPatch/);
    assert.match(source, /nativeCapabilityView\.upgradeAction/);
    assert.match(source, /refreshProviderDetailCatalogDraftState/);
    assert.match(source, /refreshed\.inspectionCorrelation/);
    assert.match(source, /此变更必须通过明确的升级预览操作完成/);
    assert.match(source, /inspectionCorrelation\s*&&\s*!isNew/);
    assert.doesNotMatch(
      source,
      /function transitionForPatch[\s\S]*return \{ action: "enableNativePriority"/,
    );
  });

  it("wires actor and legacy resolutions through the draft state machine only", () => {
    assert.match(source, /beginProviderDetailLegacyIdUpgrade/);
    assert.match(source, /resolveProviderDetailLegacyProviderId\(\s*detailStateRef\.current/);
    assert.match(source, /cancelProviderDetailLegacyProviderIdResolution\(\s*detailStateRef\.current\s*\)/);
    assert.match(source, /nativeCapabilityView\.upgradeAction === "resolveLegacyProviderId"/);
    assert.match(source, /nativeCapabilityView\.upgradeAction === "replaceActorHeader"/);
    assert.match(source, /pendingLegacyProviderIdResolution !== null/);
    assert.match(
      source,
      /switchDraft[\s\S]*detailState\.pendingLegacyProviderIdResolution !== null[\s\S]*return;/,
    );
    assert.match(source, /renamedProviderFrom/);
    assert.match(source, /renamedProviderTo/);
    assert.match(source, /只替换冲突的 Actor 标记/);
    assert.doesNotMatch(source, /replacementProviderId:\s*legacyReplacementProviderId/);
  });

  it("consumes compatibility-exit preview confirmation only through the draft state machine", () => {
    assert.match(source, /response\.status === "confirmationRequired"/);
    assert.match(source, /window\.confirm\(providerTransitionConfirmationMessage\(settled\.state\)\)/);
    assert.match(source, /confirmProviderDetailTransition\(settled\.state\)/);
    assert.match(source, /cancelProviderDetailTransition\(settled\.state\)/);
    assert.doesNotMatch(source, /applyRelayProfilePatchToFiles\([^)]*protocol:\s*"chatCompletions"/s);
  });
});
