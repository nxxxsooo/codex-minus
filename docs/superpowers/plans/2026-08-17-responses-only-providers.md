# Responses-only Providers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove every Chat Completions, local protocol-proxy, and client-side aggregate provider product path so Codex Minus accepts, persists, and exposes only ordinary Responses providers.

**Architecture:** Add one fail-closed Rust boundary invariant first, then delete the dead aggregate and Chat Completions paths in vertical slices. The frontend stops representing removed choices; the provider commit DTO stops accepting them; Rust keeps any unavoidable upstream `codex-plus-core` fields empty and omits their JSON keys, without adding migration, filtering, or fallback behavior.

**Tech Stack:** Tauri 2, Rust, serde/serde_json, React 19, TypeScript, Node test runner, Vite, `toml_edit`, pinned `codex-plus-core` git dependency.

## Global Constraints

- This is an intentional breaking change: old Chat Completions and local aggregate `settings.json` data is unsupported.
- Do not add migration, compatibility deserialization, startup filtering, fallback, read-only display, or one-way upgrade behavior for the removed paths.
- The only supported wire protocol is Responses: profile protocol is `Responses` and provider TOML uses `wire_api = "responses"`.
- A server-side composite remains an ordinary provider with one Responses Base URL, one Key, and one catalog.
- Do not implement or retain the `127.0.0.1:57321` local proxy.
- Do not vendor or fork `codex-plus-core`; unavoidable upstream fields must remain empty at the local adapter boundary until an upstream revision removes them.
- Every `config.toml` write continues through the process-wide coordinator and Context transaction guard.
- Live `auth.json` remains byte-for-byte owned by the official Codex／ChatGPT client; raw `authContents` writes remain rejected.
- External catalog files and pointers remain untouched without explicit adoption.
- The canonical native-capability contract remains provider name `OpenAI`, `wire_api = "responses"`, provider bearer, and the exact actor header.
- `src/App.tsx` remains wiring-only; delete pure aggregate/protocol helpers from its shell-budget allowlist rather than replacing them with new shell logic.
- Run failing tests before implementation and commit each task only after its focused verification passes.

---

## File Structure

### Rust contract and persistence boundary

- Modify `src-tauri/src/provider_commit.rs`: own the Responses-only commit DTO and request validation; remove aggregate draft types and topology fields.
- Modify `src-tauri/src/commands.rs`: validate loaded and incoming settings, omit removed persistence keys, simplify the UI projection, and keep upstream-only fields empty.
- Modify `src-tauri/src/provider_commit_transaction_tests.rs`: prove invalid legacy inputs fail before every mutation and valid Responses transactions remain atomic.

### Rust catalog and native-capability behavior

- Modify `src-tauri/src/model_catalog.rs`: make catalog eligibility and upstream topology operate only on accepted Responses profiles; remove aggregate and Chat Completions success-path branches.
- Modify `src-tauri/src/provider_native_capability.rs`: remove aggregate／Chat reasons and the `ExitChatCompletions` draft action.
- Modify `src-tauri/tests/provider_native_capability.rs`: remove legacy presentation fixtures and keep canonical, pure OAuth／API, external, and contract-conflict coverage.
- Modify `src-tauri/tests/provider_native_capability_draft.rs`: remove Chat Completions transition cases and keep supported transition concurrency／confirmation coverage.

### Frontend domain and commit projection

- Modify `src/backend-types.ts`: remove aggregate structures and narrow protocol to Responses-only; remove proxy-only `upstreamBaseUrl` from the frontend profile shape.
- Modify `src/relay-settings.ts`: remove aggregate creation／normalization／validation and dual-URL behavior; make relay synchronization ordinary-provider-only.
- Create `src/relay-settings.test.ts`: cover ordinary profile deletion, active fallback, derivation, and Responses URL synchronization.
- Modify `src/provider-commit.ts`: remove aggregate topology projection and mutation kind; emit only Responses profile fields.
- Modify `src/provider-config-transform-router.ts`: remove proxy constants, protocol patching, and `exitChatCompletions`.
- Modify `src/provider-onboarding.ts`: stop storing protocol as draft state while continuing to materialize `wire_api = "responses"`.
- Modify `src/provider-native-capability-view.ts`: remove Chat／aggregate presentation and protocol transition decisions.
- Modify `src/provider-detail-draft-state.ts`: remove Chat-specific transition confirmation handling.

### Frontend UI and presentation

- Modify `src/App.tsx`: remove aggregate creation, editor, branches, catalog bypasses, proxy warning, and protocol label choice.
- Modify `src/styles.css`: remove aggregate-only selectors.
- Modify `src/i18n-en.ts`: remove strings reachable only from Chat Completions, local proxy, or aggregate UI.
- Modify `src/app-shell-budget.test.ts`: remove deleted helper names and lower the line ceiling to the new `App.tsx` length.
- Modify or delete `src/provider-mode-presentation-wiring.test.ts`: assert the removed entry points are absent.

### Frontend focused tests

