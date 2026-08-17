# Reset Unowned Legacy Model Overlay Design

## Problem

Eva's pre-Codex-Minus profile retained `model = "gpt-5"` and legacy model-list
state even though FIT assigned `gpt-5.6-terra`. Version 0.4.14 correctly repaired
the profile's copied authentication payload without changing model ownership.
Once settings could load again, the existing model rules made the stale state
visible:

- an existing profile takes its startup model from top-level provider TOML;
- the legacy catalog migration converts every `modelList` slug absent from the
  bundled official baseline into a visible custom row; and
- `gpt-5` is absent from the bundled baseline, so it became a custom model and
  remained the selected startup model.

The result looked like the updater introduced GPT-5. In fact, the updater
revealed and preserved an old, manager-generated model state that the user had
never explicitly adopted.

## Decision

Reset only **unowned legacy model-list state** to the bundled official model
baseline. Do not reset every custom catalog and do not special-case Eva by
profile name.

For an affected mixed provider:

- provider mode, identifier, endpoint, bearer, actor header, ChatGPT login, and
  `requires_openai_auth` remain unchanged;
- catalog mode remains `official-plus-custom` so the Responses provider keeps
  its existing managed-catalog behavior;
- automatic rows whose exact provenance is `legacy-model-list` are removed;
- official overrides derived only from the same unowned legacy list are
  removed;
- the deprecated `modelList` and `modelWindows` copies are cleared so the
  discarded rows cannot be recreated; and
- when the selected startup model is one of the removed legacy rows, the
  startup model becomes `gpt-5.6-terra`.

“Reset to official” refers only to model catalog content. It does not switch the
profile to pure official OAuth or the reserved lowercase `openai` provider.

## Ownership Boundary

The reset is eligible only where every signal says the catalog was generated
implicitly by an old manager version:

1. the profile is an ordinary Responses mixed provider (`Official` with
   `official_mix_api_key = true`), not pure OAuth, pure API, aggregate, or Chat
   Completions;
2. the profile has non-empty legacy `modelList` and/or `modelWindows` data, or
   its catalog state contains rows with exact
   `template_provenance = "legacy-model-list"`;
3. the profile catalog mode is not `external` and has no unadopted external
   pointer;
4. the catalog state is not explicitly owned (`mode_explicit = false`); and
5. every row being removed has exact legacy provenance.

The migration preserves:

- every `user-created` custom row;
- every known-model preset copied into a profile;
- every custom row with unknown or empty provenance, because ambiguity is not
  deletion authority;
- explicit catalog modes and overlays;
- external catalogs and pointers; and
- an explicit custom default model whose row survives the reset.

If legacy and user-owned rows coexist, remove only the exact legacy rows and
retain the user-owned rows. The resulting catalog is the official baseline plus
those retained explicit custom rows.

## Canonical Default

`gpt-5.6-terra` remains the canonical default for an ordinary mixed provider.
The frontend already defines it as the first Pro model. The backend migration
will use the same literal, guarded by a cross-boundary regression that proves:

- the frontend default is `gpt-5.6-terra`;
- the bundled official catalog contains it as a visible model; and
- the backend legacy-reset default is byte-identical.

Do not choose the first catalog row or the first surviving legacy slug; those
orders answer display priority, not FIT's assigned default.

The default changes only when the current default refers to a row removed by
this migration. An already-valid official default or a retained explicit custom
default remains unchanged.

## Architecture

### Pure planner

Add one backend rule that accepts the persisted settings, catalog state, and
bundled official slugs and returns a migration plan:

- affected profile IDs;
- next overlays and legacy-field cleanup;
- old and next startup models;
- whether the active runtime generation changes; and
- the exact file mutations required.

The planner is deterministic and performs no I/O. Inspection, tests, startup,
and commit-time safety checks consume the same decision function.

### One-time migration marker

Bump the catalog-state schema and record that the legacy reset has been
evaluated for each profile. A profile that was safely reset is not reconsidered
on every startup. A profile skipped because ownership was explicit or ambiguous
is recorded as preserved, not repeatedly offered for deletion.

New profiles never enter this migration because they have no legacy model-list
fields and already default to Terra.

### Atomic persistence

Run the model reset only after the 0.4.14 authentication scrub has removed any
profile OAuth copies. Then persist all affected model state as one owner-only
generation under the process-wide coordinator:

