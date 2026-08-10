# Codex-- Manager 签名自动更新设计

## 状态

2026-08-10 经用户确认。本文独立于供应商新建设计，可单独计划、实现和验证。

## 背景

当前应用没有可工作的更新器。仓库只发布手工安装用的 macOS `.app.zip`、Windows MSI／NSIS 和 `SHA256SUMS`；没有 updater 插件、公钥、更新签名、平台 manifest 或版本一致性门禁。现有 `v0.3.0` 客户端也没有内置更新公钥，因此不能自动引导自身安装首个 updater 版本。

## 目标

1. 应用启动后异步检查 GitHub Release；发现新版时显示版本与更新说明。
2. 用户明确确认后下载、验证、安装并重启；不静默安装。
3. 顶部工具栏保留手动「检查更新」入口。
4. 使用 Tauri 2 官方 updater 的强制签名验证，不实现自制下载替换器。
5. 同时覆盖 macOS arm64、Windows x64 与 Windows arm64。
6. 保留手工下载安装包和 checksum 作为失败回退路径。

## 非目标

- 不支持降级、beta channel、灰度发布或差分更新。
- 不在首版实现后台静默下载或强制更新。
- 不让更新操作修改 Codex 配置、供应商 settings、目录状态或 `auth.json`。
- 不以 updater 签名替代 macOS Developer ID／公证或 Windows Authenticode；代码签名是独立发布改进。
- 不让正在运行的 `v0.3.0` 自动升级到首个 updater 版本；该次升级必须手工安装。

## 用户体验

### 自动检查

主窗口完成首屏加载后，每个进程只启动一次非阻塞检查。检查期间不覆盖当前通知、不阻塞供应商、会话或目录操作。macOS app 与 Windows NSIS 安装进入自动通道；Windows MSI 安装保持手工通道，避免跨安装器升级。

没有更新时保持静默。发现更新时弹出专用更新对话框，展示：

- 当前版本与目标版本
- Release 更新说明
- 「稍后」与「下载并安装」
- GitHub Release 手动下载链接

### 手动检查

顶部工具栏新增更新按钮。状态为：空闲、检查中、可更新、下载中、安装中、仅手工更新、失败。可更新时显示轻量圆点；手动检查没有新版时明确提示「当前已是最新版本」。MSI 安装点击该按钮时说明自动更新仅支持 NSIS，并打开 GitHub Release 手工下载入口，不尝试安装 NSIS 覆盖 MSI。

### 下载与安装

用户确认后显示字节进度。签名验证和安装成功后提示即将重启，并调用 Tauri process relaunch。Windows 安装器需要退出应用时沿用 Tauri updater 的正常退出路径；macOS 完成替换后重启。

当供应商切换、活动 profile 保存、目录刷新或 live transaction 正在运行时，允许检查但禁用安装，等待操作完成后由用户再次确认，避免在 owner-only journal 提交期间结束进程。

安装开始前由 Rust 原子取得带不透明 `reservation_id` 的进程级排他 reservation；取消只能携带并匹配同一 ID，防止旧回调清除后来建立的新 reservation。只要调用方进程仍存活，任何失败或重启失败路径都必须释放自己的 reservation；macOS 已替换但无法自动重启时进入「等待手工重启」，不能永久封锁后续写入。

## 技术架构

### 客户端

加入版本匹配的：

- Rust：`tauri-plugin-updater`、`tauri-plugin-process`
- 前端：`@tauri-apps/plugin-updater`、`@tauri-apps/plugin-process`
- Tauri capabilities：只授予 updater 检查、下载、安装和 process relaunch 所需权限

`tauri.conf.json` 固化 updater 公钥与 HTTPS endpoint：

`https://github.com/nxxxsooo/codex-minus/releases/latest/download/latest.json`

普通本地构建和 pull request 不生成 updater artifact，也不需要私钥。tag release 通过独立 Tauri config overlay 开启 `bundle.createUpdaterArtifacts = true`。

前端把 updater 行为封装为独立 controller，不把下载状态继续塞入已经很大的供应商或会话组件。controller 对 UI 暴露稳定状态与 `check`、`downloadAndInstall`、`relaunch` 操作。

客户端通过一个只读 Tauri command 获取当前安装 channel。debug build 先明确返回 `development`（macOS 的 `bundle_type()` 在 marker 未写入时仍会回退为 `App`，不能单靠它识别未打包运行）；release build 再读取 `BundleType`。`App`／`Nsis` 允许调用 updater；`Msi` 不调用 updater check 或 install，只显示手工渠道；development／其他 unknown 类型保持自动静默，手动操作说明必须使用 packaged build。

controller 在一个 React mount 生命周期内只创建一次，通过 ref 读取最新 busy snapshot；首屏初始化使用 `Promise.allSettled`，无论单项初始化是否失败都只声明一次自动检查。检查和下载分别使用有界 timeout；下载／安装期间拒绝新的检查，不关闭正在使用的 `Update` resource。Release 回退入口复用现有安全外链 command，不让 WebView 自行导航。

### 发布签名密钥

使用 Tauri signer 离线生成长期 updater keypair：

- 公钥提交到 `tauri.conf.json`。
- 私钥和密码存入 Bitwarden 作为长期恢复源。
- 同一私钥和密码以受限 GitHub Actions Secrets 提供给 tag release job。
- 私钥不写入仓库、日志、构建 artifact、缓存或 `.env`。

丢失私钥会使已安装客户端无法验证后续更新，因此发布前必须验证 Bitwarden 恢复记录和 GitHub secret 均可用。

### 平台 artifact

