import assert from "node:assert";
import { describe, it } from "node:test";

import {
  networkPolicyDirty,
  networkPolicyDraft,
  networkPolicyPresentation,
  networkTestCategoryLabel,
  validateNetworkPolicyDraft,
  type NetworkPolicyStatusView,
} from "./network-policy-ui.ts";

const status = (patch: Partial<NetworkPolicyStatusView> = {}): NetworkPolicyStatusView => ({
  mode: "auto",
  customProxyUrl: "",
  customNoProxy: "",
  source: "direct-fallback",
  endpoint: null,
  bypassCount: 0,
  supported: true,
  actionRequired: null,
  ...patch,
});

describe("manager network policy UI state", () => {
  it("defaults to auto and validates custom modes", () => {
    assert.deepEqual(networkPolicyDraft(null), { mode: "auto", customProxyUrl: "", customNoProxy: "" });
    assert.equal(validateNetworkPolicyDraft({ mode: "direct", customProxyUrl: "bad", customNoProxy: "" }), null);
    assert.equal(validateNetworkPolicyDraft({ mode: "custom", customProxyUrl: "", customNoProxy: "" }), "custom-proxy-required");
    assert.equal(validateNetworkPolicyDraft({ mode: "custom", customProxyUrl: "ftp://proxy", customNoProxy: "" }), "custom-proxy-scheme");
    assert.equal(validateNetworkPolicyDraft({ mode: "custom", customProxyUrl: "http://user:pass@proxy:7890", customNoProxy: "" }), "custom-proxy-credentials");
    assert.equal(validateNetworkPolicyDraft({ mode: "custom", customProxyUrl: "http://127.0.0.1:7890", customNoProxy: "localhost" }), null);
  });

  it("tracks dirty state with normalized bypass entries", () => {
    const saved = status({ mode: "custom", customProxyUrl: "http://127.0.0.1:7890", customNoProxy: "localhost,.local" });
    assert.equal(networkPolicyDirty({ mode: "custom", customProxyUrl: "http://127.0.0.1:7890", customNoProxy: ".LOCAL; localhost" }, saved), false);
    assert.equal(networkPolicyDirty({ mode: "direct", customProxyUrl: "http://127.0.0.1:7890", customNoProxy: "localhost,.local" }, saved), true);
  });

  it("presents redacted status and action-required state", () => {
    assert.deepEqual(
      networkPolicyPresentation(status({ source: "custom", endpoint: "http://user:pass@proxy.local:7890/path" })),
      { state: "ready", source: "custom", endpoint: "http://proxy.local:7890" },
    );
    assert.equal(networkPolicyPresentation(status({ supported: false, actionRequired: "PAC unsupported" })).state, "action-required");
  });

  it("keeps transport categories stable", () => {
    assert.equal(networkTestCategoryLabel("proxy-connect"), "proxy-connect");
    assert.equal(networkTestCategoryLabel("new-upstream-error"), "other");
  });
});
