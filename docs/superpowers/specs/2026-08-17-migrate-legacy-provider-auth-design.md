# Migrate legacy provider auth copies

## Problem

Older Codex Minus and Codex++ Manager versions could leave a serialized
`authContents` object inside a saved relay profile. On Eva's Windows machine,
that residue contains `OPENAI_API_KEY` together with fields copied from the
official ChatGPT OAuth store. The current startup migration accepts only an
object whose sole field is `OPENAI_API_KEY`, so the expected upgrade residue
fails settings loading with `persisted provider auth copy is not API-key-only`.

This is product-owned migration debt, not a user configuration error. The
application must convert every legacy state whose destination is deterministic
and keep failing closed only where credentials conflict or a provider key has
no safe source.

## Desired behavior

When settings are loaded, Codex Minus migrates each non-empty profile
`authContents` before normal provider processing:

1. Parse the legacy payload as a JSON object.
2. Treat a non-empty string `OPENAI_API_KEY` as provider-key evidence.
3. Reconcile that evidence with the profile's structured `apiKey` and the
   selected provider table's `experimental_bearer_token`.
4. Write one agreed provider key into the structured field and provider bearer.
5. Remove the complete `authContents` payload. OAuth fields are discarded from
   the profile; they are never migrated, backed up, logged, returned to the
   frontend, or written into live auth.
6. Persist the migrated settings through the existing owner-only atomic write.

The live `%USERPROFILE%\.codex\auth.json` or `~/.codex/auth.json` remains the
exclusive property of the official client and must be byte-for-byte unchanged.

## Migration decisions

| Legacy profile state | Result |
|---|---|
| `authContents` contains `OPENAI_API_KEY` plus OAuth or other legacy fields; no other provider key exists | Move the legacy key into `apiKey` and the provider bearer, then remove `authContents` |
| `authContents` contains `OPENAI_API_KEY`; existing `apiKey` and/or provider bearer contains the same key | Preserve the agreed key and remove `authContents` |
| `authContents` contains `OPENAI_API_KEY`; an existing structured key or bearer differs | Fail closed without writing settings |
| `authContents` contains only OAuth/legacy fields; a structured key or provider bearer already exists | Preserve that provider key and remove `authContents` |
| `authContents` contains only OAuth/legacy fields; the profile is pure OAuth | Remove `authContents`; live official auth remains untouched |
| `authContents` contains only OAuth/legacy fields; the profile requires a provider key but none exists | Fail closed and identify the profile and missing provider key without exposing credential material |
| `authContents` is invalid JSON or is not an object | Fail closed without writing settings |

A profile requires a provider key when it is pure API or official mixed with an
API key. Existing relay-mode ownership rules remain authoritative; the
migration does not reinterpret the profile's mode.

## Data flow and ownership

The existing startup migration remains the single write path:

```text
owner-only settings.json
  -> parse BackendSettings
  -> reconcile each legacy profile auth copy
  -> sanitize provider-owned config
  -> serialize without authContents
  -> owner-only atomic replacement
```

The migration may read only the profile's legacy payload and provider-owned
fields. It does not need the live OAuth file to decide how to remove an
unauthorized profile copy, and it must not create a credential-bearing backup.
The existing coordinator lock, owner-only permission preparation, and atomic
replacement stay in force.

Provider inspection and commit-time settings loading must use the same
reconciliation rule as startup so an already-loaded process cannot observe a
different migration contract.

## Error handling

Expected upgrade residue must not surface as a safety-check failure. Failures
remain limited to ambiguous or malformed states:

- invalid JSON or a non-object legacy payload;
- an empty or non-string `OPENAI_API_KEY` when no other provider key exists;
- disagreement among legacy key, structured key, and provider bearer;
- a provider-key mode with no usable provider key after reconciliation; or
- failure of owner-only validation or atomic persistence.

User-facing errors name the affected profile and the rule that prevented
migration, but never include key values, OAuth fields, serialized auth content,
or provider TOML.

## Verification

The implementation follows test-driven development. A regression first
reproduces Eva's mixed residue and must fail under the current API-key-only
gate. The minimum evidence set is:

1. `OPENAI_API_KEY` plus OAuth fields migrates successfully, writes the key to
   the provider-owned destinations, and removes `authContents`.
2. OAuth-only residue with an existing bearer is removed while the bearer is
   preserved.
3. OAuth-only residue on a pure-OAuth profile is removed without creating a
   provider key.
4. OAuth-only residue in a provider-key mode with no usable key fails without
   changing settings.
5. Conflicting keys fail without changing settings.
6. Invalid legacy payloads fail without changing settings.
7. A sentinel live `auth.json` remains byte-for-byte unchanged across successful
   and failed migrations.
8. Persisted migrated settings contain no `authContents` field or OAuth
   sentinel, and owner-only permissions remain valid.
9. Focused Rust tests pass, followed by `cargo test` in `src-tauri/` and
   `npm run verify` for the repository acceptance gate.

## Non-goals

- Do not migrate, restore, refresh, or validate ChatGPT OAuth credentials.
- Do not change provider modes, provider IDs, catalog ownership, active profile,
  model selection, or the native-capability contract.
- Do not probe Sub2API or consume Eva's quota as part of migration.
- Do not add an editor for `authContents` or expose raw credential state in the
  frontend.
- Do not add a second settings-repair path outside the existing coordinator and
  owner-only transaction boundary.
