# Responses-only 供应商设计

## 状态

2026-08-17 经用户确认。该变更是有意的破坏性升级：Codex Minus 只支持 Responses 供应商，不兼容既有 Chat Completions 或本地聚合配置。

## 背景

Codex Minus 已删除上游 launcher，不再提供 `127.0.0.1:57321` 本地代理。Chat Completions 到 Responses 的协议转换，以及客户端本地成员轮转，都依赖该代理，因而无法在当前产品中工作。

新建普通供应商已经固定使用 `protocol = "responses"`，生成的 Codex provider 配置也固定使用 `wire_api = "responses"`。但是应用仍保留「添加聚合供应商」、本地聚合编辑器、Chat Completions 兼容转换、代理地址处理、特殊目录分支和警告文案。这些不可用路径增加了产品选择、状态空间和提交逻辑，却不能产生可运行的 Codex 配置。

## 决策

Codex Minus 的供应商模型收敛为一条路径：一个普通供应商配置对应一个 Responses Base URL、一个 Key 和一个模型目录。

服务端可以在该 Base URL 后自行完成多账号、多厂商、故障切换或负载均衡。对 Codex Minus 而言，这仍是一个普通 Responses 供应商，不新增客户端聚合概念。

这是破坏性格式升级，不提供遗留配置兼容层：

- 不读取、迁移、修复或升级 Chat Completions 配置；
- 不读取、迁移、修复或升级本地聚合配置；
- 不保留单向转换按钮、隐藏入口或只读详情页；
- 不静默改写旧配置；
- 旧 `settings.json` 若包含已删除的结构或取值，不保证能够加载；
- 不为旧配置增加启动时过滤、回退、兼容反序列化或一次性迁移代码。

## 用户体验

### 供应商列表

列表只保留「添加供应商」。删除「添加聚合供应商」和所有与本地聚合相关的说明、状态、徽标及操作。

每个供应商的协议摘要固定显示 `Responses API`，不再根据 profile 数据选择 Chat Completions 文案。若固定摘要没有提供额外信息，可以在实施时进一步从列表中删掉协议摘要，但不得引入新的协议选择。

### 新建与编辑

新建供应商继续直接创建 Responses 草稿：

- `relayMode = "official"`；
- `officialMixApiKey = true`；
- `protocol = "responses"`；
- provider TOML 使用 `wire_api = "responses"`。

编辑页不显示协议选择、Chat Completions 警告、本地代理说明或聚合编辑器。Base URL 始终是实际 Responses 上游地址，不再区分代理地址和上游地址。

### 服务端复合供应商

服务端复合供应商使用普通供应商表单接入。产品文案只说明「填写服务对外暴露的 Responses Base URL 和 Key」，不提供客户端成员、权重或轮转策略。

## 数据模型与持久化

### 前端

删除本仓库拥有的以下状态：

- `RelayMode` 中本地 `aggregate` 产品分支；
- `RelayProtocol` 中 `chatCompletions` 产品分支；
- `RelayAggregateConfig`、成员、权重和策略的前端表示；
- `aggregateRelayProfiles`；
- `activeAggregateRelayId`；
- `upstreamBaseUrl` 中仅为本地协议代理服务的双地址语义；
- Chat Completions／聚合专用校验、标签、帮助、草稿转换和提交路由。

普通 Responses 主流程仍可保留上游依赖要求的类型字段，但业务代码必须把协议作为常量或已验证不变量，而不是可选产品状态。

### Rust 后端

删除本仓库拥有的聚合草稿、聚合拓扑投影、成员校验、active aggregate 联动、Chat Completions 特殊提交，以及两类配置的模型目录旁路。

供应商提交必须验证：

- profile 是普通供应商，而不是本地 aggregate；
- protocol 是 Responses；
- Codex provider TOML 的 `wire_api` 是 `responses`；
- Base URL 不是已删除的本地代理地址。

违反不变量的输入直接报错，不转换、不降级、不兼容处理。

### `settings.json`

持久化格式直接删除本地聚合元数据及其 active 指针。保存后的供应商集合只能包含 Responses profile。

本次变更不引入 schema 版本迁移器。旧文件能否被底层依赖部分反序列化不构成兼容承诺；一旦命中本项目的新验证边界，应明确失败，不继续运行遗留路径。

若上游 `codex-plus-core` 的公共结构仍强制携带已废弃字段，优先向上游提交删除并升级 git revision。本仓库不得复制、vendor 或 fork 上游 provider 逻辑来伪造新结构。升级上游 revision 前后都必须保持 Context 保护罩和 OAuth 所有权约束。

## 配置生成与提交

配置生成只保留 Responses：

