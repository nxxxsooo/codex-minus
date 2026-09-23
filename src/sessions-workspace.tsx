import { useEffect, useMemo, useState } from "react";
import { Archive, ArchiveRestore, CheckCircle2, Info, RefreshCw, Trash2, TriangleAlert } from "lucide-react";
import type { ArchiveMaintenanceResult, ArchivePreviewResult, LocalSession, LocalSessionsResult, ProviderCompatibilityResult, SessionLifecycleSettingsResult } from "./backend-types";
import { formatTime } from "./app-shell-rules";
import { isSuccessStatus } from "./status-presentation";
import { t, tf } from "./i18n";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";

export type SessionActions = {
  coreAvailable: boolean;
  refreshLocalSessions: (silent?: boolean, archived?: boolean, cursor?: string) => Promise<LocalSessionsResult | null>;
  refreshSessionLifecycle: (silent?: boolean) => Promise<SessionLifecycleSettingsResult | null>;
  refreshArchivePreview: (retentionDays?: number, silent?: boolean) => Promise<ArchivePreviewResult | null>;
  saveSessionLifecycle: (patch: Partial<SessionLifecycleSettingsResult>, silent?: boolean) => Promise<SessionLifecycleSettingsResult | null>;
  enableSessionArchiving: (retentionDays: number) => Promise<void>;
  runArchiveMaintenance: (force?: boolean) => Promise<ArchiveMaintenanceResult | null>;
  archiveOrRestoreSession: (session: LocalSession, archived: boolean) => Promise<void>;
  refreshProviderCompatibility: (silent?: boolean) => Promise<ProviderCompatibilityResult | null>;
  adaptActiveSessions: () => Promise<void>;
  deleteLocalSession: (session: LocalSession) => Promise<void>;
  deleteLocalSessions: (sessions: LocalSession[] | "archived") => Promise<void>;
};

