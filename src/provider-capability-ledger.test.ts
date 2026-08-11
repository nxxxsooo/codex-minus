import assert from "node:assert";
import { describe, it } from "node:test";

import {
  buildProviderCapabilityLedger,
  buildProviderCapabilityLedgerFromBackendEvidence,
  beginProviderCapabilityEvidenceLoad,
  createProviderCapabilityEvidenceLoadState,
  invalidateProviderCapabilityEvidenceLoad,
  providerCapabilityEvidenceRefreshAllowed,
  settleProviderCapabilityEvidenceLoad,
  type ProviderCapabilityLedgerInput,
} from "./provider-capability-ledger.ts";

const baseInput: ProviderCapabilityLedgerInput = {
  providerContract: "ready",
  oauthSession: "signedIn",
  localPlan: "unknown",
  actorMarker: "eligible",
  catalogModel: "supported",
  upstream: { textResponses: "unknown", imageGeneration: "unknown" },
  runtime: "unknown",
  routeKind: "nativePriorityMixed",
  imagePlanEvidence: { kind: "unknown" },
};

describe("provider capability evidence ledger", () => {
  it("keeps OAuth session and local plan on independent activation axes", () => {
    for (const localPlan of ["free", "paid", "unknown"] as const) {
      const ledger = buildProviderCapabilityLedger({ ...baseInput, localPlan });
      assert.equal(ledger.oauth.activationGate, "satisfied");
      assert.equal(ledger.oauth.inactiveSaveDisposition, "satisfied");
      assert.equal(ledger.plan.observed, localPlan);
      assert.equal(ledger.plan.provesCapabilitySuccess, false);
    }

    for (const oauthSession of ["signedOut", "expired"] as const) {
      const ledger = buildProviderCapabilityLedger({ ...baseInput, oauthSession });
      assert.equal(ledger.oauth.activationGate, "blocked");
      assert.equal(ledger.oauth.inactiveSaveDisposition, "actionRequired");
    }

    const unknown = buildProviderCapabilityLedger({ ...baseInput, oauthSession: "unknown" });
    assert.equal(unknown.oauth.activationGate, "unknown");

    const pureApi = buildProviderCapabilityLedger({
      ...baseInput,
      routeKind: "keyOnlyPureApi",
      oauthSession: "signedOut",
      providerContract: "invalid",
      catalogModel: "stale",
    });
    assert.equal(pureApi.oauth.activationGate, "notApplicable");
    assert.equal(pureApi.oauth.inactiveSaveDisposition, "notApplicable");
    assert.equal("mayActivate" in pureApi.oauth, false);
    assert.equal("inactiveSave" in pureApi, false);
    assert.equal(pureApi.provider.state, "invalid");
    assert.equal(pureApi.catalogModel.state, "stale");
  });

  it("blocks Free image only for one verified affected target path", () => {
    const affectedTargetPath = {
      kind: "verifiedTargetPolicy",
      policySource: "targetCliPolicy",
      targetVersion: "0.147.0-alpha.6.5",
      capabilityPath: "providerRoutedImageActorMarker",
      freePlanRule: "blocked",
    } as const;
    const affected = buildProviderCapabilityLedger({
      ...baseInput,
      localPlan: "free",
      imagePlanEvidence: affectedTargetPath,
    });
    assert.equal(affected.image.planGate, "blocked");
    assert.equal(affected.image.status, "blocked");
    assert.deepEqual(affected.image.planEvidenceScope, {
      targetVersion: "0.147.0-alpha.6.5",
      capabilityPath: "providerRoutedImageActorMarker",
    });

    const unaffected = buildProviderCapabilityLedger({
      ...baseInput,
      localPlan: "free",
      imagePlanEvidence: {
        kind: "verifiedTargetPolicy",
        policySource: "targetCliPolicy",
        targetVersion: "0.147.0-alpha.6.5",
        capabilityPath: "providerRoutedImageActorMarker",
        freePlanRule: "notBlocked",
      },
    });
    assert.equal(unaffected.image.planGate, "notBlocked");
    assert.equal(unaffected.image.status, "unknown");

    const unknownTarget = buildProviderCapabilityLedger({
      ...baseInput,
      localPlan: "free",
      imagePlanEvidence: { kind: "unknown" },
    });
    assert.equal(unknownTarget.image.planGate, "unknown");
    assert.equal(unknownTarget.image.status, "unknown");

    const paid = buildProviderCapabilityLedger({
      ...baseInput,
      localPlan: "paid",
      upstream: { textResponses: "unknown", imageGeneration: "permissionVerified" },
      runtime: "adopted",
    });
    assert.equal(paid.plan.provesCapabilitySuccess, false);
    assert.notEqual(paid.image.status, "available");

    for (const rejectedScope of [
      { ...baseInput, routeKind: "keyOnlyPureApi" as const },
      { ...baseInput, actorMarker: "ineligible" as const },
    ]) {
      const ledger = buildProviderCapabilityLedger({
        ...rejectedScope,
        localPlan: "free",
        imagePlanEvidence: affectedTargetPath,
      });
      assert.equal(ledger.image.planGate, "unknown");
      assert.equal(ledger.image.status, "unknown");
      assert.equal(ledger.image.planEvidenceScope, null);
    }
    const blankTarget = buildProviderCapabilityLedger({
      ...baseInput,
      localPlan: "free",
      imagePlanEvidence: { ...affectedTargetPath, targetVersion: " " },
    });
    assert.equal(blankTarget.image.planGate, "unknown");
    assert.equal(blankTarget.image.planEvidenceScope, null);
  });

  it("keeps actor, catalog, upstream, and runtime evidence independent and redacted", () => {
    const ledger = buildProviderCapabilityLedger({
      ...baseInput,
      actorMarker: "eligible",
      catalogModel: "missingMetadata",
      upstream: { textResponses: "reachable", imageGeneration: "unknown" },
      runtime: "restartRequired",
    });
    assert.equal(ledger.actor.provesEligibilityOnly, true);
    assert.equal(ledger.catalogModel.state, "missingMetadata");
    assert.equal(ledger.upstream.textResponses, "reachable");
    assert.equal(ledger.upstream.imageGeneration, "unknown");
    assert.equal(ledger.runtime.state, "restartRequired");
    assert.equal(ledger.image.status, "unknown");
    assert.doesNotMatch(JSON.stringify(ledger), /token|account|email|bearer|api.?key/i);

    const textDenied = buildProviderCapabilityLedger({
      ...baseInput,
      upstream: { textResponses: "denied", imageGeneration: "unknown" },
    });
    assert.equal(textDenied.image.status, "unknown");
    const imageDenied = buildProviderCapabilityLedger({
      ...baseInput,
      upstream: { textResponses: "reachable", imageGeneration: "denied" },
    });
    assert.equal(imageDenied.image.status, "blocked");
  });

  it("labels key-only routing as pure API or legacy compatibility", () => {
    assert.equal(
      buildProviderCapabilityLedger({ ...baseInput, routeKind: "keyOnlyPureApi" }).route.label,
      "pureApi",
    );
    assert.equal(
      buildProviderCapabilityLedger({ ...baseInput, routeKind: "legacyCompatibility" }).route.label,
      "compatibility",
    );
    assert.equal(
      buildProviderCapabilityLedger({ ...baseInput, routeKind: "notApplicable" }).route.label,
      "notApplicable",
    );
    assert.equal(
      buildProviderCapabilityLedger({ ...baseInput, routeKind: "unknown" }).route.label,
      "unknown",
    );
  });

  it("projects one backend-only payload without adding identity or secret fields", () => {
    const ledger = buildProviderCapabilityLedgerFromBackendEvidence({
      profileId: "relay-one",
      providerContract: "ready",
      oauthSession: "signedIn",
      localPlan: "free",
      actorMarker: "eligible",
      catalogModel: "missingMetadata",
      textResponses: "unknown",
      imageGeneration: "unknown",
      runtime: "restartRequired",
      routeKind: "nativePriorityMixed",
      imagePlanEvidence: { kind: "unknown" },
    });
    assert.equal(ledger.oauth.session, "signedIn");
    assert.equal(ledger.plan.observed, "free");
    assert.equal(ledger.image.status, "unknown");
    assert.doesNotMatch(JSON.stringify(ledger), /relay-one|token|account|email|bearer|api.?key/i);
  });

  it("reads persisted evidence only while the detail draft matches its authoritative source", () => {
    const profile = { id: "relay-one", model: "gpt-5.5" };
    const catalog = { profileId: "relay-one", mode: "official-plus-custom" };
    assert.equal(providerCapabilityEvidenceRefreshAllowed({
      applicable: true,
      currentProfile: profile,
      authoritativeProfile: { ...profile },
      currentCatalogDraft: catalog,
      authoritativeCatalogDraft: { ...catalog },
    }), true);
    assert.equal(providerCapabilityEvidenceRefreshAllowed({
      applicable: true,
      currentProfile: { ...profile, model: "unsaved-model" },
      authoritativeProfile: profile,
      currentCatalogDraft: catalog,
      authoritativeCatalogDraft: catalog,
    }), false);
    assert.equal(providerCapabilityEvidenceRefreshAllowed({
      applicable: true,
      currentProfile: profile,
      authoritativeProfile: profile,
      currentCatalogDraft: { ...catalog, mode: "custom-only" },
      authoritativeCatalogDraft: catalog,
    }), false);
    assert.equal(providerCapabilityEvidenceRefreshAllowed({
      applicable: false,
      currentProfile: profile,
      authoritativeProfile: profile,
      currentCatalogDraft: catalog,
      authoritativeCatalogDraft: catalog,
    }), false);
  });

  it("clears old evidence on refresh and accepts only the latest matching response", () => {
    const oldLedger = buildProviderCapabilityLedger(baseInput);
    const freshLedger = buildProviderCapabilityLedger({
      ...baseInput,
      runtime: "restartRequired",
    });
    const first = beginProviderCapabilityEvidenceLoad({
      ...createProviderCapabilityEvidenceLoadState(),
      ledger: oldLedger,
    });
    assert.equal(first.state.loading, true);
    assert.equal(first.state.ledger, null);
    const second = beginProviderCapabilityEvidenceLoad(first.state);
    const stale = settleProviderCapabilityEvidenceLoad(
      second.state,
      first.requestSequence,
      true,
      oldLedger,
      "catalog:v1",
    );
    assert.equal(stale.disposition, "stale");
    assert.equal(stale.state.ledger, null);
    const current = settleProviderCapabilityEvidenceLoad(
      stale.state,
      second.requestSequence,
      true,
      freshLedger,
      "catalog:v2",
    );
    assert.equal(current.disposition, "applied");
    assert.equal(current.state.loading, false);
    assert.equal(current.state.ledger?.runtime.state, "restartRequired");
    assert.equal(current.state.sourceFingerprint, "catalog:v2");

    const third = beginProviderCapabilityEvidenceLoad(current.state);
    const failed = settleProviderCapabilityEvidenceLoad(
      third.state,
      third.requestSequence,
      true,
      null,
      "catalog:v2",
    );
    assert.equal(failed.state.ledger, null);
    assert.equal(failed.state.loading, false);

    const fourth = beginProviderCapabilityEvidenceLoad(failed.state);
    const mismatched = settleProviderCapabilityEvidenceLoad(
      fourth.state,
      fourth.requestSequence,
      false,
      freshLedger,
      "catalog:v3",
    );
    assert.equal(mismatched.state.ledger, null);
    assert.equal(mismatched.state.sourceFingerprint, null);
    assert.equal(mismatched.state.loading, false);

    const invalidated = invalidateProviderCapabilityEvidenceLoad(mismatched.state);
    assert.equal(invalidated.requestSequence, mismatched.state.requestSequence + 1);
    assert.equal(invalidated.ledger, null);
  });
});
