import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const backendSource = readFileSync(
  new URL("../src-tauri/src/provider_capability_evidence.rs", import.meta.url),
  "utf8",
);

describe("provider capability ledger wiring", () => {
  it("loads only the trusted backend payload and never constructs target policy in App", () => {
    assert.match(appSource, /inspect_provider_capability_evidence/);
    assert.match(appSource, /buildProviderCapabilityLedgerFromBackendEvidence/);
    assert.doesNotMatch(appSource, /verifiedTargetPolicy/);
    assert.doesNotMatch(appSource, /buildProviderCapabilityLedger\s*\(/);
    assert.match(backendSource, /image_plan_evidence:\s*ImagePlanEvidence::Unknown/);
    assert.doesNotMatch(backendSource, /Deserialize[^\n]*ImagePlanEvidence/);
  });

  it("keeps refresh read-only and the ledger outside profile and commit payloads", () => {
    const loader = appSource.match(
      /const inspectProviderCapabilityEvidence[\s\S]*?(?=\n\s*const transformProviderNativeCapability)/,
    )?.[0] ?? "";
    assert.match(loader, /call<ProviderCapabilityEvidenceResult>\(\s*"inspect_provider_capability_evidence"/);
    assert.doesNotMatch(loader, /transformProviderNativeCapability|commitProvider|saveSettings/);
    assert.match(appSource, /updateCapabilityEvidenceLoad/);
    assert.match(appSource, /providerCapabilityEvidenceRefreshAllowed/);
    assert.match(appSource, /beginProviderCapabilityEvidenceLoad/);
    assert.match(appSource, /settleProviderCapabilityEvidenceLoad/);
    assert.doesNotMatch(appSource, /capabilityLedger\s*:/);
    assert.doesNotMatch(appSource, /\.\.\.capabilityLedger/);
  });

  it("renders every independent gate without capability-success copy", () => {
    const ledgerUi = appSource.match(
      /\{displayedCapabilityLedger \? \([\s\S]*?刷新能力证据[\s\S]*?<\/section>/,
    )?.[0] ?? "";
    for (const copy of [
      "供应商契约",
      "OAuth 会话",
      "本地账号计划",
      "Actor 资格",
      "目录／模型",
      "上游证据",
      "运行时",
    ]) {
      assert.match(ledgerUi, new RegExp(copy));
    }
    assert.doesNotMatch(ledgerUi, /所有 Pro 能力|全部原生能力已启用|Paid（能力成功）/);
    assert.doesNotMatch(
      appSource,
      /isNew \|\| isAggregateRelayProfile\(draft\) \|\| !detailState\.inspection/,
    );
    assert.match(appSource, /catalogProfile\?\.actionRequired/);
    assert.match(appSource, /catalogProfile\?\.restartRequired/);
    assert.match(appSource, /capabilityEvidenceCatalogSourceFingerprint/);
  });
});
