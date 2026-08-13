# Task 5a implementation report

## Completed scope

- OpenSpec 5.1–5.2: brand-new empty mixed providers use an explicit transient native-priority target and emit canonical OpenAI Responses TOML only after Base URL, provider bearer, and model are complete.
- OpenSpec 5.3–5.4: only a brand-new empty draft may synchronously create the actor header; existing TOML actor／auth transitions use the revisioned backend transformer, while ordinary provider edits preserve owned and unowned fields.
- OpenSpec 5.5: Base-URL-only edits preserve `requires_openai_auth = false`.
- OpenSpec 5.6: provider-detail ownership of `[features].goals` was removed; provider edits cannot introduce or change the global feature.
- OpenSpec 5.7: backend inspection／preview／blocker metadata is response-only, asynchronous detail transforms are session／profile／revision correlated, raw existing-TOML edits are backend validated, and Save／SetCurrent fail closed while a candidate is pending or blocked.
- First-save raw TOML editing is intentionally read-only; new providers are created through the structured canonical field flow. Existing provider raw editing remains revisioned and backend validated.

## Commits

- 5.1–5.2: `0c5cb79`, `055ce5d`, `537b54a`.
- 5.3–5.4: `6ab6e02`, `b5acd68`.
- 5.5–5.6: `b62ea98`.
- 5.7 state machine and App wiring: `7658d43`, `2228d79`, `5d078bd`, `3228853`, `6980346`.
- 5.7 raw-draft security and first-save UX fixes: `837fcf6`, `5a5dbf8`, `9247ed7`.

## Review status

- Every 5.1–5.7 mini-slice passed a fresh scoped review after its fix rounds.
- The final 5.7 state-machine review passed Spec／Quality with zero Critical／Important／Minor.
- The final raw-source binding and first-save read-only slice `5a5dbf8..9247ed7` passed Spec／Quality with zero Critical／Important／Minor.
- No Task 6 evaluator, capability-status, default-policy, or broader UX redesign was included.

## Current verification

- Frontend suite: 84／84; `npm run check` passes.
- Provider native-capability Rust suite: 20／20; non-test `cargo check` and `cargo fmt --check` pass.
- Final scoped diff checks pass and the implementation worktree was clean before this completion record.
- No build, package, install, deployment, live auth/config mutation, network probe, image generation, or release was performed.

## Remaining Task 5 scope

- OpenSpec 5.8 remains pending: Responses → Chat Completions must be an explicit compatibility exit with capability-loss preview.
- Task 6 and later integration／product-policy work remain pending and outside this completion record.
