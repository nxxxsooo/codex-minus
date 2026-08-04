# Codex-- Manager (codex-minus)

极度精简版 [Codex++](https://github.com/BigPizzaV3/CodexPlusPlus) 管理工具。**无渲染注入**，只保留：

- **供应商切换**（API 混 OAuth / 中转 profile，写 `~/.codex/config.toml` + `auth.json`，带切换前 backfill 与失败回滚）
- **本地会话管理**（活动 / 已归档分页、原生归档与恢复、删除前自动备份）
- **供应商兼容性检查**（切换后读取实际 `model_provider`，只检查活动会话）
- **环境变量冲突检测**（OPENAI_* 覆盖供应商配置时提示，位于供应商页）
- **Context 保护罩（本 fork 新增）**：切换/注入供应商时快照 `config.toml` 的
  `mcp_servers` / `skills` / `plugins` 三张表并在写入后原样回植；启动时自动销毁
  settings 里的 managed context 副本。上游「工具与插件」管理功能因为会用残缺副本
  覆盖真实 MCP 配置（2026-07-15 事故根因）而被整体移除。

## 架构

`codex-plus-core` / `codex-plus-data` 以 git 依赖原样引用上游（pin 到 rev），本仓库只有薄壳：
裁剪后的 Tauri 后端（`src-tauri/`）+ 裁剪后的 React 前端（`src/`）。
上游修 config 格式/会话 schema 兼容时，改 Cargo.toml 里的 rev 即可跟进，无 rebase 成本。

已删除：渲染注入、launcher、启动/重启 Codex、自动更新、watcher、广告、脚本市场、
插件市场、全历史 provider sync 与目标选择器、CC Switch 导入、Zed remote、维护页、工具与插件（context）管理、
中转站环境检测页、Stepwise 配置、启动参数面板、图片覆盖层。

已知限制：「Chat Completions 协议」与「聚合供应商」依赖上游 launcher 起的本地
57321 协议代理，codex-minus 不提供——这两种 profile 切换后 Codex 无法请求，UI 保留
（未改上游供应商逻辑），请勿使用。

## 会话生命周期

- 自动归档默认保留最近 30 天，首次启用前会显示候选数量、截止时间和目标位置并要求确认。
- 归档位置是 Codex 原生的 `$CODEX_HOME/archived_sessions`；归档不压缩、不复制，也不释放磁盘空间。
- 归档与恢复只调用目标 Codex / ChatGPT 应用内置的 `codex archive` / `codex unarchive`，不直接移动 rollout 或修改 SQLite。
- 自动检查在界面可用后异步执行，最多每 24 小时完成一次；若无法确认目标客户端空闲，会安全延后。手动归档同样要求目标客户端已关闭，恢复操作不受此限制。
- 生命周期设置存放在 `~/.codex-session-delete/session-lifecycle.json`，旧的 provider 目标设置仍可读取但不再参与行为决策。

## 供应商与会话

供应商切换成功后，以最终 `config.toml` 的 `model_provider` 为准。身份未变化时不会扫描会话；身份变化时只统计活动会话的不匹配标记，不遍历归档 rollout。

当前固定的上游 revision 尚未提供 active-only provider-sync 写入范围，因此「适配到当前 provider」会保持禁用，并明确显示原因。Codex-- 不会退回全历史扫描，也不会在本仓库复制上游修复逻辑；待上游提供范围参数后再升级固定 revision 开放写入。

当前只承诺缩小 Codex-- 自身的会话载荷和供应商检查范围，不宣称归档一定会加快 ChatGPT / Codex 客户端启动；外部客户端效果需以独立基准为准。

## 安装与更新

当前 GitHub Release 只提供 Apple Silicon（`arm64`）版 macOS 应用：

1. 从 [Releases](https://github.com/nxxxsooo/codex-minus/releases/latest) 下载 `Codex--Manager_<version>_aarch64.app.zip` 和 `SHA256SUMS`。
2. 用 `shasum -a 256 -c SHA256SUMS` 校验下载文件，解压后将 `Codex-- Manager.app` 移入 `/Applications`。
3. 首次启动若 macOS 提示无法验证开发者，请在「系统设置 → 隐私与安全性」中确认「仍要打开」。当前版本采用 ad-hoc 签名，尚未使用 Developer ID 签名或 Apple 公证。

更新现有安装时，退出 Codex-- Manager，下载最新版并覆盖 `/Applications/Codex-- Manager.app`。供应商与会话生命周期设置保存在 `~/.codex-session-delete/`，覆盖应用不会删除这些设置。本项目不包含自动更新器，GitHub Release 也不会自动更新本机应用。

## 开发

```bash
npm install
npm run dev      # tauri dev
npm run build    # tauri build（产物为 macOS .app bundle）
npm run check    # TypeScript 检查
cd src-tauri && cargo test
```

## License

AGPL-3.0-only（继承上游）。
