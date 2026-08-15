# Proposal: clarify-session-screen-actions

## Why

The session screen accumulated buttons faster than hierarchy. The archive card offered three same-weight actions (预览 / 保存策略 / 立即检查) where only one is an action the user actually means to take — the other two exist because a number input didn't save itself and a read-only projection had to be requested by hand (user: 「这三个按钮还是有点点困惑，分别干嘛 有必要么」). The bulk-selection toolbar showed 全选/清空/多选 at full strength even when nothing was selectable-mode, and the card carried a 刷新 button duplicating the top-bar refresh. Worse than clutter, two states lied: a deferred maintenance run ("Codex is running, archiving postponed") rendered as a green check with `候选 232，已归档 0`, and a manual 立即检查 inside the daily interval silently reported `候选 0` because the automatic pass's schedule also gated the button. Separately, the update banner's 更新并重启 gave no feedback until the first download event arrived over the network, so users clicked repeatedly, each click starting another download (user: 「点击按钮多次后才成功触发，没有明显点击效果」).

## What Changes

- Archive card: the retention-days input saves itself (debounced, silent on success) like the toggle already does, and the archive preview refreshes automatically — on entering the screen and after a days change. 预览 and 保存策略 disappear; 立即检查 remains as the card's single primary action.
- A manual 立即检查 always runs the check (`force` flag on the command); only the automatic startup pass keeps the daily interval.
- The maintenance result line tells the truth: a deferred or failed run shows a warning icon and the deferral reason, a not-due automatic pass shows its message instead of zero counts, and only a run that actually looked at sessions shows the counts line with a check.
- Compatibility card: 适配到当前 provider becomes the filled primary action; 重新检查 stays secondary.
- Local sessions card: the redundant 刷新 button is gone (top-bar refresh and the tabs already reload); bulk selection is progressive — one 多选 button normally, and 全选/清空/删除已选/取消 only once selection mode is on, including the previously missing way out (取消).
- Update banner: a click on 更新并重启 flips the banner to the downloading state immediately and re-entry is guarded, so the action acknowledges instantly and cannot start concurrent downloads.

## Capabilities

### New Capabilities

- `session-screen-actions`: one primary action per card, self-saving policy inputs, honest maintenance reporting, guarded update install.

### Modified Capabilities

None.

## Impact

- `src-tauri/src/commands.rs`: `run_session_archive_maintenance` gains `force`; due-check extracted and tested.
- `src/App.tsx`: archive/compatibility/local-sessions card toolbars, days auto-save effect, banner install guard; `src/i18n-en.ts`, `src/styles.css`.
- No data-path changes; archiving, adaptation, and updater flows are unchanged beyond when they are invoked and how they report.