- Modify `src/provider-commit.test.ts`.
- Modify `src/provider-config-transform-router.test.ts`.
- Modify `src/provider-onboarding.test.ts`.
- Modify `src/provider-detail-draft-state.test.ts`.
- Modify `src/provider-detail-draft-wiring.test.ts`.
- Modify `src/provider-native-capability-view.test.ts`.
- Modify `src/provider-onboarding.test.ts`: assert protocol and aggregate sentinel state are absent while generated TOML remains Responses.

### Documentation

- Modify `README.md`: state Responses-only support and describe server-side composite providers as ordinary Responses upstreams.
- Modify `AGENTS.md`: replace the instruction to retain dead options with a Responses-only invariant.
- Modify `BOARD.md`: append the completed change and its verification evidence after all checks pass.

---

### Task 1: Add the fail-closed Responses-only Rust boundary

**Files:**

- Modify: `src-tauri/src/provider_commit.rs`
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/provider_commit.rs`
- Test: `src-tauri/src/provider_commit_transaction_tests.rs`

**Interfaces:**

- Consumes: `codex_plus_core::settings::{BackendSettings, RelayMode, RelayProtocol}`.
- Produces: `ResponsesOnlyProviderError`, `validate_responses_only_settings(settings: &BackendSettings) -> Result<(), ResponsesOnlyProviderError>`, `validate_responses_only_profile(profile: &RelayProfile) -> Result<(), ResponsesOnlyProviderError>`, and a read-only `validate_persisted_responses_only_settings_at(path: &Path) -> anyhow::Result<()>` command boundary.
- Invariant: success means no aggregate metadata, no active aggregate pointer, no aggregate profile, no Chat Completions profile, and no deleted proxy Base URL.

- [ ] **Step 1: Write failing unit tests for unsupported settings and profiles**

Add tests beside the existing provider commit validation tests:

```rust
#[test]
fn responses_only_settings_reject_removed_provider_shapes() {
    let valid = settings_with(vec![mixed_profile("relay-a", "OpenAI")], "relay-a");
    assert!(validate_responses_only_settings(&valid).is_ok());

    let mut chat = valid.clone();
    chat.relay_profiles[0].protocol = RelayProtocol::ChatCompletions;
    assert!(matches!(
        validate_responses_only_settings(&chat),
        Err(ResponsesOnlyProviderError::UnsupportedProtocol)
    ));

    let mut aggregate = valid.clone();
    aggregate.relay_profiles[0].relay_mode = RelayMode::Aggregate;
    assert!(matches!(
        validate_responses_only_settings(&aggregate),
        Err(ResponsesOnlyProviderError::LocalAggregate)
    ));

    let mut metadata = valid.clone();
    metadata.active_aggregate_relay_id = "aggregate-a".to_string();
    assert!(validate_responses_only_settings(&metadata).is_err());

    let mut proxy = valid;
    proxy.relay_profiles[0].base_url = "http://127.0.0.1:57321/v1".to_string();
    assert!(matches!(
        validate_responses_only_settings(&proxy),
        Err(ResponsesOnlyProviderError::RemovedProxy)
    ));
}
```

Add a transaction test that snapshots settings, live config, catalog state, and Context tables, submits one invalid profile, and asserts all bytes remain unchanged. Add a second test whose unsupported persisted profile also contains `authContents`; call the load boundary and assert the original settings bytes remain identical, proving legacy auth migration cannot run before Responses-only rejection.

- [ ] **Step 2: Run the focused Rust tests and verify failure**

Run:

```bash
cd src-tauri
cargo test responses_only_settings_reject_removed_provider_shapes
cargo test responses_only_rejection_writes_nothing
cargo test responses_only_load_rejection_precedes_auth_migration
```

Expected: compilation fails because the two validator functions do not exist.

- [ ] **Step 3: Implement the invariant in `provider_commit.rs`**

Add one constant, one typed error enum, and two validators near the commit request validation helpers. Implement `Display` with the user-facing messages and `std::error::Error`; tests assert variants rather than exact copy:

```rust
pub(crate) const REMOVED_PROTOCOL_PROXY_BASE_URL: &str = "http://127.0.0.1:57321/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponsesOnlyProviderError {
    UnsupportedProtocol,
    LocalAggregate,
    RemovedProxy,
}

pub(crate) fn validate_responses_only_profile(
    profile: &RelayProfile,
) -> Result<(), ResponsesOnlyProviderError> {
    if profile.relay_mode == RelayMode::Aggregate {
        return Err(ResponsesOnlyProviderError::LocalAggregate);
    }
    if profile.protocol != RelayProtocol::Responses {
        return Err(ResponsesOnlyProviderError::UnsupportedProtocol);
    }
    if profile.base_url.trim_end_matches('/')
        == REMOVED_PROTOCOL_PROXY_BASE_URL.trim_end_matches('/')
    {
        return Err(ResponsesOnlyProviderError::RemovedProxy);
    }
    Ok(())
}

