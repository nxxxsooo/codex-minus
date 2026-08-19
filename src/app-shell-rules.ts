// Rules that used to live in the application shell. Each is a pure function a test can call
// without rendering anything: input parsers used by one form, label tables that translate a
// backend enum for one screen, and small formatting helpers.

import type {
  BackendSettings,
  CatalogMode,
  ReasoningLevel,
  RelayMode,
  RelayProfile,
} from "./backend-types.ts";
import { t } from "./i18n.ts";

export function stringifyError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

export function formatTime(value: number): string {
  if (!value) return "-";
  return new Date(value).toLocaleString("zh-CN");
}

export function truncateSessionDeletePreview(value: string): string {
  const normalized = value.trim();
  return normalized.length > 20 ? `${normalized.slice(0, 20)}...` : normalized;
}

export function positiveNumberOrNull(value: string): number | null {
  const parsed = Number.parseInt(value.replace(/[^\d]/g, ""), 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : null;
}

export function positiveNumberOrDefault(value: string, fallback: number): number {
  return positiveNumberOrNull(value) ?? fallback;
}

export function boundedPercentOrNull(value: string): number | null {
  const parsed = positiveNumberOrNull(value);
  return parsed !== null && parsed <= 100 ? parsed : null;
}

export function boundedPercentOrDefault(value: string, fallback: number): number {
  return boundedPercentOrNull(value) ?? fallback;
}

export function integerOrNull(value: string): number | null {
  if (!value.trim()) return null;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : null;
}

export function integerOrDefault(value: string, fallback: number): number {
  return integerOrNull(value) ?? fallback;
}

export function parseCommaListOrNull(value: string): string[] | null {
  const items = [...new Set(value.split(",").map((item) => item.trim()).filter(Boolean))];
  return items.length ? items : null;
}

export function parseReasoningLevels(value: string): ReasoningLevel[] | null {
  const efforts = parseCommaListOrNull(value);
  return efforts?.map((effort) => ({ effort, description: effort })) ?? null;
}

export function reasoningEffortsText(levels: ReasoningLevel[]): string {
  return levels.map((level) => level.effort).join(",");
}

export function managedCatalogMode(mode: CatalogMode): boolean {
  return mode === "official-plus-custom" || mode === "custom-only";
}

export function catalogDraftErrorLabel(error: string | null): string {
  if (error === "empty-custom-slug") return t("自定义模型 slug 不能为空。");
  if (error === "empty-display-name") return t("自定义模型显示名不能为空。");
  if (error === "duplicate-custom-slug") return t("自定义模型 slug 不能重复。");
  if (error === "invalid-context-window") return t("上下文窗口必须是正整数。");
  if (error === "invalid-effective-percent") return t("有效上下文百分比必须为 1 到 100。");
  if (error === "invalid-reasoning-levels") return t("推理级别不能为空或重复。");
  if (error === "invalid-reasoning-default") return t("默认推理级别必须包含在支持列表中。");
  if (error === "invalid-default-model") return t("当前默认模型不在有效目录中，请先调整目录或默认模型。");
  if (error === "empty-catalog") return t("模型列表不能为空，至少保留一个模型。");
  return "";
}

export function envConflictSourceLabel(source: string): string {
  if (source === "process") return t("当前进程");
  if (source === "user") return t("用户环境");
  return source || t("环境变量");
}

export function providerInitial(name: string): string {
  const trimmed = (name || t("供应商")).trim();
  return Array.from(trimmed)[0]?.toUpperCase() || t("供");
}

export function relayModeLabel(mode: RelayMode): string {
  if (mode === "pureApi") return t("纯 API");
  return t("官方登录");
}

export function relayProfileConfigBrief(profile: RelayProfile): string {
  if (profile.relayMode === "official") return profile.officialMixApiKey ? t("混入 API Key") : t("不写 API 文件");
  return profile.baseUrl || t("未填写 URL");
}

export function relayProfileEditorStatus(
  profile: RelayProfile,
  form: BackendSettings,
  isNew: boolean,
): string {
  if (isNew) return t("新建供应商需要先保存到列表");
  if (!form.relayProfilesEnabled) return t("供应商配置总开关已关闭；当前只保存配置，不写入 Codex live 文件");
  return profile.id === form.activeRelayId ? t("当前正在使用") : t("编辑后保存列表，再切换模式时会使用新配置");
}

export function relayProfileModeHelp(profile: RelayProfile): string {
  if (profile.relayMode === "official") {
    if (profile.officialMixApiKey) {
      return t("此供应商会保留官方登录模式，并把请求混入当前 API Key。");
    }
    return t("此供应商会切回官方登录模式，使用 ChatGPT 官方账号，不写入 API Key。");
  }
  if (profile.relayMode === "pureApi") {
    return t("此供应商只把 API Key 写入 owner-only 的 provider bearer 配置，并明确不要求 ChatGPT 认证。");
  }
  return t("此供应商会保留官方登录模式，并把请求混入当前 API Key。");
}
