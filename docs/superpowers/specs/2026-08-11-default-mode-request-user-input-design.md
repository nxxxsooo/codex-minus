# Default-Mode Request-User-Input Default Design

## Problem

Codex CLI 0.147.0 exposes the under-development feature flag
`features.default_mode_request_user_input`. When enabled, the root agent may
use the structured `request_user_input` tool in Default collaboration mode as
well as Plan mode.

Codex-- Manager currently has no ownership rule for this leaf. Adding it only
to provider templates or profile drafts would be incomplete:

- existing profiles would not receive it;
- provider switching deliberately preserves live non-provider configuration,
  so a profile-local `[features]` entry is not an authoritative live write;
- active model-catalog operations can rewrite live `config.toml` through a
  path separate from provider switching; and
- changing `config.toml` during Manager startup would violate the product's
  no-implicit-live-write behavior.

The Manager needs one narrow, explicit default at the live configuration
boundary without taking ownership of the rest of `[features]`.

## Desired Behavior

Whenever an explicit Manager operation commits live `~/.codex/config.toml`,
the committed document contains:

```toml
[features]
default_mode_request_user_input = true
```

The behavior is intentionally not user-configurable in the Manager UI.

- If `[features]` is absent, the Manager creates it.
- If the leaf is absent or `false`, the Manager writes `true`.
- If the leaf is already `true`, the transformation is byte-for-byte a no-op.
- Existing sibling feature keys remain unchanged.
- An invalid `features` shape or a non-boolean value fails closed.
- Merely launching the Manager does not modify live `config.toml`.
- The default first appears on the next explicit operation that already owns a
  live-config write.

An explicit `false` supplied through a Manager-owned live write is normalized
to `true`. This is a product default, not a fallback applied only when the key
is missing.

## Ownership and Invariants

The Manager owns exactly one TOML leaf:

`features.default_mode_request_user_input`

It does not own the `[features]` table as a whole. In particular, values such
as `goals`, `multi_agent`, `memories`, and future unknown feature keys remain
user- or Codex-owned.

Every affected transaction preserves the existing hard constraints:

- `[mcp_servers]`, `[skills]`, and `[plugins]` are re-grafted from the live
  Context snapshot and verified after the candidate is finalized;
- all other non-provider live root configuration remains unchanged except for
  the one managed leaf;
- `auth.json` remains byte-for-byte unchanged on provider operations;
- owner-only permissions, the process-wide coordinator, transaction journal,
  rollback, and crash recovery remain in force; and
- external model-catalog paths and files remain untouched.

## Architecture

### Pure TOML transformation

Add one backend helper with a narrow contract equivalent to:

```rust
fn apply_live_config_defaults(contents: &str) -> anyhow::Result<String>
```

The helper parses with `toml_edit`, locates the `features` table through a
table-like interface that supports both standard and inline tables, and
enforces the boolean leaf. It returns the original input without rendering
when the value is already `true`, guaranteeing an exact idempotent fast path.

When mutation is required, the helper changes only the target leaf and renders
the full `DocumentMut`. Sibling values, table ordering, comments, and unrelated
document formatting are preserved by `toml_edit` to the extent supported by
the mutation of that leaf. A comment decorating the target assignment should
be retained when replacing `false` with `true`; no other item decoration may
change.

The helper rejects rather than repairs:

- syntactically invalid TOML;
- a `features` item that is not table-like; or
- an existing target leaf whose value is not boolean.

This helper is independent of provider, catalog, filesystem, and transaction
state, so its behavior can be exhaustively unit-tested.

### Provider and manual-config write boundary

The `commands.rs` paths that already call `context_protected_config` finalize
the protected candidate with `apply_live_config_defaults` before constructing
the live `config.toml` `FileMutation`. This covers:

- manual live config save;
- active provider Save and Set as current;
- relay apply paths that share the active-provider transaction; and
- relay clear.

The order is significant:

1. build the provider/manual candidate;
2. restore all live non-provider roots and exact Context tables;
3. inject the single Manager-owned feature leaf;
4. validate the final document and Context postconditions; and
5. commit through the existing verified journal transaction.

