# Implementation Evidence

## Integration baseline

- Intended integration branch: `master` at `cea0c86db4e8f80126ae8f412ea7dbf643007973`.
- Raw JSON model-catalog and provider/live ownership prerequisite: `codex/raw-json-model-catalog` at `60087e2f131113cbefd4c16ffede34796e80f0cc`, merged by `3fc4cc8`.
- Provider-onboarding prerequisite: source commit `1843af7d70a9e1911a6e0f2cddf114b30e534262`, semantically transplanted as `b5e2443` after preserving the raw-JSON `providerConfigDraft` ownership boundary.
- Unified provider-detail Save design baseline: `8b61954cb2bd87c9073fb8fbebe12c5d07ed6820`; its implementation is owned by this change rather than treated as a landed dependency.
- Isolated implementation worktree: `/Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/native-capability-priority` on `codex/native-capability-priority`.

## Verification record

### Integrated prerequisite baseline

- `npm test`: 35 passed, 0 failed.
- `npm run check`: passed.
- `npm run vite:build`: passed; the only emitted warning is Node's upstream `DEP0205` deprecation notice.
- `cargo build`: passed after the required frontend `dist/` was generated.
- `cargo test`: 67 passed, 0 failed, 1 intentionally ignored live-OAuth catalog test.
- Source audit found no native-capability dependency on a standard-Pro artifact, signed-update channel, or 372k readiness policy. The existing `models_372k.json` string is only an external-catalog filename fixture.
- `npm install` reported four pre-existing dependency audit findings (one low, three high); no dependency-changing `npm audit fix` was run.

### Apply-time architecture correction

- Backend call-site audit found that hardening generic `save_settings` without a replacement would break provider enablement, reorder, copy, delete, aggregate cleanup, and provider test-model mutations.
- The change therefore uses one provider-owned canonical topology envelope with an expected provider-state fingerprint. Thin detail/topology commands share one planner and transaction engine; frontend revision is correlation metadata only.
- This correction closes an implementation gap in the original artifacts without changing OAuth, Context, external-catalog, or native-capability contracts.

Focused, final full-suite, bundle, manual-flow, and scenario reconciliation evidence is appended here as the corresponding task groups complete.

### Provider-routable capability matrix (redacted)

Row scope is the point of this table. A complete native-capability contract is evidence about the
contract, not about any row below. Each row is `success`, `denial`, `fallback`, or `unknown` on its
own observation; no row may be inferred from another, and none may be inferred from the contract,
the actor marker, or the local plan.

Recorded from this change's automated verification only. No quota-bearing probe was run: task 8.10
requires separate explicit approval, and without it every row that needs a paid call stays
`unknown` rather than being reported from a text probe.

| Row | Outcome | Basis |
| --- | --- | --- |
| Text Responses | unknown | Modeled as `TextResponsesEvidence`; never derived, only observed. No request was issued. |
| Model discovery | unknown (`missingMetadata`) | A materialized catalog artifact proves an artifact exists, not that the selected model carries capability metadata. |
| Image generation | unknown | Modeled as `ImageGenerationEvidence` plus `ImagePlanEvidence`; both stay `unknown` without a verified target policy. Permission, `gpt-image-2` allowance, and tool registration are unverified (8.8). |
| Image editing | unknown | Not modeled by this build. Nothing in any payload can report it, so it cannot be claimed. |
| Remote compaction | unknown | Not modeled by this build. Same guarantee by construction. |
| Web search | unknown | Not modeled by this build. Same guarantee by construction. |

Redaction: no provider key, OAuth token, account or workspace identity, or endpoint value appears
in any row or in the evidence payload these rows are derived from (verified by the sentinel sweep
in 8.6).

Automated proof: `capability_rows_are_scoped_and_never_inferred_from_a_complete_contract`
(`src-tauri/tests/provider_capability_evidence.rs`) holds every row at `unknown` while the contract
is `ready`, the actor marker is `eligible`, and the route is `nativePriorityMixed`.
`src/provider-capability-claims.test.ts` proves no shipped copy claims a subscription upgrade or a
blanket grant of Pro capabilities.

### Final verification record

- `npm test`: 151 passed, 0 failed (30 suites).
- `npm run check`: passed.
- `npm run vite:build`: passed; only the pre-existing 500 kB chunk-size advisory.
- `cargo test` (`src-tauri/`): 162 lib passed, 45 integration passed across four targets, 1 intentionally ignored live-OAuth catalog test, 0 failed.
- `cargo fmt --check`: clean. `cargo clippy --all-targets`: 45 pre-existing advisories, none in code this change added.
- `npm run build`: the complete macOS app bundle built and ad-hoc signed at
  `src-tauri/target/release/bundle/macos/Codex-- Manager.app`.
  **Environment-only gap**: notarization was skipped because no `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID`
  or API-key credentials are present, and no Developer ID identity is configured; the bundle is signed
  with the ad-hoc identity `-`. Nothing was installed over the production `/Applications` copy.

