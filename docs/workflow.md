# 怎么干活：分支、PR、发版、并行

写给「大概懂三成」的自己。每一节先说**为什么**，再给**照着敲的命令**。

---

## 0. 一句话地图

```
切分支 → 改代码 → push → 开 PR → CI 跑 → 合并 → 打 tag → tag 自动发版
```

这套叫 **GitHub Flow**。PR（Pull Request）是 GitHub 的叫法，GitLab 叫 MR（Merge Request），**同一个东西**。

---

## 1. 为什么不直接往 master 提交

直接提交能不能跑？能。问题在于：

- **没有回退点。** 一个功能改到一半推上去，master 就是坏的。
- **没有检查关口。** CI 跑不跑无所谓，反正已经进主线了。
- **没法并行。** 两个人（或两个 AI 对话）同时改，后推的直接覆盖前面的。

分支 + PR 解决的是这三件事，不是「显得专业」。

---

## 2. 标准流程（照着敲）

### 2.1 开一条工作流

```bash
cd ~/Documents/sync/GitHub/codex-minus

# 建一个独立工作目录 + 分支，都从最新的 master 开始
git worktree add .worktrees/<名字> -b codex/<名字> origin/master
```

**worktree 是什么**：同一个仓库的第二个工作目录。主目录留着你自己用（可以是脏的），worktree 里干活，互不干扰。这是并行的物理基础。

然后开 Claude 对话时，把工作目录指到 `.worktrees/<名字>`。

### 2.2 干活、提交

```bash
git add -A
git commit -m "feat: 做了什么"
```

提交信息第一行的前缀是约定俗成的：

| 前缀 | 用于 |
|---|---|
| `feat:` | 新功能 |
| `fix:` | 修 bug |
| `refactor:` | 重构，行为不变 |
| `docs:` | 只改文档 |
| `chore:` | 版本号、依赖等杂事 |

### 2.3 推分支、开 PR

```bash
git push -u origin codex/<名字>

gh pr create --base master --head codex/<名字> \
  --title "标题" --body "说明"
```

### 2.4 等 CI

```bash
gh pr checks <PR号> --watch
```

这个仓库有 3 个 job：macOS arm64、Windows x64、Windows arm64。约 8–15 分钟。

**CI 是什么**：GitHub 上的机器人，自动跑编译和测试。它证明的是「在一台干净机器上也能过」，而不是「只在你电脑上能过」。

### 2.5 合并

```bash
gh pr merge <PR号> --merge
```

`--merge` 保留每个 commit。另外两种：`--squash` 压成一条（历史干净但丢过程）、`--rebase` 线性化。**这个仓库统一用 `--merge`。**

### 2.6 发版

版本号写在**四个**文件里，没有任何工具会帮你同步：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`。改三个漏一个，安装包文件名和 App 里的版本号就会对不上。

所以先手写 BOARD.md 那一条（什么 / 为什么 / 怎么验证的 —— 脚本写不出来），然后：

```bash
# 1. 改完 BOARD.md 并提交，工作区必须干净
scripts/release.sh 0.4.5
```

这个脚本会：拒绝脏工作区、拒绝在 master 上跑、拒绝 BOARD.md 没有今天的条目、改四个文件、跑一遍测试、提交。任何一步不满足就停下并说原因。

```bash
# 2. 走 2.3–2.5 的 PR 流程合并

# 3. 合并之后打 tag
scripts/release-tag.sh 0.4.5
```

`release-tag.sh` 会先 fetch，确认 **origin/master 上**的版本号确实是 0.4.5，才打 tag —— 防止在没合并的提交上打 tag。

推 tag 会触发 release job，自动构建 6 个安装包并发布。

```bash
gh release view v0.4.5 --json assets --jq '.assets[].name'
```

**兜底**：`src/release-version.test.ts` 会断言四个文件版本号一致，跟着 `npm test` 在 CI 里跑。就算绕过脚本手工改，不一致也会在 PR 阶段红掉。

### 2.7 收工

```bash
git worktree remove .worktrees/<名字>
```

---

## 3. 并行开多个对话，能同步吗

**能解决一半。这个区分很重要。**

### PR 真正保证的

1. **不会静默丢代码。** 两个 PR 改同一段，第二个合并时必然报冲突，逼你处理。
2. **每个 PR 独立跑 CI。** 至少保证「我这个分支单独看是绿的」。

### PR 保证不了的：语义冲突

```
PR A: 删掉了 foo() 的一个参数
PR B: 新增了一处 foo(x, y) 的调用