pub(crate) fn validate_responses_only_settings(
    settings: &BackendSettings,
) -> Result<(), ResponsesOnlyProviderError> {
    if !settings.aggregate_relay_profiles.is_empty()
        || !settings.active_aggregate_relay_id.trim().is_empty()
    {
        return Err(ResponsesOnlyProviderError::LocalAggregate);
    }
    for profile in &settings.relay_profiles {
        validate_responses_only_profile(profile)?;
    }
    Ok(())
}
```

Call `validate_responses_only_settings` before normalization in all settings input boundaries:

- `settings_snapshot_for_ui_projection`;
- `save_settings_with_provider_guard_at_observed` for both persisted and incoming settings;
- `load_provider_commit_settings` immediately after JSON deserialization;
- the settings payload path before returning settings to the frontend.

Implement `validate_persisted_responses_only_settings_at` as a read-only file check: missing file succeeds; an existing file is read, deserialized as `BackendSettings`, and passed to `validate_responses_only_settings` without writing. Call it before `migrate_legacy_profile_auth_locked_at` in both settings load and provider commit flows. This ordering is mandatory: an unsupported old file must fail byte-for-byte before the unrelated ordinary-profile auth migration sees it.

Call `validate_responses_only_profile` inside provider commit request validation before catalog validation. Map errors through the existing `PersistedSettingsInvalid`／provider commit error paths; do not catch and convert them into a legacy repair.

- [ ] **Step 4: Run the focused Rust tests and verify pass**

Run:

```bash
cd src-tauri
cargo test responses_only_settings_reject_removed_provider_shapes
cargo test responses_only_rejection_writes_nothing
cargo test responses_only_load_rejection_precedes_auth_migration
cargo test --lib provider_commit
```

Expected: all selected tests pass; invalid inputs fail before mutation planning.

- [ ] **Step 5: Commit the boundary guard**

```bash
git add src-tauri/src/provider_commit.rs src-tauri/src/commands.rs src-tauri/src/provider_commit_transaction_tests.rs
git commit -m "fix: reject unsupported provider topologies"
```

---

### Task 2: Remove the aggregate provider product end to end

**Files:**

- Modify: `src/App.tsx`
- Modify: `src/relay-settings.ts`
- Modify: `src/backend-types.ts`
- Modify: `src/provider-commit.ts`
- Modify: `src/provider-commit.test.ts`
- Modify: `src/provider-onboarding.ts`
- Modify: `src/provider-onboarding.test.ts`
- Modify: `src/provider-detail-draft-state.test.ts`
- Modify: `src/provider-mode-presentation-wiring.test.ts`
- Modify: `src/app-shell-budget.test.ts`
- Modify: `src/styles.css`
- Create: `src/relay-settings.test.ts`
- Modify: `src-tauri/src/provider_commit.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/provider_commit_transaction_tests.rs`

**Interfaces:**

- Consumes: ordinary `RelayProfile[]` and `activeRelayId` only.
- Produces: frontend and Rust provider topology DTOs with no aggregate metadata; saved JSON with no `aggregateRelayProfiles` or `activeAggregateRelayId` keys.
- Preserves: add, edit, copy, delete, reorder, save, and switch behavior for ordinary providers.
- Adapter rule: upstream `BackendSettings.aggregate_relay_profiles` is always `Vec::new()` and `active_aggregate_relay_id` is always `String::new()` after applying a valid local topology.

- [ ] **Step 1: Replace aggregate-positive tests with absence and ordinary-provider tests**

In `src/provider-mode-presentation-wiring.test.ts`, delete the aggregate-editor source assertion. Do not replace it with another source grep; the Rust rejection tests, ordinary relay behavior tests, compile／knip checks, and final manual UI check cover the change at observable boundaries.

In `src/provider-commit.test.ts`:

- remove aggregate fixtures and `aggregateCleanup` cases;
- assert `request.topology` has exactly the keys `relayProfilesEnabled`, `relayProfiles`, `activeRelayId`, `relayBaseUrl`, `relayApiKey`, `relayCommonConfigContents`, `relayContextConfigContents`, and `relayTestModel`;
- keep literal projection, copy provenance, deletion, reorder, and stale-response tests.

Create `src/relay-settings.test.ts` with an ordinary relay-settings test proving deleting one provider never edits unrelated profiles and active fallback selects the first remaining profile:

```ts
import assert from "node:assert";
import { it } from "node:test";

import { defaultSettings, removeRelayProfile } from "./relay-settings.ts";

const ordinaryProfile = (id: string, baseUrl: string) => ({
  ...defaultSettings.relayProfiles[0],
  id,
  name: id,
  baseUrl,
  upstreamBaseUrl: baseUrl,
  apiKey: `key-${id}`,
  protocol: "responses" as const,
  relayMode: "official" as const,
  officialMixApiKey: true,
});

const settingsWithTwoProfiles = () => ({
  ...defaultSettings,
  relayProfiles: [
    ordinaryProfile("relay-a", "https://a.example/v1"),
    ordinaryProfile("relay-b", "https://b.example/v1"),
  ],
  activeRelayId: "relay-a",
  relayBaseUrl: "https://a.example/v1",
  relayApiKey: "key-relay-a",
});

it("removes one ordinary provider and selects the first remaining active profile", () => {
  const settings = settingsWithTwoProfiles();
  const untouched = settings.relayProfiles[1];
  const next = removeRelayProfile(settings, settings.activeRelayId);
  assert.deepEqual(next.relayProfiles, [untouched]);
  assert.equal(next.activeRelayId, untouched.id);
  assert.equal(next.relayBaseUrl, untouched.baseUrl);
});
```

- [ ] **Step 2: Run focused frontend tests and verify failure**

Run:

```bash
node --test --experimental-strip-types \
  src/provider-mode-presentation-wiring.test.ts \
  src/provider-commit.test.ts \
  src/relay-settings.test.ts
