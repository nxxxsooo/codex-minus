import assert from "node:assert";
import { registerHooks } from "node:module";
import { describe, it } from "node:test";

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

const { tf } = await import(`./i18n.ts?template-lookup-test=${Date.now()}`);

describe("English interpolation", () => {
  it("looks up the dormant native-catalog warning from the template dictionary", () => {
    assert.equal(
      tf("原生目录模式下有 {0} 个自定义模型暂不生效。", [7]),
      "7 custom model(s) are dormant in native catalog mode.",
    );
  });

  it("looks up the pending native-catalog warning from the template dictionary", () => {
    assert.equal(
      tf("保存后，{0} 个自定义模型将暂不生效。", [7]),
      "After Save, 7 custom model(s) will be dormant.",
    );
  });
});
