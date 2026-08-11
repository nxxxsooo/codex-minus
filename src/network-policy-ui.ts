export type NetworkPolicyModeValue = "auto" | "direct" | "custom";

export type NetworkPolicyDraft = {
  mode: NetworkPolicyModeValue;
  customProxyUrl: string;
  customNoProxy: string;
};

export type NetworkPolicyStatusView = NetworkPolicyDraft & {
  source: string;
  endpoint: string | null;
  bypassCount: number;
  supported: boolean;
  actionRequired: string | null;
};

export function networkPolicyDraft(status: NetworkPolicyStatusView | null): NetworkPolicyDraft {
  return status
    ? {
        mode: status.mode,
        customProxyUrl: status.customProxyUrl,
        customNoProxy: status.customNoProxy,
      }
    : { mode: "auto", customProxyUrl: "", customNoProxy: "" };
}

export function validateNetworkPolicyDraft(draft: NetworkPolicyDraft): string | null {
  if (draft.mode !== "custom") return null;
  const value = draft.customProxyUrl.trim();
  if (!value) return "custom-proxy-required";
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return "custom-proxy-invalid";
  }
  if (!new Set(["http:", "https:", "socks5:", "socks5h:"]).has(url.protocol)) {
    return "custom-proxy-scheme";
  }
  if (!url.hostname || url.username || url.password) return "custom-proxy-credentials";
  if ((url.pathname && url.pathname !== "/") || url.search || url.hash) return "custom-proxy-invalid";
  const entries = draft.customNoProxy
    .split(/[\s,;]+/)
    .map((entry) => entry.trim())
    .filter(Boolean);
  if (entries.length > 128 || entries.some((entry) => entry.length > 255)) return "custom-bypass-invalid";
  return null;
}

export function networkPolicyDirty(
  draft: NetworkPolicyDraft,
  status: NetworkPolicyStatusView | null,
): boolean {
  if (!status) return false;
  const saved = networkPolicyDraft(status);
  return (
    draft.mode !== saved.mode ||
    draft.customProxyUrl.trim() !== saved.customProxyUrl.trim() ||
    normalizeBypassText(draft.customNoProxy) !== normalizeBypassText(saved.customNoProxy)
  );
}

export function networkPolicyPresentation(status: NetworkPolicyStatusView | null): {
  state: "loading" | "ready" | "action-required";
  source: string;
  endpoint: string;
} {
  if (!status) return { state: "loading", source: "", endpoint: "" };
  return {
    state: status.supported ? "ready" : "action-required",
    source: status.source,
    endpoint: redactEndpoint(status.endpoint),
  };
}

export function networkTestCategoryLabel(category: string): string {
  return new Set([
    "ok",
    "dns",
    "proxy-connect",
    "proxy-auth-unsupported",
    "tls",
    "timeout",
    "unsupported-policy",
    "bundled-fallback",
  ]).has(category)
    ? category
    : "other";
}

function normalizeBypassText(value: string): string {
  return Array.from(
    new Set(
      value
        .split(/[\s,;]+/)
        .map((entry) => entry.trim().toLowerCase())
        .filter(Boolean),
    ),
  )
    .sort()
    .join(",");
}

function redactEndpoint(value: string | null): string {
  if (!value) return "";
  try {
    const url = new URL(value);
    return `${url.protocol}//${url.hostname}${url.port ? `:${url.port}` : ""}`;
  } catch {
    return "configured-proxy";
  }
}
