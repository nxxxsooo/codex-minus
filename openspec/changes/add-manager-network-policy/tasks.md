## 1. Network Policy Core

- [x] 1.1 Add versioned network-policy models, owner-only sidecar load/save, redacted status payloads, and default `auto` migration behavior.
- [x] 1.2 Implement custom URL and bypass validation, credential rejection, process-environment normalization, coherent-source precedence, and immutable resolved snapshots with unit tests.
- [x] 1.3 Implement macOS and Windows static system-proxy discovery adapters with fixture-driven tests for endpoints, bypass entries, PAC/WPAD, disabled, and unsupported states.
- [x] 1.4 Add an explicit proxy-aware Manager HTTP client and credentialless fixed-origin connection test with stable redacted failure categories.
- [x] 1.5 Register async Tauri status, save, and connection-test commands and cover invalid-save rollback plus non-networking status reads.

## 2. Official Catalog Refresh Integration

- [x] 2.1 Refactor bounded child construction to receive an explicit network snapshot while preserving the existing safe non-network environment allowlist.
- [x] 2.2 Resolve and validate the policy before OAuth projection, inject only canonical snapshot variables, and keep direct mode free of ambient proxy values.
- [x] 2.3 Classify successful-CLI/missing-cache output as bundled fallback and return redacted policy-specific guidance while preserving baseline, auth, and temporary-state invariants.
- [x] 2.4 Add regressions for process, system, custom, and direct snapshots; conflicting or unsupported policy; bundled fallback; credential filtering; and cleanup.

## 3. Manager Network UI

- [x] 3.1 Add typed frontend policy/status/test state helpers with tests for mode validation, redacted presentation, dirty state, and action-required states.
- [x] 3.2 Add a compact Config Doctor Network section with Auto / Direct / Custom controls, conditional endpoint and bypass fields, resolved-source status, save, and explicit connection test.
- [x] 3.3 Add Chinese and English scope, validation, transport-category, bundled-fallback, and unsupported-feature copy without reviving removed launcher/proxy features.

## 4. Verification And Completion

- [x] 4.1 Run strict OpenSpec validation, Rust formatting and tests, frontend tests, TypeScript check, and Vite production build; fix all regressions.
- [x] 4.2 Withdrawn — the feature was removed by `simplify-into-codex-minus` before this manual pass ran. What it would have verified no longer exists, except "direct mode strips ambient proxy values", which is now the only mode and is covered by `the_isolated_child_inherits_no_proxy_and_no_credentials`.
- [x] 4.3 Append the completed behavior and verification evidence to `BOARD.md` without changing release version or installing over the current application.
