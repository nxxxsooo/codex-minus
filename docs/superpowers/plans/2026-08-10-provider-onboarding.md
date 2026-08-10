# 单一路径供应商新建与兼容诊断 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 删除全部供应商模板，让新建供应商固定从「官方 Auth＋供应商 API Key＋Responses」开始；首次保存后留在详情页并同步模型目录；把 `max_output_tokens` 兼容重试放入上游 core，再在本仓透传诊断事实。

**Architecture:** 前端用纯 TypeScript helper 固化新建默认值、字段校验和目录视图状态；`App.tsx` 只负责 ID、React 状态与 Tauri 调用。供应商 HTTP 测试仍由 `codex-plus-core` 独占，本仓不得复制分类或请求逻辑。官方 OAuth 继续由 Codex／ChatGPT 客户端拥有，供应商 Key 继续走现有 owner-only settings 与 provider bearer 投影，绝不写 live `auth.json`。

**Tech Stack:** React 19、TypeScript、Node test runner、Tauri 2、Rust、Tokio／Reqwest、`codex-plus-core`／`codex-plus-data` git dependencies。

## Global Constraints

- 设计契约以 `docs/superpowers/specs/2026-08-10-provider-onboarding-design.md` 为准。
- 保留工作树中现有 OpenSpec、网络策略、模型目录和会话生命周期改动；禁止 stash、reset、checkout 覆盖或整文件重写。
- 新建逻辑不能修改 `defaultSettings`、`normalizeSettings` 的旧 profile fallback，也不能迁移任何既有 profile。
- 新建普通供应商固定为 `relayMode = "official"`、`officialMixApiKey = true`、`protocol = "responses"`；本地聚合供应商仍是独立且不受支持的旧路径。
- live `auth.json` 必须在所有保存、切换、测试、目录刷新前后保持字节不变；不得恢复 `authContents` 持久化。
- Provider Doctor 和快速测试必须共享上游 `test_relay_profile`；本仓只透传 `compatibilityFallbackUsed`／`initialHttpStatus`／`finalFailureCategory`。
- 上游工作只能位于 `/Users/mingjian/.cache/`；不得改 Cargo cache、使用 path dependency、`[patch]`、vendoring 或本地 fork pin。
- 每个任务开始前执行 `git status --short`。对当前已有用户改动的 `App.tsx`、`styles.css`、`i18n-en.ts`、`AGENTS.md`、`README.md`、`BOARD.md`、`model-catalog-ui.ts`／tests、Cargo files、`commands.rs`、`model_catalog.rs`、`live_state.rs`、CI，以及 Task 0 新发现的任何已修改 tracked file，必须使用 `git add -p` 精确暂存本任务 hunks，再执行 `git diff --cached --check` 与 `git diff --cached`；只有本计划新建的文件或确认未被用户修改的完整删除才可整文件暂存。禁止 `git commit -a` 或接管 untracked OpenSpec artifacts。

## OpenSpec 进度基线

计划生成时的真实进度如下，执行者必须在 Task 0 重新读取，不得用本文数字覆盖较新的状态。

- `support-server-side-composite-catalogs`：29／30。代码、测试、文档已完成；仅 6.6 的真实 disposable live switch 尚未执行。新建供应商改动必须保留 `upstreamTopology`、rich metadata、managed context cleanup 与 external adoption 行为。
- `add-manager-network-policy`：14／15。代码、测试与 BOARD 已完成；仅 4.2 的 packaged-app 人工网络验收未完成。Provider Doctor v1 仍使用上游环境客户端，本计划不把它临时接入 Manager 网络策略。
- `streamline-session-lifecycle`：18／29。会话 UI／只读扫描已经存在；1.3 仍等待上游 active-only provider sync 与 dependency revision 升级。该任务不阻塞 Provider Doctor 修复：若 active-only 已进入上游，选择包含两者的最小共同后继；若尚未进入，上游 fallback 可独立 pin，1.3 保持未完成并在以后再次升级。每次升级仍要求 `codex-plus-core` 和 `codex-plus-data` 使用同一 rev。

## 文件映射

**新建**

- `src/provider-onboarding.ts`：新建草稿默认值、字段校验、官方登录 view model、首次保存 orchestration。
- `src/provider-onboarding.test.ts`：纯 helper 单测。
- `src/provider-onboarding-source.test.ts`：模板删除和新建页面静态契约。
- `scripts/provider-probe-fixture.mjs`：packaged-app 验收用的无秘密 loopback 400→200 relay。

**修改**

- `src/App.tsx`：新建流程、详情状态机、登录引导、首次保存同步、fallback 文案。
- `src/model-catalog-ui.ts`、`src/model-catalog-ui.test.ts`：目录 loading／error／ready 判别与 after-current refresh queue。
- `src/styles.css`、`src/i18n-en.ts`、`AGENTS.md`：删除模板残留并加入新文案。
- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`：在上游提交可获取后同步 pin。
- `src-tauri/src/commands.rs`：透传 core 的兼容重试事实并做集成测试。
- `README.md`、`BOARD.md`：完成后记录稳定行为和验证证据。

**删除**

- `src/presets.ts`
- `src/components/ProviderPresetSelector.tsx`

**上游仓库修改**

- `crates/codex-plus-core/src/relay_config.rs`，实现和全部新测试都放入现有文件及其 `#[cfg(test)]` module，私有 classifier 不扩大为公共 API。

---

### Task 0：刷新 OpenSpec 与脏工作树基线

**Interfaces consumed:** 三个活动 OpenSpec change；当前 git 工作树。

**Produces:** 执行期依赖顺序，不产生代码修改。

- [ ] 运行 OpenSpec 状态检查：

  ```bash
  openspec list --json
  openspec instructions apply --change support-server-side-composite-catalogs --json
  openspec instructions apply --change add-manager-network-policy --json
  openspec instructions apply --change streamline-session-lifecycle --json
  ```

- [ ] 保存只读基线并确认本文涉及的重叠文件：

  ```bash
  git status --short
  git diff -- AGENTS.md BOARD.md README.md src/App.tsx src/styles.css src/i18n-en.ts src/model-catalog-ui.ts src/model-catalog-ui.test.ts src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/commands.rs src-tauri/src/live_state.rs .github/workflows/build.yml
  ```

- [ ] 若 `streamline-session-lifecycle` 的 1.3 已完成，记录它使用的 upstream rev，Task 5 选择包含该功能与 fallback 的最小已验证共同后继。若仍未完成，不擅自实现会话 scope，也不阻塞 fallback pin；保持 1.3 未勾选。

- [ ] 确认 `support-server-side-composite-catalogs` 已完成的代码仍在工作树中；新建 mixed-auth profile 的默认目录模式必须继续由现有逻辑解析为 `official-plus-custom`。

- [ ] 对重叠文件选择一种安全路径并记录：优先请用户先把现有 OpenSpec 进度整理为独立 baseline commit；若用户不希望这样做，则后续每个 commit 只用 `git add -p` 暂存本任务 hunks，并逐项审查 cached diff。任何 OpenSpec change 仍整体 untracked 时，本计划完全不暂存其 `tasks.md`。

