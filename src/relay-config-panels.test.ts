import assert from "node:assert";
import fs from "node:fs";
import { describe, it } from "node:test";
import { Children, isValidElement, type ReactElement, type ReactNode } from "react";

const panelsModule = await import("./relay-config-panels.ts").catch(() => null);

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

describe("relay config panels", () => {
  it("binds the provider textarea to the exact draft instead of a trailing-newline preview", () => {
    const appSource = fs.readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
    const editorStart = appSource.indexOf("function RelayFileEditors(");
    const editorEnd = appSource.indexOf("function ProviderDoctorModal(", editorStart);
    assert.ok(editorStart >= 0 && editorEnd > editorStart, "RelayFileEditors must remain present");
    const editorSource = appSource.slice(editorStart, editorEnd);

    assert.match(editorSource, /const providerConfig = profile\.configContents;/);
    assert.doesNotMatch(editorSource, /applyContextLimitPreview\(profile\.configContents, profile\)/);
  });

  it("keeps the provider draft separate from the current live config", () => {
    assert.ok(panelsModule, "the production relay config module must exist");
    assert.equal(typeof panelsModule.providerConfigDraft, "function");
    assert.equal(
      panelsModule.providerConfigDraft(
        "model_provider = \"custom\"\n",
        "[agents]\nmax_concurrent_threads_per_session = 8\n",
      ),
      "model_provider = \"custom\"\n",
    );
  });

  it("shows native official state explicitly and reads live config without a saved common copy", () => {
    assert.ok(panelsModule, "the production RelayConfigPanels component must exist");
    let providerWrites = 0;
    const tree = panelsModule.RelayConfigPanels({
      nativeOfficial: true,
      providerConfig: "",
      liveConfig: "[agents]\nmax_concurrent_threads_per_session = 8\n",
      nativeProviderMessage: "官方原生模式无独立供应商配置",
      unavailableLiveMessage: "当前 live config.toml 不可用",
      providerTitle: "供应商配置",
      providerHelp: "只保存供应商字段",
      liveTitle: "实时 config.toml",
      liveHelp: "直接读取，不保存副本",
      onProviderConfigChange: () => { providerWrites += 1; },
    });

    const provider = findElement(tree, "data-provider-config");
    const live = findElement(tree, "data-live-config");
    assert.equal(provider.props.readOnly, true);
    assert.equal(provider.props.value, "# 官方原生模式无独立供应商配置\n");
    assert.equal(live.props.readOnly, true);
    assert.equal(live.props.value, "[agents]\nmax_concurrent_threads_per_session = 8\n");
    assert.equal(providerWrites, 0);
  });

  it("edits only the provider-owned draft for API profiles", () => {
    assert.ok(panelsModule, "the production RelayConfigPanels component must exist");
    let nextProviderConfig = "";
    const tree = panelsModule.RelayConfigPanels({
      nativeOfficial: false,
      providerConfig: "model_provider = \"custom\"\n",
      liveConfig: "approval_policy = \"never\"\n",
      nativeProviderMessage: "官方原生模式无独立供应商配置",
      unavailableLiveMessage: "当前 live config.toml 不可用",
      providerTitle: "供应商配置",
      providerHelp: "只保存供应商字段",
      liveTitle: "实时 config.toml",
      liveHelp: "直接读取，不保存副本",
      onProviderConfigChange: (value) => { nextProviderConfig = value; },
    });

    const provider = findElement(tree, "data-provider-config");
    const live = findElement(tree, "data-live-config");
    assert.equal(provider.props.readOnly, false);
    assert.equal(live.props.readOnly, true);
    assert.equal(typeof provider.props.onChange, "function");
    (provider.props.onChange as (event: { currentTarget: { value: string } }) => void)({
      currentTarget: { value: "model_provider = \"next\"\n" },
    });
    assert.equal(nextProviderConfig, "model_provider = \"next\"\n");
  });

  it("keeps a brand-new provider preview read-only until its first combined save", () => {
    assert.ok(panelsModule, "the production RelayConfigPanels component must exist");
    let providerWrites = 0;
    const tree = panelsModule.RelayConfigPanels({
      nativeOfficial: false,
      providerReadOnly: true,
      providerConfig: "model_provider = \"custom\"\n",
      liveConfig: "",
      nativeProviderMessage: "官方原生模式无独立供应商配置",
      unavailableLiveMessage: "当前 live config.toml 不可用",
      providerTitle: "供应商配置",
      providerHelp: "首次保存后可编辑",
      liveTitle: "实时 config.toml",
      liveHelp: "直接读取，不保存副本",
      onProviderConfigChange: () => { providerWrites += 1; },
    });

    const provider = findElement(tree, "data-provider-config");
    assert.equal(provider.props.readOnly, true);
    (provider.props.onChange as (event: { currentTarget: { value: string } }) => void)({
      currentTarget: { value: "forged = true\n" },
    });
    assert.equal(providerWrites, 0);
  });
});
