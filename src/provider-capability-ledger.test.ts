import assert from "node:assert";
import { describe, it } from "node:test";

import {
  buildProviderCapabilityLedger,
  type ProviderCapabilityLedgerInput,
} from "./provider-capability-ledger.ts";

const baseInput: ProviderCapabilityLedgerInput = {
  providerContract: "ready",
  oauthSession: "signedIn",
  localPlan: "unknown",
  actorMarker: "eligible",
  catalogModel: "supported",
  upstream: "unknown",
  runtime: "unknown",
  routeKind: "nativePriorityMixed",
  imagePlanEvidence: "unknown",
};

describe("provider capability evidence ledger", () => {
  it("keeps OAuth session and local plan on independent activation axes", () => {
    for (const localPlan of ["free", "paid", "unknown"] as const) {
      const ledger = buildProviderCapabilityLedger({ ...baseInput, localPlan });
      assert.equal(ledger.oauth.activationGate, "satisfied");
      assert.equal(ledger.oauth.mayActivate, true);
      assert.equal(ledger.plan.observed, localPlan);
      assert.equal(ledger.plan.provesCapabilitySuccess, false);
    }

    for (const oauthSession of ["signedOut", "expired"] as const) {
      const ledger = buildProviderCapabilityLedger({ ...baseInput, oauthSession });
      assert.equal(ledger.oauth.activationGate, "blocked");
      assert.equal(ledger.oauth.mayActivate, false);
      assert.equal(ledger.inactiveSave, "allowedActionRequired");
    }

    const unknown = buildProviderCapabilityLedger({ ...baseInput, oauthSession: "unknown" });
    assert.equal(unknown.oauth.activationGate, "unknown");
    assert.equal(unknown.oauth.mayActivate, false);
  });

  it("blocks Free image only for one verified affected target path", () => {
    const affected = buildProviderCapabilityLedger({
      ...baseInput,
      localPlan: "free",
      imagePlanEvidence: "verifiedFreePlanBlocked",
    });
    assert.equal(affected.image.planGate, "blocked");
    assert.equal(affected.image.status, "blocked");

    const unaffected = buildProviderCapabilityLedger({
      ...baseInput,
      localPlan: "free",
      imagePlanEvidence: "verifiedNoFreePlanBlock",
    });
    assert.equal(unaffected.image.planGate, "notBlocked");
    assert.equal(unaffected.image.status, "unknown");

    const unknownTarget = buildProviderCapabilityLedger({
      ...baseInput,
      localPlan: "free",
      imagePlanEvidence: "unknown",
    });
    assert.equal(unknownTarget.image.planGate, "unknown");
    assert.equal(unknownTarget.image.status, "unknown");

    const paid = buildProviderCapabilityLedger({
      ...baseInput,
      localPlan: "paid",
      upstream: "imagePermissionVerified",
      runtime: "adopted",
    });
    assert.equal(paid.plan.provesCapabilitySuccess, false);
    assert.notEqual(paid.image.status, "available");
  });

  it("keeps actor, catalog, upstream, and runtime evidence independent and redacted", () => {
    const ledger = buildProviderCapabilityLedger({
      ...baseInput,
      actorMarker: "eligible",
      catalogModel: "missingMetadata",
      upstream: "textReachable",
      runtime: "restartRequired",
    });
    assert.equal(ledger.actor.provesEligibilityOnly, true);
    assert.equal(ledger.catalogModel.state, "missingMetadata");
    assert.equal(ledger.upstream.state, "textReachable");
    assert.equal(ledger.runtime.state, "restartRequired");
    assert.equal(ledger.image.status, "unknown");
    assert.doesNotMatch(JSON.stringify(ledger), /token|account|email|bearer|api.?key/i);
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
  });
});
