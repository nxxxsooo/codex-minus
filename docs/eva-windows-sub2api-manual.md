# Eva｜Windows 手工接入 Sub2API

## 要达到的结果

- Windows 上继续保留当前 ChatGPT 登录身份；
- Codex 的推理请求只使用分配给你的 Sub2API API Key；
- Sub2API 在服务端账号池中选择 OAuth 或 API Key 上游账号；
- 固定使用 FIT 当前分配的 `gpt-5.6-terra` 模型；
- 自定义 provider 优先按 Codex 原生 OpenAI Responses 能力运行，并启用原生联网。

本地只配置一个 Sub2API 地址和一枚 Sub2API API Key，不配置具体上游账号。使用 OAuth 账号还是 API Key 账号，由 Sub2API 服务端决定。

## 先把 Windows 商店网络打通

如果这台电脑已经能从 Microsoft Store 安装并正常登录 Stable「ChatGPT」，直接跳到下一节。

否则，先按团队装机流程处理网络。这里用的是 **UWP Loopback（回环豁免）**，不是 UDP Loopback。

### 1．安装并打开 Clash Verge Rev

1. 从原 FIT《OpenCode 标准工作环境》上手指南底部附件下载 Windows 版 Clash Verge Rev 安装包；
2. 完成安装并导入团队提供的订阅；
3. 选择「规则」模式，打开「系统代理」；
4. 选择一个可用节点，用浏览器确认可以正常访问外网。

不要从来路不明的网站下载所谓的「Clash 官方版」。团队现有安装包以原 FIT 上手指南底部附件为准。

### 2．给 Microsoft Store 开回环豁免

在 Clash Verge Rev 中打开 `UWP Loopback`：

1. 点击 `Exempt All`；
2. 点击 `Save Changes`。

Windows 的 Store 应用默认不能访问 Clash 在 `127.0.0.1:7897` 提供的本地代理。出现「浏览器可以联网，Microsoft Store 却打不开」时，完成回环豁免再重试。

### 3．从 Microsoft Store 安装 Stable「ChatGPT」

1. 打开 Microsoft Store；
2. 如商店要求，先登录微软账号；
3. 搜索 `ChatGPT`，安装 OpenAI 发布的正式版；
4. 不要安装「ChatGPT (Beta)」。

ChatGPT 安装完成后，回到 Clash Verge Rev 的 `UWP Loopback` 页面，再执行一次：

1. `Exempt All`；
2. `Save Changes`。

新安装的 Store 应用不会自动继承安装前设置的回环豁免，所以这里需要再做一次。

如果 Microsoft Store 仍打不开，依次检查：Clash 是否正在运行、「系统代理」是否打开、当前节点是否可用。商店网络没打通前，不要先修改后面的 Codex 配置。

## 开始前

请先确认：

- Windows 上的 Stable「ChatGPT」桌面应用可以正常打开；
- 你已经在应用内登录 ChatGPT；
- 你已经拿到管理员分配的 Sub2API API Key；
- 不使用「ChatGPT (Beta)」做本次配置和验收。

如果需要关闭 Clash Verge Rev，先在软件里关闭「系统代理」，再退出程序。否则 Windows 可能继续指向已经停止的本地代理，导致浏览器、商店或局域网访问异常。

先完全退出 Stable「ChatGPT」和 Codex Minus，包括仍在后台运行的窗口。然后按 `Win + R`，输入：

```text
notepad %USERPROFILE%\.codex\config.toml
```

按回车打开配置文件。

## 一次合并：按下面内容修改

这次固定使用大小写敏感的自定义 provider ID `OpenAI`。在当前文件中新增或更新以下内容：

```toml
model = "gpt-5.6-terra"
model_provider = "OpenAI"
web_search = "live"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://sub2api.mjshao.fun:4438/"
wire_api = "responses"
requires_openai_auth = false
experimental_bearer_token = "把管理员分配给你的 Sub2API API Key 粘贴到这里"
http_headers = { "x-openai-actor-authorization" = "local-image-extension" }
```

