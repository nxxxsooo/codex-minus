import assert from "node:assert";
import { describe, it } from "node:test";

import {
  providerCapabilityOwnershipCopy,
  providerDoctorEvidence,
  providerQuickProbeEvidence,
} from "./provider-doctor-evidence.ts";

describe("Provider Doctor capability evidence", () => {
  it("maps only a successful Responses request to text reachability", () => {
    const direct = providerDoctorEvidence({
      status: "ok",
      compatibilityFallbackUsed: false,
      checks: [{ id: "request", status: "ok" }],
    });
    assert.equal(direct.textResponses, "reachable");
    assert.equal(direct.compatibilityFallbackUsed, false);
    assert.equal(direct.imageGeneration, "unknown");
    assert.equal(direct.nativeExtension, "unknown");
    assert.equal(direct.catalogModel, "unknown");
    assert.equal(direct.selectedModel, "unknown");
    assert.equal(direct.providerGroup, "unknown");
  });

  it("keeps compatibility fallback as a separate text-only observation", () => {
    const fallback = providerDoctorEvidence({
      status: "ok",
      compatibilityFallbackUsed: true,
      checks: [{ id: "request", status: "ok" }],
    });
    assert.equal(fallback.textResponses, "fallbackReachable");
    assert.equal(fallback.compatibilityFallbackUsed, true);
    assert.equal(fallback.imageGeneration, "unknown");

    const failedFallback = providerDoctorEvidence({
      status: "failed",
      compatibilityFallbackUsed: true,
      checks: [{ id: "request", status: "failed" }],
    });
    assert.equal(failedFallback.textResponses, "unknown");
    assert.equal(failedFallback.compatibilityFallbackUsed, false);
  });

  it("keeps missing, failed, and non-request Doctor states unknown", () => {
    for (const input of [
      { status: "failed", compatibilityFallbackUsed: false, checks: [] },
      {
        status: "failed",
        compatibilityFallbackUsed: false,
        checks: [{ id: "models", status: "ok" }],
      },
      {
        status: "failed",
        compatibilityFallbackUsed: false,
        checks: [{ id: "request", status: "failed" }],
      },
    ]) {
      const evidence = providerDoctorEvidence(input);
      assert.equal(evidence.textResponses, "unknown");
      assert.equal(evidence.imageGeneration, "unknown");
    }
  });

  it("maps the quick probe conservatively and treats only explicit auth denial as denied", () => {
    assert.equal(providerQuickProbeEvidence({
      status: "ok",
      httpStatus: 200,
      compatibilityFallbackUsed: false,
    }).textResponses, "reachable");
    assert.equal(providerQuickProbeEvidence({
      status: "ok",
      httpStatus: 200,
      compatibilityFallbackUsed: true,
    }).textResponses, "fallbackReachable");
    for (const httpStatus of [401, 403]) {
      assert.equal(providerQuickProbeEvidence({
        status: "failed",
        httpStatus,
        compatibilityFallbackUsed: false,
      }).textResponses, "denied");
    }
    for (const httpStatus of [0, 400, 404, 429, 500]) {
      assert.equal(providerQuickProbeEvidence({
        status: "failed",
        httpStatus,
        compatibilityFallbackUsed: false,
      }).textResponses, "unknown");
    }
  });

  it("never projects endpoint, body, identity, or credential strings", () => {
    const evidence = providerDoctorEvidence({
      status: "ok",
      compatibilityFallbackUsed: false,
      checks: [{
        id: "request",
        status: "ok",
        title: "private@example.test",
        detail: "https://private.example.test sk-provider-secret oauth-secret response-body-secret",
      }],
      message: "sk-message-secret",
      profileName: "private@example.test",
      model: "secret-model-identity",
      summary: "response-body-secret",
      recommendation: "oauth-secret",
    });
    assert.doesNotMatch(
      JSON.stringify(evidence),
      /private\.example|provider-secret|oauth-secret|response-body|secret-model|message-secret/,
    );
  });

  it("provides truthful Chinese and English ownership copy", () => {
    const zh = providerCapabilityOwnershipCopy("zh");
    assert.match(zh.oauth, /官方客户端/);
    assert.match(zh.providerKey, /推理/);
    assert.match(zh.actor, /资格/);
    assert.match(zh.gates, /上游.*模型.*账号/);

    const en = providerCapabilityOwnershipCopy("en");
    assert.match(en.oauth, /official client/i);
    assert.match(en.providerKey, /inference/i);
    assert.match(en.actor, /eligibility/i);
    assert.match(en.gates, /upstream.*model.*account/i);
    assert.doesNotMatch(`${zh.actor} ${en.actor}`, /所有.*能力|all.*capabilities.*enabled/i);
  });
});