Applying the default before Context/non-provider restoration would allow the
live `[features]` table to overwrite it, so that ordering is forbidden.

### Active model-catalog write boundary

Active model-catalog Save and external-adoption operations can commit
`config.toml` directly from `plan_active_profile_with_state`. Their final live
candidate must pass through the same pure transformation immediately before
the `FileMutation` is added.

The catalog planner remains provider-neutral. The shared helper is applied at
the direct active-catalog live commit sites after planning and immediately
before each live `config.toml` `FileMutation` is added. Provider switching
continues through the `commands.rs` finalization path, so it receives the
default only after non-provider restoration.

The catalog transformation continues to own only `model_catalog_json`; adding
the feature default does not alter catalog composition, generated files,
external ownership, or restart semantics.

### No startup migration or stored-profile rewrite

`scrub_managed_context_store` and ordinary settings-only saves do not write
live `config.toml` and must not trigger this default. The implementation does
not rewrite every stored provider `configContents`, add a frontend field, or
introduce a new Tauri command.

This keeps the change on existing explicit live-write paths and avoids a new
source of truth. Live backfill may later copy the leaf into a stored profile as
part of existing behavior, but correctness does not depend on that copy.

## Data Flow

For an explicit live-config operation:

1. The existing command acquires the process-wide coordinator and performs
   crash recovery.
2. Provider or catalog logic produces its candidate and planned side-file
   mutations.
3. Existing live non-provider configuration and protected Context tables are
   retained according to that write path's contract.
4. `apply_live_config_defaults` enforces the target leaf on the final live
   candidate.
5. The final candidate is parsed again or otherwise checked for the target
   boolean and preserved Context invariants.
6. The existing transaction commits all mutations atomically.
7. Any planning, validation, write, or postcondition failure rolls the entire
   generation back.

No network request, CLI subprocess, or application restart is part of this
flow. Codex reads the feature on a subsequent session/config load according to
its own lifecycle.

## Error Handling

- Invalid live or candidate TOML aborts before a mutation is committed.
- A scalar, array, or other non-table `features` item aborts the operation.
- A non-boolean existing target leaf aborts the operation instead of silently
  changing an invalid user value.
- A failed Context or auth postcondition rolls back the complete transaction.
- Error messages identify the managed feature path but must not include auth
  contents, provider bearer tokens, or other configuration secrets.
- No best-effort secondary write is attempted after failure.

## Testing

Pure transformation tests cover:

- an empty document gains the standard `[features]` table and `true` leaf;
- an existing `[features]` table without the leaf gains it while preserving
  sibling keys and comments;
- `false` becomes `true` while retaining target decoration;
- `true` returns the exact original bytes;
- repeated application is byte-for-byte idempotent;
- an inline `features` table is supported and preserves siblings;
- a non-table `features` item fails;
- a non-boolean target value fails; and
- invalid TOML fails.

Write-boundary regression tests cover:

- provider switch/save output contains the default;
- manual live config save cannot persist `false` for the managed leaf;
- relay clear retains or creates the default;
- active native-official, managed, and external catalog pointer writes retain
  the default;
- sibling feature keys survive every covered path;
- Context tables remain semantically and textually identical under the
  existing full-document rendering comparison;
- unrelated live roots remain unchanged except for the target leaf;
- live OAuth bytes remain unchanged; and
- a forced transaction/postcondition failure restores the previous complete
  generation.

Repository verification runs:

- `cargo test` in `src-tauri/`;
- `cargo fmt --check` in `src-tauri/`;
- `npm run check`;
- the existing frontend test command; and
- `npm run vite:build`.

## Scope

This change adds one always-on Manager default for Codex's
`default_mode_request_user_input` feature. It does not add a UI toggle, change
collaboration-mode selection, alter when the agent chooses to ask a question,
modify approval policy, migrate configuration at startup, change provider or
model-catalog ownership, merge or archive any OpenSpec change, install an app,
or push a branch.
