import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

const appSource = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");
const commandsSource = readFileSync(
  new URL("../src-tauri/src/commands.rs", import.meta.url),
  "utf8",
);
const liveStateSource = readFileSync(
  new URL("../src-tauri/src/live_state.rs", import.meta.url),
  "utf8",
);
const platformSource = readFileSync(
  new URL("../src-tauri/src/platform_command.rs", import.meta.url),
  "utf8",
);

const saveDraft = appSource.match(
  /const saveDraft = async \(\) => \{[\s\S]*?\n  \};/,
)?.[0] ?? "";

describe("a provider save always settles", () => {
  it("was read from the real sources", () => {
    assert.ok(saveDraft.includes("commitProviderDetail"), "saveDraft body was located");
    assert.ok(commandsSource.includes("commit_provider_detail"), "the commit command was located");
  });

  it("keeps every step of the save inside the guard that clears the pending state", () => {
    const guarded = saveDraft.slice(saveDraft.indexOf("try {"));
    for (const step of [
      "draftWithModelRows(detailStateRef.current.profile)",
      "deriveRelayProfileFromFiles",
      "addRelayProfile",
      "updateRelayProfile",
    ]) {
      assert.ok(
        guarded.includes(step),
        `${step} runs inside the try so a throw still resets the button`,
      );
    }
    assert.match(saveDraft, /\} catch \(error\) \{[\s\S]*?showMessage\([\s\S]*?"failed"\)/);
    assert.match(saveDraft, /\} finally \{\s*savingRef\.current = false;\s*setSaving\(false\);/);
  });

  it("reports success as soon as the transaction commits", () => {
    const submit = appSource.match(
      /const submitProviderCommit = async[\s\S]*?\n  \};/,
    )?.[0] ?? "";
    assert.ok(submit.includes("refreshAfterCommit()"), "the post-commit reads are detached");
    assert.doesNotMatch(
      submit,
      /await Promise\.all\(\[\s*refreshRelay/,
      "a slow status read must not hold the caller's pending state",
    );
    assert.doesNotMatch(submit, /await refreshModelCatalog/);
  });

  it("bounds every helper process the save path can block on", () => {
    assert.doesNotMatch(liveStateSource, /icacls[\s\S]{0,400}?\.(status|output)\(\)\?/);
    assert.match(liveStateSource, /status_bounded\(/);
    assert.match(liveStateSource, /output_bounded\(/);
    assert.match(platformSource, /did not finish within/);
  });

  it("bounds every provider-facing probe and keeps its blocking write off the async runtime", () => {
    assert.match(commandsSource, /const PROVIDER_PROBE_TIMEOUT: Duration/);
    for (const probe of [
      "codex_plus_core::relay_config::test_relay_profile",
      "codex_plus_core::model_catalog::fetch_relay_profile_model_ids",
    ]) {
      const unbounded = new RegExp(
        `${probe.replace(/[:]/g, "[:]")}\\((&profile[^)]*)\\)\\s*\\.await`,
      );
      assert.doesNotMatch(
        commandsSource,
        unbounded,
        `${probe} must be awaited through bounded_probe`,
      );
    }
    const fetchCommand = commandsSource.match(
      /pub async fn fetch_relay_profile_models\([\s\S]*?\n\}/,
    )?.[0] ?? "";
    assert.ok(fetchCommand.length > 0, "the model-fetch command was located");
    assert.match(fetchCommand, /spawn_blocking\(move \|\| \{?\s*crate::model_catalog::record_provider_evidence/);
  });

  it("answers the caller when the blocking body panics and keeps the coordinator usable", () => {
    const command = commandsSource.match(
      /pub async fn commit_provider_detail\([\s\S]*?\n\}/,
    )?.[0] ?? "";
    assert.ok(command.length > 0, "the commit command was located");
    assert.doesNotMatch(command, /expect\("blocking command panicked"\)/);
    assert.match(command, /settle_blocking\(/);
    assert.match(command, /ProviderCommitErrorCode::TransactionFailed/);
    assert.match(liveStateSource, /unwrap_or_else\(\|poisoned\| poisoned\.into_inner\(\)\)/);
  });
});