```

Expected: the aggregate button／editor still exists and topology still includes aggregate fields.

- [ ] **Step 3: Delete aggregate UI and shell wiring**

In `src/App.tsx`:

- remove aggregate imports from `relay-settings.ts` and aggregate types from `backend-types.ts`;
- delete `createNewAggregateProfile` and the「添加聚合供应商」button;
- delete `AggregateRelayProfileEditor`;
- remove every `isAggregateRelayProfile` branch from detail save, set-current, catalog selection, list labels, status text, and live-file panels;
- remove `aggregateCleanup` from callback unions;
- delete `aggregateStrategyLabels`, `aggregateStrategyOptions`, `aggregateStrategyLabel`, and `aggregateStrategyHelp`;
- make every profile use the ordinary editor and ordinary catalog path.

Delete aggregate-only CSS selectors from `src/styles.css`. Remove the two aggregate helper names from `LOGIC_STILL_IN_THE_SHELL` and lower the line ceiling to the post-edit `App.tsx` line count.

- [ ] **Step 4: Delete aggregate domain state and request projection**

In `src/backend-types.ts`, remove:

```ts
aggregateRelayProfiles: AggregateRelayProfile[];
activeAggregateRelayId: string;
aggregate?: RelayAggregateConfig | null;
```

Delete `RelayAggregateStrategy`, `RelayAggregateConfig`, `AggregateRelayProfile`, and their member types. Narrow `RelayMode` to:

```ts
export type RelayMode = "official" | "mixedApi" | "pureApi";
```

Remove `aggregate: null` from `createNewRelayProfileDraft` in `provider-onboarding.ts` and from its object-shape expectations in `provider-onboarding.test.ts`; ordinary drafts no longer carry a sentinel for a deleted product type.

In `src/relay-settings.ts`, delete aggregate constants and helpers, including `AGGREGATE_STRATEGIES`, `createAggregateRelayProfile`, `normalizeAggregateRelayProfile`, `normalizeAggregateConfig`, `aggregateMemberCandidates`, `clampAggregateWeight`, and `aggregateRelayProfileValidation`. Simplify `syncLegacyRelayFields` to derive every ordinary profile, set `relayBaseUrl = active.baseUrl`, and never synthesize aggregate metadata.

In `src/provider-commit.ts`, delete `ProviderAggregateDraft`, the two aggregate topology fields, and `aggregateCleanup`. Make `projectProviderOwnedTopology` emit only ordinary topology fields.

Update test fixtures in `provider-detail-draft-state.test.ts` and other TypeScript tests by deleting the two removed settings keys, not replacing them with empty arrays or strings.

- [ ] **Step 5: Write failing strict-schema and persistence tests**

Add tests that deserialize a valid Responses request without aggregate fields and reject a forged request that includes either removed field because `deny_unknown_fields` is active:

```rust
#[test]
fn provider_topology_schema_rejects_removed_aggregate_fields() {
    let settings = settings_with(
        vec![mixed_profile("relay-a", "official-a")],
        "relay-a",
    );
    let request = request_for(
        &settings,
        &settings,
        Some("relay-a"),
        vec![catalog_draft(
            "relay-a",
            CatalogMode::OfficialPlusCustom,
            CatalogOverlay::default(),
        )],
        ProviderCommitAction::Save,
    );
    let mut value = serde_json::to_value(request).unwrap();
    value["topology"]["aggregateRelayProfiles"] = json!([]);
    assert!(serde_json::from_value::<ProviderCommitRequest>(value).is_err());
}
```

Add a commands persistence test:

```rust
#[test]
fn responses_only_settings_json_omits_removed_aggregate_keys() {
    let profile = RelayProfile {
        id: "relay-a".to_string(),
        name: "Relay A".to_string(),
        base_url: "https://relay.example/v1".to_string(),
        upstream_base_url: "https://relay.example/v1".to_string(),
        protocol: RelayProtocol::Responses,
        relay_mode: RelayMode::Official,
        official_mix_api_key: true,
        ..RelayProfile::default()
    };
    let settings = BackendSettings {
        relay_profiles: vec![profile],
        active_relay_id: "relay-a".to_string(),
        aggregate_relay_profiles: Vec::new(),
        active_aggregate_relay_id: String::new(),
        ..BackendSettings::default()
    };
    let bytes = serialize_settings_without_profile_auth(&settings).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value.get("aggregateRelayProfiles").is_none());
    assert!(value.get("activeAggregateRelayId").is_none());
}
```

- [ ] **Step 6: Run the new Rust tests and verify failure**

Run:

```bash
cd src-tauri
cargo test provider_topology_schema_rejects_removed_aggregate_fields
cargo test responses_only_settings_json_omits_removed_aggregate_keys
```

Expected: the request still requires the fields and the serializer still emits them.

- [ ] **Step 7: Remove aggregate DTOs and validation from `provider_commit.rs`**

Delete imports for `AggregateRelayMember`, `AggregateRelayProfile`, and `AggregateRelayStrategy`. Delete `ProviderAggregateMemberDraft`, `ProviderAggregateDraft`, their conversions, and these fields:

```rust
pub aggregate_relay_profiles: Vec<ProviderAggregateDraft>,
pub active_aggregate_relay_id: String,
```

In `ProviderOwnedTopologyDraft::from_settings`, do not project upstream aggregate fields. In `apply_to`, explicitly establish the new invariant:

```rust
next.aggregate_relay_profiles.clear();
next.active_aggregate_relay_id.clear();
```

Delete aggregate ID／member／active-linkage validation from `validate_common_request`. Keep the Task 1 Responses-only profile validator before catalog checks.

- [ ] **Step 8: Simplify commands projection and persistence**

In `ui_provider_topology_projection`:

- remove aggregate normalization and member reconstruction;
- validate settings before normalization;
- set active `relay_base_url` directly from the active ordinary profile;
- clear the two unavoidable upstream aggregate fields before `ProviderOwnedTopologyDraft::from_settings`.

In `serialize_settings_without_profile_auth`, remove the two keys from the serde JSON object:

```rust
if let Some(object) = value.as_object_mut() {
    object.remove("aggregateRelayProfiles");
    object.remove("activeAggregateRelayId");
}
```

Remove the same keys from `serialize_settings_with_raw_provider_snapshot`'s preserved provider-key list so old values can never be copied into a new valid generation. This is not a migration: Task 1 rejects a loaded old generation before save.

- [ ] **Step 9: Delete aggregate transaction fixtures and run the complete task checks**

Delete tests whose asserted behavior is aggregate creation, projection, switching, or round-trip. Replace them with ordinary Responses reorder／copy／delete cases where they also protected CAS, owner-only files, Context, or rollback behavior.

Run:

```bash
npm run check
node --test --experimental-strip-types \
  src/provider-mode-presentation-wiring.test.ts \
  src/provider-commit.test.ts \
  src/relay-settings.test.ts \
  src/app-shell-budget.test.ts
