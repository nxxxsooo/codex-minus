import { useState } from "react";
import { RefreshCw } from "lucide-react";
import { reconnectCore } from "./desktop-api";
import { useCoreConnection } from "./use-core-connection";
import { Button } from "./components/ui/button";
import { t } from "./i18n";

export function DesktopConnectionNotice({ onReconnected }: { onReconnected: () => Promise<void> }) {
  const state = useCoreConnection();
  const [reconnecting, setReconnecting] = useState(false);
  if (state.state === "ready") return null;
  const disconnected = state.state === "disconnected";
  return <div className={`desktop-connection-notice ${disconnected ? "failed" : ""}`} role="status">
    <div>
      <strong>{t(disconnected ? "核心连接已断开" : "正在连接本机核心…")}</strong>
      {disconnected ? <p>{t(state.code === "AlreadyRunning"
        ? "另一个管理器正在使用本机配置，请先退出它再重新连接。"
        : "草稿仍保留在页面中；中断请求的结果尚未确认，请重新连接并读取配置。")}</p> : null}
    </div>
    {disconnected ? <Button size="sm" variant="outline" disabled={reconnecting} onClick={async () => {
      setReconnecting(true);
      try {
        const next = await reconnectCore();
        if (next.state !== "disconnected") await onReconnected();
      } catch { /* The connection store keeps the failed state visible. */ }
      finally { setReconnecting(false); }
    }}><RefreshCw className="h-4 w-4" />{t(reconnecting ? "连接中…" : "重新连接")}</Button> : null}
  </div>;
}
