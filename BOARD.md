# codex-minus — Changelog

<!-- Append-only. Newest date first. -->
<!-- Completed work only: what / why / verified / refs. -->
<!-- Todos, next steps, priorities, and status live in OmniFocus. -->

## Changelog

### 2026-08-17

- **release**: 0.4.14 — ships automatic repair of legacy provider `authContents` so Eva can recover through the signed in-app updater instead of editing credential files by hand
  - why: PR #35 fixed the product-owned `OPENAI_API_KEY` + stale OAuth residue that blocked Eva at settings load; a passing feature PR was not deliverable until the merged fix rode a tagged Release and updated `latest.json`
  - verified: PRs #35/#36 merged; Tag, `origin/master`, and local `master` agree at `fcb51a7`; Tag workflow `31994908135` passed macOS arm64, Windows x64, Windows arm64, and Create Release; all 9 published assets match GitHub API SHA256 digests, the 4 application/installer payloads pass `SHA256SUMS`, three updater signatures match `latest.json`, and the live `/releases/latest/` route returns byte-identical version `0.4.14`; Eva received P2P update instructions as FIT user message `om_x100b671ddc81a8a8c4349e3b67ccd9d`
  - refs: PR #35, PR #36, Tag/Release `v0.4.14`, `scripts/release.sh`, `scripts/release-tag.sh`

- **fix/providers**: legacy profile `authContents` containing a provider API key plus stale OAuth fields now migrates automatically instead of blocking settings load
  - why: older Codex Minus/Codex++ Manager versions left Eva's mixed credential copy behind; the strict API-key-only upgrade gate turned product-owned residue into a user repair task
  - verified: the focused migration regressions, full `cargo test` (190 library + 17 + 21 integration tests), `npm run verify` (212 frontend tests), strict OpenSpec validation, Rust formatting, and diff checks passed
  - refs: `src-tauri/src/commands.rs`, `openspec/specs/provider-native-capability-mode/spec.md`, design and implementation plan

- **chore/repository**: reconciled every remaining branch against stable v0.4.13, closed superseded PR #27, aligned the `AGENTS.md` mixed-auth contract with the behavior released through PR #28, and removed all obsolete local/remote branches plus seven auxiliary worktrees so only `master` remains
  - why: the old hard constraint still forced `requires_openai_auth = false` even though v0.4.13 accepts both values and defaults new mixed-auth profiles to `true`; stale merged, release, drill, and abandoned pre-0.4 worktrees obscured the one authoritative line after the release proved stable
  - verified: independent read-only review found no Critical, Important, or Minor issues; PR #34 passed macOS arm64, Windows x64, and Windows arm64 CI and merged as `a64c8ba`; `npm run verify` passed again on merged `master` with 212 frontend tests; final Git inventory showed one clean worktree, local/remote `master` at the same commit, no open PRs, and no other GitHub heads
  - refs: `AGENTS.md`, PR #34, PR #27, merge commit `a64c8ba`

- **feat/providers**: made Codex Minus Responses-only by deleting Chat Completions, the removed local protocol proxy, and client-side aggregate provider product paths
  - breaking: old Chat Completions／local aggregate `settings.json` shapes are unsupported and receive no migration, filtering, fallback, or read-only compatibility
  - preserved: server-side composite providers remain ordinary Responses upstreams; Context transactions, OAuth ownership, catalog ownership, CAS, and rollback invariants are unchanged
  - verified: `npm run verify` passed TypeScript, 209 frontend tests, and knip; `npm run vite:build` passed; `cargo test` passed 208 library, 19 native-capability integration, and 20 draft integration tests with no failures; static removed-path scans found no frontend success path or UI string and no aggregate persistence keys in frontend／current docs; `git diff --check` passed; browser rendering confirmed the sole「添加供应商」action, one Base URL, one Key, Responses copy, and no protocol／proxy／aggregate／member／strategy controls; Tauri-backed save／reopen, live Provider Doctor, set-current, and persisted server-side-composite flows were not verified because the browser shell had no Tauri backend, and a full Tauri app bundle was not built

### 2026-08-16 (late)

- **docs/presence**: README (zh/en) and the mjshao.fun landing/work page caught up with 0.4.x reality — signed in-app updates (sidebar banner, version foot with manual check, graceful 重启 Codex), mixed-auth keeps the ChatGPT login with both `requires_openai_auth` values legal, sequential picker order and authoritative empty reasoning levels, active-only session adaptation, NSIS-only Windows installers, and Finder-drag install guidance against quarantine translocation; product name unified to Codex Minus everywhere
  - why: public copy still claimed 「no auto-updater」, `requires_openai_auth = false` as the canonical contract, a disabled session adaptation, v0.3.0 links, and `.msi` installers — all overtaken by 0.4.x; the landing still carried the pre-rename product name
  - verified: real 0.4.13 screenshots captured from the installed app (hero: provider list with the new version foot; detail: provider editor with the actionable restart badge, relay endpoint scrubbed to a placeholder); stale 0.3-era PNGs removed; portfolio change cherry-picked onto origin/main (5e46efa) without touching that repo's diverged local work
  - refs: README.md, README.en.md, docs/assets/codex-minus-hero.webp, mjshao-portfolio public/codex-minus + codex-minus.mdx

- **fix/catalog**: a custom model that declares no reasoning levels now materializes `supported_reasoning_levels: []` with no default, instead of inheriting the bundled GPT template's four levels; the Haiku known-model card drops its levels after a live upstream rejection
  - why: Codex offered an Effort menu for custom Claude models the user never gave levels to, and sent `effort` to the Sub2API bridge, which rejected it for Haiku (「This model does not support the effort parameter」); an empty list in the editor was silently un-declarable
  - verified: new regression `a_custom_model_without_declared_levels_materializes_no_reasoning_effort`; known-model card tests updated; Rust 175+17+21, frontend 212, tsc, knip green
  - refs: src-tauri/src/model_catalog.rs, src/known-relay-models.ts
- **feat/host**: a 重启 Codex action — in the sidebar foot beside 检查更新, and the 需重启 Codex badge in the model editor is now the same button; quit is always graceful AppleScript with a confirm, never a force-kill, and nothing triggers it automatically (Windows reports not-yet-supported)
  - why: every contract or static-catalog change ends with 「请手动退出并重开 Codex」; the badge named the action but could not perform it (user: 「工具考虑加入重启 codex 按钮」)
  - verified: tsc, 212 frontend tests, knip green; `statusLabel`/`statusClass`/`isSuccessStatus` moved out of the shell to pay the line budget (3486 ≤ 3500); macOS restart path exercised locally after install
  - refs: src-tauri/src/commands.rs (`restart_codex_host`), src/App.tsx, src/status-presentation.ts, src/i18n-en.ts

- **fix/updates**: an install failure caused by macOS App Translocation (EROFS 「os error 30」、AppTranslocation 路径) now leads with the one-time fix — drag the app into Applications with Finder or clear the quarantine attribute — instead of the bare raw error
  - why: an ad-hoc-signed download always carries quarantine; launched in place it runs on a read-only mount and the updater cannot replace its own bundle, which a colleague hit as an unexplained 「Read-only file system (os error 30)」
  - verified: guidance matcher unit-tested (positive on os error 30 / AppTranslocation, silent otherwise); raw error stays behind 详情 per the notice contract test; tsc, 212 frontend tests, knip green
  - refs: src/app-update.ts (`appUpdateInstallFailureGuidance`), src/App.tsx, src/i18n-en.ts

- **feat/updates**: the sidebar foot always shows the installed version with a 检查更新 action — a manual check that finds nothing says 已是最新, a failure invites a retry, and a found update hands over to the existing banner
  - why: the updater checked exactly once at startup and silently; an instance launched before a release never learned of it without a relaunch, and nothing in the UI answered 「我现在装的是哪版」 (user: 「做一个查看版本和更新，可以左下角」)
  - verified: tsc, 210 frontend tests, knip, vite build green; `providerTransitionConfirmationMessage` moved out of the shell and dead `providerNativeCapabilityStateLabel` deleted to pay the line budget (3482 ≤ 3500)
  - refs: src/App.tsx, src/styles.css, src/provider-transition-confirmation.ts, src/i18n-en.ts

