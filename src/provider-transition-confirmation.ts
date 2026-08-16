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
