import assert from "node:assert/strict";
import { test } from "node:test";
import { preserveEditedProviderDraft, providerConfirmationStillCurrent } from "./provider-detail-refresh.ts";

test("a changed server baseline cannot erase edits made while a commit response was lost", () => {
  const baseline = { profileId: "a", profile: "saved", catalog: "saved catalog" };
  assert(preserveEditedProviderDraft(baseline, { ...baseline, profile: "newer unsaved edit" }, "a"));
  assert(preserveEditedProviderDraft(baseline, { ...baseline, catalog: "local model override" }, "a"));
  assert(!preserveEditedProviderDraft(baseline, baseline, "a"));
  assert(!preserveEditedProviderDraft(baseline, { ...baseline, profile: "edit" }, "b"));
  assert(!preserveEditedProviderDraft(null, baseline, "a"));
});

test("async confirmation cannot authorize a replaced draft or a closed editor", () => {
  const shown = { lifecycle: "active", sessionToken: "a", pendingConfirmation: { revision: 1 } };
  assert(providerConfirmationStillCurrent({ ...shown }, shown));
  assert(!providerConfirmationStillCurrent({ ...shown, sessionToken: "reopened" }, shown));
  assert(!providerConfirmationStillCurrent({ ...shown, lifecycle: "closed" }, shown));
  assert(!providerConfirmationStillCurrent({ ...shown, pendingConfirmation: { revision: 2 } }, shown));
});
