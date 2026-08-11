# Native Catalog Mode Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent a user from accidentally hiding an adopted/custom model catalog when selecting `native-official`, and make native-mode status accurately describe the Codex-owned dynamic catalog.

**Architecture:** Pure helpers in `model-catalog-ui.ts` decide whether a mode change needs destructive confirmation and derive the catalog header from both the selected draft and persisted active mode. Before Save, persisted mode alone describes the active catalog; `CatalogProfileEditor` consumes those helpers, never writes local files directly, and continues to persist only through the existing catalog command.

**Tech Stack:** React 19, TypeScript, Node test runner, existing Tauri catalog commands and UI primitives.

## Global Constraints

- `native-official` means the Codex-owned dynamic catalog and never promises equality with `/Users/mingjian/.codex/models_372k.json`.
- A historical Manager-generated path and restart flag are not active presentation state only after `native-official` is persisted. A pending native draft is unsaved, retains the persisted catalog as active, and uses future-tense copy.
- Selecting `native-official` while custom models or an external pointer exist must require explicit destructive confirmation; cancellation preserves the selected mode, overlay, pointer, and persisted state.
- Persisted native mode with dormant custom models must visibly explain that those models are not active; a pending native draft must instead explain that they will become dormant after Save. Both offer a draft-only route back to `official-plus-custom`.
- A segmented-control click changes frontend draft state only. No settings, `config.toml`, catalog state, generated catalog, external source, or `auth.json` write occurs until the existing Save action succeeds.
- No production code is written before a focused test fails for the expected missing behavior.

---

### Task 1: Guard Native Mode and Correct Its Presentation

**Files:**
- Modify: `src/model-catalog-ui.ts`
- Modify: `src/model-catalog-ui.test.ts`
- Modify: `src/App.tsx`
- Modify: `src/i18n-en.ts`

**Interfaces:**
- Produces: `catalogModeChangeDecision(currentMode, requestedMode, externalPointer, customModelCount) -> "select" | "confirm-discard-external" | "confirm-discard-custom"`.
- Produces: `catalogModePresentation({ selectedMode, persistedMode, generatedPath, externalPointer, restartRequired, customModelCount }) -> { source: "native" | "managed" | "external" | "unsaved"; path: string | null; restart: boolean; dormantCustomCount: number; pendingDormantCustomCount: number; pathUnavailable: "managed" | "external" | null }`.
- Consumes: the existing `CatalogModeValue`, `ProfileCatalogSummary`, `CatalogOverlayDraft`, `saveProfileCatalog`, and translation helpers.

- [ ] **Step 1: Write failing behavior tests**

Add these literal tests to `src/model-catalog-ui.test.ts`:

```typescript
it("requires confirmation before native mode abandons custom or external ownership", () => {
  assert.equal(catalogModeChangeDecision("official-plus-custom", "native-official", null, 7), "confirm-discard-custom");
  assert.equal(catalogModeChangeDecision("custom-only", "native-official", null, 1), "confirm-discard-custom");
  assert.equal(catalogModeChangeDecision("external", "native-official", "models_372k.json", 0), "confirm-discard-external");
  assert.equal(catalogModeChangeDecision("official-plus-custom", "custom-only", null, 7), "select");
  assert.equal(catalogModeChangeDecision("native-official", "native-official", null, 7), "select");
});

it("native presentation hides stale managed state and reports dormant custom models", () => {
  assert.deepEqual(catalogModePresentation({
    selectedMode: "native-official",
    persistedMode: "native-official",
    generatedPath: "model-catalogs/stale.json",
    externalPointer: null,
    restartRequired: true,
    customModelCount: 7,
  }), {
    source: "native",
    path: null,
    restart: false,
    dormantCustomCount: 7,
    pendingDormantCustomCount: 0,
    pathUnavailable: null,
  });
});

it("keeps a native draft unsaved until its managed or external catalog is persisted", () => {
  for (const persistedMode of ["official-plus-custom", "external"] as const) {
    assert.deepEqual(catalogModePresentation({
      selectedMode: "native-official",
      persistedMode,
      generatedPath: "model-catalogs/current.json",
      externalPointer: "models/external.json",
      restartRequired: true,
      customModelCount: 7,
    }), {
      source: "unsaved",
      path: null,
      restart: false,
      dormantCustomCount: 0,
      pendingDormantCustomCount: 7,
      pathUnavailable: null,
    });
  }
});

it("managed presentation exposes only the matching persisted generation", () => {
  assert.deepEqual(catalogModePresentation({
    selectedMode: "official-plus-custom",
    persistedMode: "official-plus-custom",
    generatedPath: "model-catalogs/current.json",
    externalPointer: null,
    restartRequired: true,
    customModelCount: 7,
  }), {
    source: "managed",
    path: "model-catalogs/current.json",
    restart: true,
    dormantCustomCount: 0,
    pendingDormantCustomCount: 0,
    pathUnavailable: null,
  });
  assert.equal(catalogModePresentation({
    selectedMode: "custom-only",
    persistedMode: "official-plus-custom",
    generatedPath: "model-catalogs/current.json",
    externalPointer: null,
    restartRequired: true,
    customModelCount: 7,
  }).source, "unsaved");
});
```