- [ ] 不提交本任务；这是后续所有任务的防覆盖门禁。

### Task 1：用纯 helper 固化唯一的新建路径

**Files:**

- Create: `src/provider-onboarding.ts`
- Create: `src/provider-onboarding.test.ts`
- Modify: `src/App.tsx`

**Interfaces consumed:** `RelayProfile` 字段、`RelayContextSelection`、`contextSelectionForAllEntries`、`withGeneratedRelayFiles`。

**Produces:** `createNewRelayProfileDraft`、`validateNewProviderDraft`、`officialLoginGuide`、`OFFICIAL_AUTH_GUIDE_URL`。

- [ ] 先写失败测试 `src/provider-onboarding.test.ts`：

  ```ts
  import assert from "node:assert";
  import { describe, it } from "node:test";

  import {
    OFFICIAL_AUTH_GUIDE_URL,
    createNewRelayProfileDraft,
    officialLoginGuide,
    validateNewProviderDraft,
  } from "./provider-onboarding.ts";

  describe("provider onboarding", () => {
    it("creates only the official auth mixed Responses draft", () => {
      const contextSelection = { mcpServers: ["memory"], skills: [], plugins: [] };
      const draft = createNewRelayProfileDraft({ id: "relay-new", contextSelection });

      assert.equal(draft.id, "relay-new");
      assert.equal(draft.name, "");
      assert.equal(draft.baseUrl, "");
      assert.equal(draft.upstreamBaseUrl, "");
      assert.equal(draft.apiKey, "");
      assert.equal(draft.model, "");
      assert.equal(draft.relayMode, "official");
      assert.equal(draft.officialMixApiKey, true);
      assert.equal(draft.protocol, "responses");
      assert.strictEqual(draft.contextSelection, contextSelection);
      assert.equal(draft.authContents, "");
    });

    it("reports each required first-save field", () => {
      assert.deepEqual(
        validateNewProviderDraft({ baseUrl: " ", apiKey: "", model: "\n" }),
        { baseUrl: "required", apiKey: "required", model: "required" },
      );
      assert.deepEqual(
        validateNewProviderDraft({ baseUrl: "https://relay.example/v1", apiKey: "sk-test", model: "gpt-5.5" }),
        {},
      );
    });

    it("shows the official login guide only for an unauthenticated new draft", () => {
      assert.deepEqual(
        officialLoginGuide({ isNew: true, authenticated: false }),
        { visible: true, url: OFFICIAL_AUTH_GUIDE_URL },
      );
      assert.equal(officialLoginGuide({ isNew: true, authenticated: true }).visible, false);
      assert.equal(officialLoginGuide({ isNew: false, authenticated: false }).visible, false);
    });
  });
  ```

- [ ] 运行测试并确认 RED 原因是 helper 尚不存在：

  ```bash
  node --test --experimental-strip-types src/provider-onboarding.test.ts
  ```

  Expected: `ERR_MODULE_NOT_FOUND` for `src/provider-onboarding.ts`。

- [ ] 实现 `src/provider-onboarding.ts`，保持它不 import React 或 `App.tsx`：

  ```ts
  export type NewProviderFieldErrors = Partial<
    Record<"baseUrl" | "apiKey" | "model", "required">
  >;

  export function createNewRelayProfileDraft<TContext>({
    id,
    contextSelection,
  }: {
    id: string;
    contextSelection: TContext;
  }) {
    return {
      id,
      name: "" as const,
      model: "" as const,
      baseUrl: "" as const,
      upstreamBaseUrl: "" as const,
      apiKey: "" as const,
      protocol: "responses" as const,
      relayMode: "official" as const,
      officialMixApiKey: true as const,
      testModel: "" as const,
      configContents: "" as const,
      authContents: "" as const,
      useCommonConfig: true as const,
      contextSelection,
      contextSelectionInitialized: true as const,
      contextWindow: "" as const,
      autoCompactLimit: "" as const,
      modelList: "" as const,
      modelWindows: "" as const,
      userAgent: "" as const,
      aggregate: null,
    };
  }

  export function validateNewProviderDraft(profile: {
    baseUrl: string;
    apiKey: string;
    model: string;
  }): NewProviderFieldErrors {
    const errors: NewProviderFieldErrors = {};
    if (!profile.baseUrl.trim()) errors.baseUrl = "required";
    if (!profile.apiKey.trim()) errors.apiKey = "required";
    if (!profile.model.trim()) errors.model = "required";
    return errors;
  }

  export const OFFICIAL_AUTH_GUIDE_URL = "https://developers.openai.com/codex/auth";

  export function officialLoginGuide(input: {
    isNew: boolean;
    authenticated: boolean;
  }): { visible: boolean; url: string } {
    return {
      visible: input.isNew && !input.authenticated,
      url: OFFICIAL_AUTH_GUIDE_URL,
    };
  }
  ```

- [ ] 修改 `createRelayProfile`：只在 `App.tsx` 生成唯一 ID、读取全量 context selection，然后调用 helper 并传给 `withGeneratedRelayFiles`。删除默认名称和 `defaultSettings.relayBaseUrl` 注入，不触碰 aggregate 构造器。

- [ ] 在新建态调用 `validateNewProviderDraft`；Base URL、Key、配置模型分别设置 `aria-invalid` 和字段错误，任一错误时禁用首次保存。名称保持可选。

- [ ] 新建态隐藏接入模式、混入开关和协议选择，只显示不可编辑的「官方登录＋混入 API Key＋Responses API」摘要；已保存 profile 继续显示现有控件，保持兼容。

- [ ] 运行 GREEN：

  ```bash
  node --test --experimental-strip-types src/provider-onboarding.test.ts
  npm run check
  ```

  Expected: helper tests pass；TypeScript 无错误。

- [ ] 只提交本任务文件：

  ```bash
  git add src/provider-onboarding.ts src/provider-onboarding.test.ts
  git add -p src/App.tsx
  git diff --cached --check
  git diff --cached -- src/provider-onboarding.ts src/provider-onboarding.test.ts src/App.tsx
  git commit -m "feat: default provider onboarding to mixed auth"
  ```

### Task 2：删除模板代码并加入官方登录引导

**Files:**

- Create: `src/provider-onboarding-source.test.ts`
- Delete: `src/presets.ts`
- Delete: `src/components/ProviderPresetSelector.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles.css`
- Modify: `src/i18n-en.ts`
- Modify: `AGENTS.md`

**Interfaces consumed:** 全局 `relayFiles.authStatus`、现有 `actions.openExternalUrl`。

**Produces:** 无模板的新建 UI；只读官方登录说明入口。

