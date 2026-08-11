import { createElement, Fragment, type ReactElement } from "react";

import { catalogModeDraftController, type CatalogModeValue } from "./model-catalog-ui.ts";

export type CatalogModeOption = {
  value: CatalogModeValue;
  label: string;
};

export function CatalogModeControls(props: {
  currentMode: CatalogModeValue;
  externalPointer: string | null;
  customModelCount: number;
  dormantCustomCount: number;
  pendingDormantCustomCount: number;
  modeOptions: readonly CatalogModeOption[];
  dormantMessage: string;
  pendingMessage: string;
  restoreLabel: string;
  confirmDiscard: (decision: "confirm-discard-external" | "confirm-discard-custom") => boolean;
  updateDraftMode: (mode: CatalogModeValue) => void;
  saveProfileCatalog: () => unknown;
}): ReactElement {
  const actions = catalogModeDraftController({
    currentMode: props.currentMode,
    externalPointer: props.externalPointer,
    customModelCount: props.customModelCount,
    confirmDiscard: props.confirmDiscard,
    actions: {
      updateDraftMode: props.updateDraftMode,
      saveProfileCatalog: props.saveProfileCatalog,
    },
  });
  const restoreButton = () => createElement("button", {
    className: "catalog-inline-action",
    "data-catalog-restore": "true",
    onClick: actions.restoreOfficialPlusCustom,
    type: "button",
  }, props.restoreLabel);

  return createElement(Fragment, null,
    createElement("div", { className: "segmented catalog-mode-control" },
      ...props.modeOptions.map((option) => createElement("button", {
        className: props.currentMode === option.value ? "active" : "",
        "data-catalog-mode": option.value,
        key: option.value,
        onClick: () => actions.requestMode(option.value),
        type: "button",
      }, option.label)),
    ),
    props.dormantCustomCount > 0
      ? createElement("div", { className: "catalog-inline-error" },
          createElement("span", null, props.dormantMessage),
          restoreButton(),
        )
      : null,
    props.pendingDormantCustomCount > 0
      ? createElement("div", { className: "catalog-inline-error" },
          createElement("span", null, props.pendingMessage),
          restoreButton(),
        )
      : null,
  );
}