The production breaks caught are: mode buttons assigning native state without warning, stale generated state being rendered under native mode, and a generated path being attributed to a different unsaved mode.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```bash
node --test --experimental-strip-types src/model-catalog-ui.test.ts src/i18n.test.ts
```

Expected: FAIL because the executable `catalogModeDraftController` is not exported and the English `tf` lookups fall back to their Chinese source keys.

- [ ] **Step 3: Implement the pure decisions**

Implement exact branch order:

```typescript
export function catalogModeChangeDecision(
  currentMode: CatalogModeValue,
  requestedMode: CatalogModeValue,
  externalPointer: string | null,
  customModelCount: number,
): "select" | "confirm-discard-external" | "confirm-discard-custom" {
  if (requestedMode !== "native-official" || requestedMode === currentMode) return "select";
  if (externalPointer) return "confirm-discard-external";
  if (customModelCount > 0) return "confirm-discard-custom";
  return "select";
}
```

`catalogModePresentation` must return `unsaved` with no path/restart before consulting native state whenever selected and persisted modes differ; tests cover a selected native draft over both managed and external persisted modes. A selected native draft exposes pending, not actual, dormant custom models. It returns native state only when native is persisted, and exposes external or managed paths only for matching persisted modes. A persisted managed/external mode without its matching path returns an explicit unavailable-path marker rather than a blank value.

- [ ] **Step 4: Wire the controlled editor without persistence side effects**

In `CatalogProfileEditor`, derive presentation from `mode`, `summary?.mode`, paths, restart state, and `overlay.custom.length`.

Route every mode button through `requestMode`. For `confirm-discard-custom` and `confirm-discard-external`, call `window.confirm` with copy that states the current catalog remains active until Save succeeds. If confirmation returns `false`, return without calling `setMode`, `setModeExplicit`, or any action. If it returns `true`, update only the draft mode and explicit flag.

When `presentation.source === "native"`, render `使用 Codex 原生动态目录`; when an unsaved native draft is selected, render `目录模式尚未保存；保存后使用 Codex 原生动态目录`; otherwise render the available path or explicit missing-path copy. Render the restart badge only when `presentation.restart` is true.

When `presentation.dormantCustomCount > 0`, render a current-state inline warning. When `presentation.pendingDormantCustomCount > 0`, render future-tense `保存后，{0} 个自定义模型将暂不生效`. Both use a `恢复官方＋自定义` button that only updates the draft; it does not call a backend action. The executable controller interaction test must prove: cancelled confirmation leaves the selected draft unchanged; confirmed selection changes only the draft; both selection and restore perform zero persistence actions.

Add deterministic English translations for the new confirmation, unsaved-state, unavailable-path, dormant-model warning, and restore-action strings. Put every `tf` key with `{0}` in `EN_TEMPLATE`, and use a real `tf` lookup test under English language state rather than testing dictionary text.

- [ ] **Step 5: Verify GREEN and regression scope**

Run:

```bash
node --test --experimental-strip-types src/model-catalog-ui.test.ts
npm run check
npm test
npm run vite:build
```

Expected: all commands pass; no command mutates live Codex files.

- [ ] **Step 6: Commit the reviewed slice**

Stage only the task source, tests, and synchronized plan, inspect the cached diff, then commit:

```bash
git add docs/superpowers/plans/2026-08-11-native-catalog-mode-safety.md src/model-catalog-ui.ts src/model-catalog-ui.test.ts src/i18n.test.ts src/App.tsx src/i18n-en.ts
git diff --cached --check
git diff --cached -- docs/superpowers/plans/2026-08-11-native-catalog-mode-safety.md src/model-catalog-ui.ts src/model-catalog-ui.test.ts src/i18n.test.ts src/App.tsx src/i18n-en.ts
git commit -m "fix: guard native catalog mode transitions"
```