- [ ] 先写失败的静态契约测试 `src/provider-onboarding-source.test.ts`：

  ```ts
  import assert from "node:assert";
  import { existsSync, readFileSync } from "node:fs";
  import path from "node:path";
  import { fileURLToPath } from "node:url";
  import { describe, it } from "node:test";

  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

  describe("provider onboarding source contract", () => {
    it("contains no provider preset implementation", () => {
      assert.equal(existsSync(path.join(root, "src/presets.ts")), false);
      assert.equal(existsSync(path.join(root, "src/components/ProviderPresetSelector.tsx")), false);
      const app = readFileSync(path.join(root, "src/App.tsx"), "utf8");
      const styles = readFileSync(path.join(root, "src/styles.css"), "utf8");
      assert.doesNotMatch(app, /ProviderPresetSelector|PresetPatch|PRESETS/);
      assert.doesNotMatch(styles, /\.preset-/);
      const normalFactory = app.match(
        /function createRelayProfile[\s\S]*?function createAggregateRelayProfile/,
      )?.[0] ?? "";
      assert.match(normalFactory, /createNewRelayProfileDraft/);
      assert.doesNotMatch(normalFactory, /defaultSettings\.relayBaseUrl|officialMixApiKey:\s*false/);
    });
  });
  ```

- [ ] 运行 RED：

  ```bash
  node --test --experimental-strip-types src/provider-onboarding-source.test.ts
  ```

  Expected: 预设文件仍存在，测试失败。

- [ ] 删除前先执行 `git diff -- src/presets.ts src/components/ProviderPresetSelector.tsx`。只有两文件没有用户改动时才完整删除并整文件暂存；若已有改动，停止并请用户决定如何保留，不能用删除覆盖。随后删除两个模板文件和 `App.tsx` 的两个 import／挂载点。不要删除历史 `BOARD.md` 中的旧记录。

- [ ] 删除 `styles.css` 的全部 `.preset-*` 独占规则，并从共享 selector 中逐项移除 `.preset-toggle`、`.preset-btn`、`.preset-search-input`；不要整段覆盖当前网络策略或模型目录样式。

- [ ] 删除 `i18n-en.ts` 中只由模板使用的键：`从预设模板创建`、`供应商预设列表`、`搜索供应商…`、`没有匹配「`、`」的供应商`、`中国官方`、`聚合/中转`、`第三方`、`{0} 个供应商`。保留其他页面仍使用的 `官方`。

- [ ] 从 `AGENTS.md` 的 Architecture 删除 Presets 行；不要改变其 OAuth、catalog 或 Chat Completions 约束。

- [ ] 给 `RelayProfileDetail` 增加独立的 `officialAuthStatus` prop。父组件始终传 `relayFiles?.authStatus ?? null`，但仍只把 `relayFiles` 的 live config 内容传给活动 profile。

- [ ] 新建页显示以下固定说明：

  ```text
  使用官方 Codex／ChatGPT 登录身份，并通过当前供应商 Base URL 与 Key 访问模型。
  ```

  若 `officialLoginGuide({ isNew, authenticated }).visible` 为 true，再显示：

  ```text
  请先在官方 Codex／ChatGPT 客户端登录免费账号。
  ```

  「查看官方登录说明」只调用 `actions.openExternalUrl(guide.url)`；URL 来自纯 view model，不在 JSX 重复硬编码。不新增 WebView、OAuth callback、浏览器会话读取或 token 接收。OAuth 不写入由 Rust `raw_auth_save_is_rejected` 与最终 live-auth byte hash 验证，静态源码测试不冒充安全证明。

- [ ] 运行静态清理检查：

  ```bash
  rg -n "ProviderPresetSelector|PresetPatch|PRESETS|\\.preset-" src AGENTS.md
  ```

  Expected: 无匹配。

- [ ] 运行 GREEN：

  ```bash
  npm test
  npm run check
  npm run vite:build
  ```

- [ ] 提交：

  ```bash
  git add src/provider-onboarding-source.test.ts src/presets.ts src/components/ProviderPresetSelector.tsx
  git add -p src/App.tsx src/styles.css src/i18n-en.ts AGENTS.md
  git diff --cached --check
  git diff --cached -- src/provider-onboarding-source.test.ts src/presets.ts src/components/ProviderPresetSelector.tsx src/App.tsx src/styles.css src/i18n-en.ts AGENTS.md
  git commit -m "refactor: remove provider preset picker"
  ```

### Task 3：让首次保存留在详情并可靠同步目录

**Files:**

- Modify: `src/App.tsx`
- Modify: `src/model-catalog-ui.ts`
- Modify: `src/model-catalog-ui.test.ts`
- Modify: `src/provider-onboarding-source.test.ts`

**Interfaces consumed:** `save_settings` 返回的 canonical `BackendSettings`；`model_catalog_status`；现有 `ProfileCatalogSummary`。

**Produces:** `RelayDetailState` 判别联合；`profileCatalogViewState`；可等待的 after-current refresh queue；`completeFirstProviderSave`。

- [ ] 先在 `src/model-catalog-ui.test.ts` 加入失败测试：

  ```ts
  import {
    catalogRefreshOutcome,
    createAfterCurrentTaskQueue,
    profileCatalogViewState,
  } from "./model-catalog-ui.ts";

  it("does not mislabel catalog synchronization as unsupported", () => {
    assert.deepEqual(
      profileCatalogViewState({
        isDraft: true,
        profileId: "new",
        phase: "ready",
        errorMessage: null,
        catalog: null,
      }),
      { kind: "hidden" },
    );
    assert.deepEqual(
      profileCatalogViewState({
        isDraft: false,
        profileId: "saved",
        phase: "loading",
        errorMessage: null,
        catalog: null,
      }),
      { kind: "loading" },
    );
    assert.deepEqual(
      profileCatalogViewState({
        isDraft: false,
        profileId: "saved",
        phase: "error",
        errorMessage: "post-save refresh failed",
        catalog: { status: "ok", message: "stale ok", profiles: [] },
      }),
      { kind: "error", message: "post-save refresh failed" },
    );
    assert.equal(
      profileCatalogViewState({
        isDraft: false,
        profileId: "saved",
        phase: "ready",
        errorMessage: null,
        catalog: {
          status: "ok",
          message: "ok",
          profiles: [{ profileId: "saved", managedAvailable: true }],
        },
      }).kind,
      "ready",
    );
  });

  it("queues exactly one post-save refresh behind an in-flight refresh", async () => {
    const queue = createAfterCurrentTaskQueue<number>();
    let releaseFirst!: (value: number) => void;
    let releaseSecond!: (value: number) => void;
    let calls = 0;
    const first = queue.run(
      () => new Promise<number>((resolve) => {
        calls += 1;
        releaseFirst = resolve;
      }),
      "coalesce",
    );
    const afterSave = queue.run(
      () => new Promise<number>((resolve) => {
        calls += 1;
        releaseSecond = resolve;
      }),
      "after-current",
    );
    const duplicateAfterSave = queue.run(() => Promise.resolve(99), "after-current");
    assert.equal(calls, 1);
    releaseFirst(1);
    assert.equal(await first, 1);
    await Promise.resolve();
    assert.equal(calls, 2);
    releaseSecond(2);
    assert.equal(await afterSave, 2);
    assert.equal(await duplicateAfterSave, 2);
  });

  it("classifies a missing post-save summary as a synchronization error", () => {
    assert.deepEqual(
      catalogRefreshOutcome({
        profileId: "saved",
        succeeded: true,
        message: "ok",
        profiles: [],
      }),
      { phase: "error", error: "已保存供应商，但目录状态中缺少该供应商。" },
    );
  });

  it("uses the current failed refresh message instead of stale catalog data", () => {
    assert.deepEqual(
      catalogRefreshOutcome({
        profileId: "saved",
        succeeded: false,
        message: "target CLI unavailable",
        profiles: [{ profileId: "saved" }],
      }),
      { phase: "error", error: "target CLI unavailable" },
    );
  });

  it("runs the one queued after-current task even when the current task rejects", async () => {
    const queue = createAfterCurrentTaskQueue<number>();
    let rejectCurrent!: (error: Error) => void;
    let queuedCalls = 0;
    const current = queue.run(
      () => new Promise<number>((_resolve, reject) => {
        rejectCurrent = reject;
      }),
      "coalesce",
    );
    const queued = queue.run(async () => {
      queuedCalls += 1;
      return 2;
    }, "after-current");
    rejectCurrent(new Error("old refresh failed"));
    await assert.rejects(current, /old refresh failed/);
    assert.equal(await queued, 2);
    assert.equal(queuedCalls, 1);
  });
  ```

