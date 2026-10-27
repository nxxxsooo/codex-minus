import { t } from "./i18n";
import { providerImageGenerationEnabled } from "./provider-image-generation";

export function ProviderImageGenerationControl(props: {
  profile: { configContents: string; relayMode: string };
  disabled: boolean;
  isNew: boolean;
  onChange: (enabled: boolean) => void;
}) {
  return (
    <label className="switch-row">
      <input type="checkbox" checked={providerImageGenerationEnabled(props.profile)}
        disabled={props.disabled || props.isNew} onChange={(event) => props.onChange(event.currentTarget.checked)} />
      <span>
        <strong>{t("图像工具（无需登录）")}</strong>
        <small>{props.isNew ? t("新建时选择纯 API 接入即开启；保存供应商后可独立开关图像工具。")
          : t("开启：requires_openai_auth = false、image_generation = true。关闭只禁用图像工具；保存后生效，需上游支持。")}</small>
      </span>
      <span aria-hidden="true" className="toggle-switch-visual"><span className="toggle-switch-thumb" /></span>
    </label>
  );
}
