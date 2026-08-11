## Context

See `proposal.md` for motivation and `specs/model-catalog-management/spec.md` for the behavior contract. The current upstream `RelayMode::Aggregate` is not a generic topology label: it serializes member and strategy data and routes through the removed proxy at `127.0.0.1:57321`. In contrast, a service such as a sub2api composite group performs routing upstream and presents Codex with one Responses endpoint and one bearer key.

Catalog state is already Manager-owned and keyed by relay profile ID, while relay profiles and their apply primitives come from the pinned `codex-plus-core`. Catalog materialization and live provider changes already share the process-wide coordinator, owner-only transaction journal, and fail-closed Context transaction. Those ownership and safety boundaries remain mandatory.

The current overlay schema is lossy: official overrides support only visibility, context window, and order; custom models support only identity, window, visibility, and order. Composition hardcodes `effective_context_window_percent = 100` after a window override and strips all tool metadata from custom entries. Separately, the frontend projects profile-wide context and compaction values into root TOML even when a managed multi-model catalog defines different per-model windows.

## Goals / Non-Goals

**Goals:**

- Represent upstream composition without changing the wire configuration Codex receives.
- Preserve rich, explicitly selected model metadata through overlay save, composition, promotion, and adoption.
- Make context precedence deterministic and transactional for managed catalogs.
- Turn external catalog version mismatch into visible compatibility evidence backed by target-CLI offline validation.

**Non-Goals:**

- Restore local member rotation, the Chat Completions conversion proxy, launcher behavior, or port `57321`.
- Infer that a provider is composite from its hostname, `/v1/models` response, model names, or vendor mix.
- Fork or extend the pinned upstream `RelayMode` data model.
- Claim that target-CLI structural acceptance proves an upstream account can serve a model or tool.
- Add arbitrary raw model JSON editing or copy official-only service tiers, upgrade prompts, availability messages, or base instructions into custom entries.

## Decisions

### Store topology classification in Manager-owned catalog state

Add a backward-compatible `upstream_topology` enum to each profile's catalog state with `direct` as the serde default and `server-side-composite` as the explicit alternative. A server-side composite is valid only when the corresponding relay profile is `PureApi`, uses `Responses`, and has a non-empty Base URL and provider bearer token.

The frontend exposes this as a profile classification, but saving it uses the catalog-state command and the same coordinator as catalog changes. Classification does not rewrite the relay profile into `Aggregate`: the pinned upstream profile remains `PureApi`, so existing provider staging writes one `model_provider` and one Base URL exactly as it does for a direct API relay. `managed_catalog_capable` and `stage_active_relay_config` continue rejecting `RelayMode::Aggregate` and Chat Completions.

For a newly classified composite profile whose catalog mode is not explicit, mode resolution selects `official-plus-custom`; an explicit mode is never changed. Reclassifying back to direct similarly preserves an explicit mode and uses the normal pure-API default only when the mode remains implicit. Existing profiles are never inferred or auto-converted. The existing Aggregate UI is relabeled as local aggregation and continues to show its unavailable-proxy warning.

Alternative considered: make `RelayMode::Aggregate` support both local and remote aggregation. Rejected because the upstream variant has concrete member-rotation semantics, would make old profiles ambiguous, and would weaken the fail-closed guard around a removed runtime dependency.

Alternative considered: add a new relay mode to `codex-plus-core`. Rejected because codex-minus deliberately consumes pinned upstream provider logic without vendoring or forking it, and the topology label does not alter Codex's wire configuration.

### Use typed rich overlays with conservative defaults

Extend official overrides with optional `display_name`, `effective_context_window_percent`, `supported_reasoning_levels`, `default_reasoning_level`, `supported_tools`, and `tool_capabilities`. Extend custom models with the same model-owned metadata, retaining template provenance. Reasoning levels use target-shaped effort/description records; the default must name one supported effort. Effective context percentage is an integer from 1 through 100. Display names must be non-empty after trimming.

Tool fields use target-shaped structured values rather than ad hoc strings. The backend validates their outer shape and relies on the verified target CLI's offline catalog projection as the final schema compatibility gate. Empty or absent tool fields advertise no additional capability. The custom template is stripped first, then only explicitly configured tool fields are grafted back; service tiers, speed tiers, upgrade and availability metadata, instructions, and other official-backend claims remain excluded.

Official overrides apply only fields represented by non-`None` overlay values. Changing `context_window` updates `context_window` and `max_context_window`, but preserves the baseline `effective_context_window_percent` unless the percentage is explicitly overridden. This removes the current hidden `100` substitution. Custom-to-official promotion computes field-by-field differences against the new official entry and applies each compatible user value once.

The frontend uses bounded controls for effective percentage, a menu for default reasoning, ordered reasoning rows, and structured tool controls. Display-name overrides are available for official rows so compact picker labels do not require hand-edited catalogs.

Alternative considered: retain only the current thin overlay and copy all fields from an arbitrary template model. Rejected because it silently assigns unrelated capabilities and cannot represent the observed Claude reasoning choices.

