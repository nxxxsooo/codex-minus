import assert from "node:assert";
import { readFileSync } from "node:fs";
import { describe, it } from "node:test";

import { EN_PLAIN } from "./i18n-en.ts";

// Native-capability priority means the client is eligible to mark itself; it is not a plan
// change and it is not a blanket capability grant. Copy that suggests otherwise would be an
// untruthful claim about the user's account, not a wording preference.
const FORBIDDEN_CLAIMS = [
  // Subscription or plan upgrade
  "升级订阅",
  "升级为 Pro",
  "升级到 Pro",
  "开通 Pro",
  "解锁 Pro",
  "upgrade your plan",
  "upgrade to pro",
  "unlock pro",
  "subscription upgrade",
  // Blanket capability grants
  "全部 Pro 能力",
  "所有 Pro 能力",
  "全部 pro 功能",
  "all pro capabilities",
  "all pro features",
  "every pro capability",
];

const UI_SOURCES = ["./src/App.tsx", "./src/i18n-en.ts"];

describe("provider capability claims", () => {
  it("makes no plan-upgrade or blanket-capability claim in translated copy", () => {
    const copy = [...Object.keys(EN_PLAIN), ...Object.values(EN_PLAIN)]
      .join("\n")
      .toLowerCase();
    for (const claim of FORBIDDEN_CLAIMS) {
      assert.ok(
        !copy.includes(claim.toLowerCase()),
        `translated copy claims "${claim}"`,
      );
    }
  });

  it("makes no plan-upgrade or blanket-capability claim in UI source strings", () => {
    for (const source of UI_SOURCES) {
      const text = readFileSync(new URL(source, import.meta.url.replace(/\/src\/[^/]+$/, "/")), "utf8").toLowerCase();
      // Prove the file was actually read, so the absence below is a real absence.
      assert.ok(text.includes("原生能力优先"), `${source} was not read`);
      for (const claim of FORBIDDEN_CLAIMS) {
        assert.ok(
          !text.includes(claim.toLowerCase()),
          `${source} claims "${claim}"`,
        );
      }
    }
  });

  it("keeps the eligibility wording it does use", () => {
    // The guard above only proves an absence. This pins the claim the product actually makes, so
    // the absence cannot be satisfied by deleting every explanation of what the mode means.
    assert.equal(
      EN_PLAIN["原生能力优先"],
      "Native-capability priority",
    );
  });
});
