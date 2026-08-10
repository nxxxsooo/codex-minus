# 单一路径供应商新建与兼容诊断设计

## 状态

2026-08-10 经用户确认。本文只覆盖供应商新建、首次保存后的状态同步、登录引导与供应商测试兼容性；应用更新机制见同日独立设计。

## 背景

当前新建页同时展示 25 个供应商模板，其中 16 个使用已经失去本地协议代理支持的 Chat Completions 路径。模板会改写接入模式、协议、模型和模型清单，使用户在理解认证边界之前就进入多分支配置。

新建供应商在保存前被标记为草稿，模型目录与「设为当前」被隐藏。首次保存后，前端只刷新 settings，没有刷新模型目录状态，并立即返回列表；用户重新打开时仍可能得到缺少目录摘要的不完整详情页。

供应商快速测试和 Provider Doctor 都调用固定的 `codex-plus-core` 请求函数。该函数为 Responses 请求强制添加 `max_output_tokens`；部分兼容 Responses 的中转拒绝这个可选字段，导致真实 Codex 对话可用而诊断误报 HTTP 400。

## 目标

1. 新建供应商只有一条默认路径：官方 ChatGPT Auth、混入供应商 API Key、Responses API。
2. 删除全部模板、模板搜索、分类和模板选择状态，不迁移或改写既有供应商。
3. 首次保存成功后留在当前详情页，刷新目录状态并立即解锁完整功能。
4. 未登录时引导用户在官方 Codex／ChatGPT 客户端登录免费账号；Manager 不发起、不代理也不保存 OAuth。
5. 供应商诊断在确认属于可选输出限制字段不兼容、或命中已知严格 generic wrapper 时，只重试一次不带该字段的最小 Responses 请求。
6. 保持 Context 保护罩、事务回滚、OAuth 所有权和 owner-only 权限不变。

## 非目标

- 不嵌入 ChatGPT 注册或 OAuth WebView。
- 不把供应商 Key 写入、合并或恢复到 live `auth.json`。
- 不迁移既有 profile 的接入模式、协议、模型目录或 Key。
- 不恢复 Chat Completions 转 Responses 的本地代理。
- 不在 codex-minus 中复制或分叉 `codex-plus-core` 的供应商 HTTP 实现。

## 用户体验

### 新建

点击「添加供应商」后直接进入单一表单。草稿默认值为：

- `relayMode = "official"`
- `officialMixApiKey = true`
- `protocol = "responses"`
- 名称、Base URL、Key 与配置模型为空，等待用户填写

页面不再显示「从预设模板创建」。顶部说明明确表达：「使用官方 Codex／ChatGPT 登录身份，并通过当前供应商 Base URL 与 Key 访问模型。」

若官方认证状态不可用，页面显示非阻塞提示：「请先在官方 Codex／ChatGPT 客户端登录免费账号。」第一版提供「查看官方登录说明」链接，不尝试启动、驱动或嵌入登录流程，不读取浏览器会话，不接收令牌。

### 首次保存

首次保存沿用现有 owner-only settings 写入。成功后：

1. 草稿转换为已保存 profile，但页面不返回列表。
2. 前端用后端返回的规范化 profile 替换草稿。
3. 等待任何保存前已在运行的目录读取结束，再调用一次保存后的 `model_catalog_status`，让后端为新 profile 建立默认 `official-plus-custom` 状态。
4. 详情页切换为已保存模式，显示「设为当前」、完整模型目录、Provider Doctor 和配置预览。
5. 保存失败时保留草稿且不更新 canonical settings；保存成功但目录刷新失败时明确显示「已保存、目录同步失败」，留在当前页重试，不把两种结果混成含糊的半成功提示。

「设为当前」仍是单独、显式的动作；首次保存不会自动切换 live provider。

## 组件与数据边界

### 前端默认构造

把新建 profile 的默认构造抽成可独立测试的纯 helper。`App.tsx` 只负责生成唯一 ID 和调用 helper，避免默认认证语义散落在表单与模板 patch 中。

删除以下模板能力：

- `src/presets.ts`
- `src/components/ProviderPresetSelector.tsx`
- `App.tsx` 中的 import 和挂载点
- 仅供模板选择器使用的样式与翻译
- `AGENTS.md` 中「presets 不是死代码」的旧说明

### 首次保存状态机

