# Tasks — simplify-into-codex-minus

## 1. Save liveness

- [x] 1.1 Bound every subprocess wait in the save/status path (`icacls` pair in `live_state.rs`, any remaining shell-outs) with the `run_bounded_command` pattern; a timeout fails the transaction typed and rolls back the generation.
- [x] 1.2 Replace the `spawn_blocking` panic `.expect` in command wrappers with typed `transactionFailed` mapping; add a panic-injection test proving the IPC reply settles with a typed failure and the next save is not poisoned into a hang.
- [x] 1.3 Move the pre-`try` draft derivation inside the guarded block so `saving`/`savingRef` always reset through `finally`.
- [x] 1.4 Report save success when `commit_provider_detail` returns; run the three post-commit refreshes after the pending state clears, surfacing their failures as a passive notice; test that a slow catalog status cannot hold the 保存中 state.
- [x] 1.5 Add a timeout to the relay HTTP client used by 获取模型列表/测试/体检 and move their blocking evidence writes off the async worker; test the timeout path returns a plain-language failure.

## 2. Canonical identifier and single generator

- [x] 2.1 Add a pinned-core round-trip test proving identifier `OpenAI` survives normalization, storage sanitize, and staging unchanged, and that reserved lowercase `openai` remains rejected.
- [x] 2.2 Default new drafts to provider identifier `OpenAI`; assert upgrades never rewrite an existing non-reserved identifier (extend the golden and transform tests, including the fleet shape `[model_providers.OpenAI]` with `requires_openai_auth = true` upgrading in place).
- [ ] 2.3 (after 4.1, which makes the alternate targets unreachable for a new draft) Retire `src/provider-config-draft.ts` and the four-target picker; one generator materializes the contract; update every import and test that referenced the retired path.

## 3. Bundled official baseline

- [x] 3.1 Author the bundled baseline JSON (slug, corrected display name, context window, effective percent, reasoning levels, visibility) from a verified official output plus the fleet overlay `gpt-5.6-sol`; record the source client version inside the asset; review display names with the user.
- [x] 3.2 Load the bundled asset as the official baseline: composition, readiness, activation-scope, and commit planning re-anchor to it; account-scope staleness for the baseline is removed; the stored runtime `state.official` is migrated or ignored without erroring.
- [ ] 3.3 Remove the runtime refresh machinery: isolated CODEX_HOME projection, access-token handling, publisher-signature shell-out, `refresh_official_model_catalog` command, and the 官方模型目录 band; keep `/v1/models` provider evidence collection working.
- [ ] 3.4 Remove the Manager network-policy feature end to end (module, three commands, panel, `network-policy-ui.ts`, CSS `styles.css:1061-1168`, i18n block, tests); saved policy state is left on disk untouched and unread.
- [ ] 3.5 Re-anchor the "active default model disappears" continuity behavior to bundled-baseline updates with a regression test.
- [ ] 3.6 Reconcile the in-flight `add-manager-network-policy` change before archive of this one (close or annotate its remaining task; its spec delta must not land a panel this change removes).
- [x] 3.7 Promote a native profile to `official-plus-custom` when a per-model context override is supplied, generate the catalog, point live config at it, and state the restart before the save.
- [x] 3.8 List only the models the profile's catalog makes selectable (the editor listed every baseline entry, including the ones Codex hides), give every row a removal that records a hidden override, offer removed and provider-reported slugs back without duplicating a row the list already carries, and refuse an empty list in the editor.
- [x] 3.9 Add a one-action restore of the shipped Pro list that names what it would remove, keeps context overrides, and moves the startup model into the restored list.

## 3b. Save legibility (found in use)

- [x] 3b.1 Keep a failed settings read out of the compare-and-swap baseline and out of the form; report its own reason when it happens.
- [x] 3b.2 Replace the internal missing-fingerprint throw with a re-read and a plain-language outcome that never commits placeholder settings.
- [x] 3b.3 Settle `load_settings`/`save_settings` through `settle_blocking` so a panic answers the caller.
- [x] 3b.4 Give each bounded helper capture its own temp files (Windows clock granularity + shared pid collided, so one helper deleted another's output).
- [x] 3b.5 Normalize a backend-returned profile at the editor boundary; the wire omits empty strings, so `.trim()` on an absent field crashed every save on such a profile.
- [ ] 3b.6 Root-cause the reported in-session `staleState` on a second consecutive save. Backend two-save round-trip is ruled out by test in both native and managed modes; the invariant test is a guard, not a fix.

## 4. Foolproof editor

- [x] 4.1 Reduce the provider editor to 名称 / 配置模型 / Base URL / API Key / Provider Doctor; remove preset selector, 接入模式, 混入 checkbox, 上游协议, User-Agent, context-size fields; every save materializes the canonical contract; legacy profiles upgrade on save.
- [ ] 4.2 Remove the raw provider-TOML editor; the bearer never renders in plaintext anywhere (test sweeps UI state and DOM copy paths with a sentinel key).
- [x] 4.3 Reduce the catalog section to one model table (mode control, topology, external-adopt removed); official and custom models share one list; the startup model is a row selection, not a free-text field; the global and per-profile test models are retired; `model-windows.ts` and its row plumbing are deleted; existing external profiles keep their pointer behavior untouched.
- [ ] 4.4 Render every typed failure as one plain-language sentence with the code and raw detail behind 详情; no path ends in a spinner or bare code; snapshot tests cover the common failures (stale, staging-rejected, contract-gap, timeout).

## 5. Capability-evidence removal

- [x] 5.1 Remove the evidence panel and ownership section (App.tsx 3751-3861), their state/refresh plumbing, label tables, `inspect_provider_capability_evidence`, `provider_capability_evidence.rs`, and registration; delete `provider-capability-ledger.ts` / `provider-doctor-evidence.ts` and their four test files; update `provider-command-registration.test.ts` and `provider-capability-claims.test.ts` (keep the no-overclaim sweep, keep i18n line 26 usage consistent).

## 6. Rename to Codex Minus (ownership confirmed 2026-08-12: this workstream)

- [x] 6.1 `productName` "Codex Minus" + `mainBinaryName: "codex-minus"`; update window titles, brand text, user-facing strings, doc comments; verify the Windows single-instance check matches the pinned binary name.
- [x] 6.2 Update `.github/workflows/build.yml` artifact paths/names (including lines 70/81) and release naming; keep BOARD.md history, identifier, settings dir, and on-disk slugs unchanged.
- [x] 6.3 Update README/README.en/AGENTS naming and rewrite the AGENTS baseline-ownership constraint per D4.

## 7. Verification and handoff

- [ ] 7.1 Full suites green (Rust lib + integration, TypeScript, `cargo fmt --check`, `npm run check`, Vite build, macOS bundle build; Windows x64 cross-build).
- [ ] 7.2 On-screen verification of the seven-step flow on macOS and in the Windows VM: fresh provider with only URL+key, save, activate, restart guidance, `auth.json` hash equality; a forced backend stall (injected) ends in a plain-language failure, never a stuck 保存中.
- [ ] 7.3 Strict OpenSpec validation; reconcile every scenario; BOARD entry; archive.