- [ ] 运行 RED：

  ```bash
  node --test --experimental-strip-types src/model-catalog-ui.test.ts
  ```

  Expected: `profileCatalogViewState` 尚未导出。

- [ ] 在 `src/model-catalog-ui.ts` 实现判别联合和纯 helper：

  ```ts
  export type ProfileCatalogView<T> =
    | { kind: "hidden" }
    | { kind: "loading" }
    | { kind: "error"; message: string }
    | { kind: "ready"; summary: T };

  export function profileCatalogViewState<T extends {
    profileId: string;
    managedAvailable: boolean;
  }>(input: {
    isDraft: boolean;
    profileId: string;
    phase: "loading" | "ready" | "error";
    errorMessage: string | null;
    catalog: { status: string; message: string; profiles: readonly T[] } | null;
  }): ProfileCatalogView<T> {
    if (input.isDraft) return { kind: "hidden" };
    if (input.phase === "loading") return { kind: "loading" };
    if (input.phase === "error") {
      return { kind: "error", message: input.errorMessage || "模型目录状态同步失败。" };
    }
    if (!input.catalog) {
      return { kind: "error", message: "模型目录状态尚未加载。" };
    }
    const summary = input.catalog.profiles.find((item) => item.profileId === input.profileId);
    return summary
      ? { kind: "ready", summary }
      : { kind: "error", message: "已保存供应商，但目录状态中缺少该供应商。" };
  }
  ```

- [ ] 同文件实现以下接口；它只消费本次 refresh 的结果，不读取旧全局 message：失败 status／null 产生 error；成功但缺目标 profile 产生明确同步错误；找到目标 summary 才返回 ready。

  ```ts
  export function catalogRefreshOutcome<T extends { profileId: string }>(input: {
    profileId: string;
    succeeded: boolean;
    message: string;
    profiles: readonly T[];
  }): { phase: "ready" | "error"; error: string | null };

  export function createAfterCurrentTaskQueue<T>(): {
    run(task: () => Promise<T>, mode: "coalesce" | "after-current"): Promise<T>;
  };
  ```

- [ ] `catalogRefreshOutcome` 必须先看本次 `succeeded`：失败时直接返回本次 `message`（空 message 使用稳定 fallback），不得读取旧 catalog；成功后仍须找到目标 profile 才返回 ready。`refreshModelCatalog` 返回 `null` 时由调用层显式映射成 `{ phase: "error", error: "模型目录状态同步失败。" }`，不能把 null 伪装成空的成功结果。

- [ ] 同文件实现 `createAfterCurrentTaskQueue<T>()`。没有当前任务时，两种 mode 都立即启动传入 task；`run(task, "coalesce")` 在已有 current 时复用 current promise；`run(task, "after-current")` 在已有 current 后只排队一次新 task，同一时刻多个 after-current 调用复用同一个 queued promise。current 无论 resolve 还是 reject 都必须启动 queued task；current 的调用者仍观察原 rejection，queued 的调用者观察 queued 结果。current／queued 各自在 settle 后只清理仍指向自己的内部引用，避免旧 promise 的 finally 清掉新任务；测试不使用 timer。

- [ ] 在 `src/provider-onboarding.test.ts` 先加入实际 orchestration 测试：

  ```ts
  import { completeFirstProviderSave } from "./provider-onboarding.ts";

  it("enters the canonical saved profile and refreshes exactly once", async () => {
    const entered: string[] = [];
    let refreshCalls = 0;
    const result = await completeFirstProviderSave({
      profileId: "relay-new",
      saveSettings: async () => ({ profiles: [{ id: "relay-new" }] }),
      selectProfile: (settings, id) => settings.profiles.find((item) => item.id === id) ?? null,
      enterSavedLoading: (profile) => entered.push(profile.id),
      refreshCatalog: async () => {
        refreshCalls += 1;
        return { profiles: [{ profileId: "relay-new" }] };
      },
      catalogOutcome: () => ({ phase: "ready", error: null }),
    });
    assert.deepEqual(entered, ["relay-new"]);
    assert.equal(refreshCalls, 1);
    assert.deepEqual(result, { kind: "saved", phase: "ready", error: null });
  });

  it("does not refresh or leave the draft when save fails", async () => {
    let refreshCalls = 0;
    const result = await completeFirstProviderSave({
      profileId: "relay-new",
      saveSettings: async () => null,
      selectProfile: () => null,
      enterSavedLoading: () => assert.fail("must remain a draft"),
      refreshCatalog: async () => {
        refreshCalls += 1;
        return null;
      },
      catalogOutcome: () => ({ phase: "error", error: "unreachable" }),
    });
    assert.equal(refreshCalls, 0);
    assert.deepEqual(result, { kind: "save-failed" });
  });

  it("does not enter saved state when the backend omits the canonical profile", async () => {
    let refreshCalls = 0;
    const result = await completeFirstProviderSave({
      profileId: "relay-new",
      saveSettings: async () => ({ profiles: [{ id: "different-profile" }] }),
      selectProfile: (settings, id) => settings.profiles.find((item) => item.id === id) ?? null,
      enterSavedLoading: () => assert.fail("must remain a draft"),
      refreshCatalog: async () => {
        refreshCalls += 1;
        return null;
      },
      catalogOutcome: () => ({ phase: "error", error: "unreachable" }),
    });
    assert.equal(refreshCalls, 0);
    assert.deepEqual(result, { kind: "save-failed" });
  });

  it("keeps the saved detail when catalog refresh fails", async () => {
    const result = await completeFirstProviderSave({
      profileId: "relay-new",
      saveSettings: async () => ({ profiles: [{ id: "relay-new" }] }),
      selectProfile: (settings, id) => settings.profiles.find((item) => item.id === id) ?? null,
      enterSavedLoading: () => {},
      refreshCatalog: async () => null,
      catalogOutcome: () => ({ phase: "error", error: "offline" }),
    });
    assert.deepEqual(result, { kind: "saved", phase: "error", error: "offline" });
  });
  ```