- macOS arm64：Tauri 生成 `.app.tar.gz` 与 `.sig` 供 updater 使用；现有 `.app.zip` 继续供手工安装。
- Windows x64／arm64：Tauri v2 复用 NSIS `.exe` 并生成对应 `.exe.sig` 作为自动更新通道；MSI 与同一 NSIS `.exe` 继续作为手工安装附件。MSI 客户端不允许消费 NSIS updater。
- macOS 只执行一次最终 app 签名并在归档前完成。删除 CI 中会改变已归档 app 内容的重复签名步骤，保留严格签名验证。

Release job 根据实际 artifact 和 `.sig` 生成静态 `latest.json`。manifest 必须包含且只包含当前支持目标：

- `darwin-aarch64`
- `windows-x86_64-nsis`
- `windows-aarch64-nsis`

每项 URL 指向同一 Release 的最终附件名，`signature` 内联 `.sig` 文件正文；不能写签名文件路径。

不得提供通用 `windows-x86_64`／`windows-aarch64` fallback，也不得提供 `*-msi` updater key。Tauri updater 会优先匹配 `{os}-{arch}-{installer}`，再回退到通用键；省略通用 Windows key 是阻止 MSI→NSIS 跨安装器升级的 fail-closed 边界。

### 版本门禁

tag release 开始前验证以下版本完全一致：

- tag `vX.Y.Z`
- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` 中本包版本
- `src-tauri/tauri.conf.json`

任一不一致立即失败，防止客户端重复提示或安装错误版本。

## 网络边界

首版 updater 使用 Tauri 插件自身的 HTTPS 请求，不自动继承正在开发的 Manager 网络策略。界面在检查失败时显示网络失败与 GitHub Release 手动链接，不把失败描述成「没有新版」。

Manager 网络面板继续明确只覆盖其已声明的连接测试和隔离官方目录刷新。将 updater 纳入 Manager 网络策略需要独立设计，不能通过临时修改进程代理环境实现，因为并发异步操作会产生竞态。

## 安全与错误处理

- 签名缺失、错误、manifest 不完整或版本非法时拒绝下载或安装。
- 自动检查失败保持非阻塞；手动检查显示可操作错误和 Release 链接。
- 下载失败可重试，不复用部分 artifact，除非 Tauri 插件明确保证完整性。
- 安装入口只接受本进程最近一次 `check()` 返回、且 `currentVersion`／`version` 仍与当前状态一致的 `Update` handle；`check()` 不启用 `allowDowngrades`，由 Tauri comparator 保证目标版本更高。handle 缺失或版本不一致时重新检查，不取得安装 reservation。
- 更新说明按纯文本显示，不执行 Release Markdown 中的 HTML 或脚本。
- 更新日志只记录版本、平台、阶段、字节进度和错误类别，不记录供应商配置、Key、OAuth、代理凭据或用户内容。
- updater 操作不进入供应商 live-state transaction，也不修改其 journal。
- update reservation 活跃时，后台 session archive maintenance 返回 `not_checked`／`deferred = true`，等待后续周期且不弹失败提示；交互式 archive／restore／delete／adapt 才返回可操作的阻塞结果。所有拒绝都发生在文件、数据库或 CLI mutation 之前。

## 发布与引导

首个 updater-enabled 版本必须在 README 和 Release 中明确：

1. 现有 `v0.3.0` 用户需要手工下载安装一次。
2. Windows 用户应选择 NSIS `.exe` 进入后续自动更新通道；选择 MSI 就保持每次手工更新。
3. 从该版本开始，macOS app 与 Windows NSIS 的后续正式版本可在应用内更新。
4. macOS Gatekeeper／Windows SmartScreen 仍可能因平台代码签名状态给出提示；updater 签名只验证更新来源和完整性。

## 测试

### 单元与前端状态

- 版本比较、检查状态、进度累计、错误映射和按钮禁用逻辑。
- 自动检查每个进程只触发一次，手动检查可重复触发。
- 无更新自动静默、手动明确提示。
- MSI／development／unknown channel 不调用 updater check 或 install；MSI 手动入口可用。
- live operation 期间不能开始安装。

### manifest 与 CI

- 5 个本地版本源与 1 个 tag 的一致性测试。
- `latest.json` schema、macOS generic key、两个 installer-specific NSIS key、HTTPS URL、内联签名和附件存在性测试；明确拒绝通用 Windows 和 MSI updater key。
- 普通 branch／PR 在没有 updater secret 时仍能构建。
- tag build 缺少私钥或密码时 fail closed。

### 安装验证

- macOS arm64：从上一个 updater-enabled 版本升级到测试版本，验证签名、安装、重启、设置与 live auth 不变。
- Windows x64／arm64：使用 NSIS 自动更新，验证架构匹配、退出、安装、重启和卸载注册不损坏。
- Windows x64／arm64：使用 MSI 手工安装后确认 updater 不下载 NSIS，界面提供手工 Release 回退且系统不新增第二套 NSIS 卸载注册。
- 篡改 artifact、错误签名、缺少平台项与同版本 manifest 均被拒绝。
- 更新失败后仍能正常打开供应商和会话页面，并可通过 Release 手工安装。

## 验收标准

- macOS app 与 Windows NSIS 启动自动检查，发现新版时由用户确认后更新并重启；Windows MSI 明确保持手工渠道。
- 顶部存在可重复使用的手动检查入口和下载进度。
- 三个平台 target 使用签名 artifact，任何签名或版本异常均 fail closed。
- PR／本地构建不依赖发布私钥。
- 首次 updater bootstrap、手工回退与平台签名限制在文档中清楚说明。
- 更新前后 `auth.json`、供应商 settings、`config.toml` 的非 updater 内容与权限保持不变。
