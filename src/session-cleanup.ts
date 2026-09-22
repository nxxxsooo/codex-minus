import type { LocalSession, LocalSessionsResult, SessionCleanupResult, Status } from "./backend-types.ts";
import { truncateSessionDeletePreview } from "./app-shell-rules.ts";
import { t, tf } from "./i18n.ts";

type CleanupPorts = {
  invoke: <T>(command: string, args: Record<string, unknown>) => Promise<T>;
  confirm: (title: string, message: string) => Promise<boolean>;
  notice: (title: string, message: string, status: Status, detail?: string) => void;
  refresh: () => Promise<unknown>;
};

// Fetch the complete archive snapshot before asking for confirmation. Never use
// the visible page as "all", and never include sessions archived after this scan.
async function allArchivedSessions(invoke: CleanupPorts["invoke"]): Promise<LocalSession[]> {
  const sessions: LocalSession[] = [];
  const cursors = new Set<string>();
  let cursor: string | null = null;
  do {
    const page: LocalSessionsResult = await invoke("list_local_sessions", {
      request: { archived: true, pageSize: 200, cursor },
    });
    if (page.status !== "ok" || !page.archived || page.sessions.some((session) => !session.archived)) {
      throw new Error(t("归档列表读取不完整，请刷新后重试。"));
    }
    sessions.push(...page.sessions);
    cursor = page.nextCursor;
    if (cursor && cursors.has(cursor)) throw new Error(t("会话列表游标已过期，请刷新当前列表。"));
    if (cursor) cursors.add(cursor);
  } while (cursor);
  return sessions;
}

export async function cleanupSessions(target: LocalSession[] | "archived", ports: CleanupPorts): Promise<void> {
  const archivedOnly = target === "archived";
  const items = archivedOnly ? await allArchivedSessions(ports.invoke) : target;
  const sessions = [...new Map(items.map((session) => [session.id, session])).values()];
  const title = archivedOnly ? t("清空全部归档") : t("永久删除会话");
  if (!sessions.length) {
    ports.notice(title, archivedOnly ? t("没有已归档会话。") : t("请先选择要删除的会话。"), "ok");
    return;
  }
  const preview = sessions.slice(0, 6).map((session) => `- ${truncateSessionDeletePreview(session.title || session.id)}`).join("\n");
  const extra = sessions.length > 6 ? tf("\n...以及另外 {0} 个会话", [sessions.length - 6]) : "";
  const message = archivedOnly
    ? tf("永久删除全部 {0} 个已归档会话？将清理本地数据库记录和 rollout 文件，不留备份，无法恢复。", [sessions.length])
    : tf("永久删除选中的 {0} 个会话？将清理本地数据库记录和 rollout 文件，不留备份，无法恢复。\n\n{1}{2}", [sessions.length, preview, extra]);
  if (!await ports.confirm(title, message)) return;
  try {
    const result = await ports.invoke<SessionCleanupResult>("permanently_delete_local_sessions", {
      request: { sessionIds: sessions.map((session) => session.id), archivedOnly },
    });
    const summary = result.status === "ok"
      ? tf("已永久删除 {0} 个会话，未创建备份。", [result.deletedCount])
      : result.failures.length
        ? tf("已永久删除 {0} 个，未完成 {1} 个。", [result.deletedCount, result.failures.length])
        : t("会话清理失败，请查看详情。");
    const detail = result.failures.length
      ? result.failures.map((failure) => `${failure.sessionId}: ${failure.message}`).join("\n")
      : result.status === "ok" ? undefined : result.message;
    ports.notice(title, summary, result.status, detail);
  } finally {
    await ports.refresh();
  }
}
