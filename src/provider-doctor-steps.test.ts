import assert from "node:assert";
import { registerHooks } from "node:module";
import { describe, it } from "node:test";

import type { ProviderDoctorResult } from "./backend-types.ts";

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "@/i18n-en") {
      return { url: new URL("./i18n-en.ts", import.meta.url).href, shortCircuit: true };
    }
    return nextResolve(specifier, context);
  },
});

Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: { localStorage: { getItem: () => "en" } },
});

const { providerDoctorSteps } = await import("./provider-doctor-steps.ts");

const doctorResult = (overrides: Record<string, unknown>): ProviderDoctorResult =>
  ({
    status: "ok",
    message: "",
    profileName: "p",
    model: "gpt-5.5",
    summary: "",
    recommendation: "",
    checks: [],
    compatibilityFallbackUsed: false,
    initialHttpStatus: null,
    requestHttpStatus: null,
    ...overrides,
  }) as ProviderDoctorResult;

describe("provider doctor steps", () => {
  it("renders four pending steps before any result, with the first running while active", () => {
    const idle = providerDoctorSteps(null, false);
    assert.equal(idle.length, 4);
    assert.ok(idle.every((step) => step.state === "pending"));

    const running = providerDoctorSteps(null, true);
    assert.equal(running[0].state, "running");
    assert.ok(running.slice(1).every((step) => step.state === "pending"));
  });

  it("maps executed checks by id and marks skipped probe steps pending", () => {
    const steps = providerDoctorSteps(
      doctorResult({
        recommendation: "all good",
        checks: [{ id: "config", title: "Config", status: "ok", detail: "complete" }],
      }),
      false,
    );
    const byId = new Map(steps.map((step) => [step.id, step]));
    assert.equal(byId.get("config")?.state, "ok");
    assert.equal(byId.get("config")?.detail, "complete");
    assert.equal(byId.get("models")?.state, "pending");
    assert.equal(byId.get("request")?.state, "pending");
    assert.equal(byId.get("recommendation")?.state, "ok");
    assert.equal(byId.get("recommendation")?.detail, "all good");
  });

  it("downgrades the recommendation to a warning when the doctor run failed", () => {
    const steps = providerDoctorSteps(
      doctorResult({ status: "failed", recommendation: "rotate the key" }),
      false,
    );
    assert.equal(steps.at(-1)?.state, "warning");
    assert.equal(steps.at(-1)?.detail, "rotate the key");
  });
});
