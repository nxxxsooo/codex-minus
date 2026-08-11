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

  it("offers one explicit upgrade preview for Chat compatibility without classifying it ready", () => {
    const view = deriveProviderNativeCapabilityView({
      inspection: {
        profileId: "chat-one",
        state: "compatibility",
        fields: [
          { field: "protocol", outcome: "mismatch", reason: "chatCompletions" },
        ],
      },
      officialAuth: { authenticated: true, localPlan: "free" },
    });

    assert.equal(view.state, "compatibility");
    assert.equal(view.upgradeAvailability, "available");
    assert.equal(view.providerRoutableCapabilityProof, "unverified");
  });

  it("does not advertise a one-click upgrade when a legacy provider ID needs an explicit rename", () => {
    const view = deriveProviderNativeCapabilityView({
      inspection: {
        profileId: "legacy-one",
        state: "upgradeAvailable",
        fields: [
          {
            field: "providerSelection",
            outcome: "mismatch",
            reason: "legacyProviderIdRequiresRename",
          },
        ],
      },
      officialAuth: { authenticated: true, localPlan: "free" },
    });

    assert.equal(view.upgradeAvailability, "manualResolutionRequired");
    assert.equal(view.upgradeAction, "resolveLegacyProviderId");
    assert.equal(view.providerRoutableCapabilityProof, "unverified");
  });

  it("offers a distinct explicit actor replacement only when that is the sole contract conflict", () => {
    const actorConflict = deriveProviderNativeCapabilityView({
      inspection: {
        profileId: "actor-one",
        state: "degraded",
        fields: [
          { field: "relayMode", outcome: "satisfied", reason: "canonical" },
          { field: "actorHeader", outcome: "conflict", reason: "actorHeaderValueConflict" },
        ],
      },
      officialAuth: { authenticated: true, localPlan: "free" },
    });
    assert.equal(actorConflict.upgradeAvailability, "confirmationRequired");
    assert.equal(actorConflict.upgradeAction, "replaceActorHeader");

    const multipleConflicts = deriveProviderNativeCapabilityView({
      inspection: {
        profileId: "actor-broken",
        state: "degraded",
        fields: [
          { field: "providerBearer", outcome: "missing", reason: "missingProviderBearer" },
          { field: "actorHeader", outcome: "conflict", reason: "actorHeaderValueConflict" },
        ],
      },
      officialAuth: { authenticated: true, localPlan: "free" },
    });
    assert.equal(multipleConflicts.upgradeAvailability, "unavailable");
    assert.equal(multipleConflicts.upgradeAction, null);
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
    assert.deepEqual(
      providerTransitionDecisionForStructuredPatch(
        { ...nativeProfile, protocol: "chatCompletions" },
        { relayMode: "pureApi" },
      ),
      {
        kind: "transition",
        transition: { action: "exitPureApi", confirmations: [] },
      },
    );
    assert.deepEqual(
      providerTransitionDecisionForStructuredPatch(
        { ...nativeProfile, protocol: "chatCompletions" },
        { relayMode: "official", officialMixApiKey: false },
      ),
      {
        kind: "transition",
        transition: { action: "exitPureOAuth", confirmations: [] },
      },
    );
  });
});
