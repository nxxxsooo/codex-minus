## Context

See `proposal.md` for motivation. Official refresh currently clears the target CLI environment and then copies a small safe allowlist from the Manager process. This is safe for credentials but cannot recover shell-only proxy state when the Manager is launched as a GUI application, does not project static OS proxy settings, and reports a successful-CLI/bundled fallback as a missing-cache implementation error.

The pinned `codex-plus-core` constructs provider-discovery and Provider Doctor HTTP clients internally from process environment. The project constraint forbids vendoring or duplicating that provider logic, and temporary global environment mutation would race other async commands. V1 therefore integrates official catalog refresh and a new Manager-owned connection test only; the shared resolver is designed for later upstream client injection.

## Goals / Non-Goals

**Goals:**

- Resolve one deterministic Manager network policy on macOS and Windows.
- Preserve the credential-safe isolated refresh boundary and fail before token projection when policy resolution is invalid.
- Make policy source and transport failure understandable without persisting raw network errors or secrets.
- Keep the implementation extensible to upstream provider clients once they accept injected network configuration.

**Non-Goals:**

- Proxy live Codex conversations or change Codex/provider configuration.
- Modify, enable, disable, or repair Stash, Clash, Windows, or macOS network settings.
- Evaluate PAC JavaScript, WPAD, proxy auto-discovery, or authenticated proxies in v1.
- Reimplement provider discovery or Provider Doctor calls owned by `codex-plus-core`.

## Decisions

### 1. Store policy in a dedicated owner-only Manager sidecar

Add a versioned `network-policy.json` beside existing Manager state. It stores `mode`, `customProxyUrl`, and normalized `customNoProxy` only. The file is written atomically and verified owner-only through existing `live_state` permission helpers.

The pinned upstream `BackendSettings` cannot safely own new fields: unknown fields can be lost on a later upstream save, and modifying the upstream settings type violates the dependency ownership boundary. Frontend local storage was rejected because the backend must resolve policy before launching credential-bearing children.

### 2. Separate saved policy, discovered sources, and resolved snapshots

Implement a local `network_policy` module with three representations:

- Saved policy: user intent only.
- Discovery report: normalized process and platform observations, including unsupported PAC/WPAD state.
- Resolved snapshot: one immutable source, normalized child environment, redacted display fields, and an action-required error when resolution is unsafe.

`auto` selects a complete process source first, then one platform source, then direct fallback. It never fills missing variables from a lower-priority source. Conflicting case variants fail closed. `direct` produces an explicit no-proxy snapshot. `custom` validates URL structure and rejects user information before persistence or resolution.

### 3. Use native static proxy discovery adapters

On macOS, query the active SystemConfiguration proxy dictionary and map enabled static HTTP, HTTPS, and SOCKS endpoints plus exception hosts. On Windows, read the current user's active static Internet Settings proxy and override list through the existing Windows integration boundary. Platform adapters return structured values rather than preformatted environment strings.

If the selected platform state enables PAC, WPAD, or an unprojectable proxy form, the adapter returns `unsupported` instead of direct fallback. This avoids falsely claiming that the isolated Rust CLI follows the same routing as WebKit or WinINet.

### 4. Apply snapshots explicitly, never through global environment mutation

Refactor bounded child execution to accept an optional resolved network snapshot. Child construction starts from the existing safe non-network environment, removes all inherited proxy variants, then inserts only the snapshot's canonical variables. The refresh operation resolves once before reading/projecting OAuth and uses the same snapshot through command execution and result reporting.

Global `set_var`/`remove_var`, shell startup-file parsing, and ambient mutation were rejected because they create cross-command races and broaden credential/network scope.

### 5. Keep diagnostics credentialless and operation-specific

Add Tauri commands to load resolved status, save policy, and run a user-triggered connection test. The test uses a fixed official HTTPS origin without OAuth; any valid HTTP response after proxy negotiation and TLS counts as transport success. Errors are mapped to stable categories such as `dns`, `proxy-connect`, `proxy-auth-unsupported`, `tls`, `timeout`, `unsupported-policy`, and `other`.

The backend returns only a redacted endpoint, source, bypass count, category, duration, and action. Raw proxy URLs, query strings, response bodies, OAuth, API keys, and unbounded CLI stderr never cross the command boundary or enter persisted diagnostics.

### 6. Place controls in Config Doctor, not a new top-level tab

Add a compact Network section with an `Auto / Direct / Custom` segmented control, custom endpoint and bypass fields shown only for `custom`, resolved-source status, and a `Test connection` action. Saving and testing are distinct; opening the view performs discovery only and makes no request.

The UI labels the capability `Manager network` and states that it does not change Codex conversation routing or OS settings. Existing copy that treats every proxy environment variable as an error is removed or revised where reachable.

### 7. Treat missing remote cache as a classified fallback

The target CLI may return bundled models with exit status zero when remote refresh is unavailable. If isolated `models_cache.json` is absent, refresh returns the stable `bundled-fallback` network category with the resolved source and retains the prior baseline. This remains fail-closed and does not relax target trust, output/cache validation, or OAuth invariants.

## Risks / Trade-offs

- [Static OS discovery differs across platform versions] -> Isolate adapters, use fixture-driven parsing tests, and report unsupported instead of guessing.
- [A configured local proxy can accept TCP but fail upstream] -> The explicit connection test and refresh fallback category distinguish configuration presence from route health.
- [SOCKS support can differ between the Manager HTTP client and target CLI] -> Validate allowed schemes, test the exact resolved route, and treat target-CLI failure as authoritative for refresh.
- [Process proxy URLs can contain credentials] -> Use them only ephemerally, redact them completely, never persist them, and keep custom persisted URLs credential-free.
- [Provider probes still behave differently] -> State the v1 boundary in UI/specs and add integration only after upstream exposes explicit client/proxy injection.
- [A fixed test origin can change behavior] -> Count any HTTP response as transport success and keep the origin an internal constant that can change without altering policy semantics.

## Migration Plan

1. Ship with no policy file; absence resolves to `auto`, preserving process-environment behavior while adding OS discovery.
2. Add backend status/save/test commands and child snapshot injection behind the new default behavior.
3. Add the Network UI and replace opaque catalog fallback copy.
4. Roll back by removing the UI/commands and sidecar reader; an existing non-secret sidecar is ignored and can be removed by a later cleanup without affecting Codex or provider state.
