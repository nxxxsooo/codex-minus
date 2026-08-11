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
    assert.match(appSource, /requestHttpStatus:\s*result\?\.requestHttpStatus/);
    assert.match(appSource, /protocol:\s*profile\.protocol/);
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
    assert.match(apply, /sourceFingerprint/);
    assert.match(apply, /mergeProviderProbeEvidence/);
    assert.doesNotMatch(apply, /transformProvider|commitProvider|saveSettings|configContents|authContents/);
  });

  it("keeps the existing Doctor request as the only operation and renders ownership copy", () => {
    const runner = appSource.match(
      /const runProviderDoctor[\s\S]*?(?=\n\s*return \()/,
    )?.[0] ?? "";
    assert.match(runner, /actions\.diagnoseRelayProfile/);
    assert.match(runner, /providerDoctorEvidence/);
    assert.doesNotMatch(runner, /transformProvider|commitProvider|saveSettings/);
    for (const field of ["oauth", "providerKey", "actor", "gates"]) {
      assert.match(appSource, new RegExp(`capabilityOwnershipCopy\\.${field}`));
    }
  });
});
