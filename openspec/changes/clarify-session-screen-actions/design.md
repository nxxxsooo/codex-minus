# Design — clarify-session-screen-actions

## D1: One primary action per card, everything else demoted or automated

The rule applied across the session screen: each card gets at most one filled button — the action the card exists for. 立即检查 (archive card), 适配到当前 provider (compatibility card). Anything that merely persists an input or refreshes a projection is not a button: the days input auto-saves debounced (800ms) and silently on success, mirroring the toggle's existing immediate-save behavior; the preview refreshes when the screen loads (chained after the lifecycle settings fetch so it uses the saved days, not a stale default) and after a days save. 保存策略 and 预览 are removed, not hidden.

## D2: `force` splits the manual check from the automatic schedule

The daily `ARCHIVE_CHECK_INTERVAL_MS` exists so the startup pass doesn't rescan on every launch. Routing the manual button through the same gate made the button lie — a click inside the interval returned "not due" with zero counts and no visible reaction. The command takes `force: Option<bool>`; the button passes true, the startup pass passes nothing. The due-decision is extracted into `archive_maintenance_due` (pure, tested): force bypasses the interval but never the policy — archiving disabled or unreviewed still refuses.

## D3: The result line reports the run's actual disposition

Three dispositions, three renderings: ran → counts with a check; deferred (Codex running / operation mutex held) or failures counted → warning icon + the backend's own message (now in the EN dictionary — these messages previously never rendered anywhere); not due → info icon + message. The green-check-with-zero-counts state that prompted the confusion cannot render anymore: counts only appear when `due && !deferred`.

## D4: Bulk selection is progressive, and exit exists

Normally the list shows a single 多选 button. Entering selection mode reveals 全选当前列表 / 清空选择 / 删除已选 / 取消 — and 取消 is new: previously the only way out of selection mode was switching tabs. The per-card 刷新 button is deleted rather than demoted because two refresh paths already exist (top-bar refresh, tab clicks).

## D5: The update banner acknowledges the click itself

`installAppUpdate` guards re-entry (downloading/installing phases return early) and immediately sets `downloading(received: 0, total: null)` — rendering the indeterminate "正在下载…" text and removing the action button — before `downloadAndInstall` produces its first event. The reducer already treats a later `Started` event as authoritative (it resets received and adopts the real total), so the synthetic phase composes with the real event stream; a test pins that.

## Non-goals

- No changes to what archiving/adaptation write, back up, or skip.
- Per-row 归档/删除 buttons stay as they are — they are the row's actions, not card clutter.
- No updater protocol or signing changes.
