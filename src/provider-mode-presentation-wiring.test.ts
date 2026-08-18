import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

describe("provider mode presentation wiring", () => {
  it("shows no provider-mode badge or label in the detail screen", () => {
    const detail = appSource.match(
      /function RelayProfileDetail[\s\S]*?(?=\nfunction RelayProfileEditor)/,
    )?.[0] ?? "";
    assert.ok(detail.includes("saveDraft"), "the detail body was located");
    // The mode is not a user-facing state any more: every save materializes the one contract.
    assert.doesNotMatch(detail, /providerModePresentationLabel|deriveProviderModePresentation/);
    assert.doesNotMatch(appSource, /function providerModePresentationLabel/);
  });
});
