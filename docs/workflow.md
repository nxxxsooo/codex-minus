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

只在需要发版时做。三个版本号文件必须一致：

```bash
# 1. 改版本号
#    package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml
# 2. 同步 Cargo.lock
cargo update -p codex-minus --manifest-path src-tauri/Cargo.toml
# 3. BOARD.md 写一条记录
# 4. 提交、走上面的 PR 流程合并

# 合并之后再打 tag —— 打在 origin/master 上，不是分支上
git fetch origin master
git tag -a v0.4.4 origin/master -m "说明"
git push origin v0.4.4
```

推 tag 会触发 release job，自动构建 6 个安装包并发布。

```bash
gh release view v0.4.4 --json assets --jq '.assets[].name'
```

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

### 两个解法

| 做法 | 位置 | 代价 |
|---|---|---|
| 打开 `Require branches to be up to date before merging` | 仓库 Settings → Branches | 每个 PR 合并前必须 rebase 重跑 CI，并发度降到 1 |
| Merge Queue | 同上，GitHub 原生功能 | 排队测「按这个顺序合并后的结果」，既并发又安全；对小仓库偏重 |

**一旦同时有 2 个以上 PR 在跑，就去打开第一个。** 慢一点，但省掉「合完发现 master 坏了」的排查——那个比等 CI 贵得多。

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
4. **2 个以上并发 PR → 打开 branch protection。**
5. **动共享基础的先合**，依赖它的后合并再 rebase。

---

## 5. 这个仓库现在还不能真并行

原因是 `src/App.tsx`：

```
5088 行   20 个组件   100 个纯函数
```

那 100 个纯函数散布在整个文件里、改动频繁。两个对话都碰前端 = 保证冲突，而且是 rebase 到怀疑人生那种。

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

当前 20 : 100。

### 怎么让规则真的生效：棘轮测试

写在文档里没用，下一个对话照样往里加。加一个**只减不增**的清单：

```ts
// src/app-shell-budget.test.ts
//
// App.tsx 里还没搬走的纯函数。这个清单只能变短。
// 搬走一个就删一行；想往 App.tsx 加新的纯函数 —— 不行，去建模块。
const HELPERS_STILL_IN_APP = ["normalizeSettings", "codexModelFromConfig", /* ... */];
```

好处：

- 新增违规立刻红，加不进第 101 个
- 债务被点名列出，清单长度就是进度条
- 不用一次性重构，每次 PR 删几行

### 搬迁批次

| 批次 | 内容 | 行数 | 去处 | 风险 |
|---|---|---|---|---|
| 1 | TOML / context entry 解析 | ~340 | `src/codex-toml.ts` | 极低 |
| 2 | settings / profile normalizer + mutator | ~750 | `src/relay-settings.ts` | 低 |
| 3 | 20 个组件各自拆文件 | ~2000 | `src/screens/*.tsx` | 中，要动 props |

1+2 做完 App.tsx 掉到 ~3900 行，**纯搬迁、零行为变化**，测试全绿即是证明。3 先不做，收益不抵风险。

---

## 7. 建议顺序

```
1. 打薄批次 1+2 + 棘轮测试        ← 一个 PR
2. AGENTS.md 写入规则
3. 之后才开并行工作流
```

打薄是并行的**前置条件**，不是并行之后的优化。
