# Implementation Evidence

## Integration baseline

- Intended integration branch: `master` at `cea0c86db4e8f80126ae8f412ea7dbf643007973`.
- Raw JSON model-catalog and provider/live ownership prerequisite: `codex/raw-json-model-catalog` at `60087e2f131113cbefd4c16ffede34796e80f0cc`, merged by `3fc4cc8`.
- Provider-onboarding prerequisite: source commit `1843af7d70a9e1911a6e0f2cddf114b30e534262`, semantically transplanted as `b5e2443` after preserving the raw-JSON `providerConfigDraft` ownership boundary.
- Unified provider-detail Save design baseline: `8b61954cb2bd87c9073fb8fbebe12c5d07ed6820`; its implementation is owned by this change rather than treated as a landed dependency.
- Isolated implementation worktree: `/Users/mingjian/Documents/sync/GitHub/codex-minus/.worktrees/native-capability-priority` on `codex/native-capability-priority`.

## Verification record

### Integrated prerequisite baseline

- `npm test`: 35 passed, 0 failed.
- `npm run check`: passed.
- `npm run vite:build`: passed; the only emitted warning is Node's upstream `DEP0205` deprecation notice.
- `cargo build`: passed after the required frontend `dist/` was generated.
- `cargo test`: 67 passed, 0 failed, 1 intentionally ignored live-OAuth catalog test.
- Source audit found no native-capability dependency on a standard-Pro artifact, signed-update channel, or 372k readiness policy. The existing `models_372k.json` string is only an external-catalog filename fixture.
- `npm install` reported four pre-existing dependency audit findings (one low, three high); no dependency-changing `npm audit fix` was run.

### Apply-time architecture correction

- Backend call-site audit found that hardening generic `save_settings` without a replacement would break provider enablement, reorder, copy, delete, aggregate cleanup, and provider test-model mutations.
- The change therefore uses one provider-owned canonical topology envelope with an expected provider-state fingerprint. Thin detail/topology commands share one planner and transaction engine; frontend revision is correlation metadata only.
- This correction closes an implementation gap in the original artifacts without changing OAuth, Context, external-catalog, or native-capability contracts.

Focused, final full-suite, bundle, manual-flow, and scenario reconciliation evidence is appended here as the corresponding task groups complete.

### Provider-routable capability matrix (redacted)

Row scope is the point of this table. A complete native-capability contract is evidence about the
contract, not about any row below. Each row is `success`, `denial`, `fallback`, or `unknown` on its
own observation; no row may be inferred from another, and none may be inferred from the contract,
the actor marker, or the local plan.

Recorded from this change's automated verification only. No quota-bearing probe was run: task 8.10
requires separate explicit approval, and without it every row that needs a paid call stays
`unknown` rather than being reported from a text probe.

| Row | Outcome | Basis |
| --- | --- | --- |
| Text Responses | unknown | Modeled as `TextResponsesEvidence`; never derived, only observed. No request was issued. |
| Model discovery | unknown (`missingMetadata`) | A materialized catalog artifact proves an artifact exists, not that the selected model carries capability metadata. |
| Image generation | unknown | Modeled as `ImageGenerationEvidence` plus `ImagePlanEvidence`; both stay `unknown` without a verified target policy. Permission, `gpt-image-2` allowance, and tool registration are unverified (8.8). |
| Image editing | unknown | Not modeled by this build. Nothing in any payload can report it, so it cannot be claimed. |
| Remote compaction | unknown | Not modeled by this build. Same guarantee by construction. |
| Web search | unknown | Not modeled by this build. Same guarantee by construction. |

Redaction: no provider key, OAuth token, account or workspace identity, or endpoint value appears
in any row or in the evidence payload these rows are derived from (verified by the sentinel sweep
in 8.6).

Automated proof: `capability_rows_are_scoped_and_never_inferred_from_a_complete_contract`
(`src-tauri/tests/provider_capability_evidence.rs`) holds every row at `unknown` while the contract
is `ready`, the actor marker is `eligible`, and the route is `nativePriorityMixed`.
`src/provider-capability-claims.test.ts` proves no shipped copy claims a subscription upgrade or a
blanket grant of Pro capabilities.
