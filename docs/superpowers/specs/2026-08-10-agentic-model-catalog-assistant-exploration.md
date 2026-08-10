# Agentic Model Catalog Assistant Exploration

**Date:** 2026-08-10
**Status:** Exploration only; outside the raw JSON editor delivery scope

## Idea

Add an experimental `Agentic generate / repair` action beside `Add JSON model`. A catalog-specific Skill supplies the model-catalog contract and safety rules, while a Pi agent uses the newly configured model resources to investigate context and propose a complete JSON model object.

This would turn the Manager from only a deterministic catalog editor into an agent-assisted catalog workbench without making AI generation a dependency of the core editing path.

## Potential User Jobs

- Copy an official model and change its context window to 372000 while preserving the complete object.
- Keep the user's instructions while selectively syncing newer official tool capabilities.
- Compare a custom model with the current official version and propose a safe upgrade.
- Repair a model object that fails target Codex CLI validation.
- Generate a complete model object from provider evidence and a user-described intent.

## Candidate Flow

1. The user opens `Agentic generate / repair` from the custom JSON section.
2. The Manager provides only the minimum scoped context:
   - selected user JSON block, if any;
   - matching official model, if any;
   - provider model evidence;
   - target CLI version and offline validation errors;
   - the catalog Skill and its invariants.
3. The Pi agent returns a structured proposal containing:
   - the complete candidate JSON object;
   - a concise rationale;
   - assumptions and unresolved risks.
4. The Manager shows a semantic JSON diff against the current block or selected official model.
5. User confirmation inserts the proposal into the editor as an unsaved draft.
6. The normal deterministic frontend checks, backend validation, offline target-CLI validation, and catalog transaction remain the only route to persistence.

## Product Boundary

The Pi agent is a proposal engine, not a configuration writer.

It must not:

- Write `config.toml`, catalog state, generated catalogs, or external source files directly.
- Receive OAuth tokens, provider API keys, or unrelated config context.
- Bypass the raw JSON editor, preview, user confirmation, or deterministic validators.
- Make the base JSON editor depend on Pi availability.

If the agent, Skill, backend model, or network is unavailable, manual JSON editing, import, validation, and save continue to work normally.

## Architectural Sketch

```text
User intent + selected JSON + scoped evidence
                    |
                    v
          Catalog Skill contract
                    |
                    v
              Pi agent run
                    |
                    v
        Candidate JSON + rationale
                    |
                    v
       Diff and explicit user approval
                    |
                    v
           Unsaved editor draft
                    |
                    v
 Deterministic validation and transaction save
```

## Why Keep It Separate

The raw JSON editor solves a correctness and ownership problem: complete user model objects must survive import and composition. Agentic assistance is an optional creation experience with additional runtime, permission, privacy, observability, and failure-mode questions.

Keeping the tracks separate allows the editor to ship and establish a reliable substrate first. The assistant can later operate entirely through the same draft-object boundary.

## Questions for a Future Spike

- What exact Pi invocation and structured-output contract are available in the configured local stack?
- Should the first version generate new blocks only, or also edit existing blocks?
- How should the Skill version be pinned and shown in proposal provenance?
- Which context fields are necessary, and which must be redacted before an agent run?
- Should offline CLI validation run once after generation, or enter a bounded repair loop?
- What time, token, and retry limits make an agent run predictable in a desktop UI?
- How should cancellation, partial output, and malformed structured output be represented?
- Is semantic JSON diff sufficient for large instruction fields, or is a field-focused review needed?

## Suggested First Experiment

Run a non-writing spike against a representative `gpt-5.6-sol` block:

> Preserve the complete official model object, set both context windows to 372000, retain current instructions, and explain every changed field.

Success means the Pi agent returns one valid complete JSON object plus an accurate rationale, the Manager can render a trustworthy diff, no secret enters the run, and the candidate passes the existing offline Codex CLI validator without giving the agent write access.

This experiment should produce evidence for a later proposal; it is not part of the raw JSON editor implementation plan.