详情页明确区分 `draft`、`saving`、`saved` 三态。保存函数只在后端成功时提交 canonical settings；失败时保留本地 draft，不把未落盘 profile 乐观写进父状态。成功后父组件在同一页更新 profile ID，再通过 after-current queue 执行一次保存后的目录读取。刷新完成前显示稳定的加载状态，不把 `summary = null` 解释成「供应商不支持托管目录」。

### 诊断兼容重试

兼容重试属于 `codex-plus-core::relay_config::test_relay_profile`，因为快速测试和 Provider Doctor 必须共享同一请求契约。实现顺序为：

1. Responses 首次请求保持当前最小 payload。
2. 仅当 HTTP 400 且响应命中以下 allowlist 时，再发送一次删除 `max_output_tokens` 的 payload：响应明确包含 `max_output_tokens` 与 unknown／unsupported／invalid parameter 语义；或结构化错误严格等于 `type = "upstream_error"` 且 `message = "Upstream request failed"`。
3. 第二次结果作为最终结果，并在结构化结果中标记 `compatibility_fallback_used = true`、首次 HTTP status；若第二次发生传输错误，保留首次 400 并返回稳定的最终传输失败类别，不能用 HTTP 0 误报成功。
4. 非 400、认证失败、模型不存在、限流、网络错误或其他 upstream error 不重试。
5. Chat Completions 不进入此兼容分支。

该修复先进入 `BigPizzaV3/CodexPlusPlus`，本仓库只把 git dependency revision 升级到包含修复的上游 commit。若上游修复尚不可用，本仓库停止在 revision 升级之前，不复制 HTTP 请求逻辑。

## 认证与密钥安全

- live `auth.json` 继续由官方客户端独占，任何原始 auth 写入保持拒绝。
- 供应商 Key 继续由 owner-only Manager settings 持有，并通过既有 provider bearer 配置路径物化；不出现在目录状态、诊断、日志或备份正文中。
- `config.toml`、`auth.json`、settings、事务 journal 及父目录继续执行 owner-only／平台等价 ACL 检查。
- Provider Doctor 的响应预览必须截断并沿用现有脱敏规则；兼容重试不得记录请求头或 Key。
- `requires_openai_auth = true` 的混合模式不改用 `env_key`，因为官方 Codex 在此模式会忽略 `env_key`；也不把 provider Key 伪装为官方登录缓存。

## 错误处理

- 缺少 Base URL、Key 或配置模型时在保存前给出字段级错误。
- 官方认证缺失不阻止保存供应商，但阻止需要官方身份的实际应用操作，并提供官方登录指引。
- 首次保存成功但目录刷新失败时，profile 保持已保存，页面显示可重试的目录错误，不回滚已成功的 settings 写入。
- 兼容重试仍失败时同时保留首次和最终失败类别，但不重复展示敏感响应。

## 测试

### 前端

- 新建默认值精确为 official＋mixed key＋Responses。
- 新建页不再引用或渲染任何模板。
- 首次保存成功后仍停留在相同 profile 详情，并触发一次目录刷新。
- 首次保存恰逢已有目录刷新时，会在旧刷新结束后再执行一次保存后刷新；保存失败不会刷新或把草稿写入 canonical state。
- 目录刷新期间不显示错误的「不可用」状态。
- 既有 profile 的模式和协议保持不变。

### 上游 Rust

- 支持 `max_output_tokens` 时只发一次请求。
- 明确字段不兼容的 HTTP 400 会删除该字段并重试一次。
- 认证、模型、限流和普通 upstream error 不触发重试。
- fallback 成功返回 HTTP 200 并标记兼容路径。
- fallback transport 失败保留首次 HTTP 400、兼容路径与最终失败类别。
- 请求和诊断输出不包含 Key。

### 本仓库 Rust 与集成

- raw auth save 仍被拒绝。
- profile 保存和切换保持 live auth 字节不变。
- owner-only 权限与 Context 保护罩回归测试通过。
- 使用可控测试服务完成「首次 400、fallback 200」的 Provider Doctor 端到端验证。

## 验收标准

- 新建页没有模板入口，默认即官方 Auth 混 API。
- 首次保存后无需返回、重启或切换页面即可看到完整功能。
- 未登录用户得到官方登录指引，Manager 从不写 live `auth.json`。
- 对已知可选字段不兼容中转，快速测试与 Provider Doctor 都能成功且明确显示使用了兼容重试。
- 既有供应商行为、配置、Key 和目录不被迁移。
