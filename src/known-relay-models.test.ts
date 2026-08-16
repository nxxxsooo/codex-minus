import assert from "node:assert";
import { describe, it } from "node:test";

import { KNOWN_RELAY_MODELS, knownRelayModel } from "./known-relay-models.ts";

describe("the known relay model cards", () => {
  it("pins every field of every card so a revision is a reviewed diff", () => {
    // Copied field-for-field from the hand-authored catalog the fleet ran in production
    // (models_372k_claude.json, template-identical to the server cache where they overlap).
    // A mismatch here is either a deliberate card revision — update both sides — or a typo.
    const fourLevels = [
      { effort: "low", description: "Fast responses with lighter reasoning" },
      { effort: "medium", description: "Balances speed and reasoning depth for everyday tasks" },
      { effort: "high", description: "Greater reasoning depth for complex problems" },
      { effort: "xhigh", description: "Extra high reasoning depth for complex problems" },
    ];
    assert.deepEqual(KNOWN_RELAY_MODELS, [
      {
        slug: "claude-fable-5",
        displayName: "Fable 5",
        description: "Creative-writing model via the Sub2API Responses bridge.",
        contextWindow: 1_000_000,
        effectiveContextWindowPercent: 95,
        supportedReasoningLevels: fourLevels,
        defaultReasoningLevel: "medium",
      },
      {
        slug: "claude-opus-5",
        displayName: "Opus 5",
        description: "High-capability coding model via the Sub2API Responses bridge.",
        contextWindow: 1_000_000,
        effectiveContextWindowPercent: 95,
        supportedReasoningLevels: fourLevels,
        defaultReasoningLevel: "medium",
      },
      {
        slug: "claude-sonnet-5",
        displayName: "Sonnet 5",
        description: "Balanced coding model via the Sub2API Responses bridge.",
        contextWindow: 1_000_000,
        effectiveContextWindowPercent: 95,
        supportedReasoningLevels: fourLevels,
        defaultReasoningLevel: "medium",
      },
      {
        slug: "claude-haiku-4-5-20251001",
        displayName: "Haiku 4.5",
        description: "Fast model via the Sub2API Responses bridge.",
        contextWindow: 200_000,
        effectiveContextWindowPercent: 95,
        // The Sub2API bridge rejects `effort` for Haiku, so the card declares no levels
        // and Codex offers no Effort menu (2026-08-16, live rejection).
        supportedReasoningLevels: [],
        defaultReasoningLevel: null,
      },
    ]);
  });

  it("keeps every card internally coherent", () => {
    for (const card of KNOWN_RELAY_MODELS) {
      if (card.supportedReasoningLevels.length === 0) {
        assert.equal(
          card.defaultReasoningLevel,
          null,
          `${card.slug}: a card without levels must not name a default`,
        );
      } else {
        assert.ok(
          card.supportedReasoningLevels.some(
            (level) => level.effort === card.defaultReasoningLevel,
          ),
          `${card.slug}: default level is not in its supported list`,
        );
      }
      assert.ok(card.contextWindow > 0);
      assert.ok(card.effectiveContextWindowPercent >= 1 && card.effectiveContextWindowPercent <= 100);
      assert.ok(card.displayName.trim().length > 0);
    }
  });

  it("looks a card up by slug, tolerating surrounding whitespace", () => {
    assert.equal(knownRelayModel("claude-fable-5")?.displayName, "Fable 5");
    assert.equal(knownRelayModel(" claude-haiku-4-5-20251001 ")?.displayName, "Haiku 4.5");
    assert.equal(knownRelayModel("gpt-5.6-sol"), null);
  });
});
