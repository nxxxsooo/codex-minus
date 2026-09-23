import { Moon, RefreshCw, RotateCcw, Sun } from "lucide-react";
import { Button } from "./components/ui/button";
import { useDialogFocus } from "./dialog-focus";
import { getLanguage, t, toggleLanguage } from "./i18n";
import type { Theme } from "./backend-types";

export function PreferencesDialog({ theme, version, checking, onTheme, onUpdate, onRestart, onClose }: {
  theme: Theme;
  version: string;
  checking: boolean;
  onTheme: (theme: Theme) => void;
  onUpdate: () => void;
  onRestart: () => void;
  onClose: () => void;
}) {
  const dialog = useDialogFocus<HTMLDivElement>(onClose);
  return <div className="modal-backdrop" aria-labelledby="preferences-title" aria-modal="true" role="dialog"
    ref={dialog.ref} onKeyDown={dialog.onKeyDown} tabIndex={-1}>
    <div className="modal-card preferences-modal">
      <div className="modal-head"><div><h2 id="preferences-title">{t("偏好设置")}</h2>
        <p>{t("界面外观与本机应用操作")}</p></div>
        <Button onClick={onClose} size="sm" variant="outline">{t("关闭偏好设置")}</Button>
      </div>
      <section className="preferences-section" aria-label={t("外观主题")}>
        <strong>{t("外观主题")}</strong>
        <div className="preferences-choices">
          <button aria-pressed={theme === "light"} className="preferences-choice" onClick={() => onTheme("light")} type="button"><Sun />{t("浅色模式")}</button>
          <button aria-pressed={theme === "dark"} className="preferences-choice" onClick={() => onTheme("dark")} type="button"><Moon />{t("深色模式")}</button>
        </div>
      </section>
      <section className="preferences-section" aria-label={t("界面语言")}>
        <strong>{t("界面语言")}</strong>
        <div className="preferences-choices">
          <button aria-pressed={getLanguage() === "zh"} className="preferences-choice" onClick={() => { if (getLanguage() !== "zh") toggleLanguage(); }} type="button">简体中文</button>
          <button aria-pressed={getLanguage() === "en"} className="preferences-choice" onClick={() => { if (getLanguage() !== "en") toggleLanguage(); }} type="button">English</button>
        </div>
      </section>
      <section className="preferences-section preferences-maintenance" aria-label={t("版本与更新")}>
        <strong>{t("版本与更新")}</strong><span>{version ? `v${version}` : t("版本读取中…")}</span>
        <Button disabled={checking} onClick={onUpdate} size="sm" variant="outline"><RefreshCw />{checking ? t("检查中…") : t("检查更新")}</Button>
      </section>
      <section className="preferences-section preferences-maintenance" aria-label={t("重启 Codex")}>
        <strong>{t("重启 Codex")}</strong><span>{t("只在确认后重启官方 Codex 客户端。")}</span>
        <Button onClick={onRestart} size="sm" variant="outline"><RotateCcw />{t("重启 Codex")}</Button>
      </section>
    </div>
  </div>;
}
