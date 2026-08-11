## 1. Catalog State And Topology

- [x] 1.1 Add a backward-compatible `upstreamTopology` enum to per-profile catalog state and status payloads, default existing profiles to `direct`, and bump the state schema without persisting credentials or raw model capability payloads in diagnostics.
- [x] 1.2 Validate `server-side-composite` only for `PureApi` + Responses profiles with a usable Base URL and provider bearer token, while leaving the live provider projection identical to the existing single-upstream pure API path.
- [x] 1.3 Update implicit catalog-mode resolution so newly classified composite profiles default to `official-plus-custom`, explicit modes survive reclassification, and direct pure API profiles retain their `custom-only` default.
- [x] 1.4 Add backend tests proving classification is never inferred, reclassification preserves routing and explicit modes, and both `RelayMode::Aggregate` and Chat Completions remain blocked from managed catalogs and live application.

## 2. Rich Model Metadata

- [x] 2.1 Extend official and custom overlay schemas with display-name, effective-context-percentage, reasoning-level/default, and structured tool-capability fields using tolerant deserialization defaults.
- [x] 2.2 Add validation for non-empty display names, 1-100 effective percentages, unique target-shaped reasoning efforts, defaults contained in supported efforts, and target-shaped tool metadata.
- [x] 2.3 Update custom composition to strip official-backend-only fields first and graft back only explicitly configured reasoning and tool capabilities, retaining conservative empty defaults otherwise.
- [x] 2.4 Update official override application so context changes preserve the baseline effective percentage unless explicitly overridden and all non-overridden official metadata remains byte-equivalent.
- [x] 2.5 Update external-overlay extraction and custom-to-official promotion to round-trip each compatible rich field exactly once without duplicating a promoted slug.
- [x] 2.6 Add representative Claude-style fixtures and Rust tests for two- and four-level reasoning sets, picker display-name overrides, 95-percent effective windows, explicit tool capabilities, invalid metadata, promotion, and preservation of unrelated official fields.

## 3. Managed Context Precedence

- [x] 3.1 Detect `model_context_window` and `model_auto_compact_token_limit` in saved profile TOML and staged live config whenever `official-plus-custom` or `custom-only` is selected, and expose structured conflict keys in catalog status.
- [x] 3.2 Add an explicit cleanup-confirmation field to save, switch, and repair requests so an unconfirmed conflict returns a non-mutating action-required result and cannot be reported current.
- [x] 3.3 On confirmation, remove both global keys from the saved profile and staged live root inside the existing coordinator, Context transaction, and multi-file journal before committing the managed catalog pointer and generation.
- [x] 3.4 Preserve global context values for `native-official` and unadopted `external` profiles, and prevent stale frontend or raw save paths from reintroducing them into managed profiles.
- [x] 3.5 Add transaction tests for inactive saves, active apply, user decline, context-table preservation, unrelated root settings, injected failure at every commit boundary, full rollback, crash recovery, and byte-for-byte unchanged live auth.

## 4. External Adoption Compatibility

- [x] 4.1 Extend adoption preview with external declared version, verified target version, match/mismatch/unknown status, source content hash, and credential-free target-CLI offline validation results.
- [x] 4.2 Require explicit mismatch acceptance and the previewed source hash on commit, then re-read and revalidate the external file inside the transaction to reject hash drift, structural failure, target projection changes, or collisions.
- [x] 4.3 Keep exact major/minor/patch validation on official refresh and add tests proving external mismatch and missing-version catalogs can be adopted after review while official mismatches and target-incompatible external catalogs remain rejected.
- [x] 4.4 Verify every adoption failure and declined warning leaves the external pointer and file byte-for-byte unchanged and emits only redacted version, hash, count, and status diagnostics.

## 5. Frontend Experience

- [x] 5.1 Add direct versus server-side-composite classification controls for compatible API profiles, show the resulting catalog default, and relabel the legacy Aggregate editor as unsupported local aggregation without enabling its apply path.
- [x] 5.2 Extend official and custom catalog rows with compact display-name, effective-percentage, reasoning-level/default, and structured tool controls using stable responsive dimensions and accessible validation states.
- [x] 5.3 Disable profile-wide context and compaction inputs in managed modes, show detected legacy values and their runtime effect, and require explicit confirmation before invoking transactional cleanup.
- [x] 5.4 Show external catalog version match/mismatch/unknown evidence in adoption preview and require a separate acceptance action for mismatches.
- [x] 5.5 Update English translations and frontend tests for topology classification, implicit versus explicit modes, rich metadata validation, context cleanup decline/confirm flows, external warnings, and preserved local-proxy lockout.

## 6. Documentation And Verification

- [x] 6.1 Update README and provider help to document server-side composition as one Responses upstream, per-model context precedence, rich overlay semantics, external version warnings, and the continued absence of local aggregation and Chat Completions proxy support.
- [x] 6.2 After implementation verification passes, narrow the project hard constraint from generic “aggregator providers” to “client-side proxy aggregation” and append the completed change and evidence to `BOARD.md` without altering unrelated active work.
- [x] 6.3 Run `npm run check`, `npm run vite:build`, and focused frontend tests; resolve all type, build, and behavior regressions.
- [x] 6.4 Run `cargo test` in `src-tauri` with the topology, overlay, adoption, Context, journal, auth-ownership, and model-catalog suites enabled.
- [x] 6.5 Run `npm run build` and verify the packaged application still stages one pure API provider, one manager-owned catalog pointer, and no live-auth writes for a composite fixture.
- [ ] 6.6 With explicit live-switch approval, convert or create a disposable sub2api composite profile, materialize mixed OpenAI/Claude metadata, verify picker labels, reasoning choices, and effective windows in the target Codex app, then restore the prior provider generation and record any remaining provider-runtime limitations.
