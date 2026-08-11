import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

describe("provider mode presentation wiring", () => {
  it("derives the detail badge only from response-only native inspection", () => {
    const detail = appSource.match(
      /function RelayProfileDetail[\s\S]*?(?=\nfunction RelayProfileEditor)/,
    )?.[0] ?? "";
    assert.match(
      detail,
      /deriveProviderModePresentation\(detailState\.inspection\)/,
    );
    assert.match(detail, /providerModePresentationLabel/);
    assert.doesNotMatch(
      detail.match(/const modePresentation[\s\S]*?(?=\n\s*const replaceDraft)/)?.[0] ?? "",
      /commitProvider|saveSettings|updateCatalogProfileDraft|transformProvider/,
    );
  });

  it("marks the removed local aggregate path as advanced without changing its data", () => {
    const aggregate = appSource.match(
      /function AggregateRelayProfileEditor[\s\S]*?(?=\nfunction RelayFileEditors)/,
    )?.[0] ?? "";
    assert.match(aggregate, /高级兼容路径/);
    assert.match(aggregate, /本地聚合（不可用）/);
  });
});
