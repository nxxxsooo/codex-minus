import assert from "node:assert";
import { describe, it } from "node:test";

import {
  deriveProviderNativeCapabilityView,
  providerTransitionDecisionForStructuredPatch,
} from "./provider-native-capability-view.ts";

const upgradeInspection = {
  profileId: "relay-one",
  state: "upgradeAvailable",
  fields: [
    { field: "relayMode", outcome: "satisfied", reason: "canonical" },
    { field: "actorHeader", outcome: "missing", reason: "missingActorHeader" },
  ],
};

describe("provider native-capability view", () => {
  it("keeps a signed-in Free plan descriptive while actor eligibility and runtime proof stay independent", () => {
    const view = deriveProviderNativeCapabilityView({
      inspection: upgradeInspection,
      officialAuth: { authenticated: true, localPlan: "free" },
    });

    assert.equal(view.state, "upgradeAvailable");
    assert.equal(view.upgradeAvailability, "available");
    assert.equal(view.officialAuthGate, "satisfied");
    assert.equal(view.localPlan, "free");
    assert.equal(view.localPlanBlocksActivation, false);
    assert.equal(view.actorMarkerEligibility, "ineligible");
    assert.equal(view.providerRoutableCapabilityProof, "unverified");
  });

  it("treats a canonical actor marker only as eligibility and never as entitlement proof", () => {
    const view = deriveProviderNativeCapabilityView({
      inspection: {
        profileId: "relay-one",
        state: "nativePriority",
        fields: [
          { field: "actorHeader", outcome: "satisfied", reason: "canonical" },
        ],
      },
      officialAuth: { authenticated: true, localPlan: "paid" },
    });

    assert.equal(view.actorMarkerEligibility, "eligible");
    assert.equal(view.providerRoutableCapabilityProof, "unverified");
    assert.equal(view.upgradeAvailability, "unavailable");
  });

  it("gives external ownership precedence and never offers the ordinary upgrade", () => {
    const view = deriveProviderNativeCapabilityView({
      inspection: {
        profileId: "external-one",
        state: "notApplicable",
        fields: [
          { field: "catalog", outcome: "notApplicable", reason: "externalCatalog" },
        ],
      },
      officialAuth: { authenticated: null, localPlan: "unknown" },
    });

    assert.equal(view.state, "notApplicable");
    assert.equal(view.externalOwnership, true);
    assert.equal(view.upgradeAvailability, "unavailable");
    assert.equal(view.actorMarkerEligibility, "notApplicable");
    assert.equal(view.officialAuthGate, "unknown");
  });

  it("requires official sign-in without using account plan as a substitute", () => {
    const view = deriveProviderNativeCapabilityView({
      inspection: upgradeInspection,
      officialAuth: { authenticated: false, localPlan: "unknown" },
    });

    assert.equal(view.officialAuthGate, "signInRequired");
    assert.equal(view.localPlanBlocksActivation, false);
  });
});

describe("ordinary provider protocol controls", () => {
  const nativeProfile = {
    relayMode: "official",
    officialMixApiKey: true,
    protocol: "responses",
  };

  it("keeps an already-selected Responses value a no-op instead of synthesizing an upgrade", () => {
    assert.deepEqual(
      providerTransitionDecisionForStructuredPatch(nativeProfile, { protocol: "responses" }),
      { kind: "noChange" },
    );
  });

  it("requires the explicit Upgrade action when returning from Chat Completions to Responses", () => {
    assert.deepEqual(
      providerTransitionDecisionForStructuredPatch(
        { ...nativeProfile, protocol: "chatCompletions" },
        { protocol: "responses" },
      ),
      { kind: "requiresExplicitUpgrade" },
    );
  });

  it("retains explicit compatibility exits without using an implicit upgrade action", () => {
    assert.deepEqual(
      providerTransitionDecisionForStructuredPatch(nativeProfile, { protocol: "chatCompletions" }),
      {
        kind: "transition",
        transition: { action: "exitChatCompletions", confirmations: [] },
      },
    );
    assert.deepEqual(
      providerTransitionDecisionForStructuredPatch(nativeProfile, {
        relayMode: "official",
        officialMixApiKey: false,
      }),
      {
        kind: "transition",
        transition: { action: "exitPureOAuth", confirmations: [] },
      },
    );
    assert.deepEqual(
      providerTransitionDecisionForStructuredPatch(nativeProfile, { relayMode: "pureApi" }),
      {
        kind: "transition",
        transition: { action: "exitPureApi", confirmations: [] },
      },
    );
  });
});
