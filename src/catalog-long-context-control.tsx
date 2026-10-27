import { useEffect, useRef } from "react";
import { t } from "./i18n";
import { longContextState, setLongContext } from "./catalog-long-context";
import type { CatalogModeValue, CatalogOverlayDraft } from "./model-catalog-ui";

export function CatalogLongContextControl(props: {
  overlay: CatalogOverlayDraft;
  officialModels: readonly { slug: string; visible: boolean; contextWindow?: number | null }[];
  mode: CatalogModeValue;
  disabled: boolean;
  onChange: (overlay: CatalogOverlayDraft) => void;
}) {
  const state = longContextState(props);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => { if (input.current) input.current.indeterminate = state.mixed; }, [state.mixed]);
  return (
    <label className="switch-row catalog-long-context">
      <input ref={input} type="checkbox" checked={state.checked} disabled={props.disabled || !state.available}
        onChange={(event) => props.onChange(setLongContext(props, event.currentTarget.checked))} />
      <span>
        <strong>{t("解锁 1,050,000 上下文")}{state.mixed ? t("（部分已启用）") : ""}</strong>
        <small>{t("仅应用于列表中的 Astra、Sol、Terra、Luna；关闭恢复默认窗口。保存后生效，实际容量由上游决定。")}</small>
      </span>
      <span aria-hidden="true" className="toggle-switch-visual"><span className="toggle-switch-thumb" /></span>
    </label>
  );
}
