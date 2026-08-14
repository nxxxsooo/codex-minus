import assert from "node:assert";
import { describe, it } from "node:test";

import { rootTomlStringValue, tomlKey, tomlString } from "./codex-toml.ts";

/// The escaping contract, pinned because three modules used to carry their own copy of it and two
/// of the copies disagreed. Whatever `tomlString` emits is written between the quotes of a TOML
/// basic string, so the only thing that matters is that Codex can parse the line back.
describe("escaping a value into a TOML basic string", () => {
  it("escapes the two characters that would end or continue the string", () => {
    assert.equal(tomlString('say "hi"'), 'say \\"hi\\"');
    assert.equal(tomlString("C:\\Users"), "C:\\\\Users");
  });

  it("escapes a newline instead of emitting it raw", () => {
    // The reason the hand-rolled copy had to lose. A basic string may not span lines, so a raw
    // newline here does not produce a wrong value — it produces a config Codex refuses to load.
    assert.equal(tomlString("a\nb"), "a\\nb");
    assert.equal(tomlString("a\rb"), "a\\rb");
    assert.equal(tomlString("a\tb"), "a\\tb");
  });

  it("escapes the control characters TOML has no literal spelling for", () => {
    assert.equal(tomlString("a\u0000b"), "a\\u0000b");
    assert.equal(tomlString("a\u001fb"), "a\\u001fb");
  });

  it("leaves ordinary text, including non-ASCII, alone", () => {
    assert.equal(tomlString("gpt-5.6-codex"), "gpt-5.6-codex");
    assert.equal(tomlString("模型"), "模型");
  });

  it("survives a round trip back through the reader", () => {
    for (const value of ['a "quoted" path', "C:\\a\\b", "gpt-5.6-codex", "模型"]) {
      const contents = `key = "${tomlString(value)}"\n`;
      assert.equal(rootTomlStringValue(contents, "key"), value, `round trip: ${JSON.stringify(value)}`);
    }
  });

  it("writes escapes the reader does not undo, and that is the writer being right", () => {
    // The reader unescapes only `\"` `\'` `\\`, because it exists to pull a base URL or a model
    // slug out of a config a human wrote, and those never contain a newline. Writing `\n` is still
    // correct: a raw newline would make the file unparseable for Codex itself, which is worse than
    // this module reading one implausible value back as literal backslash-n. Pinned so that a
    // future reader upgrade is a deliberate choice rather than a surprise.
    const contents = `key = "${tomlString("line\nbreak")}"\n`;
    assert.equal(rootTomlStringValue(contents, "key"), "line\\nbreak");
  });
});

describe("quoting a TOML key", () => {
  it("leaves a bare key bare", () => {
    assert.equal(tomlKey("mcp_servers"), "mcp_servers");
    assert.equal(tomlKey("my-server-1"), "my-server-1");
  });

  it("quotes and escapes anything else", () => {
    assert.equal(tomlKey("has space"), '"has space"');
    assert.equal(tomlKey('has"quote'), '"has\\"quote"');
    assert.equal(tomlKey("has\nnewline"), '"has\\nnewline"');
  });
});
