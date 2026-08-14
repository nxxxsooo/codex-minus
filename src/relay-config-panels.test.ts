import assert from "node:assert";
import fs from "node:fs";
import { describe, it } from "node:test";
import { Children, isValidElement, type ReactElement, type ReactNode } from "react";

import { LiveConfigPanel } from "./relay-config-panels.ts";

type TestElement = ReactElement<Record<string, unknown>>;

function findElement(node: ReactNode, attribute: string): TestElement {
  if (isValidElement<Record<string, unknown>>(node) && node.props[attribute] === "true") return node;
  if (isValidElement<Record<string, unknown>>(node)) {
    for (const child of Children.toArray(node.props.children as ReactNode)) {
      try {
        return findElement(child, attribute);
      } catch {
        // Continue searching sibling branches.
      }
    }
  }
  throw new Error(`${attribute} element not found`);
}

/// Every string this test plants in a config, so a sweep can look for all of them at once.
const SENTINEL_KEY = "sk-provider-sentinel-DO-NOT-RENDER";

const appSource = fs.readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

describe("the live config panel", () => {
  it("renders the live file read-only", () => {
    const tree = LiveConfigPanel({
      liveConfig: "[agents]\nmax_concurrent_threads_per_session = 8\n",
      unavailableLiveMessage: "当前 live config.toml 不可用",
      liveTitle: "实时 config.toml",
      liveHelp: "直接读取，不保存副本",
    });

    const live = findElement(tree, "data-live-config");
    assert.equal(live.props.readOnly, true);
    assert.equal(live.props.value, "[agents]\nmax_concurrent_threads_per_session = 8\n");
  });

  it("says so when there is no live file rather than rendering an empty box", () => {
    const tree = LiveConfigPanel({
      liveConfig: "   \n",
      unavailableLiveMessage: "当前 live config.toml 不可用",
      liveTitle: "实时 config.toml",
      liveHelp: "直接读取，不保存副本",
    });

    assert.equal(findElement(tree, "data-live-config").props.value, "# 当前 live config.toml 不可用\n");
  });

  it("masks the bearer while keeping the line that proves one is configured", () => {
    const tree = LiveConfigPanel({
      liveConfig: [
        'model = "gpt-5.6-sol"',
        'model_provider = "OpenAI"',
        "",
        "[model_providers.OpenAI]",
        'base_url = "https://relay.example/v1"',
        `experimental_bearer_token = "${SENTINEL_KEY}"`,
        `http_headers = { "Authorization" = "Bearer ${SENTINEL_KEY}" }`,
        "",
      ].join("\n"),
      unavailableLiveMessage: "当前 live config.toml 不可用",
      liveTitle: "实时 config.toml",
      liveHelp: "直接读取，不保存副本",
    });

    const rendered = String(findElement(tree, "data-live-config").props.value);
    assert.ok(!rendered.includes(SENTINEL_KEY), `the key reached the screen:\n${rendered}`);
    assert.match(rendered, /experimental_bearer_token = "••••••••"/);
    assert.match(rendered, /"Authorization" = "••••••••"/);
    // Redaction must not cost the reader the rest of the file.
    assert.match(rendered, /base_url = "https:\/\/relay\.example\/v1"/);
    assert.match(rendered, /\[model_providers\.OpenAI\]/);
  });
});

describe("no surface renders a provider key in plaintext", () => {
  it("has no editable provider-config textarea left to type one into", () => {
    // The raw provider-TOML editor was the one place a user could hand-write a file Codex cannot
    // parse, and the one place the bearer appeared as editable text. Its absence is the rule.
    assert.doesNotMatch(appSource, /data-provider-config/);
    assert.doesNotMatch(appSource, /RelayConfigPanels|RelayFileEditors|providerConfigDraft/);
    assert.doesNotMatch(appSource, /onProviderConfigChange/);
  });

  it("routes every config the shell displays through the redactor", () => {
    // `configContents` carries `experimental_bearer_token` verbatim. Any place the shell puts it
    // into a `value=` is a place the key is on screen, so the panel that does it is the only one
    // allowed, and it redacts.
    const displayed = [...appSource.matchAll(/value=\{([^}]*configContents[^}]*)\}/g)].map((m) => m[1]);
    assert.deepEqual(displayed, [], `these render a config verbatim: ${displayed.join(", ")}`);

    const panelSource = fs.readFileSync(new URL("./relay-config-panels.ts", import.meta.url), "utf8");
    assert.match(panelSource, /redactTomlSecrets\(props\.liveConfig\)/);
  });

  it("keeps the key out of the clipboard paths too", () => {
    // A copy button that stringifies a profile would put the key on the clipboard without ever
    // painting it. There is no such path today; this fails if one appears.
    const copyPaths = [...appSource.matchAll(/writeText\(([^)]*)\)/g)].map((match) => match[1].trim());
    for (const argument of copyPaths) {
      assert.ok(
        !/configContents|apiKey|authContents/.test(argument),
        `a copy path carries provider credentials: writeText(${argument})`,
      );
    }
  });
});