改的是不同文件、不同行  →  git 认为没有冲突
两个 PR 单独跑 CI       →  都绿
合并完 master           →  编译不过
```

git 只懂**文本行**，不懂「你删的东西我还在用」。

而且默认情况下 **CI 测的是你的分支，不是合并后的结果**。

### 现在的设置（2026-08-13 已开启）

master 分支保护已生效：

| 项 | 值 | 意思 |
|---|---|---|
| 必需检查 | macOS arm64 / Windows x64 / Windows arm64 | 三个都绿才能合并 |
| `strict` | `true` | **合并前分支必须与 master 同步**，master 动过就得 rebase 重跑 CI |
| `enforce_admins` | `false` | 你本人保留紧急绕过的口子 |
| 强推 / 删分支 | 禁止 | master 不会被覆盖或删掉 |

**Merge Queue 用不了**：它只对**组织仓库**开放，这个仓库是个人所有，API 直接拒绝。`strict: true` 是等效替代 —— 代价是并发度降到 1（第二个 PR 得等第一个合完再 rebase 重跑）。

工作流里已经加好了 `merge_group:` 触发器，哪天仓库转到组织名下，打开队列就能直接用。

---

## 4. 并行的五条规矩

1. **一个工作流 = 一个区域 = 一个分支 = 一个 PR。**
2. **开工前确认没撞车：**
   ```bash
   gh pr list --state open
   gh pr diff <n> --name-only    # 看别的 PR 碰了哪些文件
   ```
3. **rebase 在开 PR 之前做**，不是合并前才补：
   ```bash
   git fetch origin master && git rebase origin/master
   ```
4. **合并被拒说「branch is out of date」不是故障**，是 `strict` 在起作用 —— rebase 再推一次，等 CI 重跑。
5. **动共享基础的先合**，依赖它的后合并再 rebase。

---

## 5. 前端并行仍需按区域切分

打薄之前 `src/App.tsx` 是 5088 行、100 个纯函数，两个对话都碰前端等于保证冲突。现在是 3464 行、33 个（见第 6 节），但 20 个组件还都在里面。

**能并行的切分**（碰不到同一个文件）：

| 可以 | 不可以 |
|---|---|
| 后端 `model_catalog.rs` ／ 前端 `App.tsx` ／ OpenSpec 文档 | 两个都改 `App.tsx` |
| 独立能力域（会话生命周期 ／ 供应商编辑） | 一个改函数签名、一个加调用方 |

---

## 6. App.tsx 打薄的规则

### 规则

> **`src/App.tsx` 是接线文件。任何不用渲染就能测的函数，不该在这里。**

判据是机械的：

| 顶层 `function` | 判定 |
|---|---|
| 首字母大写（`RelayScreen`） | React 组件，留 |
| 首字母小写（`normalizeSettings`） | 纯逻辑，搬去独立模块 |

打薄前是 20 : 100，现在是 20 : 33。

### 怎么让规则真的生效：棘轮测试

写在文档里没用，下一个对话照样往里加。`src/app-shell-budget.test.ts` 是一个**只减不增**的清单：

```ts
const LOGIC_STILL_IN_THE_SHELL = ["statusLabel", "routeTitle", /* ... 33 个 */];
```

四条断言：

1. App.tsx 里首字母小写的顶层函数，必须全在清单里 —— **加不进第 34 个**
2. 清单里的名字必须还在 App.tsx 里 —— 搬走了就得删行，清单不会烂掉
3. App.tsx 行数 ≤ 3500 —— 上限只能调低，不能为了塞下改动调高
4. 组件（首字母大写）不受限制 —— JSX 本来就要求大写，规则不需要额外标注，也没法误开后门

---

### 已经搬完的（2026-08-13）

| 模块 | 内容 | 行数 |
|---|---|---|
| `src/backend-types.ts` | Rust 那边收发的所有形状（79 个类型，删掉 27 个死的） | 463 |
| `src/codex-toml.ts` | `config.toml` 的文本级读写 | 237 |
| `src/codex-context-entries.ts` | MCP / Skills / Plugins 当成一个列表 | 200 |
| `src/relay-settings.ts` | 供应商列表的读取、修复、增删改 | 674 |

**App.tsx：5088 → 3464 行，纯函数 100 → 33。** 全部是搬迁，零行为变化 —— 测试全绿就是证明。

剩下的 33 个基本是展示层：把后端枚举翻译成某个界面的文字的标签表，和某个表单专用的小解析器。搬它们收益最小、留着代价最低，所以排最后。

### 还没做的

20 个组件各自拆文件（`RelayProfileDetail` 500 多行、`SessionsScreen` 290 行），约 2000 行。要动 props 传递，风险和收益不成比例 —— 等真有两个对话同时改前端时再说。

## 7. 现在装了哪些工具

全部是通用公开手段 —— 不依赖任何编辑器、插件或账号，人和 AI 都能用同一条命令。

| 命令 | 干什么 |
|---|---|
| `npm run check` | TypeScript 类型检查 |
| `npm test` | 前端测试（含四个版本号文件一致性） |
| `npm run knip` | 找出没人用的依赖、导出、文件 |
| `npm run verify` | 上面三个一起跑 —— **CI 里跑的就是这个** |
| `scripts/release.sh <版本>` | 改四个版本号文件 + 跑测试 + 提交 |
| `scripts/release-tag.sh <版本>` | 确认已合并，在 origin/master 上打 tag 并推送 |

**knip** 是找死代码的。像「注册了但前端从没调用过的命令」「装了但从没 import 的依赖」这类东西，它一条命令扫出来 —— 而不是等人读代码读出来。

**CI 之前从来没跑过前端测试和 tsc**，只跑了 `vite build` 和 `cargo test`。现在 `npm run verify` 加进了 macOS 那个 job（前端与平台无关，只跑一次，不用跑三遍）。

---

## 8. 现在的状态

```
✅ 打薄批次 1+2 + 棘轮测试
✅ AGENTS.md 写入规则
✅ knip + CI 跑前端测试 + 发版脚本
✅ master 分支保护
→  可以开并行工作流了（前端仍建议按区域切分）
```
