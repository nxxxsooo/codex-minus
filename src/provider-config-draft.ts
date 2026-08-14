import {
  ensureTrailingNewline,
  rootTomlStringValue,
  tomlSectionName,
  tomlString,
} from "./codex-toml.ts";
import {
  materializeNewProviderConfig,
  type NewProviderTransientTarget,
} from "./provider-onboarding.ts";

export type ProviderConfigTarget =
  | NewProviderTransientTarget
  | "pureApi"
  | "compatibility"
  | "pureOAuth"
  | "preserveExisting";

export type ProviderConfigTargetContract =
  | {
      target: Exclude<ProviderConfigTarget, "preserveExisting">;
      source: "brand-new-empty";
    }
  | {
      target: "preserveExisting";
      source: "existing";
    };

export type ProviderConfigProfile = {
  model: string;
  baseUrl: string;
  upstreamBaseUrl: string;
  apiKey: string;
  protocol: string;
  configContents: string;
  contextWindow: string;
  autoCompactLimit: string;
  transientTarget?: NewProviderTransientTarget;
};

const PROTOCOL_PROXY_BASE_URL = "http://127.0.0.1:57321/v1";
const CHAT_UPSTREAM_BASE_URL_KEY = "codex_plus_chat_base_url";

export function buildRelayConfigToml(
  profile: ProviderConfigProfile,
  contract: ProviderConfigTargetContract,
): string {
  if (contract.source === "existing") return profile.configContents;
  if (profile.configContents.trim()) return profile.configContents;
  if (contract.target === "pureOAuth") return "";
  if (contract.target === "nativePriority") {
    return materializeNewProviderConfig({
      transientTarget: contract.target,
      model: profile.model,
      baseUrl: profile.baseUrl,
      apiKey: profile.apiKey,
      configContents: profile.configContents,
    }).configContents;
  }

  const model = profile.model.trim();
  const baseUrl = profile.protocol === "chatCompletions"
    ? PROTOCOL_PROXY_BASE_URL
    : profile.baseUrl.trim();
  const apiKey = profile.apiKey.trim();
  return [
    model ? `model = "${tomlString(model)}"` : null,
    'model_provider = "custom"',
    "",
    "[model_providers.custom]",
    'name = "custom"',
    'wire_api = "responses"',
    `requires_openai_auth = ${contract.target === "pureApi" ? "false" : "true"}`,
    `base_url = "${tomlString(baseUrl)}"`,
    apiKey ? `experimental_bearer_token = "${tomlString(apiKey)}"` : null,
    "",
  ].filter((line): line is string => line !== null).join("\n");
}

export function withGeneratedRelayConfig<T extends ProviderConfigProfile>(
  profile: T,
  contract: ProviderConfigTargetContract,
): T {
  return {
    ...profile,
    configContents: buildRelayConfigToml(profile, contract),
  };
}

export function applyProviderConfigPatch<T extends ProviderConfigProfile>(
  profile: T,
  patch: Partial<ProviderConfigProfile>,
  contract: ProviderConfigTargetContract,
): T {
  let next = { ...profile, ...patch } as T;
  if (!next.configContents.trim() && contract.source === "brand-new-empty") {
    next = withGeneratedRelayConfig(next, contract);
    if (!next.configContents.trim()) return next;
  }
  if (!next.configContents.trim()) return next;

  if ("model" in patch) {
    next.configContents = setRootTomlStringKey(
      next.configContents,
      "model",
      modelSlug(patch.model ?? ""),
    );
  }
  if ("apiKey" in patch) {
    next.configContents = setProviderStringKey(
      next.configContents,
      "experimental_bearer_token",
      patch.apiKey ?? "",
    );
  }
  if ("baseUrl" in patch) next.upstreamBaseUrl = patch.baseUrl ?? "";
  if ("upstreamBaseUrl" in patch) next.baseUrl = patch.upstreamBaseUrl ?? "";
  if ("baseUrl" in patch || "upstreamBaseUrl" in patch || "protocol" in patch) {
    const baseUrl = next.protocol === "chatCompletions"
      ? PROTOCOL_PROXY_BASE_URL
      : next.upstreamBaseUrl || next.baseUrl;
    next.configContents = setProviderStringKey(next.configContents, "base_url", baseUrl);
    next.configContents = removeRootTomlKey(next.configContents, CHAT_UPSTREAM_BASE_URL_KEY);
  }
  if ("contextWindow" in patch) {
    next.configContents = setRootTomlIntKey(
      next.configContents,
      "model_context_window",
      patch.contextWindow ?? "",
    );
  }
  if ("autoCompactLimit" in patch) {
    next.configContents = setRootTomlIntKey(
      next.configContents,
      "model_auto_compact_token_limit",
      patch.autoCompactLimit ?? "",
    );
  }
  return next;
}