- [ ] 在 `src/provider-onboarding-source.test.ts` 增加一个只约束 canonical commit 边界的源码测试。它不承担 auth 安全证明，只防止以后重新引入首次保存的 optimistic parent write：

  ```ts
  it("commits canonical provider settings only after save succeeds", () => {
    const app = readFileSync(path.join(root, "src/App.tsx"), "utf8");
    const saveValue = app.match(
      /const saveSettingsValue[\s\S]*?const applyRelayInjection/,
    )?.[0] ?? assert.fail("saveSettingsValue block not found");
    const beforeInvoke = saveValue.split("await run(")[0];
    assert.doesNotMatch(beforeInvoke, /setSettings(?:Form)?\s*\(/);

    const saveRelay = app.match(
      /const saveRelaySettings[\s\S]*?const createNewAggregateProfile/,
    )?.[0] ?? assert.fail("saveRelaySettings block not found");
    assert.doesNotMatch(saveRelay, /onFormChange\s*\(/);
  });
  ```

  该测试只允许 `saveSettingsValue` 在确认后端返回 success 后调用 `setSettings`／`setSettingsForm`；failed payload 与 invoke exception 路径都不得调用 canonical setters。

- [ ] 在 `provider-onboarding.ts` 实现上述 generic `completeFirstProviderSave`，签名固定为：

  ```ts
  export async function completeFirstProviderSave<TSettings, TProfile, TCatalog>(input: {
    profileId: string;
    saveSettings: () => Promise<TSettings | null>;
    selectProfile: (settings: TSettings, profileId: string) => TProfile | null;
    enterSavedLoading: (profile: TProfile) => void;
    refreshCatalog: () => Promise<TCatalog | null>;
    catalogOutcome: (catalog: TCatalog | null) => {
      phase: "ready" | "error";
      error: string | null;
    };
  }): Promise<
    | { kind: "save-failed" }
    | { kind: "saved"; phase: "ready" | "error"; error: string | null }
  >;
  ```

  先 await `saveSettings`；null 时返回 `save-failed` 且不 refresh；从 canonical settings 找到同 ID profile 后同步调用 `enterSavedLoading`；再且只再调用一次 `refreshCatalog`；最后用 `catalogOutcome` 返回 saved ready／error。canonical 缺 profile 也返回 `save-failed`，不得进入 saved。

- [ ] 将 `saveSettingsValue` 的返回值从 `Promise<boolean>` 改为 `Promise<BackendSettings | null>`。删除请求前的 `setSettingsForm(normalized)`；只有 `result && isSuccessStatus(result.status)` 时才写 `settings`／`settingsForm` 并返回 canonical settings。failed payload 或 invoke error 返回 `null`，保持父级旧 canonical state；`RelayProfileDetail` 自己的 draft 继续保留用户输入。同步更新 `Actions`、`saveRelaySettings`、`RelayProfileDetail.onFormChange` 和三个调用点。

- [ ] 删除 `saveRelaySettings` 在请求前的 `onFormChange(next)` optimistic parent write；它只返回 `actions.saveSettingsValue(next, true)`。增加源码契约或纯 helper 测试，证明 save 失败时 parent canonical profile 数量不变、不会出现半成功 profile。

- [ ] 用一个状态替换互相冲突的 `detailProfileId`＋`newProfileDraft`：

  ```ts
  type RelayDetailState =
    | { kind: "list" }
    | { kind: "draft"; profile: RelayProfile }
    | { kind: "saving"; profile: RelayProfile }
    | {
        kind: "saved";
        profileId: string;
        catalogPhase: "loading" | "ready" | "error";
        catalogError: string | null;
      };
  ```

- [ ] 首次保存从 `draft` 原子切到 `saving`，保留同一份本地 profile 并禁用重复提交；后端失败时恢复为同一 `draft`（字段内容不丢失）。成功时从 canonical settings 按 draft ID 取回 canonical profile；找不到则显示保存同步错误并恢复 draft。找到后立即切换为同 ID 的 `{ kind: "saved", catalogPhase: "loading" }`，不返回列表、不自动设为当前。

- [ ] `RelayProfileDetail.saveDraft` 必须分开处理返回类型：活动 profile 继续消费 `saveActiveRelayProfile(): Promise<boolean>`；非活动／新建 profile 消费 `onFormChange(): Promise<BackendSettings | null>`。只有新建成功时才从 canonical settings 取 profile 并调用 `onSaved(canonicalProfile)`，不得把 boolean 强转成 settings。

- [ ] 把 App 的 `refreshModelCatalog` 改为使用一个稳定的 `createAfterCurrentTaskQueue` ref，并扩展签名为 `refreshModelCatalog(silent?, mode?: "coalesce" | "after-current")`。首次保存和「重试目录同步」都调用 `refreshModelCatalog(true, "after-current")`：若启动／手动 refresh 正在运行，必须等它结束后再执行一次能看到新 profile 的 status；不能因 `modelCatalogLoading` 直接返回 `null`。

- [ ] `completeFirstProviderSave` 紧接着调用且只调用一次 queued post-save refresh。应用异步结果前核对当前 detail 仍是同一 profile ID：

  - 返回成功且包含目标 summary：`catalogPhase = "ready"`。
  - 返回 `null`、failed status 或缺少目标 summary：`catalogPhase = "error"` 并保留可重试信息。
  - settings 已成功时，目录失败不回滚 profile。

- [ ] 给 `CatalogProfileEditor` 传入 `profileCatalogViewState` 的结果。`loading` 显示稳定 spinner；`error` 优先显示当前 detail 的 `catalogError`，点击「重试目录同步」先原子切到 loading／清空 error，再调用一次 after-current refresh，按 `catalogRefreshOutcome` 切 ready／error；只有 `ready.summary.managedAvailable === false` 才显示真正「不可用」。

- [ ] 运行 RED 后实现上述 helper／orchestration；测试必须同时证明保存失败零 refresh、catalog 失败仍 saved、并发旧 refresh 后精确执行一次 post-save refresh。不要用定时器猜测 React 更新。

- [ ] 运行 GREEN：

  ```bash
  node --test --experimental-strip-types src/model-catalog-ui.test.ts
  npm test
  npm run check
  npm run vite:build
  ```