### Scenario reconciliation (`provider-native-capability-mode`, 96 scenarios)

Every scenario is recorded below with what verifies it. `R:` is a Rust test, `T:` a TypeScript test,
`M:` a manual flow that belongs to the user's machine, `U:` explicitly unverified with the reason.
No scenario is left without a record.

**Derived native-capability-priority state (1–10)** — R: `classifies_the_binding_fixture_matrix_from_profile_catalog_and_toml`
covers 4–10 as a binding fixture matrix (complete contract, eligible legacy mixed, external mixed,
partial, pure OAuth, external pure OAuth, advanced compatibility). 1–3 are the new-provider path:
T: `creates only the official auth mixed Responses draft`, `materializes the exact native-priority
TOML only when every new-provider input is complete`, `reports each required first-save field`.

**Canonical actor-authorized provider contract (11–20)** — R: `golden_active_commit_stages_the_exact_actor_authorized_contract`
(11), `enabling_edits_only_owned_fields_and_preserves_nonlegacy_identity_evidence_and_comments`
(12, 15), `legacy_alias_moves_the_complete_table_only_to_an_available_or_identical_target` and
`different_custom_table_blocks_legacy_migration_until_an_unused_id_is_chosen` (13, 14),
`actor_header_uses_http_lookup_but_requires_one_exact_canonical_entry` and
`actor_header_conflict_and_semantic_duplicates_require_safe_explicit_resolution` (16, 17),
`commit_boundary_rejects_missing_reserved_ambiguous_malformed_and_structural_catalog_inputs` (18, 20),
`structured_key_bearer_mismatch_blocks_until_one_explicit_sync_direction_is_chosen` (19).

**Stable ownership across structured edits (21–29)** — R: `raw_edit_backend_adopts_only_parsed_non_contract_changes`
and `raw_edit_backend_blocks_malformed_and_owned_contract_changes_without_secret_errors` (21–23, 27),
`protocol_change_to_chat_completions_is_an_explicit_capability_loss_exit` (24),
`golden_active_commit_stages_the_exact_actor_authorized_contract` (25),
`successful_active_commit_preserves_context_semantics_and_auth_bytes` (26),
`topology_adapter_rejects_active_detail_and_catalog_bypasses_without_mutation` (28),
`generic_settings_save_allows_unrelated_changes_but_rejects_every_provider_owned_difference` (29).

**Explicit and atomic upgrade lifecycle (30–44)** — R: `first_and_later_inactive_save_commit_provider_and_catalog_without_live_side_effects`
(30, 31, 37), `inactive_save_without_catalog_readiness_persists_one_action_required_state` (32),
`injected_settings_persistence_failure_rolls_back_the_complete_prior_generation` and its sibling
injection tests (33, 43), `provider_commit_rejects_a_concurrent_raw_settings_generation_change` (34),
`set_current_commits_settings_catalog_pointer_activation_and_restart_together` (38),
`active_native_commit_requires_current_official_auth_and_catalog_scope` (39, 40, 42),
`golden_startup_and_inspection_never_rewrite_an_existing_contract` (44). T: `starts an explicit
upgrade as a revisioned draft transform and never as a commit` (35), `requires an explicit preview
confirmation before compatibility exits can be committed` (36), `keeps a signed-in Free plan
descriptive while actor eligibility and runtime proof stay independent` (41).

**Existing catalog contract remains authoritative (45–51)** — R: `managed_catalog_readiness_distinguishes_missing_scope_invalid_and_default_model`
(45, 47, 48), `explicit_pure_oauth_exit_deletes_the_provider_without_leaving_a_dormant_copy` (46),
`later_valid_detail_commit_clears_catalog_readiness_action_and_allows_activation_retry` (49),
`an_implicit_mode_follows_the_current_default_instead_of_deadlocking_the_profile` and
`an_explicit_mode_is_never_re_derived` (50),
`unified_provider_commit_rejects_unreviewed_external_to_managed_transition` (51).

**Evidence-based native capability status (52–59)** — R: `capability_rows_are_scoped_and_never_inferred_from_a_complete_contract`
(52, 56, 57), `trusted_read_only_evidence_is_redacted_and_never_invents_target_policy` (54),
`provider_doctor_output_redacts_provider_oauth_identity_and_endpoint_sentinels` (58, 59).
T: `blocks Free image only for one verified affected target path` (53), `keeps OAuth session and
local plan on independent activation axes` (55).

