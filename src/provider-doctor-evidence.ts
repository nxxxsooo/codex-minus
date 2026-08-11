export type ProviderProbeTextEvidence =
  | "reachable"
  | "fallbackReachable"
  | "denied"
  | "unknown";

export type ProviderProbeCapabilityEvidence = {
  textResponses: ProviderProbeTextEvidence;
  compatibilityFallbackUsed: boolean;
  imageGeneration: "unknown";
  nativeExtension: "unknown";
  catalogModel: "unknown";
  selectedModel: "unknown";
  providerGroup: "unknown";
};

export type ProviderProbeEvidenceObservation = {
  sessionToken: symbol;
  revision: number;
  catalogEvidenceFingerprint: string;
  sourceFingerprint: string;
  evidence: ProviderProbeCapabilityEvidence;
};

type ProviderDoctorEvidenceInput = {
  status: string;
  protocol: "responses" | "chatCompletions" | "unknown";
  requestHttpStatus: number | null;
  compatibilityFallbackUsed: boolean;
  checks: Array<{
    id: string;
    status: string;
    [key: string]: unknown;
  }>;
  [key: string]: unknown;
};

type ProviderQuickProbeEvidenceInput = {
  status: string;
  protocol: "responses" | "chatCompletions" | "unknown";
  httpStatus: number;
  compatibilityFallbackUsed: boolean;
  [key: string]: unknown;
};

const unknownProbeEvidence = (): ProviderProbeCapabilityEvidence => ({
  textResponses: "unknown",
  compatibilityFallbackUsed: false,
  imageGeneration: "unknown",
  nativeExtension: "unknown",
  catalogModel: "unknown",
  selectedModel: "unknown",
  providerGroup: "unknown",
});

export function providerDoctorEvidence(
  input: ProviderDoctorEvidenceInput,
): ProviderProbeCapabilityEvidence {
  const request = input.checks.find((check) => check.id === "request");
  if (
    input.protocol !== "responses"
    || request?.status !== "ok"
    || input.requestHttpStatus === null
    || input.requestHttpStatus < 200
    || input.requestHttpStatus >= 300
  ) return unknownProbeEvidence();
  return {
    ...unknownProbeEvidence(),
    textResponses: input.compatibilityFallbackUsed ? "fallbackReachable" : "reachable",
    compatibilityFallbackUsed: input.compatibilityFallbackUsed,
  };
}

export function providerQuickProbeEvidence(
  input: ProviderQuickProbeEvidenceInput,
): ProviderProbeCapabilityEvidence {
  if (input.protocol !== "responses") return unknownProbeEvidence();
  if (input.status === "ok" && input.httpStatus >= 200 && input.httpStatus < 300) {
    return {
      ...unknownProbeEvidence(),
      textResponses: input.compatibilityFallbackUsed ? "fallbackReachable" : "reachable",
      compatibilityFallbackUsed: input.compatibilityFallbackUsed,
    };
  }
  if (input.httpStatus === 401 || input.httpStatus === 403) {
    return { ...unknownProbeEvidence(), textResponses: "denied" };
  }
  return unknownProbeEvidence();
}

export function mergeProviderProbeEvidence(
  ledger: ProviderCapabilityLedger,
  evidence: ProviderProbeCapabilityEvidence,
): ProviderCapabilityLedger {
  return {
    ...ledger,
    upstream: {
      ...ledger.upstream,
      textResponses: evidence.textResponses,
    },
  };
}

export function mergeCurrentProviderProbeObservation(input: {
  ledger: ProviderCapabilityLedger | null;
  observation: ProviderProbeEvidenceObservation | null;
  currentSessionToken: symbol;
  currentRevision: number;
  currentCatalogEvidenceFingerprint: string;
  currentSourceFingerprint: string;
}): ProviderCapabilityLedger | null {
  if (!input.ledger) return null;
  if (
    !input.observation
    || input.observation.sessionToken !== input.currentSessionToken
    || input.observation.revision !== input.currentRevision
    || input.observation.catalogEvidenceFingerprint
      !== input.currentCatalogEvidenceFingerprint
    || input.observation.sourceFingerprint !== input.currentSourceFingerprint
  ) return input.ledger;
  return mergeProviderProbeEvidence(input.ledger, input.observation.evidence);
}

export function providerDoctorRequestMatchesSource(input: {
  requestSequence: number;
  latestRequestSequence: number;
  requestSourceFingerprint: string;
  currentSourceFingerprint: string;
}): boolean {
  return input.requestSequence === input.latestRequestSequence
    && input.requestSourceFingerprint === input.currentSourceFingerprint;
}

export function providerCapabilityOwnershipCopy(language: "zh" | "en"): {
  oauth: string;
  providerKey: string;
  actor: string;
  gates: string;
} {
  if (language === "zh") {
    return {
      oauth: "OAuth 登录与会话仍由官方客户端拥有和维护。",
      providerKey: "供应商 API Key 只用于中转服务的推理请求鉴权。",
      actor: "Actor 标记只提供客户端原生扩展资格，不代表订阅权益或能力成功。",
      gates: "上游路由、模型元数据、账号计划与运行时状态仍需各自验证。",
    };
  }
  return {
    oauth: "OAuth sign-in and session data remain owned and maintained by the official client.",
    providerKey: "The provider API key authenticates inference requests to the relay only.",
    actor: "The Actor marker provides client eligibility only; it proves neither entitlement nor capability success.",
    gates: "Upstream routing, model metadata, account plan, and runtime state still require independent evidence.",
  };
}
import type { ProviderCapabilityLedger } from "./provider-capability-ledger";
