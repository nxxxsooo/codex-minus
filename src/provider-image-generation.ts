import { rootTomlStringValue, tomlTablePathFromLine } from "./codex-toml.ts";

function booleanSetting(contents: string, path: string[], key: string): boolean | null {
  let section: string[] = [];
  for (const line of contents.split(/\r?\n/)) {
    const header = tomlTablePathFromLine(line.replace(/\s+#.*$/, ""));
    if (header) { section = header; continue; }
    if (section.length !== path.length || section.some((part, index) => part !== path[index])) continue;
    const match = new RegExp(`^\\s*["']?${key}["']?\\s*=\\s*(true|false)\\s*(?:#.*)?$`).exec(line);
    if (match) return match[1] === "true";
  }
  // The native transformer also preserves inline and dotted features tables.
  if (path.length === 1 && path[0] === "features") {
    const root = contents.split(/^\s*\[/m)[0];
    const dotted = new RegExp(`^\\s*features\\.${key}\\s*=\\s*(true|false)\\s*(?:#.*)?$`, "m").exec(root);
    if (dotted) return dotted[1] === "true";
    const inline = /^\s*features\s*=\s*\{([^\n]*)\}/m.exec(root);
    const match = inline && new RegExp(`(?:^|,)\\s*["']?${key}["']?\\s*=\\s*(true|false)\\s*(?=,|$)`).exec(inline[1]);
    if (match) return match[1] === "true";
  }
  return null;
}

export function providerImageGenerationEnabled(profile: { configContents: string; relayMode: string }): boolean {
  const provider = rootTomlStringValue(profile.configContents, "model_provider");
  return profile.relayMode === "pureApi"
    && !!provider
    && booleanSetting(profile.configContents, ["model_providers", provider], "requires_openai_auth") === false
    && booleanSetting(profile.configContents, ["features"], "image_generation") === true;
}
