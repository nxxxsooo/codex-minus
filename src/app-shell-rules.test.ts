import assert from "node:assert";
import { describe, it } from "node:test";

import {
  boundedPercentOrDefault,
  boundedPercentOrNull,
  catalogDraftErrorLabel,
  envConflictSourceLabel,
  formatTime,
  integerOrDefault,
  integerOrNull,
  managedCatalogMode,
  parseCommaListOrNull,
  parseReasoningLevels,
  positiveNumberOrDefault,
  positiveNumberOrNull,
  providerInitial,
  reasoningEffortsText,
  relayModeLabel,
  relayProfileConfigBrief,
  relayProfileEditorStatus,
  relayProfileModeHelp,
  stringifyError,
  truncateSessionDeletePreview,
} from "./app-shell-rules.ts";

describe("numeric and list input parsers", () => {
  it("parses positive numbers from noisy input and falls back explicitly", () => {
    assert.equal(positiveNumberOrNull("272k"), 272);
    assert.equal(positiveNumberOrNull("0"), null);
    assert.equal(positiveNumberOrNull(""), null);
    assert.equal(positiveNumberOrDefault("", 7), 7);
  });

  it("bounds percents at 100", () => {
    assert.equal(boundedPercentOrNull("95"), 95);
    assert.equal(boundedPercentOrNull("101"), null);
    assert.equal(boundedPercentOrDefault("101", 90), 90);
  });

  it("parses integers without stripping signs into digits", () => {
    assert.equal(integerOrNull(" "), null);
    assert.equal(integerOrNull("42"), 42);
    assert.equal(integerOrDefault("x", 3), 3);
  });

  it("deduplicates comma lists and maps reasoning levels", () => {
    assert.deepEqual(parseCommaListOrNull("high, low, high"), ["high", "low"]);
    assert.equal(parseCommaListOrNull(" , "), null);
    assert.deepEqual(parseReasoningLevels("high"), [{ effort: "high", description: "high" }]);
    assert.equal(
      reasoningEffortsText([
        { effort: "high", description: "h" },
        { effort: "low", description: "l" },
      ]),
      "high,low",
    );
  });
});

describe("labels and formatting", () => {
  it("classifies managed catalog modes", () => {
    assert.equal(managedCatalogMode("official-plus-custom"), true);
    assert.equal(managedCatalogMode("custom-only"), true);
    assert.equal(managedCatalogMode("native-official"), false);
    assert.equal(managedCatalogMode("external"), false);
  });

  it("names every catalog draft error and stays silent for unknown codes", () => {
    assert.notEqual(catalogDraftErrorLabel("empty-catalog"), "");
    assert.equal(catalogDraftErrorLabel("unknown-code"), "");
    assert.equal(catalogDraftErrorLabel(null), "");
  });

  it("labels env conflict sources with a readable fallback", () => {
    assert.notEqual(envConflictSourceLabel("process"), "process");
    assert.equal(envConflictSourceLabel("custom-source"), "custom-source");
  });

  it("derives a provider initial from the first character", () => {
    assert.equal(providerInitial("relay one"), "R");
    assert.equal(providerInitial("中转"), "中");
  });

  it("formats errors, times, and delete previews defensively", () => {
    assert.equal(stringifyError(new Error("boom")), "boom");
    assert.equal(stringifyError("plain"), "plain");
    assert.equal(formatTime(0), "-");
    assert.equal(truncateSessionDeletePreview("  short "), "short");
    assert.equal(truncateSessionDeletePreview("x".repeat(30)), `${"x".repeat(20)}...`);
  });
});

describe("relay profile presentation", () => {
  const profile = (relayMode: "official" | "pureApi", officialMixApiKey: boolean) => ({
    relayMode,
    officialMixApiKey,
    baseUrl: "https://relay.example/v1",
    id: "p1",
  } as never);

  it("labels both relay modes", () => {
    assert.notEqual(relayModeLabel("pureApi"), relayModeLabel("official"));
  });

  it("briefs official profiles by key mixing and pure API by endpoint", () => {
    assert.notEqual(
      relayProfileConfigBrief(profile("official", true)),
      relayProfileConfigBrief(profile("official", false)),
    );
    assert.equal(relayProfileConfigBrief(profile("pureApi", false)), "https://relay.example/v1");
  });

  it("explains pure API as bearer-only without ChatGPT auth", () => {
    assert.match(relayProfileModeHelp(profile("pureApi", false)), /不要求 ChatGPT 认证/);
    assert.match(relayProfileModeHelp(profile("official", false)), /不写入 API Key/);
  });

  it("reports editor status by lifecycle", () => {
    const form = { relayProfilesEnabled: true, activeRelayId: "p1" } as never;
    assert.match(relayProfileEditorStatus(profile("official", true), form, true), /新建/);
    assert.match(relayProfileEditorStatus(profile("official", true), form, false), /当前正在使用/);
  });
});