- [ ] 提交：

  ```bash
  git add -p src/App.tsx src/model-catalog-ui.ts src/model-catalog-ui.test.ts src/provider-onboarding.ts src/provider-onboarding.test.ts src/provider-onboarding-source.test.ts
  git diff --cached --check
  git diff --cached -- src/App.tsx src/model-catalog-ui.ts src/model-catalog-ui.test.ts src/provider-onboarding.ts src/provider-onboarding.test.ts src/provider-onboarding-source.test.ts
  git commit -m "fix: keep first provider save in details"
  ```

### Task 4：在上游 core 实现一次性 Responses 兼容重试

**Files in upstream checkout:**

- Modify: `crates/codex-plus-core/src/relay_config.rs`

**Interfaces consumed:** 上游 `RelayProfile`、`RelayProtocol`、`proxied_client`；现有 404→`/v1` 修复。

**Produces:** `RelayProfileTestResult.compatibility_fallback_used`、`initial_http_status`、`final_failure_category`；可从 `BigPizzaV3/CodexPlusPlus` 获取的 commit。

- [ ] 在 `/Users/mingjian/.cache/codexplusplus-provider-fallback-20260810` 创建隔离 checkout。先只读确认该路径不存在；若已存在，检查 remote／branch／status，只有它就是本任务干净 checkout 时才复用，否则停止并选择新的明确 cache 路径，不覆盖或删除。clone／网络和 cache 写入按 sandbox 规则请求授权：

  ```bash
  test ! -e /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810
  git clone https://github.com/BigPizzaV3/CodexPlusPlus.git /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810
  git -C /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810 checkout -b codex/provider-test-output-limit-fallback origin/main
  ```

  后续每条上游命令都使用该绝对 `--manifest-path` 或 `git -C`；不得依赖本仓 cwd，也不得把 checkout 放入本项目。

- [ ] 只在 `relay_config.rs` 现有 `#[cfg(test)] mod tests` 中增加 classifier 与 loopback tests，因此可测试 private helper 而不扩大公共 API。不要创建二选一的 integration-test 结构。

- [ ] 先写 classifier 单测，覆盖：

  ```rust
  #[test]
  fn responses_output_limit_fallback_is_strictly_allowlisted() {
      assert!(responses_output_limit_fallback_allowed(
          RelayProtocol::Responses,
          400,
          r#"{"error":{"message":"Unknown parameter: max_output_tokens","type":"invalid_request_error"}}"#,
      ));
      assert!(responses_output_limit_fallback_allowed(
          RelayProtocol::Responses,
          400,
          r#"{"error":{"message":"Upstream request failed","type":"upstream_error"}}"#,
      ));
      assert!(!responses_output_limit_fallback_allowed(
          RelayProtocol::Responses,
          401,
          r#"{"error":{"message":"Unknown parameter: max_output_tokens"}}"#,
      ));
      assert!(!responses_output_limit_fallback_allowed(
          RelayProtocol::ChatCompletions,
          400,
          r#"{"error":{"message":"Unknown parameter: max_output_tokens"}}"#,
      ));
      assert!(!responses_output_limit_fallback_allowed(
          RelayProtocol::Responses,
          400,
          r#"{"error":{"message":"model not found","type":"invalid_request_error"}}"#,
      ));
      assert!(!responses_output_limit_fallback_allowed(
          RelayProtocol::Responses,
          400,
          r#"{"error":{"message":"Upstream request failed later","type":"upstream_error"}}"#,
      ));
      assert!(!responses_output_limit_fallback_allowed(
          RelayProtocol::Responses,
          400,
          r#"{"error":{"message":"Upstream request failed","type":"invalid_request_error"}}"#,
      ));
  }
  ```

- [ ] 写 loopback HTTP 测试，精确覆盖：

  1. 首次 `/v1/responses` 收到含 `max_output_tokens` 的 body，返回 allowlisted 400；
  2. 同一 endpoint 第二次 body 不含该字段，返回 200；
  3. 最终 `http_status == 200`、`compatibility_fallback_used == true`、`initial_http_status == Some(400)`；
  4. server 总请求数为 2；
  5. 首次 200 只请求一次，flag 为 false、initial status 与 final failure category 均为 None、payload 保留字段；
  6. 401、403、429、500、普通 400、Chat Completions 各只请求一次；404 使用以 `/v1` 结尾的 base URL 时只请求一次。另保留一个现有 404 自动补 `/v1` 测试，允许它做一次 path correction，但断言 compatibility retry 次数为 0；
  7. fallback 第二次若发生 timeout／connect transport error，结果保留首次 HTTP 400、fallback-used 和稳定 final failure category，不用 `http_status = 0`；
  8. API Key 即使被恶意响应原样回显，`response_preview` 也不含 Key。

- [ ] 运行 RED：

  ```bash
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core responses_output_limit_fallback_is_strictly_allowlisted -- --nocapture
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core relay_profile_test_retries_without_max_output_tokens -- --nocapture
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core relay_profile_test_redacts_api_key -- --nocapture
  ```

  Expected: helper／result fields 尚不存在。

- [ ] 扩展结果类型：

  ```rust
  pub struct RelayProfileTestResult {
      pub http_status: u16,
      pub endpoint: String,
      pub response_preview: String,
      pub compatibility_fallback_used: bool,
      pub initial_http_status: Option<u16>,
      pub final_failure_category: Option<String>,
  }
  ```

- [ ] 把 payload helper 改为 `relay_profile_test_payload(protocol, model, include_output_limit)`；Responses 仅在 `include_output_limit` 为 true 时插入 `max_output_tokens`，Chat Completions 始终保留原 `max_tokens`。

- [ ] 实现 `responses_output_limit_fallback_allowed`：仅 Responses＋HTTP 400；第一分支要求小写正文同时包含 `max_output_tokens`，并命中 `unknown parameter`、`unknown field`、`unrecognized parameter`、`unsupported parameter`、`unsupported field`、`invalid parameter`、`invalid field` 或 `not supported` 中至少一个完整语义短语；不能因任意 `invalid` 单词重试。第二分支必须成功解析 JSON，且 `error.type == "upstream_error"` 与 `error.message == "Upstream request failed"` 完全相等。

- [ ] 将响应读取抽成可复用的单次请求 helper。保留 404 自动补 `/v1`，但以最终有效 endpoint 的响应作为兼容判定输入；同一 endpoint 最多执行一次删除字段的 fallback。第二次 HTTP 结果是最终 status，`initial_http_status` 保留首次 400。第二次 `.send()` 若失败，返回结构化 failed result：`http_status = 400`、`compatibility_fallback_used = true`、`initial_http_status = Some(400)`，`final_failure_category` 只允许 `timeout`／`connect`／`request`／`body`／`other`，preview 使用已脱敏的首次响应并追加稳定类别；不得让 `?` 丢失首次证据。

- [ ] 在截断前先把实际 `api_key` 从响应正文替换为 `[REDACTED]`。任何日志、error、preview 都不得包含 Authorization header、Key 或完整请求 payload。

- [ ] generic `upstream_error` fallback 成功时只能报告「使用了省略可选字段的兼容重试」，不能声称上游已明确确认字段根因。

