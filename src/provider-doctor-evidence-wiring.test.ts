import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const backendSource = readFileSync(
  new URL("../src-tauri/src/commands.rs", import.meta.url),
  "utf8",
);

describe("Provider Doctor evidence wiring", () => {
  it("binds the mapper to the actual protocol and structured final HTTP status", () => {
    const runner = appSource.match(
      /const runProviderDoctor[\s\S]*?(?=\n\s*return \()/,
    )?.[0] ?? "";
    assert.match(runner, /requestHttpStatus:\s*result\?\.requestHttpStatus/);
    assert.match(runner, /protocol:\s*probedProfile\.protocol/);
    assert.match(backendSource, /pub request_http_status:\s*Option<u16>/);
    assert.match(backendSource, /request_http_status = Some\(result\.http_status\)/);
    assert.doesNotMatch(
      appSource,
      /requestHttpStatus:\s*result\.initialHttpStatus/,
    );
  });

  it("correlates Doctor evidence to the current authoritative profile and merges only the text row", () => {
    const apply = appSource.match(
      /const applyDoctorEvidence[\s\S]*?(?=\n\s*const nativeCapabilityView)/,
    )?.[0] ?? "";
    assert.match(apply, /JSON\.stringify\(probedProfile\).*JSON\.stringify\(currentDetail\.profile\)/s);
    assert.match(apply, /capabilityEvidenceRefreshAllowedForState/);
    assert.match(apply, /setDoctorEvidenceObservation/);
    assert.doesNotMatch(apply, /transformProvider|commitProvider|saveSettings|configContents|authContents/);
    const display = appSource.match(
      /const displayedCapabilityLedger[\s\S]*?(?=\n\s*const capabilityOwnershipCopy)/,
    )?.[0] ?? "";
    assert.match(display, /mergeCurrentProviderProbeObservation/);
    assert.match(display, /observation:\s*doctorEvidenceObservation/);
    assert.match(display, /currentRevision:\s*detailState\.latestTransformRevision/);
    assert.match(display, /currentCatalogEvidenceFingerprint/);
  });

  it("keeps the existing Doctor request as the only operation and rejects stale source responses", () => {
    const runner = appSource.match(
      /const runProviderDoctor[\s\S]*?(?=\n\s*return \()/,
    )?.[0] ?? "";
    assert.match(runner, /actions\.diagnoseRelayProfile/);
    assert.match(runner, /providerDoctorEvidence/);
    assert.match(runner, /requestSourceFingerprint/);
    assert.match(runner, /providerDoctorRequestMatchesSource/);
    assert.doesNotMatch(runner, /transformProvider|commitProvider|saveSettings/);
    assert.match(appSource, /const doctorSourceRevision = JSON\.stringify\(\{ profile, modelWindowRows \}\)/);
    assert.match(appSource, /\[doctorSourceRevision\]/);
  });

  it("renders ownership copy for new and existing non-aggregate providers", () => {
    const ownership = appSource.match(
      /\{isAggregateRelayProfile\(draft\) \? null : \(\s*<section[\s\S]*?capabilityOwnershipCopy\.gates[\s\S]*?<\/section>\s*\)\}/,
    )?.[0] ?? "";
    assert.ok(ownership.length > 0);
    assert.doesNotMatch(ownership, /isNew/);
    for (const field of ["oauth", "providerKey", "actor", "gates"]) {
      assert.match(ownership, new RegExp(`capabilityOwnershipCopy\\.${field}`));
    }
  });
});