1. 用户填写名称、Base URL、Key 和模型。
2. 前端构造或编辑普通供应商草稿。
3. 后端验证 Responses-only 不变量。
4. 目录规划仅处理普通 Responses 供应商。
5. Context 事务保护下写入 settings、live `config.toml` 和模型目录。

`PROTOCOL_PROXY_BASE_URL`、`codex_plus_chat_base_url`、代理地址替换和 `exitChatCompletions` 转换动作全部删除。

## 错误处理

- 收到 Chat Completions profile：返回明确的「只支持 Responses」错误。
- 收到 aggregate profile 或聚合拓扑：返回明确的「不支持本地聚合」错误。
- 收到本地代理 Base URL：返回明确的「本地代理已删除」错误。
- 错误不得触发旧配置修复、自动转换或部分提交。
- 失败沿用现有事务边界，不能改变 live `config.toml`、模型目录、settings 或 Context 表。

## 删除范围

实施应删除或收口以下代码面：

- 供应商列表中的聚合创建入口；
- 聚合编辑器、策略和成员 UI；
- Chat Completions 与聚合相关翻译、样式和说明；
- 前端 relay settings 中的聚合创建、规范化、校验、候选和同步逻辑；
- 前端 provider transform 中的代理 URL、双 Base URL 和 Chat Completions 退出动作；
- native-capability 视图中的 Chat Completions／aggregate 展示分支；
- Rust provider commit、native capability、model catalog 和 transaction tests 中的遗留产品分支；
- README、AGENTS.md 和现行规格中要求保留不可用入口的陈述。

上游类型中无法由本仓库删除、且对 Responses 主流程无运行时影响的符号，不作为 UI 或业务兼容能力保留；应记录为待上游 revision 删除的结构债务。

## 保持不变的约束

- `config.toml` 写路径继续使用 process-wide coordinator 和 Context 事务保护罩；
- live `auth.json` 继续由官方 Codex／ChatGPT 客户端独占；
- provider profile 不保存、应用或恢复 `authContents`；
- managed catalog 的四种目录所有权模式保持不变；
- 外部目录未经明确采用不得改写；
- 原生能力契约仍使用 provider 名称 `OpenAI`、Responses、provider bearer 和一个 actor header；
- 普通保存不得改写非当前 profile 的原生能力契约；
- 服务端复合供应商仍可作为普通 Responses 上游使用。

## 测试

### 前端

- 新建 profile 固定为 Responses；
- 页面不存在「添加聚合供应商」和聚合编辑器；
- 页面不存在 Chat Completions 选择、警告或转换动作；
- 配置编辑始终把 Base URL 视为真实 Responses 地址；
- settings 投影不再包含聚合元数据；
- 普通 Responses 保存、复制、删除、排序和切换继续工作；
- 服务端复合供应商能作为普通 Responses profile 保存和切换。

### Rust

- Responses profile 可以规划、保存和切换；
- Chat Completions profile 在提交前被拒绝；
- aggregate profile／拓扑在提交前被拒绝；
- 本地代理 Base URL 在提交前被拒绝；
- 拒绝路径不产生任何 settings、live config、目录或 Context 写入；
- Context、OAuth、owner-only 权限和事务恢复测试继续通过；
- 模型目录不再存在 Chat Completions／aggregate 特殊旁路。

### 静态与文档

- 产品源码不再出现 `127.0.0.1:57321`、`chatCompletions`、`exitChatCompletions` 或本地 aggregate 产品逻辑；
- 若上游公共类型迫使边界适配器引用旧枚举，引用必须集中在拒绝输入的边界，且不得进入 UI、持久化生成或成功提交路径；
- README 和 AGENTS.md 只描述 Responses-only 支持范围。

## 验收标准

1. 用户只能创建、编辑、保存和切换 Responses 供应商。
2. UI 不再出现 Chat Completions、本地协议代理或本地聚合入口。
3. 服务端复合供应商通过一个普通 Responses Base URL 和 Key 接入。
4. 本仓库不包含旧配置迁移、回退、过滤或单向升级流程。
5. 旧 `settings.json` 不在支持范围内；遗留输入在新提交边界明确失败。
6. Responses 主流程的目录、Provider Doctor、切换和事务保护全部通过回归验证。
7. live `auth.json` 保持字节不变，Context 表在成功和失败路径都受保护。

## 非目标

- 不实现本地协议代理；
- 不实现客户端供应商聚合；
- 不提供旧配置迁移工具；
- 不自动删除或修复用户旧文件；
- 不改变服务端复合供应商的服务端实现；
- 不放宽 OAuth、Context、目录所有权或文件权限约束。
