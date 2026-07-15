# Codex-- Manager (codex-minus)

极度精简版 [Codex++](https://github.com/BigPizzaV3/CodexPlusPlus) 管理工具。**无渲染注入**，只保留：

- **供应商切换**（API 混 OAuth / 中转 profile，写 `~/.codex/config.toml` + `auth.json`，带切换前 backfill 与失败回滚）
- **本地会话管理**（列出 / 删除 Codex 会话，删除前自动备份）
- **环境体检**（OPENAI 环境变量冲突检测、relay 配置检查）
- **自定义配置段编辑**（管理 config.toml 中的自定义 table，如 `mcp_servers.*`）

## 架构

`codex-plus-core` / `codex-plus-data` 以 git 依赖原样引用上游（pin 到 rev），本仓库只有薄壳：
裁剪后的 Tauri 后端（`src-tauri/`，~2100 行）+ 裁剪后的 React 前端（`src/`）。
上游修 config 格式/会话 schema 兼容时，改 Cargo.toml 里的 rev 即可跟进，无 rebase 成本。

已删除：渲染注入、launcher、启动/重启 Codex、自动更新、watcher、广告、脚本市场、
插件市场、provider sync、CC Switch 导入、Zed remote、维护页。

## 开发

```bash
npm install
npm run dev      # tauri dev
npm run build    # tauri build（bundle.active=false，产物为裸二进制）
```

## License

AGPL-3.0-only（继承上游）。