function modelSlug(value: string): string {
  const trimmed = value.trim();
  const match = /^(.*?)\[(\d+(?:[KkMm])?)\]$/.exec(trimmed);
  return match ? match[1].trim() : trimmed;
}

function setRootTomlStringKey(contents: string, key: string, value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return removeRootTomlKey(contents, key);
  return setRootTomlLine(contents, key, `${key} = "${tomlString(trimmed)}"`);
}

function setRootTomlIntKey(contents: string, key: string, value: string): string {
  const trimmed = value.replace(/[^\d]/g, "");
  if (!trimmed) return removeRootTomlKey(contents, key);
  return setRootTomlLine(contents, key, `${key} = ${trimmed}`);
}

function setRootTomlLine(contents: string, key: string, lineText: string): string {
  const lines = contents.split(/\r?\n/);
  const firstTable = lines.findIndex((line) => /^\s*\[[^\]]+\]\s*$/.test(line));
  const rootEnd = firstTable >= 0 ? firstTable : lines.length;
  for (let index = 0; index < rootEnd; index += 1) {
    if (new RegExp(`^\\s*${key}\\s*=`).test(lines[index])) {
      lines[index] = lineText;
      return ensureTrailingNewline(lines.join("\n").trimEnd());
    }
  }
  lines.splice(key === "model" ? 0 : rootEnd, 0, lineText);
  return ensureTrailingNewline(lines.join("\n").trimEnd());
}

function setProviderStringKey(contents: string, key: string, value: string): string {
  const provider = rootTomlStringValue(contents, "model_provider");
  if (!provider) return contents;
  const section = `model_providers.${provider}`;
  return value.trim()
    ? setTomlSectionRawKey(contents, section, key, `"${tomlString(value.trim())}"`)
    : removeTomlSectionKey(contents, section, key);
}

function setTomlSectionRawKey(
  contents: string,
  sectionName: string,
  key: string,
  value: string,
): string {
  const lines = contents.split(/\r?\n/);
  let sectionStart = -1;
  let sectionEnd = lines.length;
  for (let index = 0; index < lines.length; index += 1) {
    const section = tomlSectionName(lines[index]);
    if (section === null) continue;
    if (sectionStart >= 0) {
      sectionEnd = index;
      break;
    }
    if (section === sectionName) sectionStart = index;
  }
  if (sectionStart < 0) return contents;
  const replacement = `${key} = ${value}`;
  for (let index = sectionStart + 1; index < sectionEnd; index += 1) {
    if (new RegExp(`^\\s*${key}\\s*=`).test(lines[index])) {
      lines[index] = replacement;
      return ensureTrailingNewline(lines.join("\n").trimEnd());
    }
  }
  let insertAt = sectionEnd;
  while (insertAt > sectionStart + 1 && lines[insertAt - 1].trim() === "") insertAt -= 1;
  lines.splice(insertAt, 0, replacement);
  return ensureTrailingNewline(lines.join("\n").trimEnd());
}

function removeTomlSectionKey(contents: string, sectionName: string, key: string): string {
  const lines = contents.split(/\r?\n/);
  let sectionStart = -1;
  let sectionEnd = lines.length;
  for (let index = 0; index < lines.length; index += 1) {
    const section = tomlSectionName(lines[index]);
    if (section === null) continue;
    if (sectionStart >= 0) {
      sectionEnd = index;
      break;
    }
    if (section === sectionName) sectionStart = index;
  }
  if (sectionStart < 0) return contents;
  return ensureTrailingNewline(lines.filter((line, index) => (
    index <= sectionStart
    || index >= sectionEnd
    || !new RegExp(`^\\s*${key}\\s*=`).test(line)
  )).join("\n").trimEnd());
}

function removeRootTomlKey(contents: string, key: string): string {
  const lines: string[] = [];
  let inRoot = true;
  for (const line of contents.split(/\r?\n/)) {
    if (/^\s*\[[^\]]+\]\s*$/.test(line)) inRoot = false;
    if (inRoot && new RegExp(`^\\s*${key}\\s*=`).test(line)) continue;
    lines.push(line);
  }
  return ensureTrailingNewline(lines.join("\n").trimEnd());
}

