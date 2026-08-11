import { createElement, Fragment, type ReactElement } from "react";

export function providerConfigDraft(savedProviderConfig: string, _liveConfig: string): string {
  return savedProviderConfig;
}

function commentLine(message: string): string {
  return `# ${message.trim()}\n`;
}

export function RelayConfigPanels(props: {
  nativeOfficial: boolean;
  providerConfig: string;
  liveConfig: string;
  nativeProviderMessage: string;
  unavailableLiveMessage: string;
  providerTitle: string;
  providerHelp: string;
  liveTitle: string;
  liveHelp: string;
  onProviderConfigChange: (value: string) => void;
}): ReactElement {
  const providerValue = props.nativeOfficial
    ? commentLine(props.nativeProviderMessage)
    : props.providerConfig;
  const liveValue = props.liveConfig.trim()
    ? props.liveConfig
    : commentLine(props.unavailableLiveMessage);

  return createElement(Fragment, null,
    createElement("div", { className: "relay-file-panel" },
      createElement("div", { className: "relay-file-head" },
        createElement("div", null,
          createElement("strong", null, props.providerTitle),
          createElement("span", null, props.providerHelp),
        ),
      ),
      createElement("textarea", {
        className: "relay-file-textarea",
        "data-provider-config": "true",
        onChange: (event: { currentTarget: { value: string } }) => {
          if (!props.nativeOfficial) props.onProviderConfigChange(event.currentTarget.value);
        },
        readOnly: props.nativeOfficial,
        spellCheck: false,
        value: providerValue,
      }),
    ),
    createElement("div", { className: "relay-file-panel" },
      createElement("div", { className: "relay-file-head" },
        createElement("div", null,
          createElement("strong", null, props.liveTitle),
          createElement("span", null, props.liveHelp),
        ),
      ),
      createElement("textarea", {
        className: "relay-file-textarea live-config-readonly",
        "data-live-config": "true",
        readOnly: true,
        spellCheck: false,
        value: liveValue,
      }),
    ),
  );
}
