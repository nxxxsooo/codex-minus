import type { RelayProfile } from "./backend-types";
import { t, tf } from "./i18n.ts";
import type { ProviderDetailDraftState } from "./provider-detail-draft-state";

export function providerTransitionConfirmationMessage(
  state: ProviderDetailDraftState<RelayProfile>,
): string {
  const pending = state.pendingConfirmation;
  if (!pending) return t("当前没有等待确认的供应商转换。");
  if (pending.requiredConfirmation === "replaceActorHeader") {
    return t("当前自定义 Actor 标记与原生能力优先标记冲突。确认后只替换冲突的 Actor 标记并更新草稿，仍需点击保存或设为当前才会生效。是否继续？");
  }
  if (pending.transition.action === "exitPureApi") {
    return t("切换到纯 API 后不再需要官方登录：只用中转 Key 请求，移除本管理器的 Actor 标记，模型目录切为仅自定义，不再声明官方登录派生的原生能力。确认后只更新草稿，仍需点击保存或设为当前才会生效。是否继续？");
  }
  if (pending.transition.action === "exitPureOAuth") {
    const providerId = state.preview?.removedProviderId || t("当前自定义供应商");
    const fields = state.preview?.removedProviderFields.join("、") || t("全部供应商字段");
    return tf(
      "切换到纯 OAuth 将删除自定义供应商 {0} 及其全部配置字段（{1}）。确认后只更新草稿，仍需点击保存或设为当前才会生效。是否继续？",
      [providerId, fields],
    );
  }
  return t("切换到兼容模式将失去原生能力优先配置。确认后只更新草稿，仍需点击保存或设为当前才会生效。是否继续？");
}

/// Confirmation shown before a pure-OAuth profile holding a newly entered provider key is saved.
/// The save is the explicit action that applies the enablement; declining keeps the profile pure
/// OAuth and drops the draft key.
export function providerPureOAuthEnablementConfirmationMessage(): string {
  return t("此供应商当前是官方登录模式。保存这个 Key 会把它升级为「官方登录＋混入 API Key」的混合契约，使用时需要已登录的 ChatGPT 客户端；取消则保持官方登录模式并丢弃这个 Key。确认升级并继续保存吗？");
}
