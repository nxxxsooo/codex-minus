## Status: withdrawn (superseded by `simplify-into-codex-minus`, 2026-08-13)

Shipped and then removed without ever being archived, so its spec deltas MUST NOT be applied. The
premise below — that isolated Codex CLI children need a configurable proxy so official catalog
refreshes can succeed — stopped holding when the official baseline moved into the application as a
bundled asset. With no runtime refresh left to configure, the policy had no production consumer: the
provider connection test and Provider Doctor run on the pinned core's own HTTP client, which reads
the process environment directly and never consulted this policy. The panel therefore described a
route the user could not reach and could not affect.

Saved `network-policy.json` is left on disk untouched and unread. The one thing that outlived the
feature is `reqwest`'s `socks` feature in `src-tauri/Cargo.toml`: the pinned core does not enable it,
so that entry is what gives provider connection tests SOCKS support, and it stays.

## Why

Codex-- Manager starts network work from a desktop GUI, where shell-only proxy variables are not reliably visible, while its isolated Codex CLI children currently receive only proxy variables inherited by the Manager process. This makes valid official catalog refreshes fail as opaque cache errors on machines that depend on an operating-system or local proxy.

## What Changes

- Add one Manager-owned network policy with `auto`, `direct`, and `custom` modes for outbound requests initiated by Codex-- Manager and isolated child CLIs.
- Make `auto` resolve process proxy variables first, then supported static operating-system proxy settings, then direct access; preserve bypass rules and report the resolved source.
- Add a compact Network section to Config Doctor / advanced settings with the current mode, resolved source, redacted endpoint, connection test, and actionable failure state rather than a separate top-level proxy product area.
- Apply the resolved policy to official model-catalog refresh and a Manager-owned connection test without changing live Codex conversation routing, system proxy settings, `config.toml`, or `auth.json`.
- Keep v1 custom proxy configuration non-credential-bearing; reject embedded credentials and report PAC/WPAD and authenticated proxies as unsupported rather than silently misapplying them.
- Distinguish a target CLI network fallback to bundled models from a valid remote refresh, retain the last validated baseline, and surface a sanitized network-specific error.

## Capabilities

### New Capabilities

- `manager-network-policy`: Resolve, apply, test, display, and safely persist Manager-scoped outbound proxy policy across supported desktop platforms.

### Modified Capabilities

- `model-catalog-management`: Apply the Manager network policy to isolated official refresh and report remote-refresh fallback without weakening credential isolation or catalog validation.

## Impact

- Backend: shared network-policy resolver, macOS and Windows static system-proxy discovery, owner-readable non-secret settings, sanitized connectivity diagnostics, and child-process environment construction.
- Frontend: Config Doctor / advanced Network controls and refresh error/status copy.
- Existing flows: official catalog refresh uses the resolved Manager policy; provider model discovery and Provider Doctor remain on the pinned upstream client's environment behavior until upstream exposes safe client injection; live Codex routing and OAuth ownership remain unchanged.
- Tests: precedence, bypass, unsupported proxy forms, secret rejection, cross-platform discovery adapters, child environment, network fallback, redaction, and UI state coverage.