cd src-tauri
cargo test --lib provider_commit
cargo test --lib provider_commit_transaction_tests
```

Expected: TypeScript compiles; the aggregate product surface is absent; the strict Rust schema and persistence omission tests pass; all ordinary provider transactions pass.

- [ ] **Step 10: Commit the vertical aggregate removal**

```bash
git add src/App.tsx src/relay-settings.ts src/backend-types.ts src/provider-commit.ts \
  src/relay-settings.test.ts src/provider-commit.test.ts src/provider-onboarding.ts \
  src/provider-onboarding.test.ts \
  src/provider-detail-draft-state.test.ts src/provider-mode-presentation-wiring.test.ts \
  src/app-shell-budget.test.ts src/styles.css src-tauri/src/provider_commit.rs \
  src-tauri/src/commands.rs src-tauri/src/provider_commit_transaction_tests.rs
git commit -m "feat: remove local aggregate providers"
```

---

### Task 3: Remove Chat Completions and proxy behavior end to end

**Files:**

- Modify: `src/backend-types.ts`
- Modify: `src/provider-config-transform-router.ts`
- Modify: `src/provider-config-transform-router.test.ts`
- Modify: `src/provider-native-capability-view.ts`
- Modify: `src/provider-native-capability-view.test.ts`
- Modify: `src/provider-detail-draft-state.ts`
- Modify: `src/provider-detail-draft-state.test.ts`
- Modify: `src/provider-detail-draft-wiring.test.ts`
- Modify: `src/provider-onboarding.ts`
- Modify: `src/provider-onboarding.test.ts`
- Modify: `src/relay-settings.ts`
- Modify: `src/App.tsx`
- Modify: `src-tauri/src/provider_commit.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/provider_native_capability.rs`
- Modify: `src-tauri/src/provider_commit_transaction_tests.rs`
- Modify: `src-tauri/tests/provider_native_capability.rs`
- Modify: `src-tauri/tests/provider_native_capability_draft.rs`

**Interfaces:**

- Consumes: one real Responses `baseUrl`.
- Produces: frontend profiles without proxy-only `upstreamBaseUrl`; commit profile DTOs that accept no protocol choice and construct upstream `RelayProtocol::Responses`; no `exitChatCompletions` action.
- Preserves: supported explicit native-priority, pure API, pure OAuth, and legacy-provider-ID transitions unrelated to Chat Completions.

- [ ] **Step 1: Write failing frontend absence and routing tests**

In `provider-config-transform-router.test.ts`, delete Chat exit cases and add:

```ts
it("patches the real Responses Base URL without a proxy indirection", () => {
  const routed = routeProviderConfigDraftEdit({
    profile: existingProfile(),
    patch: { baseUrl: "https://next.example/v1" },
    target: existingTarget,
  });
  assert.equal(routed.kind, "synchronous");
  if (routed.kind !== "synchronous") return;
  assert.equal(routed.profile.baseUrl, "https://next.example/v1");
  assert.match(routed.profile.configContents, /base_url = "https:\/\/next\.example\/v1"/);
  assert.doesNotMatch(routed.profile.configContents, /codex_plus_chat_base_url|127\.0\.0\.1:57321/);
});
```

In `provider-native-capability-view.test.ts`, assert supported mode patches still route correctly and remove all protocol-patch expectations. In `provider-detail-draft-state.test.ts`, remove `exitChatCompletions` confirmation／settlement tables while retaining concurrency, stale response, pure API／OAuth, actor conflict, and legacy provider ID cases.

- [ ] **Step 2: Run focused frontend tests and verify failure**

Run:

```bash
node --test --experimental-strip-types \
  src/provider-config-transform-router.test.ts \
  src/provider-native-capability-view.test.ts \
  src/provider-detail-draft-state.test.ts
