import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const app = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
// The wire shapes moved out of the shell; the field is still declared, just not here.
const backendTypes = readFileSync(new URL("./backend-types.ts", import.meta.url), "utf8");
const styles = readFileSync(new URL("./styles.css", import.meta.url), "utf8");
const english = readFileSync(new URL("./i18n-en.ts", import.meta.url), "utf8");

describe("provider goals ownership", () => {
  it("keeps the compatibility settings field but removes goals from provider detail ownership", () => {
    assert.match(backendTypes, /codexGoalsEnabled: boolean/);
    assert.doesNotMatch(app, /relay-field-goals/);
    assert.doesNotMatch(app, /configHasCodexGoalsFeature/);
    assert.doesNotMatch(app, /setCodexGoalsFeatureInConfig/);
    assert.doesNotMatch(app, /t\("Codex 目标"\)/);
    assert.doesNotMatch(app, /t\("启用目标功能"\)/);
    assert.doesNotMatch(styles, /relay-field-goals/);
    assert.doesNotMatch(english, /"Codex 目标"/);
    assert.doesNotMatch(english, /"启用目标功能"/);
  });
});
