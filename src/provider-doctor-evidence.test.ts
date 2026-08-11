import assert from "node:assert";
import { describe, it } from "node:test";

import {
  mergeProviderProbeEvidence,
  providerCapabilityOwnershipCopy,
  providerDoctorEvidence,
  providerQuickProbeEvidence,
} from "./provider-doctor-evidence.ts";
import { buildProviderCapabilityLedger } from "./provider-capability-ledger.ts";

describe("Provider Doctor capability evidence", () => {
  it("maps only a successful Responses request to text reachability", () => {
    const direct = providerDoctorEvidence({
      status: "ok",
      protocol: "responses",
      requestHttpStatus: 200,
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
      protocol: "responses",
      requestHttpStatus: 200,
      compatibilityFallbackUsed: true,
      checks: [{ id: "request", status: "ok" }],
    });
    assert.equal(fallback.textResponses, "fallbackReachable");
    assert.equal(fallback.compatibilityFallbackUsed, true);
    assert.equal(fallback.imageGeneration, "unknown");

    const failedFallback = providerDoctorEvidence({
      status: "failed",
      protocol: "responses",
      requestHttpStatus: 500,
      compatibilityFallbackUsed: true,
      checks: [{ id: "request", status: "failed" }],
    });
    assert.equal(failedFallback.textResponses, "unknown");
    assert.equal(failedFallback.compatibilityFallbackUsed, false);
  });

  it("keeps missing, failed, and non-request Doctor states unknown", () => {
    for (const input of [
      { status: "failed", protocol: "responses" as const, requestHttpStatus: null, compatibilityFallbackUsed: false, checks: [] },
      {
        status: "failed",
        protocol: "responses" as const,
        requestHttpStatus: null,
        compatibilityFallbackUsed: false,
        checks: [{ id: "models", status: "ok" }],
      },
      {
        status: "failed",
        protocol: "responses" as const,
        requestHttpStatus: 500,
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
      protocol: "responses",
      httpStatus: 200,
      compatibilityFallbackUsed: false,
    }).textResponses, "reachable");
    assert.equal(providerQuickProbeEvidence({
      status: "ok",
      protocol: "responses",
      httpStatus: 200,
      compatibilityFallbackUsed: true,
    }).textResponses, "fallbackReachable");
    for (const httpStatus of [401, 403]) {
      assert.equal(providerQuickProbeEvidence({
        status: "failed",
        protocol: "responses",
        httpStatus,
        compatibilityFallbackUsed: false,
      }).textResponses, "denied");
    }
    for (const httpStatus of [0, 400, 404, 429, 500]) {
      assert.equal(providerQuickProbeEvidence({
        status: "failed",
        protocol: "responses",
        httpStatus,
        compatibilityFallbackUsed: false,
      }).textResponses, "unknown");
    }
    for (const protocol of ["chatCompletions", "unknown"] as const) {
      assert.equal(providerQuickProbeEvidence({
        status: "ok",
        protocol,
        httpStatus: 200,
        compatibilityFallbackUsed: false,
      }).textResponses, "unknown");
      assert.equal(providerDoctorEvidence({
        status: "ok",
        protocol,
        requestHttpStatus: 200,
        compatibilityFallbackUsed: false,
        checks: [{ id: "request", status: "ok" }],
      }).textResponses, "unknown");
    }
    assert.equal(providerQuickProbeEvidence({
      status: "ok",
      protocol: "responses",
      httpStatus: 302,
      compatibilityFallbackUsed: false,
    }).textResponses, "unknown");
  });

  it("never projects endpoint, body, identity, or credential strings", () => {
    const evidence = providerDoctorEvidence({
      status: "ok",
      protocol: "responses",
      requestHttpStatus: 200,
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

  it("merges only the text row into an existing trusted ledger", () => {
    const ledger = buildProviderCapabilityLedger({
      providerContract: "ready",
      oauthSession: "signedIn",
      localPlan: "free",
      actorMarker: "eligible",
      catalogModel: "supported",
      upstream: { textResponses: "unknown", imageGeneration: "permissionVerified" },
      runtime: "restartRequired",
      routeKind: "nativePriorityMixed",
      imagePlanEvidence: { kind: "unknown" },
    });
    const merged = mergeProviderProbeEvidence(ledger, providerDoctorEvidence({
      status: "ok",
      protocol: "responses",
      requestHttpStatus: 200,
      compatibilityFallbackUsed: true,
      checks: [{ id: "request", status: "ok" }],
    }));
    assert.equal(merged.upstream.textResponses, "fallbackReachable");
    assert.equal(merged.upstream.imageGeneration, "permissionVerified");
    assert.deepEqual(merged.provider, ledger.provider);
    assert.deepEqual(merged.oauth, ledger.oauth);
    assert.deepEqual(merged.plan, ledger.plan);
    assert.deepEqual(merged.actor, ledger.actor);
    assert.deepEqual(merged.catalogModel, ledger.catalogModel);
    assert.deepEqual(merged.runtime, ledger.runtime);
    assert.deepEqual(merged.image, ledger.image);
  });
});
