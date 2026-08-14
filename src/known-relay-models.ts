import type { ReasoningLevel } from "./backend-types";

/// A relay-served model the fleet has verified end to end, shipped as a complete card.
///
/// These are facts about the model — context window, reasoning ladder, effective percent — not
/// preferences, so the editor prefills them instead of asking. The values are copied
/// field-for-field from the hand-authored catalog the fleet ran in production; the contract test
/// pins every field, so revising a card is a reviewed diff, never a drive-by. A card only
/// prefills the draft: a saved profile owns its copy, and a later revision must not rewrite it.
export type KnownRelayModel = {
  slug: string;
  displayName: string;
  description: string;
  contextWindow: number;
  effectiveContextWindowPercent: number;
  supportedReasoningLevels: ReasoningLevel[];
  defaultReasoningLevel: string;
};

const FOUR_LEVELS: ReasoningLevel[] = [
  { effort: "low", description: "Fast responses with lighter reasoning" },
  { effort: "medium", description: "Balances speed and reasoning depth for everyday tasks" },
  { effort: "high", description: "Greater reasoning depth for complex problems" },
  { effort: "xhigh", description: "Extra high reasoning depth for complex problems" },
];

export const KNOWN_RELAY_MODELS: readonly KnownRelayModel[] = [
  {
    slug: "claude-fable-5",
    displayName: "Fable 5",
    description: "Creative-writing model via the Sub2API Responses bridge.",
    contextWindow: 1_000_000,
    effectiveContextWindowPercent: 95,
    supportedReasoningLevels: FOUR_LEVELS,
    defaultReasoningLevel: "medium",
  },
  {
    slug: "claude-opus-5",
    displayName: "Opus 5",
    description: "High-capability coding model via the Sub2API Responses bridge.",
    contextWindow: 1_000_000,
    effectiveContextWindowPercent: 95,
    supportedReasoningLevels: FOUR_LEVELS,
    defaultReasoningLevel: "medium",
  },
  {
    slug: "claude-sonnet-5",
    displayName: "Sonnet 5",
    description: "Balanced coding model via the Sub2API Responses bridge.",
    contextWindow: 1_000_000,
    effectiveContextWindowPercent: 95,
    supportedReasoningLevels: FOUR_LEVELS,
    defaultReasoningLevel: "medium",
  },
  {
    slug: "claude-haiku-4-5-20251001",
    displayName: "Haiku 4.5",
    description: "Fast model via the Sub2API Responses bridge.",
    contextWindow: 200_000,
    effectiveContextWindowPercent: 95,
    supportedReasoningLevels: [
      { effort: "low", description: "Fast responses with lighter reasoning" },
      { effort: "high", description: "Greater reasoning depth for complex problems" },
    ],
    defaultReasoningLevel: "low",
  },
];

export function knownRelayModel(slug: string): KnownRelayModel | null {
  return KNOWN_RELAY_MODELS.find((model) => model.slug === slug.trim()) ?? null;
}
