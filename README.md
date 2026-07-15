# Codex-- Manager (codex-minus)

极度精简版 [Codex++](https://github.com/BigPizzaV3/CodexPlusPlus) 管理工具。**无渲染注入**，只保留：

- **供应商切换**（API 混 OAuth / 中转 profile，写 `~/.codex/config.toml` + `auth.json`，带切换前 backfill 与失败回滚）
- **本地会话管理**（列出 / 删除 Codex 会话，删除前自动备份）
- **环境变量冲突检测**（OPENAI_* 覆盖供应商配置时提示，位于供应商页）
- **Context 保护罩（本 fork 新增）**：切换/注入供应商时快照 `config.toml` 的
  `mcp_servers` / `skills` / `plugins` 三张表并在写入后原样回植；启动时自动销毁
  settings 里的 managed context 副本。上游「工具与插件」管理功能因为会用残缺副本
  覆盖真实 MCP 配置（2026-07-15 事故根因）而被整体移除。

## 架构

`codex-plus-core` / `codex-plus-data` 以 git 依赖原样引用上游（pin 到 rev），本仓库只有薄壳：
裁剪后的 Tauri 后端（`src-tauri/`，~2100 行）+ 裁剪后的 React 前端（`src/`）。
上游修 config 格式/会话 schema 兼容时，改 Cargo.toml 里的 rev 即可跟进，无 rebase 成本。

已删除：渲染注入、launcher、启动/重启 Codex、自动更新、watcher、广告、脚本市场、
插件市场、provider sync、CC Switch 导入、Zed remote、维护页、工具与插件（context）管理、
中转站环境检测页、Stepwise 配置、启动参数面板、图片覆盖层。

已知限制：「Chat Completions 协议」与「聚合供应商」依赖上游 launcher 起的本地
57321 协议代理，codex-minus 不提供——这两种 profile 切换后 Codex 无法请求，UI 保留
（未改上游供应商逻辑），请勿使用。

## 开发

```bash
npm install
npm run dev      # tauri dev
npm run build    # tauri build（bundle.active=false，产物为裸二进制）
```

## License

AGPL-3.0-only（继承上游）。