Alternative considered: expose a raw JSON editor. Rejected because malformed or official-only fields would bypass the Manager's compatibility and ownership contract.

### Enforce managed context precedence in the backend transaction

Treat `model_context_window` and `model_auto_compact_token_limit` as incompatible with `official-plus-custom` and `custom-only`, but not with `native-official` or unadopted `external`. The backend detects both keys in saved profile TOML and, for an active profile, in the staged live configuration. Detection sets structured action-required state and blocks a catalog generation from being reported current.

The save/apply request gains an explicit confirmation flag. Without confirmation, any conflicting save or switch returns a non-mutating response that lists the keys and leaves settings, live config, generated catalog, pointer, and generation unchanged. With confirmation, the existing multi-file transaction removes the keys from the saved profile projection and staged live root, composes and validates the catalog, updates the pointer, and commits one recoverable generation. Context-table snapshot, verbatim graft, parse-back verification, and full rollback remain unchanged.

The frontend disables the two profile-wide fields while a managed mode is selected and routes context editing to model rows. Raw or legacy values remain visible in the warning until the confirmed transaction removes them. Backend enforcement is authoritative so stale frontends and raw save paths cannot reintroduce the conflict.

Alternative considered: let global values coexist and show a passive warning. Rejected because Codex gives those values precedence and silently clamps heterogeneous models, so the resulting managed catalog would not describe runtime behavior.

Alternative considered: delete the values automatically on first load. Rejected because they may be intentional for native or external profiles and the active configuration change must remain explicit and rollback-safe.

### Separate external adoption compatibility from official provenance

Official refresh continues to require an exact parsed major/minor/patch match with the verified target CLI because it establishes the authoritative baseline. External adoption first performs structure validation, then runs the existing credential-free target-CLI offline projection. The declared external `client_version`, when present, is compared for reporting only.

The adoption preview returns catalog version, target version, a match/mismatch/unknown status, the source content hash, and the existing model diff. A mismatch requires an explicit acceptance flag on commit. Commit re-reads the source and requires the previewed hash and version status to match, preventing a reviewed file from being replaced before adoption. Structural failure, target projection changes, collisions, or hash drift always block adoption. The external pointer and file remain untouched until the managed replacement commits.

Alternative considered: apply the official exact-version gate to adoption. Rejected because hand-maintained catalogs can remain target-compatible across CLI releases and the target's offline parser is stronger evidence than the declaration alone.

Alternative considered: ignore `client_version` completely. Rejected because the mismatch is useful maintenance evidence and should be an explicit user decision rather than accidental leniency.

### Version and diagnose the new state without storing sensitive data

Bump the Manager catalog-state schema and deserialize all new fields with safe defaults. Status payloads expose topology, context-conflict keys, external version status, and metadata-validation errors. Diagnostics record only profile ID, topology, versions, hashes, counts, and redacted failures; provider keys, auth, model instructions, and raw tool payloads remain excluded.

## Risks / Trade-offs

- [A profile is mislabeled as server-side composite] -> Validate the required `PureApi` + Responses shape, never infer classification, and keep routing identical to the already configured endpoint.
- [Explicit capability metadata overstates provider support] -> Default to no extra claims, label `/v1/models` as evidence only, validate through the target CLI, and keep runtime availability outside the catalog guarantee.
- [Tool metadata schema changes across Codex releases] -> Preserve target-shaped structured values, reject invalid offline projections, and retain the previous effective generation on failure.
- [Removing global context values surprises an existing user] -> Require explicit confirmation, scope cleanup to managed modes, preview both keys, and use the existing full-generation rollback journal.
- [A stale adoption preview commits different external content] -> Bind commit to the reviewed source hash and repeat structure plus target-offline validation inside the transaction.
- [Manager-owned topology state is lost or read by an older build] -> Default safely to `direct`; the profile still routes as ordinary `PureApi`, while only the implicit catalog default and label are lost.

## Migration Plan

1. Add the tolerant state-schema fields and status payloads. Existing profiles deserialize as `direct`; existing overlay entries receive conservative defaults and retain their current materialized output until changed.
2. Add backend topology validation and mode-default resolution, then expose explicit classify/reclassify controls. Do not convert existing `Aggregate` profiles or infer composites.
3. Add rich overlay parsing, validation, composition, promotion, and target-offline tests before exposing the new frontend controls.
4. Add context-conflict detection in read-only status, followed by the confirmed transactional cleanup path. Existing active generations remain in place until the user confirms.
5. Extend external adoption preview and commit binding with version evidence and source hashes.
6. Update the provider UI, English strings, tests, README, `BOARD.md`, and the project hard constraint to say “client-side proxy aggregation” once verification passes.

Rollback uses the existing journal to restore settings, live configuration, generated catalog, pointer, and catalog state as one prior generation. Downgrading the application treats unknown state fields as inert; server-side composite profiles continue to be valid pure API profiles, but an older build will not present the classification or rich overlay controls.