export function SessionsWorkspace({ sessions, archiveView, lifecycle, archivePreview, archiveMaintenance,
  archiveMaintenanceRunning, cleanupRunning, providerCompatibility, providerCompatibilityLoading, actions,
}: {
  sessions: LocalSessionsResult | null; archiveView: boolean;
  lifecycle: SessionLifecycleSettingsResult | null; archivePreview: ArchivePreviewResult | null;
  archiveMaintenance: ArchiveMaintenanceResult | null; archiveMaintenanceRunning: boolean; cleanupRunning: boolean;
  providerCompatibility: ProviderCompatibilityResult | null; providerCompatibilityLoading: boolean;
  actions: SessionActions;
}) {
  const items = sessions?.sessions ?? [];
  const activeCount = sessions?.activeCount ?? 0;
  const archivedCount = sessions?.archivedCount ?? 0;
  const [retentionDays, setRetentionDays] = useState(lifecycle?.retentionDays ?? 30);
  const [selectedSessionIds, setSelectedSessionIds] = useState<Set<string>>(() => new Set());
  const [selectionMode, setSelectionMode] = useState(false);
  const selectedSessions = useMemo(() => items.filter(session => selectedSessionIds.has(session.id)), [items, selectedSessionIds]);
  const selectedCount = selectedSessions.length;
  const allSelected = items.length > 0 && selectedCount === items.length;

  useEffect(() => setRetentionDays(lifecycle?.retentionDays ?? 30), [lifecycle?.retentionDays]);
  useEffect(() => {
    const itemIds = new Set(items.map(session => session.id));
    setSelectedSessionIds(current => {
      const next = new Set(Array.from(current).filter(id => itemIds.has(id)));
      return next.size === current.size ? current : next;
    });
  }, [items]);
  useEffect(() => { setSelectedSessionIds(new Set()); setSelectionMode(false); }, [archiveView]);

  const toggleSessionSelection = (sessionId: string, checked: boolean) => {
    setSelectedSessionIds(current => {
      const next = new Set(current);
      if (checked) next.add(sessionId);
      else next.delete(sessionId);
      return next;
    });
  };
  const clearSelectedSessions = () => setSelectedSessionIds(new Set());
  const toggleArchivePolicy = async (enabled: boolean) => {
    if (enabled) await actions.enableSessionArchiving(retentionDays);
    else await actions.saveSessionLifecycle({ archiveEnabled: false, retentionDays });
  };
  useEffect(() => {
    if (!lifecycle || retentionDays === lifecycle.retentionDays || !actions.coreAvailable) return;
    const timer = window.setTimeout(() => {
      void actions.saveSessionLifecycle({ retentionDays }, true).then(saved => {
        if (saved && isSuccessStatus(saved.status)) void actions.refreshArchivePreview(retentionDays, true);
      });
    }, 800);
    return () => window.clearTimeout(timer);
  }, [retentionDays, lifecycle, actions]);

  return <div className="sessions-workspace">
    <div className="sessions-topline">
      <div>
        <strong>{t(archiveView ? "已归档会话" : "活动会话")}</strong>
        <span>{archiveView ? t("归档可恢复，不释放磁盘空间；原 provider 标记保留。") : t("查看、归档与管理本地会话；切换供应商后仅适配活动会话。")}</span>
      </div>
      <div className="session-view-tabs" role="tablist" aria-label={t("会话视图")}>
        <button aria-selected={!archiveView} className={!archiveView ? "active" : ""} disabled={cleanupRunning}
          onClick={() => void actions.refreshLocalSessions(true, false)} role="tab" type="button">
          {t("活动")} <small>{activeCount}</small>
        </button>
        <button aria-selected={archiveView} className={archiveView ? "active" : ""} disabled={cleanupRunning}
          onClick={() => void actions.refreshLocalSessions(true, true)} role="tab" type="button">
          {t("已归档")} <small>{archivedCount}</small>
        </button>
      </div>
    </div>

    <div className="session-status-grid">
      <section className="session-status-panel">
        <div className="session-status-head">
          <div><strong>{t("会话生命周期")}</strong><small>{t("原生归档，可随时恢复")}</small></div>
          <span className="session-status-tag">{lifecycle?.archiveEnabled ? t("自动归档已启用") : t("自动归档未启用")}</span>
        </div>
        <div className="session-status-summary">
          <span>{tf("超过 {0} 天未活动", [retentionDays])}</span>
          <span>{t("上次检查")}：{lifecycle?.lastCompletedAtMs ? formatTime(lifecycle.lastCompletedAtMs) : t("尚未执行")}</span>
        </div>
        <div className="session-policy-controls">
          <label className="session-policy-toggle">
            <input checked={lifecycle?.archiveEnabled ?? false} disabled={!actions.coreAvailable || !lifecycle}
              onChange={event => void toggleArchivePolicy(event.currentTarget.checked)} type="checkbox" />
            {t("定期归档旧会话")}
          </label>
          <label className="session-retention-label">{t("保留天数")}
            <Input aria-label={t("保留天数")} disabled={!actions.coreAvailable || !lifecycle} max={3650} min={1}
              onChange={event => setRetentionDays(Math.max(1, Math.min(3650, Number(event.currentTarget.value) || 1)))}
              type="number" value={retentionDays} />
          </label>
          <Button disabled={!actions.coreAvailable || !lifecycle?.archiveEnabled || archiveMaintenanceRunning}
            onClick={() => void actions.runArchiveMaintenance(true)} size="sm" variant="outline">
            <RefreshCw />{archiveMaintenanceRunning ? t("检查中…") : t("立即检查")}
          </Button>
        </div>
        {archivePreview ? <p className="session-status-note"><Info aria-hidden="true" />
          {tf("截止 {0}，候选 {1} 个；位置：{2}。{3}", [formatTime(archivePreview.cutoffAtMs), archivePreview.candidateCount, archivePreview.destination, t(archivePreview.capability.message)])}</p> : null}
        {archiveMaintenance ? <p className="session-status-note" data-tone={archiveMaintenance.deferred || archiveMaintenance.failedCount ? "warn" : undefined}>
          {archiveMaintenance.deferred || archiveMaintenance.failedCount ? <TriangleAlert /> : archiveMaintenance.due ? <CheckCircle2 /> : <Info />}
          {archiveMaintenance.due && !archiveMaintenance.deferred
            ? tf("候选 {0}，已归档 {1}，跳过 {2}，失败 {3}。", [archiveMaintenance.candidateCount, archiveMaintenance.archivedCount, archiveMaintenance.skippedCount, archiveMaintenance.failedCount])
            : t(archiveMaintenance.message)}</p> : null}
      </section>
      <section className="session-status-panel">
        <div className="session-status-head">
          <div><strong>{t("供应商兼容性")}</strong><small>{t("只检查活动会话")}</small></div>
          <span className="session-status-tag">{providerCompatibility?.currentProvider || t("未检查")}</span>
        </div>
        <div className="session-status-summary">
          <span>{t("活动会话")} <strong>{providerCompatibility?.activeCount ?? 0}</strong></span>
          <span>{t("需要适配")} <strong>{providerCompatibility?.mismatchCount ?? 0}</strong></span>
        </div>
        <div className="session-policy-controls">
          <Button disabled={!actions.coreAvailable || providerCompatibilityLoading} onClick={() => void actions.refreshProviderCompatibility()} size="sm" variant="outline">
            <RefreshCw />{providerCompatibilityLoading ? t("检查中…") : t("重新检查")}
          </Button>
          <Button disabled={!actions.coreAvailable || !providerCompatibility?.adaptationAvailable || !providerCompatibility.mismatchCount}
            onClick={() => void actions.adaptActiveSessions()} size="sm"><RefreshCw />{t("适配到当前 provider")}</Button>
        </div>
        <label className="session-policy-toggle">
          <input checked={lifecycle?.autoAdaptProviderOnSwitch ?? true} disabled={!actions.coreAvailable || !lifecycle}
            onChange={event => void actions.saveSessionLifecycle({ autoAdaptProviderOnSwitch: event.currentTarget.checked })} type="checkbox" />
          {t("切换供应商后自动适配")}
        </label>
        <p className="session-status-note"><Info aria-hidden="true" />
          {providerCompatibility?.mismatchCount ? t(providerCompatibility.adaptationMessage) : t("仅活动会话，写前自动备份；归档不受影响")}</p>
      </section>
    </div>

    <section className="sessions-list-panel" data-selection-mode={selectionMode}>
      <div className="sessions-list-head">
        <div><strong>{t("本地会话")}</strong><small>{t("按更新时间倒序显示")}</small></div>
        <div className="session-list-actions">
          {archiveView && archivedCount ? <Button className="session-delete-button" disabled={cleanupRunning || !actions.coreAvailable}
            onClick={() => void actions.deleteLocalSessions("archived")} size="sm" variant="outline"><Trash2 />{t("清空全部归档")}</Button> : null}
          {selectionMode ? <>
            <span className="session-selection-summary">{tf("已选择 {0} / {1} 个会话", [selectedCount, items.length])}</span>
            <Button disabled={allSelected || cleanupRunning} onClick={() => setSelectedSessionIds(new Set(items.map(session => session.id)))} size="sm" variant="outline">{t("全选当前列表")}</Button>
            <Button disabled={!selectedCount || cleanupRunning} onClick={clearSelectedSessions} size="sm" variant="outline">{t("清空选择")}</Button>
            <Button className="session-delete-button" disabled={!selectedCount || cleanupRunning || !actions.coreAvailable} onClick={() => void actions.deleteLocalSessions(selectedSessions)} size="sm" variant="outline"><Trash2 />{cleanupRunning ? t("正在删除…") : t("永久删除已选")}</Button>
            <Button disabled={cleanupRunning} onClick={() => { setSelectionMode(false); clearSelectedSessions(); }} size="sm" variant="ghost">{t("取消")}</Button>
          </> : items.length ? <Button disabled={cleanupRunning} onClick={() => setSelectionMode(true)} size="sm" variant="outline">{t("多选")}</Button> : null}
        </div>
      </div>
      {sessions && !isSuccessStatus(sessions.status) ? <p className="session-status-note" role="alert">{sessions.message}</p> : null}
      {items.length ? <>
        <div className="session-table-head" aria-hidden="true"><span>{t("会话标题与标识")}</span><span>{t("绑定 provider")}</span><span>{t("最后更新")}</span><span>{t("操作")}</span></div>
        <div className="session-list">
          {items.map(session => <div className="session-row" data-selection-mode={selectionMode} data-selected={selectedSessionIds.has(session.id)} key={session.id}>
            {selectionMode ? <label className="session-select"><input aria-label={tf("选择会话 {0}", [session.title || session.id])} disabled={cleanupRunning}
              checked={selectedSessionIds.has(session.id)} onChange={event => toggleSessionSelection(session.id, event.currentTarget.checked)} type="checkbox" /></label> : null}
            <div className="session-main"><strong>{session.title || t("未命名会话")}</strong><span>{session.id}</span><small>{session.cwd || t("未记录项目路径")}</small></div>
            <div className="session-provider">{session.modelProvider || t("provider 未记录")}</div>
            <time className="session-updated">{formatTime(session.updatedAtMs ?? 0)}</time>
            <div className="session-row-actions">
              <Button disabled={!actions.coreAvailable || cleanupRunning} onClick={() => void actions.archiveOrRestoreSession(session, !archiveView)} size="sm" variant="outline">
                {archiveView ? <ArchiveRestore /> : <Archive />}{archiveView ? t("恢复") : t("归档")}
              </Button>
              <Button className="session-delete-button" disabled={!actions.coreAvailable || cleanupRunning} onClick={() => void actions.deleteLocalSession(session)} size="sm" variant="outline"><Trash2 />{t("永久删除")}</Button>
            </div>
          </div>)}
        </div>
        {sessions?.nextCursor ? <div className="session-pagination"><Button disabled={!actions.coreAvailable || cleanupRunning} onClick={() => void actions.refreshLocalSessions(true, archiveView, sessions.nextCursor ?? undefined)} size="sm" variant="outline">
          {tf("显示更多（已显示 {0} 个）", [items.length])}</Button></div> : null}
      </> : <div className="sessions-empty"><Info aria-hidden="true" /><strong>{archiveView ? t("没有已归档会话。") : t("没有活动会话。")}</strong>
        <span>{archiveView ? t("归档会话可随时恢复；归档不会释放磁盘空间。") : t("本地会话会在 Codex 创建任务后显示在这里。")}</span>
      </div>}
    </section>
  </div>;
}
