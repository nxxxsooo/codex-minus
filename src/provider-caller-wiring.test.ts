import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const source = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

describe("provider caller wiring", () => {
  it("routes list, detail, set-current, copy, and test-model writes through ProviderCommit", () => {
    assert.match(source, /buildProviderMutationInvocation/);
    assert.match(source, /commitProviderTopology/);
    assert.match(source, /commitProviderDetail/);
    assert.match(source, /saveRelaySettings\(next, "enablement"\)/);
    assert.match(source, /saveRelaySettings\(normalizeSettings\(form\), "testModel"\)/);
    assert.match(source, /onFormChange\(next, "reorder"\)/);
    assert.match(source, /"copy",\s*profile\.id/);
    assert.match(source, /removeRelayProfile\(form, profile\.id\), "delete"/);
    assert.match(source, /actions\.commitProviderDetail\([\s\S]*?"detailSave"/);
    assert.match(source, /actions\.switchRelayProfile\([\s\S]*?catalogDraft/);
    assert.doesNotMatch(source, /call<RelaySwitchResult>\("switch_relay_profile"/);
    assert.doesNotMatch(source, /call<RelaySwitchResult>\("save_active_relay_profile"/);
    assert.doesNotMatch(source, /relayProfileSwitchCommand/);
    assert.doesNotMatch(source, /call<SettingsResult>\("save_settings", \{ settings: settingsForm \}\)/);
  });
});