```

Expected: proxy constants, protocol patch routing, and Chat transition behavior still exist.

- [ ] **Step 3: Collapse the frontend profile to one Responses URL**

In `src/backend-types.ts`:

- delete `upstreamBaseUrl` from `RelayProfile`;
- delete `protocol` from `RelayProfile` and delete the frontend `RelayProtocol` type; Responses is an invariant, not UI state.

In `src/provider-onboarding.ts`, remove `protocol` from the new draft object. Keep the generated TOML line `wire_api = "responses"` unchanged. Update `provider-onboarding.test.ts` to assert the draft has no own `protocol` property while the materialized TOML contains exactly one Responses wire line:

```ts
assert.equal(Object.hasOwn(draft, "protocol"), false);
assert.equal(materialized.configContents.match(/^wire_api = "responses"$/gm)?.length, 1);
```

In `src/relay-settings.ts`, stop hydrating or synchronizing `upstreamBaseUrl`; `deriveRelayProfileFromFiles` reads `base_url` directly into `baseUrl`; profile creation and duplication carry only the real URL.

In `src/provider-config-transform-router.ts`:

- delete `PROTOCOL_PROXY_BASE_URL` and `CHAT_UPSTREAM_BASE_URL_KEY`;
- remove `protocol` from `ProviderConfigProfile` and `BACKEND_TRANSFORM_FIELDS`;
- patch `base_url` directly from `next.baseUrl`;
- remove `exitChatCompletions` from `ProviderDraftTransformAction`;
- delete protocol branches from `providerTransitionDecisionForStructuredPatch`.

In `provider-native-capability-view.ts`, remove Chat／aggregate reasons from advanced presentation and delete protocol from `ProviderModeProtocol` and its transition comparison. In `provider-detail-draft-state.ts`, remove `exitChatCompletions` from capability-loss confirmation routing.

- [ ] **Step 4: Delete the remaining Chat UI branches**

In `src/App.tsx`:

- delete catalog bypasses based on `profile.protocol === "chatCompletions"`;
- delete the proxy warning block;
- remove the redundant protocol fragment from the list summary, leaving mode and configuration brief;
- delete `relayProtocolLabel` and remove it from the shell-budget allowlist;
- keep `MessageCircle` because the sessions navigation still uses it.

Delete the obsolete Chat-specific assertion from `provider-detail-draft-wiring.test.ts`. Do not replace it with a source grep; supported transition behavior remains covered in `provider-detail-draft-state.test.ts`.

- [ ] **Step 5: Remove protocol choice from the Rust provider commit DTO**

In `ProviderRelayProfileDraft`, delete `upstream_base_url` and `protocol`. In `From<&RelayProfile>`, validate via `validate_responses_only_profile` at the caller before projection and omit both fields. In `From<&ProviderRelayProfileDraft> for RelayProfile`, construct the upstream-only fields deterministically:

```rust
base_url: profile.base_url.clone(),
upstream_base_url: profile.base_url.clone(),
protocol: RelayProtocol::Responses,
```

Because `ProviderRelayProfileDraft` uses `deny_unknown_fields`, forged `protocol`, `upstreamBaseUrl`, or Chat Completions data in a new commit request is rejected during deserialization rather than converted.

In `serialize_settings_without_profile_auth`, remove `protocol` and `upstreamBaseUrl` from every serialized `relayProfiles` object together with `authContents`; the upstream `RelayProfile` defaults reconstruct Responses and an empty compatibility-only upstream URL when loading the new format. The canonical Base URL remains in `configContents` and in the commit DTO's structured `baseUrl` during edits. Extend `responses_only_settings_json_omits_removed_aggregate_keys` with:

```rust
let saved_profile = &value["relayProfiles"][0];
assert!(saved_profile.get("protocol").is_none());
assert!(saved_profile.get("upstreamBaseUrl").is_none());
```

In `provider_native_capability.rs`, delete:

- `NativeCapabilityReason::ChatCompletions` and `NativeCapabilityReason::Aggregate`;
- `NativeCapabilityDraftAction::ExitChatCompletions`;
- Chat／aggregate inspection branches and transformer arms;
- capability-loss confirmation conditions that exist only for the deleted action.

Keep Task 1's fail-closed validation at settings and commit boundaries. Remove the Chat transition cases from `provider_native_capability_draft.rs`; do not replace them with migration tests.

Remove Chat／aggregate presentation fixtures from `src-tauri/tests/provider_native_capability.rs`. In `provider_commit_transaction_tests.rs`, delete Chat-specific successful round trips and retain the Task 1 rejection-with-no-writes cases. In `commands.rs`, remove the last proxy／Chat projection branches and apply the per-profile persistence-key omission described above.

- [ ] **Step 6: Run frontend and Rust focused verification**

Run:

```bash
npm run check
node --test --experimental-strip-types \
  src/provider-config-transform-router.test.ts \
  src/provider-native-capability-view.test.ts \
  src/provider-detail-draft-state.test.ts \
  src/provider-detail-draft-wiring.test.ts
