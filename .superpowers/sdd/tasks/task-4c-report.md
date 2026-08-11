# Task 4c implementation report

## Completed scope

- OpenSpec 4.9: strict new-request `authContents` rejection and exact persisted API-key-only legacy migration.
- OpenSpec 4.10: active NativePriority Save／SetCurrent official-auth plus target／account／workspace scope gate, with auth-generation concurrency protection.
- OpenSpec 4.11: owner-only provider-key transaction artifacts, OAuth-free journal／catalog／diagnostic outputs, structured log and provider-surface redaction.
- OpenSpec 4.12a: generic `save_settings` raw provider snapshot gate, exact UI canonical compatibility, first-run default baseline, and concurrent-generation protection.
- OpenSpec 4.12b: all provider list／detail／SetCurrent callers use the unified ProviderCommit envelope; legacy Tauri write bypasses are unregistered; catalog drafts, frontend revision correlation, active-delete safety, and managed Context confirmation are fail closed.

## Commits

- 4.9: `55e25e5`, `3220094`, completion record `aeed7f3`.
- 4.10: `72de28a`, `f4fcf4b`, `4ac8430`, completion record `b0fb53b`.
- 4.11: `c5e3d00`, `e38b3f8`, `e62f6a9`, `18225ca`.
- 4.12a: `0196731`, `da8781b`, `2dced66`, `93e5ff5`.
- 4.12b backend／builder／wiring and fix rounds: `5621099`, `645aeab`, `3e60fcd`, `cb7aeb1`, `42ecd92`, `88a3148`, `a32f4c1`, `1c7dfa4`, `2d0e1fc`, `1a35668`, `3c04a8a`, `0d1126c`.

## Review status

- 4.9 approved after one fix round; one coverage-only Minor was explicitly deferred.
- 4.10 approved after two fix rounds with zero findings.
- 4.11a approved with zero findings.
- 4.11b approved after two fix rounds with zero Critical／Important／Minor.
- 4.12a approved after two fix rounds and one test-precision cleanup with zero Critical／Important／Minor.
- 4.12b approved after backend, TypeScript-adapter, App-wiring, command-registration, UI-safety, revision-state-machine, and managed-context fix reviews; every final scoped review is clean with zero Critical／Important／Minor.

## Current verification

- Provider transaction suite: 31／31.
- Provider compatibility／diagnostic suite: 7／7.
- Live-state suite: 7／7.
- Non-test `cargo check`, `cargo fmt --check`, and `git diff --check`: pass.
- Frontend suite: 50／50; TypeScript check passes.
- Registration boundary proves the only ordinary provider write command is `commit_provider_detail`; source-hash-bound external adoption remains the explicit exception.
- No build, deployment, live auth mutation, network probe, image generation, or release action performed.

## Remaining Task 4c scope

- OpenSpec 4.13: direct-invoke bypass and stale-fingerprint regression matrix.
