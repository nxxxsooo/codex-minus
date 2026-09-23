<p align="right"><a href="README.en.md">English</a></p>

<p align="center">
  <img src="docs/assets/codex-minus-hero.webp" alt="Codex Minus 供应商配置界面" width="960">
</p>

<h1 align="center">Codex Minus</h1>

<p align="center">安全切换供应商，管理模型目录，不交出你的 OAuth 与 Context。</p>

<p align="center">
  <a href="https://github.com/nxxxsooo/codex-minus/releases/latest"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/nxxxsooo/codex-minus?style=flat-square&color=197547"></a>
  <img alt="macOS arm64" src="https://img.shields.io/badge/macOS-arm64-202720?style=flat-square&logo=apple&logoColor=white">
  <img alt="Windows x86_64" src="https://img.shields.io/badge/Windows-x86__64-0078D4?style=flat-square&logo=windows&logoColor=white">
  <a href="LICENSE"><img alt="AGPL-3.0-only" src="https://img.shields.io/badge/license-AGPL--3.0--only-197547?style=flat-square"></a>
</p>

Codex Minus 是 [Codex++ Manager](https://github.com/BigPizzaV3/CodexPlusPlus) 的精简 fork，只保留供应商切换、模型目录、本地会话生命周期和配置诊断，并内置签名校验的应用内更新。没有渲染注入、launcher 或市场。

## 下载

支持 Apple Silicon（`arm64`）和 Windows（`x86_64`）。

| 平台 | 架构 | 格式 | 最新版本 |
|------|------|------|----------|
| macOS | arm64 | .app.zip | [最新版](https://github.com/nxxxsooo/codex-minus/releases/latest) |
| Windows | x64 / arm64 | -setup.exe (NSIS) | [最新版](https://github.com/nxxxsooo/codex-minus/releases/latest) |

- [前往 Release 页面下载](https://github.com/nxxxsooo/codex-minus/releases)
- [查看项目页面](https://mjshao.fun/codex-minus/)

```bash
# 校验 macOS
shasum -a 256 -c SHA256SUMS
```

校验后解压，**用 Finder 将 `Codex Minus.app` 拖入 `/Applications` 再启动**——不要在解压目录里直接运行，否则 macOS 隔离会让应用跑在只读位置，应用内更新会报 `Read-only file system (os error 30)`（也可执行 `xattr -dr com.apple.quarantine "/Applications/Codex Minus.app"` 后重试）。当前版本采用 ad-hoc 签名，尚未使用 Developer ID 签名或 Apple 公证。首次启动如被 macOS 拦截，请在「系统设置 → 隐私与安全性」中选择「仍要打开」。

## 为什么需要它

供应商切换只应该改供应商配置。Codex Minus 会在每条写入路径执行前快照 `~/.codex/config.toml` 中的三张 Context 表，并在上游写入结束后把原始 TOML 内容逐字回植：

```toml
[mcp_servers]
[skills]
[plugins]
```

这层保护来自一次真实事故：旧的 managed context 副本在供应商切换时覆盖了有效 MCP 配置。Codex Minus 删除了该管理功能，并用 Rust 测试固定保护契约。

## 功能范围

### 供应商切换

- ChatGPT OAuth 始终由官方 Codex/ChatGPT 客户端管理；供应商 profile 不保存、回填或应用 `authContents`。
- 纯 API 与混合供应商的 API Key 只写入 owner-only settings 和 `config.toml` 的 provider bearer 配置；供应商操作绝不写 live `auth.json`。
- settings、provider config、模型目录与 live 指针通过一个可恢复事务提交，失败时恢复完整上一代。
- 切换后读取实际 `model_provider`，相同 provider 不触发会话扫描。
- 检查可能覆盖供应商配置的 `OPENAI_*` 环境变量。
- 供应商快速测试与 Provider Doctor 遇到严格匹配的 Responses HTTP 400 字段兼容错误时，会在 Manager 内省略可选的 `max_output_tokens` 重试一次并明确标记；认证、模型、限流和普通上游错误不重试。

### 原生能力优先

- 混合供应商（官方登录 + 自定义 Base URL 与 Key）可采用一份固定契约：provider 名称为 `OpenAI`、`wire_api = "responses"`、provider bearer 使用你的 Key，并带一个 Actor 标记请求头。`requires_openai_auth` 默认保持 `true`，让 ChatGPT 登录态与插件能力继续可用；`true`/`false` 都是合法值，保存不会改写你已有的选择（退出到纯 API 模式时才写 `false`）。
- Actor 标记只表示「本客户端有资格以本地扩展身份发起请求」，不是订阅升级，也不代表任何具体能力被授予。是否放行由上游决定：文本 Responses、模型发现、图像生成、图像编辑、远端压缩、联网搜索各自独立，任一项的成功或拒绝都不能推断其他项。未实测的能力一律显示为「未知」，不会被写成成功。
- 升级为原生能力优先是显式动作，带预览与确认，只改这一个 profile。启动、读取和检视都不会自动改写任何已有 profile 的契约；保存某个供应商也不会顺带迁移其他供应商。
- 退出到纯 OAuth 是破坏性动作：预览会列出将被删除的 provider 表与字段，确认后该 provider 及其 Key 会从 profile、settings 与 live `config.toml` 中一并删除，不保留休眠副本。
- profile 配置只拥有 provider 相关的键。写入 live 配置时，profile 内的全局键不会进入 live 根，live 中既有的 `mcp_servers`、`skills`、`plugins` 等全局内容也不会被覆盖。
- 活动供应商的契约或静态目录发生变化后需要重启 Codex：请退出并重开 Codex 宿主，然后新建任务；已经在运行的会话仍使用旧配置，提示不会自动消失。
- 遗留 provider 标识（`CodexPlusPlus`、`CodexPP`）和保留标识（如 `openai`）无法承载该契约——固定的上游内核会把它们改写成自己的 `custom` 形态并丢弃 Actor 标记，因此必须先显式改名再升级。

### 模型目录

- 当前维护预设（2026-09-23）：`gpt-6-astra`、`gpt-6-sol`、`gpt-5.6-terra`、`gpt-6-luna`；新建供应商使用该列表，已有供应商通过「还原 Pro 列表」显式更新。用户自行修改的显示名称和上下文保留，旧型号不会在打开配置时被自动替换。
- GPT-6 Sol／Luna 已于 2026-09-22 发布，经 [models.dev](https://models.dev) 与 [OpenAI Codex 模型文档](https://developers.openai.com/codex/models)核对。OpenAI 签名 CLI 0.156.0 的 bundled 清单尚未包含这两项，因此以注明来源的自定义模型卡补充；已有官方条目的名称仍保留 CLI 原值。
- 「解锁 1,050,000 上下文」修改列表中的 Astra、Sol、Terra、Luna（含保留的 5.6 版本），通过统一保存写入模型目录；关闭恢复默认工作窗口，实际容量取决于上游。其他候选模型收在「添加模型」中。
- 「图像工具（无需登录）」开启时通过纯 API 草稿转换写入 `requires_openai_auth = false` 和 `[features] image_generation = true`，保留模型目录。关闭只写入 `image_generation = false`；保存后按提示重启 Codex。新建时选择「纯 API＋开启生图」会直接生成开启配置，保存后可独立开关图工具。上游需支持图像生成。
- Codex 在未配置静态 `model_catalog_json` 时，OAuth 或 API provider 都可能通过各自的 `/models` 路径更新共享 `models_cache.json`；混合模式会走当前 custom provider，因此该 live cache 具有 provider 歧义，不能作为官方基线。
- 官方清单随应用发布，来源与人工维护差异记录在内置目录资产中。运行时不使用 OAuth 刷新目录，也不把供应商 `/v1/models` 当作官方来源。
- 每个可用供应商可选择「官方原生」「官方 + 自定义」「仅自定义」或「外部目录」。服务端复合供应商仍以一个纯 API Responses Base URL 和 Key 接入，模型聚合由上游完成，默认使用「官方 + 自定义」。
- 官方条目保留目标 CLI 返回的全部字段与隐藏模型；overlay 可管理显示名、可见性、顺序、上下文与有效百分比、推理级别以及显式工具能力。自定义模型默认不声明官方后端专属能力。
- 托管多模型目录以每个模型的上下文元数据为准；已有 `model_context_window` 和 `model_auto_compact_token_limit` 会先显示冲突，只有确认后才在可恢复事务中移除。
- 外部文件保持只读，采用前执行结构与目标 CLI 离线验证。目录声明版本与目标版本不同会显示警告并要求单独确认，但不会仅因版本字符串不同而拒绝兼容目录。
- 外部目录优先于托管模式：只要 profile 的配置指向一个非本工具生成的目录文件，该 profile 就按「外部」处理，托管目录动作与原生能力优先判定都不适用，直到你显式改用内置目录。
- 供应商 `/v1/models` 仅作为有时间戳的「已报告／未报告」证据和自定义候选；遗漏不会隐藏官方模型。
- 生成目录中每个模型的 `priority` 按最终列表顺序重编，Codex 模型选择器的顺序与管理器列表完全一致，自定义模型不会穿插在官方模型之间。未声明推理级别的自定义模型生成 `supported_reasoning_levels: []`，Codex 不会展示 Effort 菜单或发送 `effort` 参数。
- 托管目录写入 `~/.codex/model-catalogs/codex-minus-<profile>-<hash>.json`。活动静态目录变化后会提示重启 Codex；「需重启 Codex」提示和左下角的「重启 Codex」按钮可以一键优雅重启（先请求退出，绝不强制结束；仅 macOS），除此之外不会自动结束或重启官方客户端。

### 会话生命周期

- 分页查看活动与已归档会话。
- 通过目标 Codex CLI 执行原生 `archive` 与 `unarchive`。
- 自动归档默认保留最近 30 天，首次启用前必须确认候选预览。
- 自动检查在界面可用后异步执行，最多每 24 小时完成一次。
- 单条或多选会话可永久删除；「清空全部归档」跨所有分页重新检查归档状态后清理本地数据库记录与 rollout 文件，**不创建备份，无法恢复**。归档与永久删除是不同操作，归档可恢复且不释放磁盘空间。
- 「适配到当前 provider」只改活动会话：逐会话重写 rollout 头部与逐 id 更新 sqlite，归档历史按构造不可达；先备份、锁定文件整体跳过，切换供应商时可选自动执行（默认开）。

### Context 保护

- 供应商切换、应用、清除、活动保存和目录指针写入都经过统一 coordinator 与失败即关闭的 Context 事务。
- 写前快照、TOML 解析、回植、写后校验或恢复任一失败时，命令整体失败，不会报告伪成功。
- 不保存或合并 managed context 副本。
- 不恢复上游「工具与插件」管理页面。

## 更新与卸载

应用启动时会自动检查 GitHub Release 上的新版本：有新版会在侧边栏底部出现横幅，点击「安装更新」后才下载和安装。Electron 更新清单与安装包都使用原有 minisign 公钥验证；macOS 经外部 helper 原子替换并重启管理器，替换失败会回滚；Windows 交给已验签的 NSIS 安装器。更新管理器不会重启官方 Codex 客户端。

v0.5.0 从 Tauri 迁移到 Electron。旧版通过 `latest.json` 自动取得签名的升级包，安装完成后使用 `electron-latest.json` 独立通道继续更新。Windows 旧版 NSIS 的自动升级沿用原安装路径，并在成功后替换卸载记录。更早的 `.msi` 安装仍需先卸载再安装 `-setup.exe`，避免重复条目。

左下角和偏好设置显示当前版本，可随时手动「检查更新」。启动检查失败（如离线）保持静默，手动检查会明确反馈；开发／隔离预览实例不启用安装更新。

用户设置位于 `~/.codex-session-delete/`，覆盖应用不会删除。卸载应用时可单独决定是否保留该目录。

## 已知限制

- 当前没有 Intel 构建、Developer ID 签名或 Apple 公证。
- Windows 构建通过 CI 自动生成，未在本地进行 Windows 实机测试。
- Credential-bearing 官方目录刷新当前验证下限为内嵌 `codex-cli 0.147.0-alpha.1`；已验证 macOS OpenAI Team ID `2DC432GLL2`。不支持 keyring-only 或无法安全读取的认证存储。
- Windows 已实现 Authenticode/OpenAI publisher gate，但尚未完成 Windows 实机 OAuth 刷新验证。
- Codex Minus 仅支持 Responses 供应商。服务端复合供应商应对 Codex 暴露一个 Responses Base URL 和 Key，并作为普通供应商接入。
- 会话归档用于整理，不会压缩数据或释放磁盘空间。

## 架构

- 前端：React 19、Vite、TypeScript。
- 桌面与后端：Electron、受限 preload 桥接、通过私有 stdio 连接的 Rust 核心。
- 上游逻辑：`codex-plus-core` 与 `codex-plus-data`，固定到明确 git revision，不在本仓库 vendoring。
- 应用标识：`fun.mjshao.codex-minus`。
- 状态目录：`~/.codex-session-delete/`。

## 开发

```bash
npm install
npm run verify
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

在目标操作系统上构建 Electron 包（Windows 可用 `node scripts/desktop-package.mjs win32 arm64` 交叉构建 ARM64）。产物包括：

- macOS：`dist-desktop/mac-arm64/Codex Minus.app`、`.app.zip` 和更新用 `.app.tar.gz`。
- Windows：`dist-desktop/CodexMinus_<version>_<arch>-setup.exe`。

`scripts/verify-package.mjs` 检查打包资源、核心架构和本机无窗口启动；`npm run update:smoke:mac` 用一次性应用树验证替换、回滚与管理器重启。旧更新器验收是独立的 `scripts/legacy-updater/` 测试工程，其 Tauri 依赖不进入发布应用。签名和发布只由 tag CI 执行，本地构建不会发布或替换已安装应用。

## License

AGPL-3.0-only，继承上游项目许可。
