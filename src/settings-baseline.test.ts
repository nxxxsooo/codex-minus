import assert from "node:assert";
import { describe, it } from "node:test";

import type {
  BackendSettings,
  ProviderCommitResult,
  SettingsResult,
} from "./backend-types.ts";

const baselineModule = await import("./settings-baseline.ts").catch(() => null);

function requireBaselineModule() {
  assert.ok(baselineModule, "the shared settings-baseline epoch helper must exist");
  return baselineModule;
}

describe("the settings baseline epoch", () => {
  it("rejects refresh A after either same-token generic save installs newer settings", () => {
    const baseline = requireBaselineModule();
    for (const savePath of ["saveSettings", "saveSettingsValue"]) {
      const before = {
        providerFingerprint: "provider-generation-x",
        nonProviderValue: "before",
      };
      const afterSave = {
        ...before,
        nonProviderValue: `after-${savePath}`,
      };
      assert.equal(afterSave.providerFingerprint, before.providerFingerprint);

      const refreshA = baseline.registerSettingsRead(
        baseline.createSettingsBaselineEpochState(),
      );
      const stateAfterSaveB = baseline.advanceSettingsBaselineEpoch(
        refreshA.state,
      );

      assert.equal(
        baseline.settingsReadResponseCanAdopt(refreshA.request, stateAfterSaveB),
        false,
        `${savePath} must invalidate the older read without relying on the provider token`,
      );
    }
  });

  it("allows initial load and an unchanged-token explicit refresh", () => {
    const baseline = requireBaselineModule();
    const initialLoad = baseline.registerSettingsRead(
      baseline.createSettingsBaselineEpochState(),
    );
    assert.equal(
      baseline.settingsReadResponseCanAdopt(initialLoad.request, initialLoad.state),
      true,
    );

    const afterInitialLoad = baseline.advanceSettingsBaselineEpoch(initialLoad.state);
    const explicitRefresh = baseline.registerSettingsRead(afterInitialLoad);
    assert.equal(
      baseline.settingsReadResponseCanAdopt(
        explicitRefresh.request,
        explicitRefresh.state,
      ),
      true,
    );
  });

  it("keeps competing settings reads last-issued-wins", () => {
    const baseline = requireBaselineModule();
    const first = baseline.registerSettingsRead(
      baseline.createSettingsBaselineEpochState(),
    );
    const second = baseline.registerSettingsRead(first.state);

    assert.equal(
      baseline.settingsReadResponseCanAdopt(first.request, second.state),
      false,
    );
    assert.equal(
      baseline.settingsReadResponseCanAdopt(second.request, second.state),
      true,
    );
  });

  it("surfaces an obsolete read's reset notice once without adopting its settings", () => {
    const baseline = requireBaselineModule();
    const first = baseline.registerSettingsRead(
      baseline.createSettingsBaselineEpochState(),
    );
    const second = baseline.registerSettingsRead(first.state);
    let noticeState = baseline.createLegacyModelResetNoticeState();

    const current = baseline.consumeLegacyModelResetNotice(noticeState, null);
    noticeState = current.state;
    assert.equal(current.notice, null);
    assert.equal(
      baseline.settingsReadResponseCanAdopt(second.request, second.state),
      true,
    );

    const obsolete = baseline.consumeLegacyModelResetNotice(
      noticeState,
      "legacy reset completed",
    );
    noticeState = obsolete.state;
    assert.equal(obsolete.notice, "legacy reset completed");
    assert.equal(
      baseline.settingsReadResponseCanAdopt(first.request, second.state),
      false,
    );
    assert.deepEqual(second.state, {
      baselineEpoch: 0,
      latestReadRevision: 2,
    });

    const duplicate = baseline.consumeLegacyModelResetNotice(
      noticeState,
      "legacy reset completed",
    );
    assert.equal(duplicate.notice, null);
    assert.equal(duplicate.state, noticeState);
  });

  it("lets a provider success invalidate an earlier settings read", () => {
    const baseline = requireBaselineModule();
    const refresh = baseline.registerSettingsRead(
      baseline.createSettingsBaselineEpochState(),
    );
    const afterProviderSuccess = baseline.advanceSettingsBaselineEpoch(refresh.state);

    assert.equal(afterProviderSuccess.baselineEpoch, 1);
    assert.equal(
      baseline.settingsReadResponseCanAdopt(refresh.request, afterProviderSuccess),
      false,
    );
  });

  it("builds the provider response baseline without shell-owned projection logic", () => {
    const baseline = requireBaselineModule();
    const normalizedSettings = {
      relayProfilesEnabled: true,
    } as unknown as BackendSettings;
    const priorBaseline = {
      settings_path: "/settings.json",
      user_scripts: { enabled: true },
    } as unknown as SettingsResult;
    const result = {
      status: "ok",
      message: "saved",
      providerFingerprint: "provider-generation-2",
    } as unknown as ProviderCommitResult;

    assert.deepEqual(
      baseline.settingsBaselineFromProviderCommit(
        result,
        normalizedSettings,
        priorBaseline,
      ),
      {
        status: "ok",
        message: "saved",
        settings: normalizedSettings,
        settings_path: "/settings.json",
        user_scripts: { enabled: true },
        provider_fingerprint: "provider-generation-2",
      },
    );
  });
});
