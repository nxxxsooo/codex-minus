## Why

The provider surface still exposes machinery the user cannot act on — a capability-evidence panel, an official-catalog management band, a network-policy screen, four catalog modes, protocol/auth switches, and a raw provider-TOML editor that displays the bearer key in plaintext. On Windows a provider save can hang at 「保存中」 forever with no message because unbounded `icacls`/PowerShell subprocesses run under the process-wide lock and a panicked blocking closure drops the IPC response without settling the promise. The official catalog baseline depends on a runtime shell-out to the installed Codex CLI, so a missing or stale refresh strands `official-plus-custom` profiles with `catalogUnavailable`, and the alpha CLI's cache dictates model display names the user considers wrong. The product is also being renamed.

The confirmed product goal: a user with no technical background completes the whole journey — official OAuth sign-in, open Codex Minus, find the profile, keep the default model, fill Base URL and key, save, take effect — with nothing else on screen, and every failure explained in plain language.

## What Changes

- **Foolproof provider flow**: the editor shows only 名称, 配置模型 (prefilled Pro list + default), Base URL, API Key, and Provider Doctor. No 接入模式/协议/混入/User-Agent/context-size/catalog-mode controls, no preset selector, no capability panel, no raw provider-TOML editor. Every save materializes the one canonical actor-authorized contract; opening a legacy profile and saving upgrades it in place.
- **Canonical provider identifier policy**: an upgrade never rewrites an existing non-reserved identifier (session continuity — the user's fleet uses `OpenAI`); brand-new drafts default their provider table identifier to `OpenAI` (distinct from Codex's reserved built-in lowercase `openai`, which stays impossible). The divergent secondary generator (`src/provider-config-draft.ts` target picker) is retired; one generator owns the contract.
- **Save liveness**: every subprocess in the save/status path gets a bounded wait (reusing the `run_bounded_command` pattern); a panicked blocking closure returns a typed failure instead of dropping the IPC response; the pre-`try` draft derivation moves inside the guarded block; post-commit refreshes no longer hold the 保存中 state — success reports as soon as the transaction commits; the no-timeout HTTP client used by 获取模型列表/体检 gets a timeout and its blocking evidence write moves off the async worker.
- **Bundled official baseline (内置 JSON)**: the official catalog ships inside the app as a versioned JSON generated from a real verified official output (correct display names, context windows, reasoning levels, visibility, and the custom-overlay models the fleet relies on, e.g. `gpt-5.6-sol`). The runtime isolated-CODEX_HOME refresh machinery, its access-token projection, its publisher-signature shell-out, and the Manager network-policy module/commands/panel are removed. Baseline freshness follows the app version; relay `/v1/models` stays evidence-only.
- **Catalog UI reduction**: the 官方模型目录 band, the mode segmented control, upstream-topology and external-adopt controls disappear from the default flow. Official and custom models become one table with one context field per row, and the model Codex starts on becomes a selection on a row of that table instead of a free-text field — so a profile cannot name a model its own catalog lacks, and 「模型」 stops meaning five different things. The global and per-profile test models are retired; a provider is tested with the model it starts on. Existing external-catalog profiles keep working untouched.
- **A context number means something**: native mode generates no catalog and points at none, so supplying a per-model context override promotes the profile to `official-plus-custom`, generates the catalog, points live config at it, and says so before the save rather than storing a value that sits dormant.
- **Plain-language failures**: typed commit failures render as one human sentence stating what happened and what to do; codes, rule names, and raw details move behind a 详情 affordance.
- **Rename to Codex Minus**: display name in `productName`, window titles, brand text, user-facing error strings, CI artifact names, README/AGENTS; `mainBinaryName` pinned to `codex-minus` (also fixing the Windows single-instance process check); bundle identifier, settings directory, on-disk slugs, and BOARD history stay unchanged.

## Capabilities

### New Capabilities

- `foolproof-provider-flow`: the seven-step switchless journey, save-always-settles liveness, and plaintext-secret prohibition.
- `product-identity`: one product name across shipped surfaces with pinned binary name and unchanged identity/state paths.

### Modified Capabilities

- `model-catalog-management`: the trusted official baseline becomes a bundled, versioned asset; the runtime credential-bearing refresh requirement is removed; catalog status reports bundled-baseline provenance.
- `provider-native-capability-mode`: the canonical contract gains the identifier policy (preserve on upgrade, default `OpenAI` for new drafts); the capability-evidence status and row-scoped acceptance-matrix requirements are removed with their surfaces while truthfulness of copy remains a standing requirement.

## Impact

- Frontend: `src/App.tsx` (evidence panel ~3751–3861, official band 2121–2175, network panel 2177–2286, catalog editor 2321–2641, provider editor 3900–4220, raw TOML editors 4450–4496), `src/provider-onboarding.ts`, retirement of `src/provider-capability-ledger.ts`, `src/provider-doctor-evidence.ts`, `src/network-policy-ui.ts`, `src/provider-config-draft.ts`; i18n and CSS blocks listed in design.md; affected test files replaced or updated.
- Backend: `src-tauri/src/commands.rs` (commit path, bounded waits, panic typing, generator), `src-tauri/src/live_state.rs` (bounded `icacls`), `src-tauri/src/model_catalog.rs` (bundled baseline, refresh removal), removal of `src-tauri/src/provider_capability_evidence.rs` and `src-tauri/src/network_policy.rs`, `src-tauri/src/lib.rs` command registration and window title.
- Build/CI: `src-tauri/tauri.conf.json` (`productName`, `mainBinaryName`), `.github/workflows/build.yml` artifact names (lines 70/81 hardcode the app path), README/AGENTS rewrites.
- Sequencing: the in-flight `add-manager-network-policy` change (14/15) must be closed or reconciled first — its unarchived delta specifies the panel this change deletes. `support-server-side-composite-catalogs` and `streamline-session-lifecycle` remain unaffected in behavior but share files.
- Open ownership question: a parallel Codex session on the user's machine has started 统一产品名/删掉 WinHTTP edits in another worktree; the rename slice here is gated on the user assigning ownership to exactly one workstream.
- Unchanged invariants: Context 保护罩, OAuth ownership (no `auth.json` writes, hash-only observation), owner-only transaction journal, compare-and-swap commit boundary, no bulk migration, dead Chat Completions/aggregate paths keep their warnings.