cd src-tauri
cargo test provider_native_capability
cargo test --lib provider_commit
```

Expected: ordinary Responses edits and supported native-capability transitions pass; no Chat transition remains.

- [ ] **Step 7: Commit the Chat／proxy removal**

```bash
git add src/backend-types.ts src/provider-config-transform-router.ts \
  src/provider-config-transform-router.test.ts src/provider-native-capability-view.ts \
  src/provider-native-capability-view.test.ts src/provider-detail-draft-state.ts \
  src/provider-detail-draft-state.test.ts src/provider-detail-draft-wiring.test.ts \
  src/provider-onboarding.ts src/provider-onboarding.test.ts src/relay-settings.ts \
  src/App.tsx src/app-shell-budget.test.ts \
  src-tauri/src/provider_commit.rs src-tauri/src/commands.rs \
  src-tauri/src/provider_native_capability.rs \
  src-tauri/src/provider_commit_transaction_tests.rs \
  src-tauri/tests/provider_native_capability.rs \
  src-tauri/tests/provider_native_capability_draft.rs
git commit -m "feat: make provider routing responses only"
```

---

### Task 4: Simplify model catalog eligibility to the accepted provider contract

**Files:**

- Modify: `src-tauri/src/model_catalog.rs`
- Modify: `src-tauri/src/provider_commit.rs`
- Modify: `src/provider-commit.ts`
- Modify: `src/provider-commit.test.ts`

**Interfaces:**

- Consumes: profiles already validated by `validate_responses_only_profile`.
- Produces: catalog planning with no Chat／aggregate non-capable branch; direct and server-side-composite topology remain valid for Responses profiles.
- Error rule: any unvalidated legacy profile passed directly to a public catalog helper must return an error, not `managedAvailable = false`.

- [ ] **Step 1: Replace catalog-incapable tests with fail-closed legacy tests**

In `model_catalog.rs` tests, replace assertions such as `!managed_catalog_capable(&aggregate)` with:

```rust
#[test]
fn catalog_rejects_profiles_outside_the_responses_only_contract() {
    let mut chat = mixed_profile("chat");
    chat.protocol = RelayProtocol::ChatCompletions;
    assert!(validate_upstream_topology(&chat, UpstreamTopology::Direct).is_err());

    let mut aggregate = mixed_profile("aggregate");
    aggregate.relay_mode = RelayMode::Aggregate;
    assert!(validate_upstream_topology(&aggregate, UpstreamTopology::Direct).is_err());
}
```

Retain and strengthen the existing server-side-composite test: a normal Responses profile with one Base URL and Key supports `UpstreamTopology::ServerSideComposite`.

- [ ] **Step 2: Run focused catalog tests and verify failure**

Run:

```bash
cd src-tauri
cargo test catalog_rejects_profiles_outside_the_responses_only_contract
cargo test server_side_composite -- --nocapture
```

Expected: the direct topology path still accepts or merely classifies one removed profile instead of rejecting it.

- [ ] **Step 3: Remove catalog compatibility branches**

In `model_catalog.rs`:

- make `validate_upstream_topology` call `provider_commit::validate_responses_only_profile(profile)` first;
- delete `RelayMode::Aggregate => CatalogMode::NativeOfficial` from default mode selection and make unsupported upstream enum values unreachable only after validation;
- delete branches that skip profile state for aggregate or Chat Completions;
- delete `managed_catalog_capable`; validate at fallible entry points and set `managed_available: true` only after the profile passes the Responses-only boundary;
- retain four catalog ownership modes and server-side-composite topology for accepted Responses profiles.

In `provider_commit.rs`, require one complete catalog draft for every new accepted profile using the existing pure OAuth/native mode rules; remove aggregate／Chat exemptions.

In `provider-commit.ts`, delete `managedCatalogCapable` if every frontend profile is now accepted by construction. Replace callers with the existing catalog availability state, not a protocol／mode predicate.

- [ ] **Step 4: Run catalog and provider commit tests**

Run:

```bash
cd src-tauri
cargo test --lib model_catalog
cargo test --lib provider_commit
cd ..
node --test --experimental-strip-types src/provider-commit.test.ts
```

Expected: catalog ownership, external adoption, official refresh, overlay, server-side composite, and ordinary commit tests pass without removed-path branches.

- [ ] **Step 5: Commit the catalog simplification**

```bash
git add src-tauri/src/model_catalog.rs src-tauri/src/provider_commit.rs \
  src/provider-commit.ts src/provider-commit.test.ts