- [ ] 运行 GREEN 和完整 core 回归：

  ```bash
  cargo fmt --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml --all -- --check
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core responses_output_limit_fallback_is_strictly_allowlisted -- --nocapture
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core relay_profile_test_sends_supported_payload_once -- --nocapture
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core relay_profile_test_retries_without_max_output_tokens -- --nocapture
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core relay_profile_test_preserves_initial_status_on_fallback_transport_error -- --nocapture
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core relay_profile_test_does_not_retry_non_allowlisted_errors -- --nocapture
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core relay_profile_test_redacts_api_key -- --nocapture
  cargo test --manifest-path /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810/Cargo.toml -p codex-plus-core
  ```

- [ ] 在隔离 checkout 提交上游改动：

  ```bash
  git -C /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810 add crates/codex-plus-core/src/relay_config.rs
  git -C /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810 diff --cached --check
  git -C /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810 diff --cached
  git -C /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810 commit -m "fix(core): retry Responses probe without output limit"
  ```

- [ ] **外部写入检查点：** 在 push 分支或创建 `BigPizzaV3/CodexPlusPlus` PR 前，向用户明确请求授权。未获授权时停在本地 commit，并把 commit SHA 和测试结果交回；不要把本仓改成不可共享的 local/path dependency。

### Task 5：升级精确上游 revision 并透传 fallback 事实

**Files:**

- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src/App.tsx`
- Modify: `src/i18n-en.ts`

**Interfaces consumed:** 已合并到 `BigPizzaV3/CodexPlusPlus` 的精确 merge／squash commit；Task 4 结果字段；OpenSpec 会话任务 1.3 的 active-only API（仅在该 SHA 已包含时）。

**Produces:** 同 rev 的 core/data pin；快速测试与 Doctor 的可观察兼容提示。

- [ ] Task 4 获批 push／PR 后，等待 PR 确实 merged。按固定 branch name 查询 merged PR，再读取官方 merge／squash SHA。不得用本地 commit ancestry 判断 squash merge，也不得 pin 当时 `origin/main` tip：

  ```bash
  provider_fallback_pr_url="$(gh pr list --repo BigPizzaV3/CodexPlusPlus --state merged --search 'head:codex/provider-test-output-limit-fallback' --limit 2 --json url --jq 'if length == 1 then .[0].url else error("expected exactly one merged provider fallback PR") end')"
  provider_fallback_merge_sha="$(gh pr view "$provider_fallback_pr_url" --repo BigPizzaV3/CodexPlusPlus --json state,mergeCommit --jq 'select(.state == "MERGED") | .mergeCommit.oid')"
  test -n "$provider_fallback_merge_sha"
  git -C /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810 fetch origin "$provider_fallback_merge_sha"
  git -C /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810 checkout --detach "$provider_fallback_merge_sha"
  test "$(git -C /Users/mingjian/.cache/codexplusplus-provider-fallback-20260810 rev-parse HEAD)" = "$provider_fallback_merge_sha"
  ```

  若查询不到唯一 URL、state 不是 `MERGED` 或 `mergeCommit.oid` 为空则停止。checkout 该精确 SHA，在该 SHA 重跑 Task 4 core tests，再把它作为候选 pin。若 active-only 已另行合并，先从其 OpenSpec 证据取得对应的精确 merged SHA，再用 `git merge-base --is-ancestor` 比较两个已知 SHA：其中一个包含另一个时选择较新的那个；互不包含时只选择官方 main 上经 `git merge-base --is-ancestor <两个SHA> <候选SHA>` 双重验证的第一个共同后继。对共同后继也重跑 Task 4 tests，并记录最终 SHA；不得直接采用 fetch 时的 `origin/main` tip。若 active-only 尚未合并，不阻塞 fallback，保持 OpenSpec 1.3 未完成。

- [ ] 先在 `commands.rs` 测试区写失败集成测试 `provider_doctor_reports_compatibility_fallback`：loopback server 依次返回 `/v1/models` 200、第一次 `/v1/responses` allowlisted 400、第二次 200；断言请求数为 3、首次 body 有字段、第二次没有、Doctor request check 为 ok、提示含兼容重试、所有结果不含测试 Key。

- [ ] 为快速测试增加 `relay_test_payload_preserves_compatibility_fallback` 测试，断言 payload：

  ```rust
  compatibility_fallback_used: true
  initial_http_status: Some(400)
  final_failure_category: None
  http_status: 200
  ```

- [ ] 运行 RED：

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml provider_doctor_reports_compatibility_fallback -- --nocapture
  cargo test --manifest-path src-tauri/Cargo.toml relay_test_payload_preserves_compatibility_fallback -- --nocapture
  ```

- [ ] 将 `src-tauri/Cargo.toml` 中 `codex-plus-core` 与 `codex-plus-data` 的 rev 同时更新为上一步已测试的精确 SHA，然后执行：

  ```bash
  cargo update --manifest-path src-tauri/Cargo.toml -p codex-plus-core -p codex-plus-data
  ```

- [ ] 扩展 Rust `RelayProfileTestPayload`／`ProviderDoctorPayload` 和前端 `RelayProfileTestResult`／`ProviderDoctorResult`：

  ```rust
  pub compatibility_fallback_used: bool,
  pub initial_http_status: Option<u16>,
  pub final_failure_category: Option<String>,
  ```

  ```ts
  compatibilityFallbackUsed: boolean;
  initialHttpStatus: number | null;
  finalFailureCategory: string | null;
  ```

  Doctor 在 URL／models 等早退路径返回 `false`／`null`／`null`；执行真实请求后透传上游值。

- [ ] 快速测试成功消息与 Doctor 的「真实请求」detail 在 flag 为 true 且最终 HTTP 成功时追加：「已通过省略 `max_output_tokens` 的兼容重试。」若 `finalFailureCategory` 非空，显示「首次 HTTP 400；兼容重试发生 {category} 传输失败」，保持 failed。第二次 HTTP 失败同时展示 initial 与 final status。本仓不得重新检查 response 字符串或自行发第二次 HTTP 请求。

- [ ] 增加本仓 transport 回归：上游测试 fixture 首次 400 后断开连接，断言 `httpStatus == 400`、`initialHttpStatus == 400`、`compatibilityFallbackUsed == true`、`finalFailureCategory == "connect"`（或 fixture 对应稳定类别），快速测试／Doctor 均为 failed，不能因 0 `< 400` 误报成功。

- [ ] 若共同上游 SHA 同时包含 session lifecycle 1.3 所需的 active-only API，只执行并记录该 OpenSpec 指定的上游测试和本仓兼容回归；本计划不修改 1.3 的任务状态。是否完成本仓适配、回滚与 scope tests，以及是否勾选 1.3，留给 `streamline-session-lifecycle` 自己的 OpenSpec apply／complete workflow。