上面是**同一次配置合并**，不是两个方案：

- 前三行放在文件开头、任何 `[……]` 配置段之前；如果已有 `model`、`model_provider` 或 `web_search`，直接把原值分别改成上面的值，不要新增第二行；
- 从 `[model_providers.OpenAI]` 开始的内容，合并进文件里已经存在的同名配置段，不要再复制一份新的配置段；
- 如果原文件只有 `[model_providers.custom]`，把这一行直接改成 `[model_providers.OpenAI]`；只把该段内的 `name` 改为 `"OpenAI"` 不算完成，因为真正的 provider ID 来自方括号里的表名；
- `model_provider = "OpenAI"` 与 `[model_providers.OpenAI]` 的大小写必须完全一致；
- 不要写成全小写 `openai`。全小写是 Codex 官方 OAuth 的内置保留 ID；这里的大写 `OpenAI` 是 Sub2API 使用的自定义 ID。

这是把这些字段合并进现有 `config.toml`，不是用上面的片段替换整个文件。编辑时还要注意：

- 已有 `model = "gpt-5"` 或其他旧模型时，直接改为 `model = "gpt-5.6-terra"`；
- 已有 `name`、`base_url`、`wire_api`、`requires_openai_auth` 或 `experimental_bearer_token` 时，直接修改原值，不要再写第二份；
- 把 `experimental_bearer_token` 的提示文字完整替换为实际分配给你的 Sub2API API Key；
- 不要新增第二个同名 provider 表；
- 不要删除当前 provider 中其他仍在使用的字段；
- 不要修改 `model_catalog_json`、MCP、skills、plugins 或其他无关配置；
- TOML 中同一个表名或字段名不能重复。

如果当前 provider 已有 `http_headers`，把新增请求头合并进去。例如：

```toml
http_headers = { "原有请求头" = "原有值", "x-openai-actor-authorization" = "local-image-extension" }
```

不要同时保留两行 `http_headers = ...`。

如果现有配置使用的是独立子表：

```toml
[model_providers.OpenAI.http_headers]
```

就在这个子表下面新增：

```toml
"x-openai-actor-authorization" = "local-image-extension"
```

这种情况下不要再增加 inline `http_headers = { ... }`。

## 保存并验证

保存 `config.toml`，关闭记事本，然后：

1. 重新打开 Stable「ChatGPT」；
2. 如果出现「完成 Windows 设置」，点击「重试 Windows 设置」，并在随后真正弹出的 Windows「用户账户控制」窗口中点击「是」。页面中间展示的「是／否」窗口只是操作示意图，不是可点击的真实弹窗；
3. Windows 设置成功后，新建一个任务，不要继续使用修改前已经打开的旧任务，也不要修复历史会话；
4. 发送：

```text
只回复：SUB2API_ROUTE_OK
```

能够正常回复，说明基础推理请求已经通过 Sub2API。

本次只在新任务中验收，无需修改或修复任何历史任务。

如果反复显示「Windows 安装未完成」：

1. 先重新检查 `model`、`model_provider` 和 provider 表名是否与本指南完全一致；
2. 完全退出 ChatGPT，包括后台进程，再重新打开并重试；
3. ChatGPT 自动生成的 `notify = [..., "turn-ended"]` 属于 Windows 本地能力配置，保留原样，不要删除或手工照抄他人电脑中的路径；
4. 仍失败时，截取真实 UAC 弹窗或后续错误页面，而不是只重复截当前引导页。

再发送：

```text
请使用当前可用的原生联网能力查询 OpenAI 官方文档首页，并告诉我你实际调用了什么工具。
```

联网能力可能在界面或模型请求中折叠到 `exec`，不一定直接显示为一个独立按钮；能够完成联网查询即可。

本地免费 ChatGPT 身份下，图片生成会受到官方客户端的账户计划限制，不把图片生成是否出现作为本流程的验收项。

## 这几项分别负责什么

