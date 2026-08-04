<p align="right"><a href="README.en.md">English</a></p>

<p align="center">
  <img src="docs/assets/codex-minus-hero.webp" alt="Codex-- Manager 供应商配置界面" width="960">
</p>

<h1 align="center">Codex-- Manager</h1>

<p align="center">安全切换供应商，整理本地会话，不交出你的 Context 配置。</p>

<p align="center">
  <a href="https://github.com/nxxxsooo/codex-minus/releases/latest"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/nxxxsooo/codex-minus?style=flat-square&color=197547"></a>
  <img alt="macOS arm64" src="https://img.shields.io/badge/macOS-arm64-202720?style=flat-square&logo=apple&logoColor=white">
  <img alt="Windows x86_64" src="https://img.shields.io/badge/Windows-x86__64-0078D4?style=flat-square&logo=windows&logoColor=white">
  <a href="LICENSE"><img alt="AGPL-3.0-only" src="https://img.shields.io/badge/license-AGPL--3.0--only-197547?style=flat-square"></a>
</p>

Codex-- Manager 是 [Codex++ Manager](https://github.com/BigPizzaV3/CodexPlusPlus) 的精简 fork，只保留供应商切换、本地会话生命周期和配置诊断。没有渲染注入、launcher、市场或自动更新器。

## 下载

支持 Apple Silicon（`arm64`）和 Windows（`x86_64`）。

| 平台 | 架构 | 格式 | 最新版本 |
|------|------|------|----------|
| macOS | arm64 | .app.zip | [v0.2.0](https://github.com/nxxxsooo/codex-minus/releases/latest) |
| Windows | x86_64 | .msi / .exe | [v0.2.0](https://github.com/nxxxsooo/codex-minus/releases/latest) |

- [前往 Release 页面下载](https://github.com/nxxxsooo/codex-minus/releases)
- [查看项目页面](https://mjshao.fun/codex-minus/)

```bash
# 校验 macOS
shasum -a 256 -c SHA256SUMS
```

校验后解压，将 `Codex-- Manager.app` 移入 `/Applications`。当前版本采用 ad-hoc 签名，尚未使用 Developer ID 签名或 Apple 公证。首次启动如被 macOS 拦截，请在「系统设置 → 隐私与安全性」中选择「仍要打开」。

## 为什么需要它

供应商切换只应该改供应商配置。Codex-- 会在每条写入路径执行前快照 `~/.codex/config.toml` 中的三张 Context 表，并在上游写入结束后把原始 TOML 内容逐字回植：

```toml
[mcp_servers]
[skills]
[plugins]
```

这层保护来自一次真实事故：旧的 managed context 副本在供应商切换时覆盖了有效 MCP 配置。Codex-- 删除了该管理功能，并用 Rust 测试固定保护契约。

## 功能范围

### 供应商切换

- 管理 OAuth、API Key 和混合认证的中转 profile。
- 写入 `config.toml` 与 `auth.json`，失败时回滚。
- 切换后读取实际 `model_provider`，相同 provider 不触发会话扫描。
- 检查可能覆盖供应商配置的 `OPENAI_*` 环境变量。

### 会话生命周期

- 分页查看活动与已归档会话。
- 通过目标 Codex CLI 执行原生 `archive` 与 `unarchive`。
- 自动归档默认保留最近 30 天，首次启用前必须确认候选预览。
- 自动检查在界面可用后异步执行，最多每 24 小时完成一次。
- 删除会话前创建本地备份。

### Context 保护

- 供应商切换、应用和清除路径都经过 `with_context_tables_protected`。
- 不保存或合并 managed context 副本。
- 不恢复上游「工具与插件」管理页面。

## 更新与卸载

更新时先退出 Codex-- Manager，再下载最新版覆盖 `/Applications/Codex-- Manager.app`。应用不包含自动更新器，GitHub Release 不会自动更新本机副本。

用户设置位于 `~/.codex-session-delete/`，覆盖应用不会删除。卸载应用时可单独决定是否保留该目录。

## 已知限制

- 当前没有 Intel 构建、Developer ID 签名或 Apple 公证。
- Windows 构建通过 CI 自动生成，未在本地进行 Windows 实机测试。
- 「Chat Completions 协议」和「聚合供应商」依赖上游 launcher 提供的 `127.0.0.1:57321` 代理，本项目不包含该代理，请勿使用。
- 固定的上游 revision 尚未提供 active-only provider-sync 写入范围，因此「适配到当前 provider」保持禁用，不会回退到全历史改写。
- 会话归档用于整理，不会压缩数据或释放磁盘空间。

## 架构

- 前端：React 19、Vite、TypeScript。
- 桌面与后端：Tauri 2、Rust。
- 上游逻辑：`codex-plus-core` 与 `codex-plus-data`，固定到明确 git revision，不在本仓库 vendoring。
- 应用标识：`fun.mjshao.codex-minus`。
- 状态目录：`~/.codex-session-delete/`。

## 开发

```bash
npm install
npm run check
npm run vite:build
cd src-tauri && cargo test
npm run build
```

完整 Tauri 构建会生成：

- macOS: `src-tauri/target/release/bundle/macos/Codex-- Manager.app`
- Windows: `src-tauri/target/release/bundle/msi/*.msi` 或 `src-tauri/target/release/bundle/nsis/*.exe`

## 版本说明

当前 `master` 的公开页面与 README 在 `v0.1.0` 应用 tag 之后补充。`v0.1.0` macOS 二进制及其 SHA-256 未发生变化。

## License

AGPL-3.0-only，继承上游项目许可。
