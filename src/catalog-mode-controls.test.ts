import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";
import { Children, isValidElement, type ReactElement, type ReactNode } from "react";

import type { CatalogModeValue } from "./model-catalog-ui.ts";

const controlsModule = await import("./catalog-mode-controls.ts").catch(() => null);

type TestElement = ReactElement<Record<string, unknown>>;

function findButton(node: ReactNode, attribute: string, value = "true"): TestElement {
  if (isValidElement<Record<string, unknown>>(node) && String(node.props[attribute]) === value) return node;
  if (isValidElement<Record<string, unknown>>(node)) {
    for (const child of Children.toArray(node.props.children as ReactNode)) {
      try {
        return findButton(child, attribute, value);
      } catch {
        // Continue searching sibling branches.
      }
    }
  }
  throw new Error(`button ${attribute}=${value} not found`);
}

function click(element: TestElement) {
  assert.equal(typeof element.props.onClick, "function");
  (element.props.onClick as () => void)();
}

describe("catalog mode controls", () => {
  it("wires cancel, confirm, and restore to draft updates without persisting", () => {
    assert.ok(controlsModule, "the production CatalogModeControls component must exist");
    let selectedMode: CatalogModeValue = "official-plus-custom";
    let allowDiscard = false;
    const render = () => controlsModule.CatalogModeControls({
      currentMode: selectedMode,
      externalPointer: null,
      customModelCount: 2,
      dormantCustomCount: selectedMode === "native-official" ? 2 : 0,
      pendingDormantCustomCount: 0,
      modeOptions: [
        { value: "native-official", label: "官方原生" },
        { value: "official-plus-custom", label: "官方 + 自定义" },
      ],
      dormantMessage: "2 个自定义模型暂不生效",
      pendingMessage: "保存后，2 个自定义模型将暂不生效",
      restoreLabel: "恢复官方＋自定义",
      confirmDiscard: () => allowDiscard,
      updateDraftMode: (mode) => { selectedMode = mode; },
    });

    const cancelledNative = findButton(render(), "data-catalog-mode", "native-official");
    click(cancelledNative);
    assert.equal(selectedMode, "official-plus-custom");

    allowDiscard = true;
    const confirmedNative = findButton(render(), "data-catalog-mode", "native-official");
    click(confirmedNative);
    assert.equal(selectedMode, "native-official");

    const restore = findButton(render(), "data-catalog-restore");
    click(restore);
    assert.equal(selectedMode, "official-plus-custom");
  });

  it("does not update the ordinary draft when combined save is unavailable", () => {
    assert.ok(controlsModule, "the production CatalogModeControls component must exist");
    let selectedMode: CatalogModeValue = "official-plus-custom";
    const rendered = controlsModule.CatalogModeControls({
      currentMode: selectedMode,
      externalPointer: null,
      customModelCount: 0,
      dormantCustomCount: 0,
      pendingDormantCustomCount: 0,
      modeOptions: [
        { value: "native-official", label: "官方原生" },
        { value: "official-plus-custom", label: "官方 + 自定义" },
      ],
      dormantMessage: "",
      pendingMessage: "",
      restoreLabel: "恢复官方＋自定义",
      confirmDiscard: () => true,
      updateDraftMode: (mode) => { selectedMode = mode; },
      disabled: true,
    });

    const native = findButton(rendered, "data-catalog-mode", "native-official");
    assert.equal(native.props.disabled, true);
    click(native);
    assert.equal(selectedMode, "official-plus-custom");
  });
});

describe("the editor routes every overlay edit through the promotion rule", () => {
  const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
  const editor = appSource.match(
    /const applyOverlay = \(nextOverlay: CatalogOverlayDraft\)[\s\S]*?\n  return \(/,
  )?.[0] ?? "";

  it("was read from the real editor", () => {
    assert.ok(editor.length > 0, "the catalog editor body was located");
  });

  it("has no overlay edit that bypasses it", () => {
    // A bypassing edit would store a number in native mode, where nothing is generated to read it.
    assert.doesNotMatch(editor, /updateCatalogProfileDraft\(draft, \{ overlay/);
    for (const edit of ["setOfficialOverride", "updateCustom", "addCustom"]) {
      const body = editor.match(new RegExp(`const ${edit} = [\\s\\S]*?\\n  \\};`))?.[0] ?? "";
      assert.ok(body.length > 0, `${edit} was located`);
      assert.match(body, /applyOverlay\(/, `${edit} goes through the promotion rule`);
    }
  });

  it("marks the promoted mode as owned so the backend keeps it", () => {
    assert.match(editor, /catalogModeForOverlay\(mode, nextOverlay\)/);
    assert.match(editor, /mode: nextMode, modeExplicit: true/);
  });
});