**Row-scoped provider-routable capability acceptance (60–65)** — U: no row was observed. The
redacted matrix above records every row as `unknown`. Row scoping itself is verified by R:
`capability_rows_are_scoped_and_never_inferred_from_a_complete_contract`, which holds all rows
unknown while the contract is ready; rows this build does not model at all cannot be reported by
any payload. Observing 60–65 requires quota-bearing probes, which task 8.10 gates behind separate
explicit approval that was not requested or given in this run.

**Restart and new-task activation guidance (66–70)** — R: `active_commit_refires_restart_when_the_runtime_contract_changes_without_a_catalog_generation`
(66), `active_commit_binds_restart_to_the_runtime_fingerprint_without_a_catalog_generation` (70),
`inactive_save_records_no_restart_marker_and_no_applied_runtime_fingerprint` (68),
`an_unobserved_runtime_stays_unknown_instead_of_being_reported_as_adopted` (69). T: `restart
guidance` and `says nothing until a committed generation requires a restart` (67).

**OAuth and provider-secret isolation (71–77)** — R: `sentinel_credentials_never_reach_artifacts_payloads_errors_or_logs`
(71, 72, 74), `concurrent_official_auth_update_is_preserved_while_manager_targets_roll_back` and
`auth_update_after_scope_gate_cannot_become_the_commit_baseline` (73),
`real_transaction_keeps_secret_stages_private_and_cleans_faulted_recovery_material` (75),
`direct_command_rejects_auth_contents_and_never_echoes_them` and `raw_auth_save_is_rejected` (76),
`normalization_projects_api_key_only_legacy_copy_to_provider_config`,
`load_time_legacy_migration_rejects_mixed_oauth_payload_without_writing`, and
`golden_startup_and_inspection_never_rewrite_an_existing_contract` (77).

**Explicit exit and compatibility behavior (78–82)** — R: `pure_api_and_legacy_exits_preserve_unowned_provider_content`
and `target_switching_changes_the_contract_only_on_commit_and_keeps_unowned_content` (78, 80),
`explicit_pure_oauth_exit_deletes_the_provider_without_leaving_a_dormant_copy` and
`pure_oauth_requires_destructive_confirmation_then_removes_the_complete_selected_table` (79),
`every_compatibility_exit_rejects_a_non_string_semantic_actor_header` (81, 82).

**Provider drafts remain repairable in steps (83–85)** — R: `a_legacy_custom_profile_reaches_the_canonical_contract_at_the_real_entry_points`
and `a_degraded_contract_saves_instead_of_locking_the_profile_out_of_its_own_repair` (83),
`commit_boundary_rejects_missing_reserved_ambiguous_malformed_and_structural_catalog_inputs` (84),
`upgrade_is_withheld_when_a_gap_the_transform_cannot_fill_remains` (85).

**Typed commit failures name their rejecting rule (86–88)** — R: `provider_commit_failures_are_typed_and_failure_payloads_are_secret_free`
and `injected_staging_failure_is_typed_as_staging_rejected_and_mutates_nothing` (86),
`a_contract_gap_names_the_field_it_is_missing` and
`an_incomplete_native_contract_reports_its_own_reason_rather_than_the_generic_fallback` (87),
`sentinel_credentials_never_reach_artifacts_payloads_errors_or_logs` (88).

**Built-in Pro model list (89–90)** — T: `prefills a new provider with the Pro list and a default
that the official catalog carries` and `ships no slug the official bundled catalog hides` (89).
For 90, R: `a_legacy_custom_profile_reaches_the_canonical_contract_at_the_real_entry_points` commits
a brand-new default through catalog planning, R:
`a_custom_only_catalog_keeps_a_model_whose_slug_the_official_baseline_also_carries` covers the case
where a declared default was previously discarded, and T: `records the retired slugs it guards
against` keeps a hidden slug from being shipped as the default.

**Provider routing switch (91)** — R: `a_disabled_routing_switch_saves_the_draft_without_writing_live_config`.

**A contract changes only for the profile the user is editing (92–94)** — R:
`golden_startup_and_inspection_never_rewrite_an_existing_contract` (92, 93),
`golden_commit_never_migrates_a_bystander_profile_contract` (94).

**Compare-and-swap accepts the generation the editor was shown (95)** — R:
`golden_commit_never_migrates_a_bystander_profile_contract`, which only reaches its subject after
the non-canonical bystander stops making every save stale.

**A confirmed pure OAuth exit leaves no dormant provider copy (96)** — R:
`explicit_pure_oauth_exit_deletes_the_provider_without_leaving_a_dormant_copy`.

**Manual flows still owed (not scenarios above, tasks 8.7 and 8.8)** — M: the on-screen create →
preview → inactive save → activate → restart-guidance → `auth.json` hash-equality flow, and the
image-generation permission, `gpt-image-2` allowance, and runtime tool-registration observations,
which remain unverified until independently observed on the user's machine.