| 配置／系统 | 作用 |
|---|---|
| `model = "gpt-5.6-terra"` | 使用 FIT 当前为 Eva 分配的模型；不要继续沿用旧的 `gpt-5` |
| 本地 ChatGPT 登录 | 保持官方客户端的登录身份和账户计划状态；不作为当前自定义 provider 的推理凭据 |
| `experimental_bearer_token` | 作为推理请求访问 Sub2API 的 API Key |
| Sub2API 服务端组／账号池 | 在服务端决定本次请求由 OAuth 账号还是 API Key 账号处理 |
| `name = "OpenAI"` | 让当前 Codex 客户端把自定义 provider 识别为 OpenAI 能力路径 |
| `wire_api = "responses"` | 使用 Codex 原生 Responses 协议 |
| `requires_openai_auth = false` | 推理请求不使用本地 ChatGPT 登录凭据，改用 Sub2API API Key |
| `x-openai-actor-authorization` | 将自定义 provider 标记为 actor-authorized，满足原生扩展的 provider 资格条件 |
| `web_search = "live"` | 启用原生实时联网；当前 code-mode 模型可能把 `web.run` 折叠进 `exec` |

本地 ChatGPT 登录不会代替 Sub2API API Key，两者角色独立：

- ChatGPT 登录保持在官方客户端中，并继续提供账户计划状态；
- Sub2API API Key 负责推理请求鉴权；
- Windows 客户端不选择具体上游账号；Sub2API 服务端负责 OAuth／API 混合路由。

这些配置让客户端优先进入原生能力路径，但具体工具是否出现仍取决于当前 Stable 客户端版本、所选模型、本地账户计划和服务端能力。

## 当前验证边界

这份配置已经在 Windows 11 VM 的 Stable「ChatGPT」配套 runtime 上验证：

- Responses 配置可以严格解析；
- 使用实际分配给 Eva 的 Sub2API API Key，基础文本请求成功返回；
- 服务端记录显示该请求进入 Fit 组并实际命中一个 OpenAI OAuth 上游；Fit 账号池同时包含 OAuth 和 API Key 上游；
- `web_search = "live"` 时，Stable desktop app-server 的真实请求元数据中可以看到原生 `web__run` 注册；
- 尚未执行一次真实联网查询，因此仍需按上一节在新任务中做人工验收；
- 免费身份下没有注册图片生成工具，图片不属于本流程的验收范围。

## 常见错误判断

| 现象 | 优先检查 |
|---|---|
| 配置文件无法加载，或应用启动后立即报配置错误 | `model` 是否为 `gpt-5.6-terra`；是否仍保留 `[model_providers.custom]`；是否重复写了 provider 表、字段或 `http_headers`；`model_provider` 与 provider 表 ID 是否一致；引号是否完整 |
| `401` 或 `INVALID_API_KEY` | Sub2API API Key 未粘贴完整、已失效、已被替换，或仍保留提示文字；这不代表本地 ChatGPT 登录失效 |
| `403` | 先读取错误正文，再请管理员检查 Key、服务端组和模型权限 |
| `404`、`model not found` 或 `503` | 请管理员检查模型映射、服务端路由和当前是否有可用上游账号 |
| `429` | 服务端或上游账号达到速率、并发或额度限制 |
| `502`、其他 `5xx` 或持续超时 | Sub2API 服务、服务端路由或上游账号暂时不可用 |
| 普通文本可以回复，但无法联网 | 先确认使用 Stable、已经完全重启并新建任务；再检查当前模型、客户端版本和服务端能力 |
| 修改后仍走错 provider | 检查顶部 `model_provider` 是否与刚修改的 `[model_providers.<ID>]` 完全一致 |

报错时请把以下两项发给管理员：

1. 完整错误码和错误正文；
2. 已遮住 `experimental_bearer_token` 的 provider 配置片段。

任何截图都必须先完整遮住 `experimental_bearer_token` 的值。若 Key 已经出现在聊天或截图中，请立即通知管理员废止旧 Key 并换新，不要继续使用。
