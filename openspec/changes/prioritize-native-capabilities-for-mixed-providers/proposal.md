## Why

Codex-- already steers new providers toward official ChatGPT OAuth mixed with a provider API key over the Responses API, but its generated provider block still uses compatibility semantics (`name = "custom"` and `requires_openai_auth = true`). Current Codex consequently does not recognize the custom provider as actor-authorized for local OpenAI extensions such as image generation, and later structured edits can silently restore the incompatible auth flag.

The product needs one explicit, reproducible “native-capability priority” path that routes inference through the provider key while leaving official OAuth under the official client’s ownership and truthfully reporting the remaining account, model, upstream, and restart gates.

## What Changes

- Introduce a first-class native-capability-priority state for official-OAuth-plus-provider-key Responses profiles.
- Make newly created ordinary provider drafts select that target by default after the provider-onboarding work is integrated; incomplete drafts remain visibly degraded until their required inputs exist.
- Offer eligible existing mixed profiles an explicit draft-only upgrade action; never silently migrate existing profiles.
- Materialize one coherent provider contract: OpenAI-compatible provider identity, Responses wire API, no OpenAI-auth requirement for the custom provider, owner-only provider bearer projection, and a non-empty `x-openai-actor-authorization` header for local extensions.
- Derive effective state from the provider TOML instead of persisting a second boolean that can drift from the configuration.
- When the target mode retains a custom provider table, preserve unrelated provider headers and fields; prevent later model/Base URL/key edits from undoing the native-capability contract; and keep global live configuration outside the provider profile. True pure OAuth remains the explicit, previewed destructive exception because it removes the custom provider table.
- Show capability evidence and degraded reasons without promising that configuration alone unlocks every native feature. In particular, identify the current Free-plan image-generation restriction, model/catalog capability requirements, upstream image permission, and restart/new-task requirement.
- Complete and use one unified provider-detail Save/Set-as-current transaction for first save, inactive save, upgrade, and activation through the existing Context/OAuth protection boundary; add no direct `config.toml` or `auth.json` write path.
- Keep pure OAuth as true `native-official`, backed by Codex’s dynamic catalog. Keep pure API and legacy compatibility paths available as advanced, non-default options.
- Reuse the existing `official-plus-custom` catalog contract and its currently authoritative validated official baseline for mixed profiles. Any future standard-Pro baseline, signed-update policy, or optional Sol 372k overlay remains a separate catalog change and is not defined or required here.

## Capabilities

### New Capabilities

- `provider-native-capability-mode`: Defines eligibility, provider-TOML materialization, explicit upgrade behavior, truthful capability status, restart handling, and OAuth/configuration safety for the native-capability-priority mixed-provider path.

### Modified Capabilities

None. The existing `model-catalog-management` specification already assigns mixed profiles to `official-plus-custom` and requires atomic active-profile provider/catalog commits; this change consumes those contracts without redefining their catalog source or composition rules.

## Impact

- Frontend provider onboarding, provider detail state, upgrade presentation, capability diagnostics, and restart guidance.
- Backend provider-TOML parsing and transformation, normalized profile persistence, staged live application, and sanitized auth/capability status.
- Existing provider generation and structured edit helpers that currently force `requires_openai_auth = true` or reconstruct provider tables.
- Provider Doctor presentation and validation boundaries; it may report observable prerequisites but must not claim end-to-end image generation without a real verified result.
- Regression coverage for new-profile target defaults, explicit migration, external-catalog precedence, TOML/header preservation, subsequent structured edits, unified first/inactive/active transactions, Context preservation, and OAuth concurrency safety.
- Sequencing dependency on the provider-onboarding branch and live-config/provider-config separation; the missing unified provider-detail transaction is implemented as part of this change rather than assumed to exist.
- No change to the official client’s OAuth ownership, no OAuth token persistence, no `auth.json` mutation, no local Chat Completions proxy, and no Sub2API server-side implementation.