git commit -m "refactor: simplify responses provider catalogs"
```

---

### Task 5: Delete removed copy, translations, and static identifiers

**Files:**

- Modify: `src/i18n-en.ts`
- Modify: `src/styles.css`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `src/app-shell-budget.test.ts`

**Interfaces:**

- Consumes: final product source after Tasks 1–4.
- Produces: current product copy and repository instructions that describe only Responses support.
- Verification: use behavior tests plus one final `rg` audit; do not commit source-string tests.

- [ ] **Step 1: Remove dead translations and CSS**

Delete every `i18n-en.ts` entry reachable only from:

- Chat Completions labels／warnings;
- local proxy descriptions;
- aggregate provider creation, editor, strategy, members, preview, save, and status messages.

Delete remaining aggregate-only CSS selectors. Run `rg` to ensure no source references remain before deleting each translation key; do not delete generic uses of「聚合」that describe server-side composite behavior in model catalog copy.

- [ ] **Step 2: Update current documentation and repository instructions**

Replace the README dead-path warning with a direct supported contract:

```markdown
- Codex Minus 仅支持 Responses 供应商。服务端复合供应商应对 Codex 暴露一个 Responses Base URL 和 Key，并作为普通供应商接入。
```

Also rewrite the Provider Doctor compatibility bullet so it describes the allowlisted Responses HTTP 400 retry without contrasting it with Chat Completions; current documentation must contain no suggestion that the removed protocol remains a product mode.

Replace the `AGENTS.md` hard constraint that says to keep dead options with:

```markdown
- **Responses-only provider contract**: Codex Minus does not expose or persist Chat Completions, the removed `127.0.0.1:57321` protocol proxy, or client-side aggregate providers. A server-side composite that exposes one Responses Base URL and key is an ordinary upstream. Do not add compatibility, migration, filtering, or fallback paths for the removed provider shapes.
```

Do not edit prior dated specs or plans to rewrite history. Current README, AGENTS.md, code, and tests define the live behavior.

- [ ] **Step 3: Run TypeScript, test, and unused-code checks**

Run:

```bash
npm run check
npm test
npm run knip
```

Expected: behavior tests pass, TypeScript compiles, and no deleted helper／translation import is left unused.

- [ ] **Step 4: Commit product-surface cleanup**

```bash
git add src/i18n-en.ts src/styles.css \
  README.md AGENTS.md src/app-shell-budget.test.ts
git commit -m "docs: declare responses-only provider support"
```

---

### Task 6: Run full verification and record completion

**Files:**

- Modify: `BOARD.md`
- Verify: all files changed in Tasks 1–5

**Interfaces:**

- Consumes: the complete Responses-only implementation.
- Produces: repository-level proof for TypeScript, frontend tests, unused code, frontend build, Rust unit／integration tests, static removed-path scans, and transaction invariants.

- [ ] **Step 1: Run the complete frontend verification**

Run:

```bash
npm run verify
npm run vite:build
```

Expected: TypeScript, every Node test, knip, and Vite production build pass.

- [ ] **Step 2: Run the complete Rust verification**

Run:

```bash
cd src-tauri
cargo test
```

Expected: every non-ignored Rust unit and integration test passes. The live OAuth test remains ignored unless explicitly enabled, matching repository policy.

- [ ] **Step 3: Run final source and persistence scans**

Run from the repository root:

```bash
rg -n "127\.0\.0\.1:57321|chatCompletions|exitChatCompletions|添加聚合供应商|AggregateRelayProfileEditor" \
  src src-tauri/src README.md AGENTS.md
rg -n "aggregateRelayProfiles|activeAggregateRelayId" src README.md AGENTS.md
git diff --check
```

Expected:

- the first scan returns only the intentional Rust fail-closed constant and rejection tests, never a success path or UI string;
- the second scan returns no frontend or current documentation matches; unavoidable upstream adapter references may remain only in Rust clearing／omission code and Task 1 rejection tests;
- `git diff --check` reports nothing.

- [ ] **Step 4: Inspect the built product surface**

Run the app in development mode or open the built app, then verify manually:

1. the provider list has only「添加供应商」;
2. a new provider has one Base URL, one Key, and Responses copy;
3. no protocol picker, proxy warning, aggregate editor, member list, or strategy selector appears;
4. a normal Responses provider saves, opens again, runs Provider Doctor, and can be set current;
5. a server-side composite is configured through the same ordinary form.

Record any visual verification gap explicitly if the app cannot be launched in the current environment.

- [ ] **Step 5: Append the completion entry to `BOARD.md`**

Append one changelog item containing:

```markdown
- **feat/providers**: made Codex Minus Responses-only by deleting Chat Completions, the removed local protocol proxy, and client-side aggregate provider product paths
  - breaking: old Chat Completions／local aggregate `settings.json` shapes are unsupported and receive no migration, filtering, fallback, or read-only compatibility
  - preserved: server-side composite providers remain ordinary Responses upstreams; Context transactions, OAuth ownership, catalog ownership, CAS, and rollback invariants are unchanged
  - verified: `npm run verify`, `npm run vite:build`, `cargo test`, static removed-path scans, and manual provider-list／detail checks
```

If manual verification was not possible, replace the last clause with the exact unverified surface; do not claim it passed.

- [ ] **Step 6: Commit verification evidence**

```bash
git add BOARD.md
git commit -m "docs: record responses-only provider release"
```

- [ ] **Step 7: Review the final diff and commit sequence**

Run:

```bash
git status --short
git log --oneline -7
git diff HEAD~6..HEAD --stat
```

Expected: the worktree is clean; the six task commits are visible; the diff contains no unrelated user changes.