- [ ] 运行安全回归与 GREEN：

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml provider_doctor_reports_compatibility_fallback -- --nocapture
  cargo test --manifest-path src-tauri/Cargo.toml relay_test_payload_preserves_compatibility_fallback -- --nocapture
  cargo test --manifest-path src-tauri/Cargo.toml raw_auth_save_is_rejected
  cargo test --manifest-path src-tauri/Cargo.toml pure_api_staging_uses_config_bearer_without_touching_live_auth
  cargo test --manifest-path src-tauri/Cargo.toml context_transaction_preserves_unrelated_root_settings
  cargo test --manifest-path src-tauri/Cargo.toml atomic_write_repairs_owner_only_mode
  cargo test --manifest-path src-tauri/Cargo.toml
  npm test
  npm run check
  npm run vite:build
  ```

- [ ] 提交：

  ```bash
  git add -p src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/commands.rs src/App.tsx src/i18n-en.ts
  git diff --cached --check
  git diff --cached -- src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/commands.rs src/App.tsx src/i18n-en.ts
  git commit -m "fix: surface provider probe compatibility fallback"
  ```

### Task 6：打包验收、OpenSpec 对账和文档收口

**Files:**

- Modify: `README.md`
- Modify: `BOARD.md`
- Create: `scripts/provider-probe-fixture.mjs`
- Read only: relevant `openspec/changes/*/tasks.md`。OpenSpec artifacts 由各自 change 负责提交，本计划不接管。

**Interfaces consumed:** Tasks 1–5 的 commit；三个 OpenSpec changes；真实 packaged app。

**Produces:** 可安装构建、可重复验证证据、准确的完成记录。

- [ ] 在写文档前跑完整检查：

  ```bash
  npm test
  npm run check
  npm run vite:build
  cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
  cargo test --manifest-path src-tauri/Cargo.toml
  npm run build
  ```

- [ ] 若完整 Rust suite 因 sandbox／系统权限失败，使用获批的 host 执行边界重跑；不能把 targeted tests 或编译成功写成「完整 cargo test 通过」。

- [ ] 实现 `scripts/provider-probe-fixture.mjs`：只监听显式 `--host 127.0.0.1 --port 18765`；`GET /v1/models` 返回 `{"data":[{"id":"gpt-5.5"}]}`；`POST /v1/responses` 按调用次数奇数返回 allowlisted 400 generic wrapper、偶数返回 200 response object；只把 method／path 和 request body 是否含 `max_output_tokens` 写到 `--record` JSONL，不记录 Authorization header 或其值。收到 SIGINT／SIGTERM 时关闭 listener。

- [ ] 启动 fixture 前确认端口空闲并建立 audit 临时目录：

  ```bash
  lsof -nP -iTCP:18765 -sTCP:LISTEN
  provider_audit_dir="$(mktemp -d /private/tmp/codex-minus-provider-audit.XXXXXX)"
  shasum -a 256 /Users/mingjian/.codex/auth.json /Users/mingjian/.codex/config.toml > "$provider_audit_dir/live-before.sha256"
  stat -f '%Sp %Su:%Sg %N' /Users/mingjian/.codex/auth.json /Users/mingjian/.codex/config.toml /Users/mingjian/.codex-session-delete/settings.json > "$provider_audit_dir/mode-before.txt"
  node scripts/provider-probe-fixture.mjs --host 127.0.0.1 --port 18765 --record "$provider_audit_dir/requests.jsonl"
  ```

  最后一条在独立终端保持运行；测试 profile 只使用 Base URL `http://127.0.0.1:18765/v1`、Key `sk-codex-minus-fixture`、model `gpt-5.5`。

- [ ] 用 packaged app 人工验收：

  1. 点击「添加供应商」，确认没有模板、模式固定为 Auth mixed Responses、四个业务字段为空。
  2. 未登录时只显示官方登录说明；点击后打开官方文档，`auth.json` 不变。
  3. 填写 Base URL、Key、`gpt-5.5`，保存后仍停在同一 profile 详情。
  4. 目录加载态不显示「不支持」；完成后显示 `official-plus-custom` 完整编辑器。
  5. 首次保存不会自动设为当前。
  6. 对可控 400→200 relay 同时运行快速测试和 Provider Doctor，二者均显示兼容重试。

- [ ] 测试完成后从 UI 删除 disposable profile，停止 fixture，再核对：

  ```bash
  shasum -a 256 /Users/mingjian/.codex/auth.json /Users/mingjian/.codex/config.toml > "$provider_audit_dir/live-after.sha256"
  diff -u "$provider_audit_dir/live-before.sha256" "$provider_audit_dir/live-after.sha256"
  stat -f '%Sp %Su:%Sg %N' /Users/mingjian/.codex/auth.json /Users/mingjian/.codex/config.toml /Users/mingjian/.codex-session-delete/settings.json > "$provider_audit_dir/mode-after.txt"
  diff -u "$provider_audit_dir/mode-before.txt" "$provider_audit_dir/mode-after.txt"
  sed -n '1,20p' "$provider_audit_dir/requests.jsonl"
  ```

  Expected: live auth／config hashes完全一致，三文件 owner／mode 一致且 Unix 文件为 600；JSONL 中每个 fallback pair 的第一次为 `true`、第二次为 `false`，没有 Key。Windows 用 `Get-FileHash` 与 `Get-Acl` 做等价检查。

- [ ] 重读 OpenSpec：

  ```bash
  openspec instructions apply --change support-server-side-composite-catalogs --json
  openspec instructions apply --change add-manager-network-policy --json
  openspec instructions apply --change streamline-session-lifecycle --json
  openspec validate --all --strict --no-interactive
  ```

- [ ] `support-server-side-composite-catalogs` 6.6 仍需要显式 live-switch 授权；本计划的普通新建验收不能冒充该任务。`add-manager-network-policy` 4.2 只有完成其 direct／custom／bundled-fallback packaged 验收后才勾选。会话生命周期任务只按真实 scoped upstream／集成进展更新。

- [ ] 更新 README：删除模板说明；明确新建默认 Auth mixed Responses、登录归官方客户端、供应商 Key 不进入 `auth.json`、Provider Doctor 可能使用一次省略输出限制的兼容重试。

- [ ] 只在全部已声明验证真实通过后向 `BOARD.md` 追加一条完成记录，保留既有历史。

- [ ] 检查文档和 diff：

  ```bash
  rg -n "ProviderPresetSelector|PresetPatch|PRESETS|\\.preset-" src README.md AGENTS.md
  rg -n "TO[D]O|TB[D]" docs/superpowers
  git diff --check
  git status --short
  ```

  Expected: 两次 `rg` 均无匹配。

- [ ] 提交本计划文档变化，不暂存 OpenSpec artifacts：

  ```bash
  git add scripts/provider-probe-fixture.mjs
  git add -p README.md BOARD.md
  git diff --cached --check
  git diff --cached -- scripts/provider-probe-fixture.mjs README.md BOARD.md
  git commit -m "docs: record provider onboarding verification"
  ```

  若没有文档变化，不创建空 commit。OpenSpec 进度只读对账；需要勾选时回到对应 OpenSpec apply workflow，并把完整 change 作为独立审查／提交单元。