- **fix/catalog**: renumber materialized model `priority` sequentially over the final composed list (officials sorted, customs sorted by `order` and appended) so the Codex picker order always matches the manager list order
  - why: Codex sorts its picker by `priority`, not file order; official baseline priorities (1,1,2,3…) and custom orders (0,1,2,3) were two colliding 0-based sequences, interleaving Claude custom models between GPT officials at runtime
  - verified: new regression `runtime_priorities_are_sequential_so_customs_never_interleave_officials`; Rust 174+17+21 green
  - refs: src-tauri/src/model_catalog.rs (`compose_profile_catalog`)
- **fix/providers**: mixed-auth (官方登录＋混入 API Key) keeps `requires_openai_auth = true` — the contract now accepts both values as canonical, the onboarding/upgrade transform writes `true`, and existing values are never repaired away
  - why: forcing `false` silently downgraded working mixed-auth setups to pure-API runtime — the ChatGPT-attached login disappeared from Codex and plugins/account surfaces got limited (user: 「不可接受，那个模式下插件受限」); ExitPureApi still writes `false` because pure API is exactly what it names
  - verified: contract tests updated (a value flip is no longer drift, a missing field still is); Rust 174+17+21, frontend 210, tsc, knip green
  - refs: src-tauri/src/provider_native_capability.rs, src/provider-onboarding.ts
- **fix/providers**: a pure-login profile (no provider table) is switchable regardless of the catalog mode its state carries, and a machine without a settings file bootstraps defaults instead of refusing every save; the three `inputUnavailable` causes now name themselves (unreadable file / invalid JSON / profile failed migration)
  - why: switching to 「OpenAI Official」 died with `stagingRejected: provider runtime identity is incomplete` because the profile's state still carried `official-plus-custom` from an older version; a colleague machine hit `inputUnavailable` with no way to tell which of three causes it was (user: 「加了太多不该有的门禁」)
  - verified: Rust 174+17+21 green including the runtime-fingerprint and commit-transaction suites
  - refs: src-tauri/src/model_catalog.rs (`applied_runtime_fingerprint`), src-tauri/src/commands.rs (`load_provider_commit_settings`)
- **feat/relay**: official-auth panel shows live auth status for every profile including new drafts (auth.json is global; new providers inherit the current login), gains a manual refresh button, and refreshes on detail open
  - why: the status hid itself on non-active/new profiles with 「仅活动供应商显示实时认证状态」, implying login state was per-profile when it is machine-global
  - verified: tsc, 210 frontend tests, knip green; `providerDoctorSteps` moved out of the shell to pay the App.tsx line budget (3470 ≤ 3500)
  - refs: src/App.tsx (`RelayLiveFilePanels`), src/provider-doctor-steps.ts, src/i18n-en.ts
- **ops/incident**: a stale local master (239 commits behind origin) was built as 0.3.1 and installed over the released 0.4.10, briefly rewriting settings with the old schema; recovered by resetting master to origin/master, restoring the official 0.4.11 release bundle from GitHub (checksum + signature verified), and repairing the catalog pointer plus `requires_openai_auth` flips in settings/config
  - why: the build was made without fetching origin first — the release line had moved to GitHub PRs while the local checkout stayed on the pre-0.4 base
  - verified: auth.json byte-untouched throughout (mtime + key shape); local-only work preserved (Eva-guide checkpoint cherry-picked, fixes re-landed on the 0.4.11 base); stale branch kept at `backup/stale-master-0.3.1`
  - refs: BOARD 2026-08-13 docs/windows entry, this entry

### 2026-08-16

- **verify/updater**: the 0.4.10→0.4.11 in-app upgrade completed on a real macOS machine — the macOS half of the updater's real-machine verification is done
  - why: 0.4.9/0.4.10 exercised the chain through drills and CI; this is the first user-machine upgrade observed end to end (banner → download → signature verify → install → relaunch into 0.4.11)
  - verified: v0.4.11 release published with all 9 assets and `latest.json` at 0.4.11 for all three platforms; the upgrade banner appeared in the 0.4.10 position as expected (the banner belongs to the version being upgraded *from*); noted en route: the client checks for updates exactly once at startup (`src/App.tsx` one-shot effect), so an instance launched before a release never sees it without a relaunch
  - refs: OpenSpec `add-in-app-updates` 4.2
