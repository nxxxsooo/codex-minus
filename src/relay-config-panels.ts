import { createElement, type ReactElement } from "react";

import { redactTomlSecrets } from "./codex-toml.ts";

function commentLine(message: string): string {
  return `# ${message.trim()}\n`;
}

/// The live `config.toml`, shown as evidence and nothing more.
///
/// There used to be a second panel beside this one: an editable textarea holding the profile's own
/// provider TOML. It was the last place in the editor where a user could produce a file Codex
/// cannot parse, and every field it exposed now has a labelled input of its own — so what it
/// offered was the chance to break a working provider by hand, and no way to succeed that the
/// seven-step flow did not already offer.
///
/// What remains is read-only and redacted. The question this panel answers is "what is Codex
/// actually reading right now", which needs the file's shape, not its secrets.
export function LiveConfigPanel(props: {
  liveConfig: string;
  unavailableLiveMessage: string;
  liveTitle: string;
  liveHelp: string;
}): ReactElement {
  const liveValue = props.liveConfig.trim()
    ? redactTomlSecrets(props.liveConfig)
    : commentLine(props.unavailableLiveMessage);

  return createElement("div", { className: "relay-file-panel" },
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
  );
}
