# Design — simplify-into-codex-minus

Evidence sources: three read-only scouting reports over this worktree (rename surface, stuck-save trace, panel/catalog mapping), the user's live Mac state (`~/.codex-session-delete/model-catalog-state.json`, live `config.toml`), and the pinned core source at rev `59a2f90`.

## D1. Canonical provider identifier: preserve on upgrade, `OpenAI` for new drafts

The user's real fleet stores `[model_providers.OpenAI]` with `model_provider = "OpenAI"` (observed live on macOS and Windows). Codex sessions reference that identifier; changing it triggers session-repair prompts. The pinned core reserves lowercase `openai` (`relay_config.rs:14-16`) and upstream Codex ignores user tables that collide with built-in identifiers, so the built-in identifier can never carry the mixed contract.

Decision: the existing spec scenario "Existing provider identifier is upgraded → preserves its provider identifier" is promoted to a hard rule restated in the requirement text, and new drafts default to identifier `OpenAI` instead of `custom` (`src/provider-onboarding.ts:105-115` currently emits `model_provider = "custom"`). `OpenAI` is not reserved (case-sensitive match) and not a legacy alias, so it round-trips the pinned core unchanged; a pinned-core round-trip test must prove this before the default flips. The retired secondary generator `src/provider-config-draft.ts:60-71` (emits `name = "custom"`, conditional `requires_openai_auth`) is deleted with its target picker (`App.tsx:5379-5399`).

## D2. Save liveness — six observed mechanisms, one contract

The stuck-save trace ranked these mechanisms; each gets a specific fix, and the contract is "a save always settles":

1. Unbounded `icacls` under the process-wide lock (`live_state.rs:675-678, 685-687`, ~64 spawns/save) → bounded wait via the `run_bounded_command` pattern (`model_catalog.rs:2046-2066`, 10s); on timeout the transaction fails typed and rolls back.
2. Unbounded `Get-AuthenticodeSignature` (`model_catalog.rs:1738-1747`) → deleted outright with the runtime refresh machinery (D4); the two bounded target-CLI calls around it go with it.
3. Panic in the blocking closure: `.await.expect("blocking command panicked")` (`commands.rs:2395-2396`) drops the Tauri IPC responder, so the frontend promise never settles → replace with a match that maps a `JoinError` (panic) to a typed `transactionFailed` payload. A panic-injection test must observe a settled, typed reply.
4. Post-commit refresh chain holds the spinner (`App.tsx:1526-1530`, `refreshModelCatalog` re-takes the lock) → success reports when `commit_provider_detail` returns; the three refreshes run after the button resets, failures surfacing as a passive notice, not a save failure.
5. Pre-`try` draft derivation (`App.tsx:3648-3653`) can strand `saving`/`savingRef` → move inside the `try`; the `finally` reset stays the single exit.
6. No-timeout HTTP + blocking work on async workers: `proxied_client` has no timeout and `fetch_relay_profile_models` calls blocking evidence writes on a tokio worker (`commands.rs:4277, 4289`) → add a request timeout and route through `spawn_blocking`.

A frontend watchdog is deliberately NOT added: with every backend wait bounded and panics typed, the promise always settles; a watchdog would mask regressions.

## D3. Plain-language failures

Typed failures (`StaleState`, `StagingRejected`, `catalogUnavailable`, contract-gap names, ...) keep their payloads but the default rendering becomes one sentence in the user's language stating what happened and the next action ("上游没有确认这份配置，请重试一次" style). The code, rejecting rule, and raw detail move behind a 详情 affordance. No error path may end with a spinner or a bare code.

## D4. Bundled official baseline replaces runtime refresh

There is no bundled JSON today; the baseline comes from shelling out to the installed CLI (`run_isolated_refresh`, `model_catalog.rs:1883-1952`) and the current cache carries alpha-CLI display names the user rejects (`GPT-5.6-Terra`, verbatim from `clientVersion 0.147.0-alpha.6.5`). Missing/stale baseline strands `official-plus-custom` commits (`compose_profile_catalog:2339-2344`, `provider_commit.rs:846-849`).

Decision (user-confirmed 2026-08-12): ship the baseline inside the app as a versioned JSON asset — slug, corrected display name, context window, effective percent, reasoning levels, visibility — generated from a real verified official output and reviewed by hand, including the fleet's overlay model `gpt-5.6-sol` (absent from the 5-model official cache). Baseline identity = app version + recorded source client version; scope-staleness against account identity disappears. Removed with the runtime path: isolated-CODEX_HOME projection, access-token handling for refresh, publisher-signature shell-out, `refresh_official_model_catalog` command and its band, and the whole Manager network-policy feature (`network_policy.rs`, its three commands, panel, tests — its only production consumer was the refresh). The stored `state.official` remains readable for migration but is superseded by the bundled asset; `/v1/models` stays evidence-only. AGENTS.md's "official baselines require a platform-verified target CLI" constraint is rewritten to "the bundled baseline is authored from verified official output at release time and labeled with its source version".

Consequence: the `catalogUnavailable` family for missing baselines disappears; app updates can retire a model, so the existing "active default model disappears" continuity scenario re-anchors to bundled-baseline updates.

## D5. Editor reduction

Hidden or removed (all default to canonical-contract values): preset selector, 接入模式 select, 混入 API KEY checkbox, 上游协议 select, User-Agent, 上下文/压缩上下文 fields, catalog mode segmented control, upstream topology, external adopt, official override table, provider evidence rows and refresh, capability panels. Kept: 名称, 配置模型 (custom model list, prefilled), Base URL, API Key, Provider Doctor, 返回列表. The raw provider-TOML editor (`RelayFileEditors`, App.tsx:4450-4496) is removed for provider config — it displays `experimental_bearer_token` in plaintext and is the one escape hatch that can silently break the contract; the masked draft IPC remains the only bearer carrier. Pure-OAuth exit and pure-API remain reachable as the existing explicit previewed destructive/advanced paths, not as editor switches.

## D6. Rename mechanics

`productName` → "Codex Minus" plus `mainBinaryName: "codex-minus"` in `tauri.conf.json` (pins the executable name; makes the `lib.rs:235` single-instance check correct); window titles (`lib.rs:43`, `tauri.conf.json:17`, `index.html:6`, `App.tsx:1723`), brand div (`App.tsx:1801`), two user-facing error strings (`commands.rs:2242, 5482`), warning copy (`App.tsx:4202`), doc comments, and the 18 CI lines in `.github/workflows/build.yml` (70/81 hardcode `Codex-- Manager.app` for codesign/zip — the build breaks if missed). Unchanged: identifier `fun.mjshao.codex-minus`, settings dir, `codex-minus-` catalog prefix and `.codex-minus-*.tmp` markers (on-disk contracts), BOARD.md history. Residual: old and new app bundles coexist until the old one is removed by hand.

## D7. What survives untouched

Context 保护罩 and the owner-only transaction journal; the provider-owned commit transaction with compare-and-swap and dual-baseline acceptance; golden no-rewrite guarantees (startup/inspection/bystander); OAuth ownership and hash-only observation; typed failure payload shape; restart/new-task guidance; the built-in Pro prefill invariant (default must be representable in the bundled baseline); dead-path warnings for Chat Completions/aggregate.