- **release**: 0.4.11 — the sidebar-foot update banner ships (PR #25), closing the same night's walkthrough feedback loop
  - why: the banner under the page header read as part of whichever screen was open; machines picking this update up through the 0.4.10 banner also re-exercise the instant-click-feedback fix that release carried
  - verified: three-platform CI green on the release PR; release assets and `latest.json` checked after tagging
  - refs: `scripts/release.sh`, PR #25
- **release**: 0.4.10 — the session-screen cleanup and banner-click fix ride the updater the day after 0.4.9 (PR #23)
  - why: the fixes answer live walkthrough feedback on 0.4.9, and shipping them immediately exercises the update chain again — any 0.4.9 machine picking this banner up also demonstrates the new instant click feedback it carries
  - verified: three-platform CI green on the release PR; release assets and `latest.json` checked after tagging
  - refs: `scripts/release.sh`, OpenSpec `clarify-session-screen-actions`, PR #23
- **fix/sessions-ui**: the session screen's buttons regained a hierarchy — one primary action per card, policy inputs that save themselves, and status lines that stop lying
  - why: the 0.4.9 walkthrough surfaced three confusions in a row (user: 「这三个按钮还是有点点困惑」「点击按钮多次后才成功触发，没有明显点击效果」) — the archive card offered 预览/保存策略/立即检查 at equal weight when only one is an intention, a deferred maintenance run rendered as a green check with `候选 232，已归档 0`, a manual 立即检查 inside the daily interval silently answered "not due" with zeros, and the update banner ignored clicks until the first download event crossed the network, so every extra click started another concurrent download
  - verified: retention days auto-save debounced+silent and the preview follows (on screen load it chains after the lifecycle fetch so it uses the saved days); `run_session_archive_maintenance` takes `force` — the button always runs, only the startup pass keeps the interval, and the extracted `archive_maintenance_due` is pinned by a test (force bypasses the schedule, never the policy); the result line renders by disposition (counts+check / warning+deferral reason / info+not-due message) with the two deferral messages finally in the EN dictionary; 适配到当前 provider is the compatibility card's filled primary; the sessions list drops its redundant 刷新, shows one 多选 normally and the full 全选/清空/删除/取消 set only in selection mode (取消 — the exit — didn't exist before); the banner flips to 下载中 on the click itself with re-entry guarded, and a reducer test pins that the plugin's later `Started` supersedes the synthetic phase; Rust 173+17+21, frontend 207, tsc, knip, `cargo fmt --check` green
  - also: the update banner moved from under the page header — where it read as part of whichever screen was open (user: 「放到会话管理页面了 换个地方更合适」) — to a card at the sidebar's foot, the app-scoped region it belongs to
  - refs: `src-tauri/src/commands.rs`, `src/App.tsx`, `src/app-update.ts`, `src/i18n-en.ts`, OpenSpec `clarify-session-screen-actions`

### 2026-08-15

- **release**: 0.4.9 — ships the active-only session adaptation (manual button + on-switch automatic, PR #21) the day after 0.4.8
  - why: the fleet's two mismatched active sessions need the released build to clear, and this is the first release an installed updater can receive — both 0.4.8 machines updating through the banner is the real-machine half of `add-in-app-updates` 4.2, including the Windows side the drill hadn't covered yet
  - verified: three-platform CI green on the release PR; release assets and `latest.json` checked after tagging
  - refs: `scripts/release.sh`, OpenSpec `add-active-only-session-adaptation`, `add-in-app-updates` 4.2

### 2026-08-14

- **feat/sessions**: the「适配到当前 provider」button is alive, and switching providers now carries active sessions along automatically (toggle, default on)
  - why: the button was a hard-disabled stub for a real reason — the upstream `provider_sync` rewrites the whole history by construction (its rollout walk hard-codes `archived_sessions`, its sqlite update has no WHERE), so enabling it meant rewriting the fleet's 10k+ archived sessions; the user asked for the original auto-repair capability back (「拉出来，并且切供应商自动」) without that cost
  - verified: the manager grew its own active-only counterpart in `session_adaptation.rs` — per-session rollout `session_meta` rewrite plus per-id sqlite update, addressed by the `rollout_path`/`db_path` the inventory carries, so archived history is unreachable by construction; backup-first into a marker-scoped namespace (pruning can never eat the core's provider-sync backups), locked files skip whole rather than splitting a session across providers, a failed row update restores the rewritten rollout, and foreign `encrypted_content` is flagged with the core's established warning; tests pin the archived-untouched guarantee byte-for-byte; Rust 172+17+21, frontend 206, tsc, knip, `cargo fmt --check` green
  - also: `saveSessionLifecycle` now takes a partial patch — the refactor paid for the feature's shell footprint (App.tsx 3490/3500)
  - refs: `src-tauri/src/session_adaptation.rs`, `src-tauri/src/commands.rs`, `src/App.tsx`, OpenSpec `add-active-only-session-adaptation`
- **feat/distribution**: releases are signed and the app updates itself — one silent check at startup, a banner naming the new version, and「更新并重启」downloads, verifies, installs, and relaunches
  - why: every release reached the fleet by hand — download from the release page, send over Lark, re-install on each machine — and with fix releases days apart that loop became the slowest link in the debug cycle: Eva sat blocked on a save error whose 详情 payload only the next version could show her
  - verified: the minisign keypair lives only in `~/.tauri/` (600) and the `TAURI_SIGNING_PRIVATE_KEY` repo secret, with the public key pinned in `tauri.conf.json`; every build signs (a missing secret fails loudly), the release job refuses to publish without one signature per updater artifact, and `latest.json` pins tag-specific URLs; the banner phase reducer and labels are pinned by tests, a failed startup check is silence by spec, and an install failure reports through the standard notice with 详情; real-update drill (old local build → banner → land in released version) is the change's open verification task
  - also: the Windows MSI target is dropped entirely (user decision:「只留一个」) — the updater applies NSIS packages, so the MSI's one remaining effect was making it possible to install through the channel updates cannot ride; machines that installed an older `.msi` uninstall it once before the `-setup.exe` hand-install, and the NSIS installer now carries SimpChinese
  - refs: `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`, `src/app-update.ts`, `src/App.tsx`, `.github/workflows/build.yml`, OpenSpec `add-in-app-updates`
- **feat/catalog**: the four Claude models the fleet's Sub2API composite group serves are now one-click preset cards in the model editor
  - why: the backend could already compose a mixed catalog carrying display name, description, context, and reasoning levels — the owner ran exactly that by hand-authoring an external file during the 372k work — but the simplified editor could not author it: a custom row edited only slug and context, the display name was forcibly the slug, and reasoning metadata had no entry point; re-opening per-row editors would undo the foolproofing that removed them
  - verified: the four cards (Fable 5, Opus 5, Sonnet 5, Haiku 4.5) are copied field-for-field from the owner's validated catalog and pinned by a contract test so a revision is a reviewed diff; adding a preset carries the whole card into a custom row, presets only prefill (a saved profile owns its copy), display names follow the slug until edited independently, and a compose test proves a preset-shaped row round-trips into the generated catalog while old persisted state still loads; Rust 168 + 17 + 21, frontend 205, tsc, knip, `cargo fmt --check`, three-platform CI green
  - refs: `src/known-relay-models.ts`, `src/model-catalog-ui.ts`, `src-tauri/src/model_catalog.rs`, OpenSpec `add-known-relay-model-presets`, PR #17
- **fix/catalog**: the bundled baseline carried a listed `gpt-5.2` where the fleet's `gpt-5.3-codex-spark` should be (user-reported)
  - why: no bundled client list carries spark — it exists only in the account's server model list — so the 3.1 authoring took the client's bundled output at face value and shipped an entry the fleet does not offer while omitting the one the Pro prefill list depends on; a user creating a provider and picking spark would have hit an unrepresentable default on their very first save
  - verified: the replacement entry is the account's own server-list entry (128k context, 95%, listed, reasoning high) hydrated locally by the user's earlier 372k work — template byte-identical to the server cache of 2026-08-13, `base_instructions` present so the materialization guard still holds; a new asset-anchored test cross-checks every `PRO_MODEL_SLUGS` entry as listed and every `RETIRED_MODEL_SLUGS` entry as hidden in the shipped asset itself, which is the test that would have caught this
  - refs: `src-tauri/assets/official-model-catalog.json`, `src/provider-onboarding.test.ts`, `src/model-catalog-ui.test.ts`
- **fix/catalog**: a bundled-baseline update that retires an active profile's default model now names the repair instead of failing generic — and the continuity behavior is pinned by regression tests
  - why: continuity was already structural (an update touches neither the generated catalog nor the live pointer), but the reporting half of the scenario was not: the active save collapsed into "normalized provider catalog planning failed", and the persisted readiness sentinel `catalog-readiness-unavailable` rendered on screen as a bare code
  - verified: transaction tests against the real bundled asset using the very slug the sibling fix removed — an inactive stranded profile saves with `action_required` set and its last valid generation untouched; an active one fails typed as `active provider default model is absent from the bundled baseline` with a reason-level sentence telling the user to pick a new startup model, and nothing on disk moves; Rust 166 + 17 + 21, frontend 187, tsc, knip, `cargo fmt --check` green
  - refs: `src-tauri/src/provider_commit.rs`, `src-tauri/src/commands.rs`, `src/provider-commit.ts`, `src/model-catalog-ui.ts`, OpenSpec `simplify-into-codex-minus` 3.5
- **refactor**: retired `provider-config-draft.ts` and the four-target picker; one generator materializes the provider contract
  - why: the picker chose between generators by profile mode, but every alternate (`pureApi`/`compatibility`/`pureOAuth`) became unreachable when the editor stopped offering 接入模式 — what remained was a second, hand-rolled `[model_providers.custom]` generator that could only ever produce a contract the canonical one wouldn't, kept alive by four imports in App.tsx that turned out to have no call sites at all
  - verified: the contract collapsed to provenance alone (brand-new materializes through `materializeNewProviderConfig`, saved patches in place), pinned by a wiring assertion that the picker reads no profile field; the surviving TOML patcher moved byte-identical (verified by diff against HEAD) into the transform router — not relay-settings, which imports `@/i18n` and would have cut every importing test off the node runner; 185 frontend tests, tsc, knip green
  - refs: `src/provider-config-transform-router.ts`, `src/relay-settings.ts`, `src/provider-config-patch.test.ts`, `src/relay-settings-wiring.test.ts`, OpenSpec `simplify-into-codex-minus` 2.3
- **fix/providers**: every typed save failure now leads with one plain-language sentence and keeps the raw code, rule, and backend message behind a 详情 disclosure — and a failed notice stays on screen until dismissed
  - why: the notice used to concatenate the backend message, a hint, and `（code: reason）` into one wall and then auto-dismiss it after 4.2 seconds, so a user could neither read what to do nor capture what to report — the exact gap that reduced Eva's field report to a screenshot of「保存失败」; worse, the backend collapsed every request-validation rejection into `provider draft validation failed`, blaming the form for rules like external-pointer preservation that the form cannot even see (the misattribution found during 3b.7)
  - verified: `sanitized_provider_validation_error` interns the anyhow message back to the static rule that rejected it, so the payload keeps its no-dynamic-content redaction property while naming the actual gate — the 3b.7-shaped test now asserts the ownership rule by name; snapshot tests pin the full `{sentence, detail}` for stale, staging-rejected, contract-gap, and timeout; a sweep proves no sentence carries any discriminator, and source assertions pin that every `stringifyError` rides behind 详情 and that failures outlive the toast timer; Rust 164 + 17 + 21, frontend 186, tsc, knip, `cargo fmt --check` all green
  - also: the `catalogScopeStale` hint stopped instructing「刷新官方模型目录」— an instruction pointing at the band 3.3 removed
  - refs: `src-tauri/src/commands.rs`, `src/provider-commit.ts`, `src/App.tsx`, `src/i18n-en.ts`, `src/provider-commit.test.ts`, OpenSpec `simplify-into-codex-minus` 4.4
- **refactor**: removed the runtime catalog refresh machinery — command, isolated CODEX_HOME projection, access-token handling, and publisher-signature shell-out (−965 lines)
  - why: `refresh_official_model_catalog` was a registered command with no UI entry left to reach it, and behind it sat the only reason this manager ever touched OAuth tokens — it wrote a projection of the user's credentials into an isolated home, ran the official CLI there, and verified that CLI's code signature first so it knew whose binary was receiving them; the baseline ships inside the application now, so the whole chain had no purpose left
  - verified: 164 Rust lib + 17 + 24 integration tests pass with **no ignored tests remaining** (the live-OAuth test went with the refresh it exercised); `cargo fmt --check` clean, clippy 47 warnings down from 48 with none new, `npm run verify` green, three-platform CI green; `/v1/models` provider evidence collection is untouched, and `adopt_external_model_catalog` stays because the editor still reaches it
  - also: `freshness` had already been permanently `scope-stale` — the bundled baseline carries `fetched_at_ms: 0` and a content hash in the field the scope comparison reads — so it and the other fields refresh left constant or wrong (`targetTrusted`, `refreshAvailable`, `lastSuccessfulRefreshAtMs`, `diff`) were dropped rather than kept as lies; an expired access token no longer fails a save closed, because that check existed only so a projected token would still work and the official client refreshes its own tokens, while unreadable auth and auth that cannot name an account still fail closed
  - refs: `src-tauri/src/model_catalog.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/platform_command.rs`, `src/backend-types.ts`, `src/provider-command-registration.test.ts`, OpenSpec `simplify-into-codex-minus` 3.3, PR #11
- **fix/windows**: gave the target-CLI probe's output capture its own files per capture, closing the same collision the helper capture had
  - why: `run_bounded_command` named its stdout/stderr from the clock nonce and the pid, exactly the shape fixed in `platform_command` on 2026-08-12 — Windows advances that clock in ~15.6 ms steps and every thread shares the pid, so two probes started in one tick derive one name and the first to finish deletes the output the other is still reading; found while removing the duplicate that had been fixed
  - verified: the naming regression follows the rule to its new home rather than being deleted with the code it guarded; a doc comment now records that `platform_command`'s output half is Windows-only live, so the macOS dead-code warnings about it are false — that misreading broke both Windows jobs once before it was caught
  - refs: `src-tauri/src/model_catalog.rs`, `src-tauri/src/platform_command.rs`
- **fix/providers**: a catalog pointer an older version left in a profile's stored config no longer makes that profile permanently unsaveable
  - why: migration read *any* `model_catalog_json` as「用户拥有的外部目录」, and the rule that an ordinary save must preserve every external pointer then rejected every native or managed draft — which is all of them, since the editor has no way to send a pointer; the profile deadlocked, because correcting the mode is itself a save, and the failure surfaced as a bare `InvalidDraft` with a misattributed message
  - verified: 170 Rust lib + 17 + 24 integration tests pass with the live OAuth test intentionally ignored; the regression was confirmed red without the fix rather than merely green with it; a pointer the user actually chose is still preserved, since the discriminator is equality with the exact path this manager generates for this exact profile — `manager_owned_pointer_path` cannot answer that question here, as it asks the state for a generated path while that state is still being built
  - refs: `src-tauri/src/model_catalog.rs`, `src-tauri/src/provider_commit_transaction_tests.rs`, OpenSpec `prioritize-native-capabilities-for-mixed-providers` 3b.7, PR #9
- **test/providers**: pinned the compare-and-swap invariant that the fingerprint a save returns is the one the next save is judged against
  - why: the reported `staleState` had two candidate explanations — backend fingerprint accounting and frontend baseline adoption — and neither could be ruled out by reading alone
  - verified: the returned fingerprint, the fingerprint of the returned settings, and the fingerprint of the settings re-read from disk agree across a catalog-generating save, for both「official mixed with an API key」and「official OAuth only」profile shapes; this rules the backend out, leaving the two fingerprints an actual session sends as the remaining unknown
  - refs: `src-tauri/src/provider_commit_transaction_tests.rs`
- **refactor**: one TOML escaper instead of three, and the one that escapes newlines
  - why: `codex-toml.ts` escaped backslash and double-quote and stopped, while `provider-config-draft.ts` and `provider-onboarding.ts` both used the `JSON.stringify` form — and the difference is not cosmetic, because a TOML basic string may not span lines, so a value carrying a newline does not read back slightly wrong, it leaves Codex unable to parse its own config
  - verified: `npm run verify` green — TypeScript, 174 frontend tests, knip; JSON's escapes are a subset of TOML's basic-string escapes, so the surviving implementation is correct for every input; the escaping had no test in any of the three copies and now has one, including the asymmetry that the reader undoes only quote and backslash escapes
  - refs: `src/codex-toml.ts`, `src/codex-toml.test.ts`, `src/provider-config-draft.ts`, `src/provider-onboarding.ts`, PR #9

### 2026-08-13

- **refactor**: made `src/App.tsx` a wiring file and added a ratchet that keeps it one — 5088 → 3464 lines, 100 pure functions → 33
  - why: none of those functions could be tested without reading the file as text, which is why four tests did exactly that and broke whenever a neighbour moved; worse, every unrelated change had to merge around the same 5000-line file, so two branches touching different features conflicted by construction — the one thing standing between this repository and working on two fronts at once
  - verified: TypeScript, 168 frontend tests, and knip pass; every move was mechanical (bytes copied, nothing rewritten), so the suite passing is the proof of unchanged behaviour; `app-shell-budget.test.ts` fails on a 34th rule in the shell, fails when a listed name has left so the list cannot rot, and caps the file at 3500 lines
  - refs: new `src/backend-types.ts` (79 wire shapes, 27 of them found dead and deleted — `ZedRemoteProject`, the plugin-marketplace trio, ads, script market, watcher/install/update, all shapes of the removed launcher), `src/codex-toml.ts`, `src/codex-context-entries.ts`, `src/relay-settings.ts`, `AGENTS.md`, `docs/workflow.md`
- **build**: ran the frontend checks in CI for the first time, added knip, and scripted the release
  - why: CI built the frontend and ran the Rust tests but never ran `tsc` or a single frontend test, so 164 tests only ever ran on a laptop; and the version lives in four files nothing synchronises, with `Cargo.lock` the one that gets forgotten because only cargo writes it
  - verified: `npm run verify` (type check + tests + knip) runs in the macOS job — the frontend is platform-independent, so it is checked once on the fastest runner rather than three times; knip's first run found `@fontsource/inter` installed and never referenced and an unnecessary `export`; `release-version.test.ts` asserts the four versions agree; both release scripts' guards were exercised (dirty tree, wrong branch, missing BOARD entry, unmerged version)
  - refs: `.github/workflows/build.yml`, `knip.json`, `scripts/release.sh`, `scripts/release-tag.sh`, `src/release-version.test.ts`, `docs/workflow.md`
- **chore**: turned on branch protection for master and committed the Serena project definition
  - why: master accepted anything; and Serena runs with `--project-from-cwd`, so it wrote a project definition named after whichever worktree it started in — one that disappears with `git worktree remove` and is regenerated differently by the next one
  - verified: three required checks with `strict` on, no force-push, admins exempt as an escape hatch — PR #6 and #7 both merged through it; merge queue is unavailable (organisation-only, this repository is user-owned) so the `merge_group` trigger is in place but dormant; the Serena definition is named for the repository and adds `rust` beside `typescript`, since half this project is the Tauri backend
  - refs: `.serena/project.yml`, `.gitignore`, `docs/workflow.md`
- **refactor**: removed the Manager network policy — module, three commands, panel, UI helper, CSS, and 52 i18n entries — and fixed the isolated official-catalog refresh to direct
  - why: it configured a proxy for exactly one consumer, the isolated Codex CLI child that pulled the official catalog; that refresh lost its premise when the baseline moved into the app as a bundled asset and lost its UI entry when the 官方模型目录 band was removed, so the panel steered a route nobody could reach — and its other advertised purpose was untrue of the code, since the provider connection test and Provider Doctor run on the pinned core's own HTTP client, which reads the process environment directly and never consulted the policy
  - verified: 168 Rust lib + 17 + 24 integration tests pass with the live OAuth test intentionally ignored, including a rewritten regression that the isolated child inherits no proxy and no credentials; 160 frontend tests, TypeScript, Vite build and `cargo fmt --check` pass; clippy reports no dead code beyond the pre-existing runtime-refresh remnants; saved `network-policy.json` is left on disk untouched and unread, and `reqwest`'s `socks` feature stays in `src-tauri/Cargo.toml` because the pinned core does not enable it and feature unification is additive — dropping it would silently break provider tests reached through a `socks5://` proxy
  - refs: deleted `src-tauri/src/network_policy.rs` and `src/network-policy-ui.ts`, `src-tauri/src/model_catalog.rs`, `src-tauri/src/lib.rs`, `src/App.tsx`, OpenSpec `add-manager-network-policy` marked withdrawn
- **feat/model-catalogs**: made the model list show exactly what Codex offers, gave every row a delete, and added a one-action restore of the shipped Pro list
  - why: the editor rendered every entry in the bundled baseline, including the three it marks hidden (`gpt-5.4`, `gpt-5.4-mini`, `codex-auto-review`), so the table disagreed with the Codex picker for the same profile; official rows had no delete at all, so the list could only grow; and the provider-reported candidate strip offered slugs the table already carried, which would have added a second row colliding with an official one in the generated catalog
  - verified: 168 Rust lib + 17 + 24 integration tests pass with the live OAuth test intentionally ignored, including that a deleted official model is generated as `visibility: "hide"` with its baseline entry and instructions intact, and that a list with everything deleted is refused; 164 frontend tests covering effective visibility, the difference-from-baseline override, candidate de-duplication, the restore's kept context windows and named losses, and the empty-list draft error
  - refs: `src/model-catalog-ui.ts`, `src/App.tsx`, `src-tauri/src/model_catalog.rs`, `src/foolproof-provider-flow.test.ts`
- **fix/providers**: cleared the test models an older version left behind on every provider save
  - why: the controls that set the global 供应商测试模型 and each profile's own test model are gone, but the backend prefers a profile's stored test model over the model it starts on — so a stale value would have outranked the startup model permanently, with no way left to see or change it
  - verified: 160 frontend tests including guards that the ordinary profile branch and the settings normalizer both emit an empty test model while the aggregate branch keeps its own field, since an aggregate profile has no startup model to follow
  - refs: `src/App.tsx`, `src/foolproof-provider-flow.test.ts`
- **feat/model-catalogs**: reduced every model question in the provider editor to one table — the startup model is a radio on a row, official and custom models share one list, the global provider test model is gone, and Provider Doctor's check reads 「供应商支持的模型」
  - why: 「模型」named five different things across the editor, one of which had no way in at all — `modelList`/`modelWindows` still held state, rode along on every save and fed Provider Doctor while no control could edit them, and the hint under the startup-model field still sent the user to that removed section by name; the table's CSS also still declared eleven columns and a 1150px minimum from before the reduction to a single context field, so it scrolled sideways
  - verified: 175 Rust lib + 17 + 24 integration tests pass with the live OAuth test intentionally ignored; 154 frontend tests including selection-carrying on rename and delete, no selection of a slugless row, and a guard that no second place to name a model returns; TypeScript, Vite build and `cargo fmt --check` pass; the backend already fell back from `profile.test_model` to the profile's own model, so removing the global field needed no backend change
  - refs: `src/App.tsx`, `src/styles.css`, `src/foolproof-provider-flow.test.ts`, deleted `src/model-windows.ts`
- **feat/model-catalogs**: made a context window typed against an official model promote its profile from native to managed, generate the catalog, point live `config.toml` at it, and say so before saving
  - why: native mode generates no catalog and points at none, so the number was stored in the overlay and then ignored — on the reporting machine the active profile held `contextWindow: 372000` against gpt-5.6-sol while live config carried no catalog at all — and the mode control that would have explained this no longer exists in the editor
  - verified: 175 Rust lib + 17 + 24 integration tests pass with the live OAuth test intentionally ignored, including a promoted save that generates the file, writes the pointer, and reports the restart; 159 frontend tests including the promotion rule and a guard that no overlay edit bypasses it; TypeScript, Vite build and `cargo fmt --check` pass
  - refs: `src/model-catalog-ui.ts`, `src/App.tsx`, `src-tauri/src/provider_commit_transaction_tests.rs`
- **fix/providers**: normalized a backend-returned profile where it enters the editor and stopped reading a possibly-absent model straight into `.trim()`
  - why: serde omits an empty string rather than sending `""`, so a profile whose model, Base URL and Key live only inside its TOML came back without those keys while the editor's type claimed they were present — every save on such a profile died on `Cannot read properties of undefined (reading 'trim')`, reproduced on Windows where the only profile has exactly that shape
  - verified: the field names missing from the Windows `settings.json` were read directly from the VM; 159 frontend tests including two regressions on the boundary and the read; TypeScript and Vite build pass
  - refs: `src/App.tsx`, `src/foolproof-provider-flow.test.ts`
- **fix/windows**: gave each bounded helper capture its own temp files, so two helpers started in the same clock tick stop deleting each other's output
  - why: the capture name was derived from `SystemTime::now` and the pid, and Windows advances that clock in ~15.6 ms steps while every thread shares the pid — colliding helpers made the first to finish delete the file the other was still reading, which surfaced as a bare `os error 2` with no context from the ACL verifier and failed `live_state` on the Windows x64 runner
  - verified: the naming and concurrent-capture regressions pass, 173 Rust lib + 17 + 24 integration tests pass with the live OAuth test intentionally ignored, and GitHub Actions macOS arm64 + Windows x64 + Windows arm64 pass
  - refs: `src-tauri/src/platform_command.rs`
- **fix/providers**: stopped a failed settings read from becoming the compare-and-swap baseline, made a save with no baseline re-read and report which case it hit instead of dying on `provider settings fingerprint is unavailable`, and settled `load_settings`/`save_settings` through `settle_blocking` so a panic still answers the caller
  - why: a failed read answers with default settings and no fingerprint, so adopting it replaced the profiles on screen and postponed the reason until the next save, which then failed with an internal condition the user could do nothing about; re-reading is not enough on its own because the form holds placeholder settings at that moment and committing it would publish an empty provider list
  - verified: 171 Rust lib + 17 + 24 integration tests pass with the live OAuth test intentionally ignored; 151 frontend tests including new baseline-gate and failed-read regressions, TypeScript, Vite build, and `cargo fmt --check` pass; the on-disk preconditions on the reporting machine (state dir 0700, `auth.json` 0600, parseable settings, empty journal) were all healthy, so the exact trigger of the empty baseline there is not pinned
  - refs: `src/App.tsx`, `src/provider-save-liveness.test.ts`, `src-tauri/src/commands.rs`
- **feat**: renamed the product to Codex Minus, reduced the provider screen to the seven-step flow (名称 / 配置模型 / Base URL / Key / Provider Doctor), shipped the official model baseline inside the application, and made every provider save settle
  - why: the editor exposed switches, evidence panels, and a raw TOML editor that a non-technical user could not act on, while a save could hang at 「保存中」 forever and a stale or absent CLI catalog cache could strand a managed profile
  - verified: 171 Rust lib + 41 integration tests pass with the live OAuth test intentionally ignored, including bounded-helper, panic-settlement, coordinator-poison-recovery, bundled-baseline, and pinned-core `OpenAI` round-trip regressions; 149 frontend tests, TypeScript, Vite build, `cargo fmt --check`, and GitHub Actions macOS arm64 + Windows x64 + Windows arm64 all pass; the renamed bundle installed and launched on macOS and in the Windows ARM64 VM, and live `auth.json` and `config.toml` stayed byte-identical throughout
  - refs: OpenSpec `simplify-into-codex-minus`, `src-tauri/assets/official-model-catalog.json`, `src-tauri/src/platform_command.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/model_catalog.rs`, `src/App.tsx`, `.github/workflows/build.yml`
- **docs/windows**: corrected and redelivered Eva's Windows Sub2API guide with the assigned `gpt-5.6-terra` model, an explicit `[model_providers.custom]` → `[model_providers.OpenAI]` migration, Windows/UAC recovery steps, and mandatory API-key redaction guidance
  - why: Eva's live config retained the old `gpt-5` model and `custom` provider table while selecting `OpenAI`, and the resulting setup failure exposed gaps in the original merge instructions
  - verified: the local Markdown passes `git diff --check`; the FIT Feishu Docx was updated in place and read back with the corrected model, provider migration, Windows setup troubleshooting, and key-rotation warning
  - refs: `docs/eva-windows-sub2api-manual.md`, https://favhej6sxti.feishu.cn/docx/KeLBdvZEPolp3yx6MyqcFVXXnpc

### 2026-08-12

- **docs/windows**: created and delivered Eva's Windows manual fallback for keeping the Stable ChatGPT login while routing Codex inference through a case-sensitive custom `OpenAI` provider, Sub2API API-key authentication, server-side OAuth/API account pooling, Responses, and live web search
  - why: the desktop app still could not reliably complete this onboarding flow, so FIT needed a deterministic manual path covering Clash/UWP Store setup and one exact TOML merge without touching OAuth files or historical sessions
  - verified: Windows 11 ARM Stable ChatGPT launched with the existing Free login; an Eva-key Responses request succeeded through the FIT Sub2API pool and selected an OAuth upstream; Stable app-server metadata registered `web__run` with `web_search = "live"`; the final Markdown was imported and read back as a FIT Feishu Docx, Eva received view access, and the default FIT product group received a real `@Eva` link message
  - refs: `docs/eva-windows-sub2api-manual.md`, https://favhej6sxti.feishu.cn/docx/KeLBdvZEPolp3yx6MyqcFVXXnpc, message `om_x100b68fc0697b4a0dee5efff2a8f45c`
- **feat/providers**: made the canonical native-capability contract (provider name `OpenAI`, `wire_api = "responses"`, `requires_openai_auth = false`, provider bearer, one exact actor header) change only through an explicit, previewed, revisioned commit on the focused profile, and fixed four defects each exposed by a failing test first: startup migration silently rewriting contracts of profiles never opened, compare-and-swap permanently reporting stale for any non-core-canonical profile, a confirmed pure-OAuth exit leaving the provider table and bearer in live `config.toml`, and custom-only catalogs discarding a declared model whose slug the official baseline also carries
  - why: the pinned core rewrites legacy aliases to its own `custom` shape, drops the replaced table's actor header, and defaults `requires_openai_auth` back to `true`, so any implicit normalization path destroys the mixed OAuth+key contract; there is no bulk migration — an existing profile upgrades only when the user opens it and saves, and no path writes or restores `auth.json`
  - verified: 164 Rust lib + 45 integration tests pass with the live OAuth test intentionally ignored, including golden startup/inspection/bystander no-rewrite regressions, the pure-OAuth-exit liveness test, pinned-core semantics, and sentinel-credential sweeps proving secret-free failures and zero OAuth writes; 151 frontend tests, TypeScript check, Vite production build, `cargo fmt --check`, and the full ad-hoc-signed macOS Tauri bundle build pass; a 96-scenario reconciliation is recorded in the change's implementation-notes; the on-screen manual flow (8.7/8.8) and quota-bearing capability probes (8.10) remain user-owned and unverified
  - refs: OpenSpec `prioritize-native-capabilities-for-mixed-providers`, `src-tauri/src/commands.rs`, `src-tauri/src/model_catalog.rs`, `src-tauri/src/provider_commit_transaction_tests.rs`, `src/provider-capability-claims.test.ts`, `AGENTS.md`, `README.md`

### 2026-08-11

- **fix/model-catalogs**: made native-official catalog selection a confirmed draft until Save, described every unsaved mode by its actual destination, kept dormant custom models recoverable, and wired the production mode controls without pre-Save persistence
  - why: selecting official native could display stale managed-catalog state or the wrong model source, while a control regression could silently save or discard catalog ownership before explicit confirmation
  - verified: all 32 frontend tests pass, including production-control cancel/confirm/restore coverage; TypeScript and Vite production builds pass
  - refs: `src/model-catalog-ui.ts`, `src/catalog-mode-controls.ts`, `src/App.tsx`
- **fix/providers**: made live `config.toml` the only global-config source, reduced stored profile TOML to provider-owned fields, and added an upgrade-time scrub for legacy common/context copies and global fields embedded in profiles
  - why: Manager-owned global copies became stale and made provider detail ambiguous; native official mode needs an explicit no-provider-config state while preserving live settings such as `[agents] max_concurrent_threads_per_session = 8`
  - verified: 67 Rust tests pass with one live-OAuth test intentionally ignored, including upgrade scrub, provider-only normalization, and live-global-preservation regressions; Rust formatting, diff checks, TypeScript, all 32 frontend tests, Vite production build, and the full ad-hoc-signed macOS Tauri bundle build pass
  - refs: `src-tauri/src/commands.rs`, `src/relay-config-panels.ts`, `src/App.tsx`

### 2026-08-10

- **fix/providers**: made Quick Test and Provider Doctor retry one strictly allowlisted Responses HTTP 400 without the optional `max_output_tokens` field, while preserving the original status, final endpoint, redacted preview, and an explicit compatibility marker
  - why: the configured relay returned a generic `upstream_error` for the synthetic output limit even though the same model and request succeeded when that optional field was omitted, causing the Manager to report a false provider failure
  - verified: a live controlled probe reproduced HTTP 400 with the field and HTTP 200 without it; 65 Rust tests pass with one live-OAuth test intentionally ignored, including five new classifier, retry, negative-path, redaction, Quick Test, and Provider Doctor regressions; all 19 frontend tests, TypeScript check, Vite production build, Rust formatting, diff checks, and the full ad-hoc-signed macOS Tauri bundle build pass
  - refs: `src-tauri/src/commands.rs`, `src/App.tsx`, `README.md`
- **feat/model-catalogs**: distinguished server-side composite relays from the removed local aggregation proxy, added rich official/custom metadata overlays, made per-model context authoritative in managed modes, and made external version mismatches explicit review evidence
  - why: one upstream Responses Base URL and key can legitimately route multiple vendors server-side, while global context keys and thin overlays silently distorted mixed OpenAI/Claude catalogs
  - verified: strict OpenSpec validation; 60 Rust tests pass with one live-OAuth test intentionally ignored, including composite single-provider/pointer/no-auth, 95-percent context, reasoning/tool metadata, external version, Context, journal, and auth-ownership regressions; all 19 frontend tests, TypeScript, Vite production build, Rust formatting, diff checks, and the full signed macOS Tauri bundle build pass
  - refs: OpenSpec `support-server-side-composite-catalogs`, `src-tauri/src/model_catalog.rs`, `src-tauri/src/commands.rs`, `src/App.tsx`, `src/model-catalog-ui.ts`, `README.md`

### 2026-08-09

- **fix/windows**: launched ACL, target CLI, session, proxy, and signature-verification subprocesses as detached no-window processes, cached ACL checks within one coordinator transaction and unchanged target verification results within the process, skipped the synchronous minimized-state query on normal Windows resizes, and removed unnecessary screen animation promotion
  - why: every save applied and verified owner-only paths through dozens of `icacls` calls; the x64 release opened a Windows Terminal for each call, making one save flash repeatedly and take about 20.6 seconds
  - verified: the installed native ARM64 build completed a real UI active-profile save with `ok`; all 64 `icacls` processes created zero console hosts, while the app created zero PowerShell, Windows Terminal, or visible console processes; 55 Rust tests pass with one live-OAuth test intentionally ignored, all 18 frontend tests pass, TypeScript and Vite production builds pass, and both Windows release targets cross-compile
  - refs: `src-tauri/src/platform_command.rs`, `src-tauri/src/live_state.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/model_catalog.rs`, `src-tauri/src/lib.rs`, `src/styles.css`
- **build/windows**: added native Windows ARM64 MSI and NSIS builds alongside x64, with architecture-specific artifacts, release names, checksums, and install guidance
  - why: the available Windows release was x64-only and ran through emulation on ARM devices
  - verified: local release builds produce PE32+ GUI executables for x86-64 and AArch64; workflow YAML and artifact-count checks parse cleanly
  - refs: `.github/workflows/build.yml`
- **fix/ux**: made provider saves immediately enter a disabled `保存中` state with a spinner, reject duplicate clicks, keep failures in the editor, and show an explicit success toast before returning to the list
  - why: the full active-profile transaction takes about three seconds, while the previous button stayed visually unchanged and made a successful click look ineffective
  - verified: Windows WebView UI automation confirmed the pending button text, spinner, `aria-busy`, and disabled state, then observed the success toast and detail-page exit; the save path no longer performs an unrelated model-catalog status refresh; TypeScript, all 18 frontend tests, Vite production build, and final x64 and ARM64 release builds pass
  - refs: `src/App.tsx`, `src/i18n-en.ts`

### 2026-08-08

- **fix/macos**: restored and focused the tray-hidden main window when Finder or another app copy launches the existing Manager instance, and made local macOS bundles receive a complete ad-hoc signature during `npm run build`
  - why: the inherited port guard detected duplicate launches but could not signal a hidden macOS instance, while unsigned local bundles could retain an incompatible resource signature when copied over an installed app
  - verified: 52 Rust tests pass with one live-OAuth test intentionally ignored; TypeScript check, all 18 frontend tests, Vite production build, full signed Tauri app build, strict deep code-sign verification, Gatekeeper assessment, and a real close/reopen cycle against `/Applications/Codex-- Manager.app` pass with one process and one restored foreground window
  - refs: `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`

### 2026-08-07

- **feat/safety**: added a Manager-scoped network policy with Auto / Direct / Custom modes, coherent process-or-system proxy resolution, credentialless route testing, redacted diagnostics, and immutable proxy snapshots for official catalog refreshes
  - why: official catalog refresh previously inherited whatever proxy environment happened to launch the app, turning local Stash or system-route failures into misleading model-cache errors with no inspectable or controllable Manager policy
  - verified: strict OpenSpec validation; 51 Rust tests pass with one live-OAuth test intentionally ignored, including macOS and Windows discovery fixtures, precedence, conflict, credential, direct, custom, bundled-fallback, and auth-preservation regressions; all 18 frontend tests, TypeScript check, Vite production build, full Tauri macOS app bundle build, Rust formatting, and diff checks pass
  - refs: OpenSpec `add-manager-network-policy`, `src-tauri/src/network_policy.rs`, `src-tauri/src/model_catalog.rs`, `src/network-policy-ui.ts`
- **fix**: accepted official refresh caches that omit CLI-hydrated `base_instructions` while keeping the verified target CLI's complete output authoritative and rejecting every other cache/output mismatch
  - why: ChatGPT's embedded Codex `0.147.0-alpha.6.5` can receive sparse remote cache entries and deterministically hydrate their instructions, but the manager still applied the older all-fields-present contract to both representations and failed refresh with `model has no instructions`
  - verified: an offline target-CLI probe changed only the omitted instruction field; 42 Rust tests pass with one live-OAuth test intentionally ignored, including new omission and conflict regressions; all 14 frontend tests, TypeScript check, Vite production build, and full Tauri macOS app bundle build pass
  - refs: `src-tauri/src/model_catalog.rs`

### 2026-08-06

- **release**: published Codex-- Manager `v0.3.0` for macOS arm64 and Windows x86_64, refreshed the GitHub project surface, and deployed the matching project and portfolio pages
  - why: ship managed model catalogs as one coherent, verifiable release across the application, documentation, downloads, and public product surfaces
  - verified: GitHub Actions run `31065157585` passed macOS and Windows tests and builds; the published `.app.zip`, `.msi`, and NSIS `.exe` all pass the downloaded `SHA256SUMS`, the released macOS bundle reports `0.3.0` and passes strict deep signature verification, and the live GitHub README, landing page, portfolio page, and WebP product image return the updated content
  - refs: https://github.com/nxxxsooo/codex-minus/releases/tag/v0.3.0, https://mjshao.fun/codex-minus/, https://mjshao.fun/work/codex-minus
- **fix/release**: normalized release asset names before checksum generation and aligned CI branch filters with the repository's `master` default branch
  - why: GitHub rewrites spaces in uploaded asset names, which made the first Windows checksum entries unresolvable, while the stale `main` filter skipped ordinary default-branch CI
  - verified: the corrected release has exactly one macOS archive, one MSI, one NSIS installer, and one checksum file with matching names and hashes; workflow YAML and shell packaging paths parse cleanly
  - refs: `.github/workflows/build.yml`, release `v0.3.0`
- **docs**: synced the completed model-catalog behavior contract into the main OpenSpec tree and archived `manage-model-catalogs`
  - why: preserve the nine implemented requirements as the durable capability specification while removing the completed change from the active queue
  - verified: the main spec matches the archived delta semantically, both strict validations pass, all 49 tasks are complete, and commit `9c68b8b` is pushed to `master`
  - refs: `openspec/specs/model-catalog-management/spec.md`, `openspec/changes/archive/2026-08-06-manage-model-catalogs/`

### 2026-08-05

- **feat/safety**: added managed official and per-provider model catalogs with native-official, official-plus-custom, custom-only, and explicit external-ownership modes; provider `/v1/models` is evidence only, while OAuth refreshes use a verified official target CLI in an isolated non-refreshable home
  - why: keep ordinary users on the official model list, allow deliberate provider-specific customization, and remove the ambiguity and credential risk of sharing Codex's dynamic cache across OAuth and API-backed providers
  - verified: 41 Rust tests plus one opt-in live test, 14 frontend tests, TypeScript check, Vite production build, full Tauri macOS bundle build, and Playwright desktop/responsive interaction QA; two real OAuth refreshes against ChatGPT's embedded Codex `0.147.0-alpha.1.2` produced a stable 9-entry official catalog without changing `auth.json`, while installed-app testing confirmed official-plus-custom materialization, exact external-pointer restoration, unchanged external bytes, and restart persistence
  - refs: OpenSpec `manage-model-catalogs`, `src-tauri/src/live_state.rs`, `src-tauri/src/model_catalog.rs`, `src/model-catalog-ui.ts`
- **safety**: made provider switching and active saves one crash-recoverable live-state transaction, removed profile OAuth persistence and raw `auth.json` editing, migrated pure API keys into owner-only provider config, and made Context table preservation fail closed
  - why: provider workflows must never overwrite official-client auth or silently drop MCP, Skills, Plugins, or unrelated root config
  - verified: interruption-boundary rollback, coordinator serialization, owner-only permission, auth-byte preservation, legacy migration, and protected-table regression tests pass; no credential payload or model instructions are stored in the journal
  - refs: OpenSpec `manage-model-catalogs`, `src-tauri/src/commands.rs`, `src-tauri/src/live_state.rs`
- **launch**: announced Codex-- Manager to the FIT product group and directly notified Eva
  - why: introduce the simplified Codex++ workflow to the FIT team as a lower-friction, less lag-prone option for provider switching and session management
  - verified: the FIT Mingjian bot delivered the message to `AI产品数据库（飞特卡车配件）`, message `om_x100b681d61edccacc4c327205b074a4`; the user confirmed it was visible
  - refs: https://mjshao.fun/codex-minus/

### 2026-08-04

- **release**: published Codex-- Manager `v0.2.0` with Windows x86_64 support
  - why: make the manager available on Windows; the only release was macOS-only v0.1.0
  - verified: CI built and released macOS .app.zip (6.7 MB), Windows .msi (6.2 MB), and Windows .exe (4.3 MB) with SHA256SUMS; landing page, portfolio, and README all updated to reflect dual-platform; all 12 Rust tests pass on both platforms
  - refs: https://github.com/nxxxsooo/codex-minus/releases/tag/v0.2.0, `.github/workflows/build.yml`

- **infra**: added Windows x86_64 build target and cross-platform CI/CD
  - why: make Codex-- Manager available to Windows users; the existing codebase was macOS-only
  - verified: tauri.conf.json targets include `msi` and `nsis`; windows-app-manifest relaxed from `requireAdministrator` to `asInvoker`; CI workflow builds both macOS (aarch64) and Windows (x86_64), runs Rust tests on both, and attaches artifacts to tagged releases
  - refs: `.github/workflows/build.yml`, `src-tauri/tauri.conf.json`, `src-tauri/windows-app-manifest.xml`

- **launch**: published the bilingual public project surface with real application imagery, install guidance, portfolio entry, and a dedicated landing page
  - why: make the first release understandable and verifiable outside the repository while keeping the presentation faithful to the shipped macOS app
  - verified: GitHub README renders at desktop and mobile widths in light and dark modes; `mjshao.fun/codex-minus` and `mjshao.fun/work/codex-minus` are live from portfolio commit `6cf99bb`; the landing passed HTML validation, responsive browser QA, and Lighthouse scores of 98 performance with 100 accessibility, best practices, and SEO
  - refs: https://mjshao.fun/codex-minus/, https://mjshao.fun/work/codex-minus, https://github.com/nxxxsooo/codex-minus
- **release**: published Codex-- Manager `v0.1.0` for Apple Silicon macOS and refreshed the maintainer installation
  - why: establish the first immutable public application release with consumer-facing install and update paths
  - verified: annotated tag resolves to `38d24cb`; GitHub Release is public with a 6.6 MB app zip and `SHA256SUMS`; the remote-downloaded archive passed SHA-256 and strict deep codesign verification; `/Applications/Codex-- Manager.app` reports version `0.1.0`, bundle ID `fun.mjshao.codex-minus`, architecture `arm64`, and launches successfully
  - refs: https://github.com/nxxxsooo/codex-minus/releases/tag/v0.1.0, SHA-256 `982f90a54db29a354472146253786952b640f178ef88aa51d500d27a4a85e12f`

### 2026-07-25

- **feat**: added consented 30-day session lifecycle management with active / archived cursor pagination, target-matched Codex native archive and restore, delayed 24-hour maintenance, per-operation postcondition checks, and a dedicated private settings sidecar
  - why: keep the active session set manageable without inventing archive semantics or touching real session state directly
  - verified: 12 Rust tests, TypeScript check, Vite production build, full Tauri macOS bundle build, mocked Tauri browser QA in Chinese and English at desktop and mobile widths, and a real target-CLI archive / unarchive round trip in a disposable `CODEX_HOME`
  - refs: OpenSpec `streamline-session-lifecycle`, `src-tauri/src/commands.rs`, `src/App.tsx`
- **fix**: replaced independent provider target selection and full-history repair with effective-provider identity tracking plus read-only active-session mismatch scanning
  - why: provider repair belongs to a successful switch and must scale with active mismatches, not archived history
  - verified: same-provider switch payload reports no provider change; compatibility diagnostics report zero archived rollout traversal; legacy target fields remain tolerant but ignored
  - refs: OpenSpec `streamline-session-lifecycle`
- **safety**: added a fail-closed capability gate for provider adaptation because pinned upstream provider-sync has no active-only scope
  - why: the project constraint forbids vendoring provider logic, and falling back to the upstream full-history operation would reintroduce the original repair-time problem
  - verified: old full-history commands are absent from the Tauri command registry and frontend
  - refs: `src-tauri/src/lib.rs`, upstream pin `59a2f90`

### 2026-07-16 (late)

- **deploy**: published to GitHub as public repo https://github.com/nxxxsooo/codex-minus
  - why: local-only repo; AGPL derivative is fine to publish openly
  - verified: push succeeded, repo visible as PUBLIC
  - refs: none
- **chore**: rewrote git history with git-filter-repo to drop node_modules / src-tauri/target / dist blobs committed before .gitignore existed
  - why: history carried 255MB of build artifacts; unacceptable clone size for a public repo. No remote/collaborators existed, so hash rewrite was free
  - verified: .git 255MB → 1.2MB, all 9 commits preserved; pre-rewrite backup at /tmp/codex-minus-git-backup
  - refs: none
- **chore**: added AGPL-3.0 LICENSE full text
  - why: Cargo.toml declared AGPL-3.0-only but license text was missing; required for public distribution of a derivative work
  - refs: LICENSE

### 2026-07-16

- **docs**: added AGENTS.md (constraints: pinned-rev upstream deps, context 保护罩, dead 57321 protocol path, toml_edit implicit-table gotcha) and this BOARD.md
  - why: project had no on-disk constraints/history; resume flow depended on memory only
  - verified: not applicable
  - refs: AGENTS.md, memory `codex-minus` (2026-07-15)
- **chore**: confirmed `src/presets.ts` is live code (imported via `components/ProviderPresetSelector.tsx`, used at App.tsx:2166) after a false dead-code diagnosis; restored from git untouched
  - why: initial grep missed the `../presets` import path; deletion would have broken the preset picker
  - verified: `tsc --noEmit` clean, worktree clean
  - refs: src/presets.ts, src/components/ProviderPresetSelector.tsx

### 2026-07-15

- **feat**: codex-minus built and installed as `/Applications/Codex-- Manager.app`, replacing Codex++ (Codex++.app deleted)
  - why: Codex++'s "工具与插件" (context) feature stored managed MCP copies and merged them back into config.toml on provider switch — a stale copy ate `[mcp_servers.memory]` transport on 2026-07-15 morning; feature removed wholesale in the fork
  - verified: real launch log shows `store_scrubbed` fired; 4 unit tests pin the context guard
  - refs: src-tauri/src/commands.rs (`with_context_tables_protected`), commits dba979c..5ee5db4
- **feat**: sessions screen — provider-sync repair restored (async), all IO-heavy commands moved off the main thread, session list rendering paginated
  - refs: commit 5ee5db4
- **feat**: ux — dual-mount screens (no remount jank), dropped about/settings pages (test-model field moved to relay screen), restored v1.2.35 green theme; per-route data loading parallelized
  - refs: commits 72c197f, 2d14c62
- **feat**: macOS app bundle target + icns icon; frontend trimmed 7122→5024 lines (relay/sessions/context/doctor screens only)
  - verified: tsc clean, vite + cargo build green
  - refs: commits 5453bd1, 6b78c56
