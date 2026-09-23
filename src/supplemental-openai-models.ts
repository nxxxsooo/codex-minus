import type { KnownRelayModel } from "./known-relay-models";

// Checked 2026-09-23: https://models.dev/api.json (openai/gpt-6-sol, openai/gpt-6-luna)
// and https://developers.openai.com/codex/models. Both launched on 2026-09-22.
// Verified OpenAI-signed CLI 0.156.0 still bundles the 5.6 rows: these are supplemental custom
// cards, not fabricated official snapshots. The ordinary 272k working window is our preset;
// the explicit long-context control requests the documented 1,050,000-token total window.
const reasoning = ["none", "low", "medium", "high", "xhigh", "max"].map((effort) => ({ effort, description: effort }));

export const SUPPLEMENTAL_OPENAI_MODELS: readonly KnownRelayModel[] = [
  {
    slug: "gpt-6-sol", displayName: "GPT-6 Sol",
    description: "OpenAI model for complex coding and agentic workflows.",
    contextWindow: 272000, effectiveContextWindowPercent: 100,
    supportedReasoningLevels: reasoning, defaultReasoningLevel: "medium",
  },
  {
    slug: "gpt-6-luna", displayName: "GPT-6 Luna",
    description: "OpenAI model for focused, high-volume tasks.",
    contextWindow: 272000, effectiveContextWindowPercent: 100,
    supportedReasoningLevels: reasoning, defaultReasoningLevel: "medium",
  },
];
