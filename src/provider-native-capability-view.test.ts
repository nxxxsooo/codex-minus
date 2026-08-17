import assert from "node:assert";
import { describe, it } from "node:test";

import {
  deriveProviderNativeCapabilityView,
  deriveProviderModePresentation,
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
  it("presents ordinary pure OAuth as native-official without converting it", () => {
    assert.equal(deriveProviderModePresentation({
      profileId: "official-only",
      state: "notApplicable",
      fields: [
        { field: "relayMode", outcome: "notApplicable", reason: "pureOAuth" },
      ],
    }), "nativeOfficial");
  });

  it("gives external ownership presentation precedence for every topology", () => {
    for (const state of ["notApplicable", "upgradeAvailable", "degraded"]) {
      const inspection = {
        profileId: `external-${state}`,
        state,
        fields: [
          { field: "catalog", outcome: "notApplicable", reason: "externalCatalog" },
          { field: "relayMode", outcome: "satisfied", reason: "canonical" },
        ],
      };
      assert.equal(deriveProviderModePresentation(inspection), "external");
      assert.equal(deriveProviderNativeCapabilityView({
        inspection,
        officialAuth: { authenticated: true, localPlan: "unknown" },
      }).upgradeAction, null);
    }
  });

  it("keeps pure API and legacy paths visibly advanced", () => {
    const reasons = [
      ["compatibility", "pureApi"],
      ["upgradeAvailable", "legacyProviderIdRequiresRename"],
    ] as const;
    for (const [state, reason] of reasons) {
      assert.equal(deriveProviderModePresentation({
        profileId: `advanced-${reason}`,
        state,
        fields: [{ field: "relayMode", outcome: "mismatch", reason }],
      }), "advancedCompatibility");
    }
    assert.equal(deriveProviderModePresentation({
      profileId: "advanced-legacy-contract",
      state: "upgradeAvailable",
      fields: [
        { field: "providerName", outcome: "mismatch", reason: "providerNameMismatch" },
        { field: "requiresOpenAiAuth", outcome: "mismatch", reason: "openAiAuthRequired" },
        { field: "actorHeader", outcome: "missing", reason: "missingActorHeader" },
      ],
    }), "advancedCompatibility");
  });

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

    const legacyAndMissingBearer = deriveProviderNativeCapabilityView({
      inspection: {
        profileId: "legacy-broken",
        state: "degraded",
        fields: [
          {
            field: "providerSelection",
            outcome: "mismatch",
            reason: "legacyProviderIdRequiresRename",
          },
          {
            field: "providerBearer",
            outcome: "missing",
            reason: "missingProviderBearer",
          },
        ],
      },
      officialAuth: { authenticated: true, localPlan: "free" },
    });
    assert.equal(legacyAndMissingBearer.upgradeAvailability, "unavailable");
    assert.equal(legacyAndMissingBearer.upgradeAction, null);
  });
});

describe("ordinary provider mode controls", () => {
  const nativeProfile = {
    relayMode: "official",
    officialMixApiKey: true,
  };

  it("routes supported mode exits without using an implicit upgrade action", () => {
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