```text
legacy auth scrub
  -> plan legacy model reset
  -> stage settings + catalog state + generated catalog
  -> context-protected live config for an affected active profile
  -> verify generation + rollback on any failure
```

For an inactive profile, update only its stored settings/catalog generation.
For an affected active profile, change only the top-level startup model and the
manager-owned catalog pointer/materialization needed by that profile. Preserve
the complete provider table and re-graft live MCP, skills, and plugins tables
through the existing Context protection boundary.

Never write live `auth.json`. Never create a recovery artifact until copied
OAuth residue has already been removed.

If live config changed concurrently or no longer selects the expected active
profile, abort and roll back the complete model-reset generation rather than
clobbering the newer state.

### Restart behavior

An active default-model or catalog change sets the existing restart-required
marker. Codex Minus does not force-quit Codex automatically. The existing
restart action remains the only relaunch path, with its normal confirmation.

## User Experience

After migration, the provider editor shows the bundled official list (plus any
retained explicit custom rows), with `5.6 Terra` selected. `GPT-5` is absent
when it existed only as an unowned legacy row.

The app reports one concise migration notice:

> 已丢弃旧版自动生成的模型列表，并恢复官方模型；启动模型已设为 5.6 Terra。请重启 Codex 后新建任务。

Do not present this as a provider, account, quota, or authentication repair.

## Failure Handling

- Invalid settings, catalog state, or provider TOML fails without mutation.
- An official baseline that does not contain visible `gpt-5.6-terra` fails the
  migration build/test contract; runtime does not invent another default.
- A removed legacy row mixed with ambiguous ownership is preserved and reported
  for manual review rather than deleted.
- Multi-profile migration is all-or-nothing on disk.
- Errors and logs contain profile labels and static rule names, never bearer,
  OAuth, raw TOML, or serialized catalog content.

## Verification

Implementation follows test-driven development. The minimum evidence set is:

1. Eva fixture: implicit legacy state containing `gpt-5`, top-level
   `model = "gpt-5"`, and official model rows resets to official content with
   `gpt-5.6-terra` selected.
2. The Eva fixture preserves provider ID `OpenAI`, endpoint, bearer, actor
   header, `requires_openai_auth`, unrelated provider fields, and live
   `auth.json` bytes.
3. `gpt-5` is absent from the resulting catalog and generated artifact.
4. A user-created Claude/custom row survives while an adjacent
   `legacy-model-list` row is removed.
5. Explicit mode, external mode/pointer, known preset, and unknown-provenance
   rows remain byte/semantically unchanged.
6. A valid official or retained custom default remains unchanged.
7. Legacy `modelList` and `modelWindows` are cleared after successful reset and
   cannot recreate discarded rows on a second load.
8. A second migration run is a byte-identical no-op.
9. An affected active profile updates settings, catalog state, generated
   catalog, and live config in one transaction; injected failures at every
   boundary restore the complete prior generation.
10. Context tables and live `auth.json` remain byte-identical.
11. Ordinary profiles without eligible legacy state take a byte-identical
    no-op path and gain no new save/switch gate.
12. The frontend/backend/default-catalog Terra contract is cross-checked.
13. Full Rust tests, `npm run verify`, strict OpenSpec validation, formatting,
    diff checks, and three-platform CI pass.

## Specification And Documentation Changes

- Update the model-catalog specification with unowned legacy-reset, explicit
  ownership preservation, default repair, atomicity, and idempotency scenarios.
- Update the provider-native-capability specification only where it must permit
  this explicit legacy-model exception to the ordinary no-startup-rewrite rule.
- Update `AGENTS.md` to state the narrow exception: startup may reset only exact
  unowned `legacy-model-list` catalog state under the coordinator; it must never
  normalize or rewrite explicit/user-owned model state.
- Append completed behavior and verification to `BOARD.md` only after delivery.

## Non-Goals

- Do not remove every custom model or reset explicit overlays.
- Do not switch mixed providers to pure official OAuth.
- Do not change Sub2API groups, model mappings, quotas, or account pools.
- Do not make provider `/v1/models` authoritative.
- Do not silently restart Codex.
- Do not special-case profile names, users, tenants, or Eva in production code.
- Do not alter session history or existing task model selections; the repaired
  default applies to new tasks after the required restart.
