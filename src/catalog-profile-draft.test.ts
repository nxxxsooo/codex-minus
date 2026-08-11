import assert from "node:assert";
import { describe, it } from "node:test";

const draftModule = await import("./catalog-profile-draft.ts").catch(() => null);

describe("catalog profile parent draft", () => {
  it("initializes one complete request draft and applies mode, topology, and overlay patches without persistence actions", () => {
    assert.ok(draftModule, "the parent catalog draft helper must exist");
    const initial = draftModule.catalogProfileDraft({
      profileId: "relay-a",
      fallbackMode: "official-plus-custom",
      summary: {
        profileId: "relay-a",
        mode: "custom-only",
        modeExplicit: true,
        upstreamTopology: "direct",
        externalPointer: null,
        overlay: { official: {}, custom: [] },
        generatedPath: "/response-only/models.json",
        actionRequired: "response-only",
      },
    });

    assert.deepEqual(initial, {
      profileId: "relay-a",
      mode: "custom-only",
      modeExplicit: true,
      upstreamTopology: "direct",
      externalPointer: null,
      overlay: { official: {}, custom: [] },
    });

    const overlay = {
      official: {},
      custom: [{
        slug: "custom-a",
        displayName: "Custom A",
        contextWindow: 128000,
        effectiveContextWindowPercent: 100,
        visible: true,
        order: 0,
        supportedReasoningLevels: [],
        defaultReasoningLevel: null,
        supportedTools: [],
        toolCapabilities: null,
        templateProvenance: "user-created",
      }],
    };
    const changed = draftModule.updateCatalogProfileDraft(initial, {
      mode: "official-plus-custom",
      modeExplicit: true,
      upstreamTopology: "server-side-composite",
      overlay,
    });

    assert.deepEqual(changed, {
      profileId: "relay-a",
      mode: "official-plus-custom",
      modeExplicit: true,
      upstreamTopology: "server-side-composite",
      externalPointer: null,
      overlay,
    });
    assert.equal("save" in draftModule, false);
    assert.equal("persist" in draftModule, false);
  });

  it("reports ordinary controls as read-only until combined save is connected", () => {
    assert.ok(draftModule, "the catalog editing availability helper must exist");

    assert.deepEqual(draftModule.catalogEditingAvailability(false), {
      editable: false,
      label: "统一保存接入后可编辑",
    });
  });
});
