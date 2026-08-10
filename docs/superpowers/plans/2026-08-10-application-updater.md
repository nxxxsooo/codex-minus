# Codex-- Manager 签名自动更新 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在应用启动后自动检查正式版更新，发现新版时展示版本与纯文本更新说明；经用户确认后下载 Tauri 签名 artifact、安装并重启，同时保留手动「检查更新」按钮和 Release 手工下载回退。

**Architecture:** 前端由纯 reducer＋可注入 controller 管理更新状态，Tauri 官方 updater／process 插件负责检查、签名验证、安装和重启。Rust 增加进程级 mutation gate：供应商／目录 live transaction 与会话归档／适配等写操作取得 mutation guard，更新安装取得排他 reservation，从后端原子消除退出竞态。发布流水线只在 tag 构建启用 updater artifact，以静态 `latest.json` 指向同一不可变 Release 的 macOS `.app.tar.gz` 和 Windows NSIS `.exe`。

**Tech Stack:** React 19、TypeScript、Node test runner、Tauri 2 updater／process plugins、Rust、GitHub Actions、GitHub Releases、Bitwarden、NSIS、macOS app bundle。

## Global Constraints

- 设计契约以 `docs/superpowers/specs/2026-08-10-application-updater-design.md` 为准。
- 第一版只更新正式 channel，不支持降级、beta、静默安装、强制更新或差分更新。
- 首个 updater-enabled 正式版本按 `0.4.0` 实施；现有 `v0.3.0` 必须手工安装 `0.4.0` 一次，后续版本才可应用内升级。
- 使用 Tauri 官方 Ed25519／minisign updater 签名；它不等同于 macOS Developer ID／公证或 Windows Authenticode。
- 私钥和密码永不进入仓库、`.env`、artifact、cache、日志或普通终端输出；公钥正文提交到 Tauri config。
- 本地、branch 与 PR 构建不得依赖签名私钥，也不得生成 updater artifact；tag 构建缺任一 secret 必须 fail closed。
- Windows 自动更新只支持通过 NSIS 安装的客户端，使用 Tauri v2 `createUpdaterArtifacts: true` 生成的 `.exe`＋`.exe.sig`；同一 `.exe` 也是手工安装附件。不得改用只为 v1 兼容生成的 `.nsis.zip`。MSI 安装的客户端永远是 manual-only，不得被 NSIS updater 覆盖。
- `latest.json.signature` 必须内联 `.sig` 正文，不能写路径或 URL；platforms 必须且只能有 `darwin-aarch64`、`windows-x86_64-nsis`、`windows-aarch64-nsis`。明确省略通用 `windows-x86_64`／`windows-aarch64` 和所有 `*-msi` key，防止 Tauri installer-specific lookup 回退到通用 NSIS URL。
- updater v1 直接使用插件 HTTPS，调用 `check()` 时不传 Manager network proxy。网络失败显示 GitHub Release 手工链接；不得临时修改进程代理环境。
- 更新器不写 `config.toml`、`auth.json`、Manager settings、目录状态或 journal；安装前必须取得后端排他 reservation。
- 每个任务开始前运行 `git status --short`，保留当前脏工作树。对已有改动的重叠文件只用 `git add -p -- <path>` 暂存本任务 hunk；新建文件可精确 `git add -- <path>`。每次 commit 前必须检查 `git diff --cached --name-only` 和 `git diff --cached`，不得整文件暂存用户／OpenSpec 既有改动。
- OpenSpec 在本计划中是只读进度基线；不得在 updater commit 中暂存或改写 `openspec/changes/*/tasks.md`。真实任务进度只通过对应 OpenSpec apply／complete 工作流单独更新。
- 所有 GitHub prerelease、Release、Secrets、environment 或远端删除动作都属于外部状态变更，执行前必须取得用户明确授权；正式 tag／Release 不在本计划内自动创建。

## OpenSpec 进度合并

执行时必须重新读取 OpenSpec；以下只是 2026-08-10 的基线。

- `add-manager-network-policy`：14／15。现有网络 resolver、sidecar、UI、官方目录刷新已实现；仅 packaged-app 人工验收 4.2 未完成。更新器首版明确不接入它，避免把未设计的插件代理注入混入此 change。
- `support-server-side-composite-catalogs`：29／30。catalog state、Context cleanup、live-state journal 和 UI 已实现；更新安装门禁必须包裹而不能改写这些 transaction。剩余 6.6 的真实 live switch 与 updater 无直接依赖。
- `streamline-session-lifecycle`：18／29。后台 archive maintenance、archive／restore 与 provider compatibility UI 已存在，且会在应用启动后异步运行。update reservation 必须把这些 mutation 纳入同一 gate，避免安装退出时截断原生 archive／adaptation；只读 list／scan 不需要阻塞检查更新。

## 文件映射

**新建**

- `src/app-updater-state.ts`、`src/app-updater-state.test.ts`：纯状态机、安装 channel、自动检查 gate、安装禁用判定。
- `src/app-updater-controller.ts`、`src/app-updater-controller.test.ts`：Tauri adapter、资源持有和可注入测试。
- `src/components/AppUpdaterDialog.tsx`：更新说明、下载进度和操作按钮。
- `src/app-updater-config.test.ts`：依赖、capability、endpoint、公钥和 overlay 静态测试。
- `src-tauri/src/update_gate.rs`：跨 live-state／session mutation 的更新安装排他门禁。
- `src-tauri/tauri.updater.conf.json`：tag build 专用 `createUpdaterArtifacts` overlay。
- `scripts/release-updater.mjs`、`scripts/release-updater.test.mjs`：版本门禁与 manifest 生成器。
- `src/release-workflow.test.ts`：始终执行的 workflow 语义契约测试；`actionlint` 只是补充。
- `RELEASE_NOTES.md`：GitHub Release 与 `latest.json` 共用的更新说明源。
- `docs/updater-release-runbook.md`：密钥恢复、发布、回退和升级矩阵。

**修改**

- `package.json`、`package-lock.json`：JS plugins 与 release tests/scripts。
- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`：Rust plugins 与版本更新。
- `src-tauri/src/lib.rs`：plugin／command 注册。
- `src-tauri/src/commands.rs`：安装 bundle channel 命令及 mutation gate 集成。
- `src-tauri/src/live_state.rs`、`src-tauri/src/commands.rs`：mutation gate 集成。
- `src-tauri/capabilities/default.json`：最小 updater／restart 权限。
- `src-tauri/tauri.conf.json`：公钥、HTTPS endpoint、Windows passive install 与版本。
- `src/App.tsx`、`src/styles.css`、`src/i18n-en.ts`：启动检查、手动按钮、modal、busy 集成。
- `.github/workflows/build.yml`：preflight、tag signing、artifact、manifest 与 Release。
- `README.md`、`AGENTS.md`、`BOARD.md`：发布与新 mutation gate 约束。

---

### Task 0：刷新 OpenSpec、依赖与发布基线

**Interfaces consumed:** 三个活动 OpenSpec changes；当前 Tauri／CI 配置；当前 dirty worktree。

**Produces:** 防覆盖执行顺序，不产生代码修改。

- [ ] 读取实时进度：

  ```bash
  openspec list --json
  openspec instructions apply --change add-manager-network-policy --json
  openspec instructions apply --change support-server-side-composite-catalogs --json
  openspec instructions apply --change streamline-session-lifecycle --json
  ```

- [ ] 检查重叠文件的用户改动：

  ```bash
  git status --short
  git diff -- src/App.tsx src/styles.css src/i18n-en.ts src-tauri/src/live_state.rs src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json .github/workflows/build.yml
  ```

- [ ] 确认 5 个本地版本源均为 `0.3.0`：`package.json`、`package-lock.json` root package、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 的 `codex-minus` package block，以及 `src-tauri/tauri.conf.json`；另确认当前正式 release tag 为 `v0.3.0`。Task 5 会一次性把 5 个本地源升为 `0.4.0`。

- [ ] 记录现有 CI 的二次 `codesign --force --deep --sign -`；Task 6 必须删除这一内容变更，只保留验证，保证 updater tar 与手工 zip 来自同一个最终 app。

- [ ] 不提交本任务。

### Task 1：用纯状态机锁定更新交互

**Files:**

- Create: `src/app-updater-state.ts`
- Create: `src/app-updater-state.test.ts`

**Interfaces consumed:** Tauri `Update` metadata 和 download progress 的抽象值。

**Produces:** `InstallationChannel`、`AppUpdateState`、`appUpdaterReducer`、`claimAutomaticUpdateCheck`、`updateInstallBlockReason`。

- [ ] 先写失败测试 `src/app-updater-state.test.ts`：

  ```ts
  import assert from "node:assert";
  import { describe, it } from "node:test";

  import {
    appUpdaterReducer,
    claimAutomaticUpdateCheck,
    initialAppUpdateState,
    resetAutomaticUpdateCheckForTests,
    updateInstallBlockReason,
  } from "./app-updater-state.ts";

  describe("app updater state", () => {
    it("keeps automatic no-update silent and manual no-update visible", () => {
      const initial = initialAppUpdateState("0.4.0", "app");
      const automatic = appUpdaterReducer(
        appUpdaterReducer(initial, { type: "check-started", origin: "automatic" }),
        { type: "no-update" },
      );
      assert.equal(automatic.phase, "idle");
      assert.equal(automatic.origin, null);

      const manual = appUpdaterReducer(
        appUpdaterReducer(initial, { type: "check-started", origin: "manual" }),
        { type: "no-update" },
      );
      assert.equal(manual.phase, "up-to-date");
      assert.equal(manual.origin, "manual");
    });

    it("tracks an available update and cumulative download bytes", () => {
      let state = initialAppUpdateState("0.4.0", "app");
      state = appUpdaterReducer(state, { type: "check-started", origin: "manual" });
      state = appUpdaterReducer(state, {
        type: "update-available",
        version: "0.4.1",
        notes: "Fixes",
      });
      state = appUpdaterReducer(state, { type: "download-started", totalBytes: 100 });
      state = appUpdaterReducer(state, { type: "download-progress", chunkBytes: 30 });
      state = appUpdaterReducer(state, { type: "download-progress", chunkBytes: 25 });
      assert.equal(state.targetVersion, "0.4.1");
      assert.equal(state.downloadedBytes, 55);
      assert.equal(state.totalBytes, 100);
    });

    it("claims the automatic check once per process", () => {
      resetAutomaticUpdateCheckForTests();
      assert.equal(claimAutomaticUpdateCheck(), true);
      assert.equal(claimAutomaticUpdateCheck(), false);
    });

    it("keeps MSI installs manual-only and development bundles inert", () => {
      assert.equal(initialAppUpdateState("0.4.0", "msi").phase, "manual-only");
      assert.equal(initialAppUpdateState("0.4.0", "development").phase, "idle");
      assert.equal(initialAppUpdateState("0.4.0", "unsupported").phase, "idle");
    });

    it("blocks install for frontend mutations but not read-only checks", () => {
      assert.equal(
        updateInstallBlockReason({
          relaySwitching: true,
          modelCatalogWriting: false,
          sessionMutationRunning: false,
          settingsSaving: false,
        }),
        "relay-switching",
      );
      assert.equal(
        updateInstallBlockReason({
          relaySwitching: false,
          modelCatalogWriting: false,
          sessionMutationRunning: false,
          settingsSaving: false,
        }),
        null,
      );
    });
  });
  ```

- [ ] 运行 RED：

  ```bash
  node --test --experimental-strip-types src/app-updater-state.test.ts
  ```

  Expected: module 尚不存在。

- [ ] 实现状态类型：

  ```ts
  export type UpdatePhase =
    | "idle"
    | "checking"
    | "up-to-date"
    | "available"
    | "downloading"
    | "installing"
    | "restart-ready"
    | "manual-only"
    | "failed";

  export type UpdateCheckOrigin = "automatic" | "manual";
  export type InstallationChannel = "app" | "nsis" | "msi" | "development" | "unsupported";

  export type AppUpdateState = {
    phase: UpdatePhase;
    origin: UpdateCheckOrigin | null;
    installationChannel: InstallationChannel;
    currentVersion: string;
    targetVersion: string | null;
    notes: string;
    downloadedBytes: number;
    totalBytes: number | null;
    error: string | null;
  };
  ```

- [ ] `initialAppUpdateState(currentVersion, installationChannel)` 对 `app`／`nsis` 返回 idle，对 `msi` 返回 `manual-only`，对 `development`／`unsupported` 保持 inert idle。reducer 支持这些 action：`check-started`、`no-update`、`update-available`、`download-started`、`download-progress`、`download-finished`、`install-started`、`restart-ready`、`install-blocked`、`manual-only`、`failed`、`dismiss`。`install-blocked` 保持 `phase = "available"` 和 update metadata，允许重试；下载／安装失败也保留 target metadata。

- [ ] `claimAutomaticUpdateCheck` 使用模块级 boolean；测试 reset 只重置该 boolean，不接触 localStorage。进程重启后自然允许下一次检查。

- [ ] `updateInstallBlockReason` 返回稳定 code，优先级为 settings save、relay switch、catalog write、session mutation；它只用于提前禁用，Rust reservation 仍是权威门禁。

- [ ] 运行 GREEN：

  ```bash
  node --test --experimental-strip-types src/app-updater-state.test.ts
  npm run check
  ```

- [ ] 提交：

  ```bash
  git add src/app-updater-state.ts src/app-updater-state.test.ts
  git commit -m "feat: add app updater state machine"
  ```

### Task 2：安装 Tauri plugins、最小权限与签名配置

**Files:**

- Create: `src/app-updater-config.test.ts`
- Create: `src-tauri/tauri.updater.conf.json`
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces consumed:** Tauri updater／process v2；长期 updater keypair。

**Produces:** 普通构建可用的 plugins；tag overlay；提交的公钥；不提交的私钥；`app_installation_channel`。

- [ ] 先写失败静态测试 `src/app-updater-config.test.ts`：读取 JSON／TOML／Rust 文件并断言：

  ```ts
  assert.match(packageJson.dependencies["@tauri-apps/plugin-updater"], /^\^2\./);
  assert.match(packageJson.dependencies["@tauri-apps/plugin-process"], /^\^2\./);
  assert.match(cargoToml, /^tauri-plugin-updater\s*=\s*"2"/m);
  assert.match(cargoToml, /^tauri-plugin-process\s*=\s*"2"/m);
  assert.match(libRs, /tauri_plugin_updater::Builder::new\(\)\.build\(\)/);
  assert.match(libRs, /tauri_plugin_process::init\(\)/);
  assert.deepEqual(
    capability.permissions.filter((item: string) => item.startsWith("updater:") || item.startsWith("process:")),
    ["updater:allow-check", "updater:allow-download-and-install", "process:allow-restart"],
  );
  assert.equal(
    tauriConfig.plugins.updater.endpoints[0],
    "https://github.com/nxxxsooo/codex-minus/releases/latest/download/latest.json",
  );
  assert.equal(tauriConfig.plugins.updater.windows.installMode, "passive");
  assert.ok(tauriConfig.plugins.updater.pubkey.trim().length > 40);
  assert.equal(updaterOverlay.bundle.createUpdaterArtifacts, true);
  assert.equal(tauriConfig.bundle.createUpdaterArtifacts, undefined);
  assert.match(commandsRs, /pub fn app_installation_channel/);
  assert.match(libRs, /app_installation_channel/);
  ```

- [ ] 运行 RED：

  ```bash
  node --test --experimental-strip-types src/app-updater-config.test.ts
  ```

- [ ] 安装 JS dependencies：

  ```bash
  npm install "@tauri-apps/plugin-updater@^2.10.1" "@tauri-apps/plugin-process@^2.3.1"
  ```

- [ ] 在 `src-tauri/` 安装 Rust dependencies，锁定 v2 family：

  ```bash
  cargo add tauri-plugin-updater@2 tauri-plugin-process@2
  ```

- [ ] 在 `tauri::Builder` 注册：

  ```rust
  .plugin(tauri_plugin_updater::Builder::new().build())
  .plugin(tauri_plugin_process::init())
  ```

- [ ] 在 `commands.rs` 增加 `app_installation_channel`，由后端而不是 UA／平台字符串判断实际 bundle 类型，并在 `lib.rs` 的 `generate_handler!` 注册：

  ```rust
  #[tauri::command]
  pub fn app_installation_channel() -> &'static str {
      use tauri::utils::platform::BundleType;

      if cfg!(debug_assertions) {
          return "development";
      }

      match tauri::utils::platform::bundle_type() {
          Some(BundleType::App) => "app",
          Some(BundleType::Nsis) => "nsis",
          Some(BundleType::Msi) => "msi",
          _ => "unsupported",
      }
  }
  ```

  不能只信 `bundle_type()`：当前 tauri-utils 在 macOS debug／未打包运行时也可能返回 `App`。把映射抽成接收 `is_debug` 的可测 helper，先以 `cfg!(debug_assertions)` 返回 `development`，再逐项映射 `App`／`Nsis`／`Msi`／`None`。Rust 测试覆盖 debug＋`App` 仍为 development，以及 release 的四种映射。macOS `.app` 和 Windows NSIS 允许自动更新；MSI 只能打开 Release 手工安装；`development`／`unsupported` 手动检查说明「仅打包应用支持自动更新」。

- [ ] capability 只加入：

  ```json
  "updater:allow-check",
  "updater:allow-download-and-install",
  "process:allow-restart"
  ```

  不使用更宽的 `updater:default` 或 `process:default`，因为 UI 不调用独立 download／install／exit。

- [ ] 创建 overlay，文件内容精确为：

  ```json
  {
    "bundle": {
      "createUpdaterArtifacts": true
    }
  }
  ```

- [ ] **外部 secret 写入检查点：** 生成长期 keypair、写 Bitwarden 或 GitHub Secrets 前先请求用户授权。获批后加载 `mj-bitwarden-cli`，使用 `mktemp -d /private/tmp/codex-minus-updater-key.XXXXXX` 创建受控临时目录，运行：

  ```bash
  UPDATER_KEY_DIR="$(mktemp -d /private/tmp/codex-minus-updater-key.XXXXXX)"
  npx tauri signer generate -w "$UPDATER_KEY_DIR/codex-minus-updater.key"
  ```

  为 key 设置独立强密码；不得把密码写进 shell history 参数。后续命令只引用同一 shell 中的 `$UPDATER_KEY_DIR`，并在持久化验证完成后处理该已解析的精确目录。

- [ ] 把 `.pub` 正文原样写入 `src-tauri/tauri.conf.json > plugins.updater.pubkey`；写入 endpoint 和 Windows `passive` install mode。公钥必须是正文，不是路径。

- [ ] 在 Bitwarden 建立一个长期恢复项，包含 private key attachment／secure note、密码和生成日期；随后写受保护 GitHub `updater-release` environment secrets：

  ```text
  TAURI_SIGNING_PRIVATE_KEY
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  ```

  不在任何工具输出中回显 secret 值。完成持久化和一次读取校验后，把临时 key 目录移入废纸篓或安全删除，并报告恢复位置，不报告值。

- [ ] 运行 GREEN：

  ```bash
  node --test --experimental-strip-types src/app-updater-config.test.ts
  npm run check
  cargo check --manifest-path src-tauri/Cargo.toml
  npm run vite:build
  ```

- [ ] 先精确暂存新文件；对 `package*.json`、Cargo files、`commands.rs`、`lib.rs`、capability 和 `tauri.conf.json` 只交互式暂存本任务 hunk：

  ```bash
  git add -- src/app-updater-config.test.ts src-tauri/tauri.updater.conf.json
  git add -p -- package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/capabilities/default.json src-tauri/tauri.conf.json
  git diff --cached --name-only
  git diff --cached --check
  ```

- [ ] 在暂存之后审计 staged diff，确认没有 private key／password：

  ```bash
  git diff --cached --check
  git diff --cached | rg -n "TAURI_SIGNING_PRIVATE_KEY|PRIVATE KEY|PASSWORD|codex-minus-updater\.key"
  ```

  Expected: 只出现 secret 名称，不出现值或 private key block。

- [ ] 提交：

  ```bash
  git diff --cached
  git commit -m "feat: configure signed Tauri updater"
  ```

### Task 3：建立跨 live-state 与会话写入的安装 reservation

**Files:**

- Create: `src-tauri/src/update_gate.rs`
- Modify: `src-tauri/src/live_state.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces consumed:** 现有 `live_state::lock()`；`session_operation_mutex()`；archive／restore／maintenance／adaptation／delete mutation commands。

**Produces:** `begin_update_install`、`cancel_update_install`；RAII `MutationGuard`。

- [ ] 在 `update_gate.rs` 先写本地 gate 单测：

  ```rust
  #[test]
  fn active_mutation_rejects_install_reservation() {
      let gate = UpdateGate::default();
      let mutation = gate.begin_mutation().unwrap();
      assert!(gate.try_reserve_install().is_err());
      drop(mutation);
      assert!(gate.try_reserve_install().is_ok());
  }

  #[test]
  fn install_reservation_rejects_new_mutations_until_cancelled() {
      let gate = UpdateGate::default();
      let reservation_id = gate.try_reserve_install().unwrap();
      assert!(gate.begin_mutation().is_err());
      gate.cancel_install(&reservation_id).unwrap();
      assert!(gate.begin_mutation().is_ok());
  }

  #[test]
  fn stale_cancel_cannot_clear_a_newer_reservation() {
      let gate = UpdateGate::default();
      let first_id = gate.try_reserve_install().unwrap();
      gate.cancel_install(&first_id).unwrap();
      let second_id = gate.try_reserve_install().unwrap();

      assert!(gate.cancel_install(&first_id).is_err());
      assert!(gate.begin_mutation().is_err());
      gate.cancel_install(&second_id).unwrap();
      assert!(gate.begin_mutation().is_ok());
  }

  #[test]
  fn only_one_concurrent_reservation_wins() {
      let gate = Arc::new(UpdateGate::default());
      let barrier = Arc::new(Barrier::new(3));
      let workers = (0..2)
          .map(|_| {
              let gate = Arc::clone(&gate);
              let barrier = Arc::clone(&barrier);
              std::thread::spawn(move || {
                  barrier.wait();
                  gate.try_reserve_install().is_ok()
              })
          })
          .collect::<Vec<_>>();
      barrier.wait();
      let successes = workers
          .into_iter()
          .filter(|worker| worker.join().unwrap())
          .count();
      assert_eq!(successes, 1);
  }
  ```

- [ ] 运行 RED：

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml update_gate -- --nocapture
  ```

- [ ] 实现 gate，不持有长生命周期 mutex：

  ```rust
  #[derive(Default)]
  struct GateState {
      active_mutations: usize,
      install_reservation: Option<String>,
      next_reservation: u64,
  }

  #[derive(Default)]
  pub struct UpdateGate {
      state: Mutex<GateState>,
  }
  ```

  `begin_mutation` 在同一 mutex 内检查 `install_reservation.is_none()` 并递增 counter，返回 RAII guard；guard Drop 递减。`try_reserve_install` 在同一 mutex 内要求 counter 为 0 且未占位，由后端生成新的不透明 `reservation_id`（进程 nonce＋单调 counter；前端不得解析），保存并返回它。`cancel_install(reservation_id)` 只有 token 精确匹配时才清除；缺失、重复或 stale token 都返回稳定错误，绝不能清掉更新的 reservation。测试同时覆盖并发唯一胜者与 ABA stale-cancel。

- [ ] 增加 process-wide static wrapper，错误 code 固定为 `mutation-active`、`install-already-reserved`、`install-reserved`，UI 不解析任意 Rust 文本。

- [ ] 修改 `live_state::lock()`：先取得 `MutationGuard`，再取得现有串行 `LIVE_STATE_LOCK`；把两个 guard 都存入 `LiveStateGuard`。因此 settings、provider、catalog、network sidecar 等所有现有 live-state 写路径自动阻止安装／被安装阻止，Context 与 journal 语义不变。

- [ ] 在每个会话 mutation 的 blocking entry point 取得 `MutationGuard`，并保留现有 `session_operation_mutex`：

  - 删除单个／批量 session；
  - native archive；
  - native restore；
  - archive maintenance batch；
  - active-session adaptation 和 restored-session adaptation（它们在 OpenSpec 后续落地时必须使用同一 helper）。

  `list_local_sessions`、archive preview、compatibility scan 等只读调用不取得 guard，更新检查也不取得。后台 archive maintenance 若遇到 `install-reserved`，必须在任何文件／DB／CLI mutation 前返回既有结果形状中的 `not_checked`／`deferred = true`，不弹错误；交互式 archive／restore／delete／adaptation 则在任何 mutation 前返回 `action-required`。为两种路径分别写「gate 拒绝后 mutation fake 未被调用」测试。

- [ ] 新增 Tauri commands：

  ```rust
  #[tauri::command]
  pub fn begin_update_install() -> CommandResult<UpdateInstallReservationPayload>;

  #[tauri::command]
  pub fn cancel_update_install(reservation_id: String) -> CommandResult<UpdateInstallReservationPayload>;
  ```

  begin 成功 payload 包含 `reserved: true`、`reservation_id: Some(String)`、`reason: null`；失败 payload 不返回 token。cancel 必须回传同一 reservation 的释放结果。字段在 Rust 结构中用 snake_case，并按项目现有 Tauri serialization 约定映射前端；注册到 `generate_handler!`。

- [ ] 增加命令级测试（测试函数名固定，供下面命令精确过滤）：`update_install_reservation_blocks_live_state_before_write`、`update_install_reservation_blocks_session_mutation_before_write`、`background_archive_maintenance_defers_before_mutation`、`interactive_session_mutation_requires_action_before_mutation`。覆盖 live transaction／session mutation 活跃时 begin 失败；reservation 后 `save_settings_blocking` 与 archive mutation 在写前失败；matching cancel 后均恢复；失败路径不创建／修改 journal、settings、rollout 或 auth。

- [ ] 运行 GREEN 和核心事务回归：

  ```bash
  cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
  cargo test --manifest-path src-tauri/Cargo.toml
  ```

  Full suite 必须在输出中实际列出上述四个命令级测试和 `update_gate` unit tests；不得以一个匹配 0 tests 仍返回 0 的 name filter 代替。现有 Context、live-state、session 和 `raw_auth_save_is_rejected` 回归也由同一次 full suite 执行。

- [ ] 提交：

  ```bash
  git add -- src-tauri/src/update_gate.rs
  git add -p -- src-tauri/src/live_state.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
  git diff --cached --name-only
  git diff --cached --check
  git diff --cached
  git commit -m "feat: gate updater install against active writes"
  ```

### Task 4：实现 Tauri controller、启动检查与更新 modal

**Files:**

- Create: `src/app-updater-controller.ts`
- Create: `src/app-updater-controller.test.ts`
- Create: `src/components/AppUpdaterDialog.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles.css`
- Modify: `src/i18n-en.ts`

**Interfaces consumed:** `@tauri-apps/api/app.getVersion`、`@tauri-apps/plugin-updater.check`、`Update.downloadAndInstall`、`@tauri-apps/plugin-process.relaunch`、Task 2 bundle-channel command、Task 3 reservation commands、现有 `actions.openExternalUrl`。

**Produces:** `createAppUpdaterController`；启动自动检查；topbar 手动按钮；专用 modal。

- [ ] 先用 fake ports 写 `src/app-updater-controller.test.ts`，覆盖：

  - 自动检查 `null` 后回到 silent idle；手动检查 `null` 后进入 up-to-date。
  - 自动检查抛错后回到 idle 且不打开 modal／覆盖 notice；手动检查抛错后进入 failed 并提供 Release 链接。
  - `msi` 自动检查完全静默且不调用 updater plugin；手动检查进入 `manual-only`，按钮经注入的 `openReleaseUrl` 打开 Release。`development`／`unsupported` 自动静默，手动返回「仅打包应用支持自动更新」。
  - update metadata 映射 `currentVersion`、`version`、`body ?? ""`。
  - `Started.contentLength` 设 total；每个 `Progress.chunkLength` 累加；`Finished` 进入 installing。
  - 创建 controller 后更新 `busySnapshotRef.current`，install 使用新 snapshot；busy reason 存在时不调用 reservation 或 download，证明没有 stale closure。
  - Rust reservation 失败时保留 available metadata 并显示 action-required。
  - `UpdateHandle.currentVersion`／`version` 与 reducer 当前状态任一不一致时，不调用 reservation 或 download，并要求重新检查。
  - download／install 抛错时用 matching `reservation_id` 调用 `cancel_update_install` 一次，保留重试能力；stale token 不得取消新 reservation。
  - install 成功后调用 `relaunch`；若 `relaunch()` 失败或返回但当前进程仍继续，取消 matching reservation、进入 `restart-ready` 并显示手工重启指引。Windows 若 installer 先退出进程，promise 后代码允许不执行。
  - 手动／自动新检查在 downloading／installing 阶段直接拒绝，既不调用 `check()` 也不 `close()` 正在使用的 `Update`。
  - 非 busy 的新检查或 `dispose()` 调用旧 `Update.close()`；busy dispose 不关闭 in-use resource，完成后的仍存活路径负责 close，避免 Tauri resource 泄漏。
  - `check` 15 秒 timeout 和 `downloadAndInstall` 10 分钟 timeout 都通过 adapter 传入。

- [ ] 运行 RED：

  ```bash
  node --test --experimental-strip-types src/app-updater-controller.test.ts
  ```

- [ ] 定义可注入 ports：

  ```ts
  export type AppUpdaterPorts = {
    checkForUpdate: () => Promise<UpdateHandle | null>;
    beginInstallReservation: () => Promise<{
      reserved: boolean;
      reservationId: string | null;
      reason: string | null;
    }>;
    cancelInstallReservation: (reservationId: string) => Promise<void>;
    relaunchApp: () => Promise<void>;
    getState: () => AppUpdateState;
    getBusySnapshot: () => AppMutationBusySnapshot;
    openReleaseUrl: () => Promise<void>;
    dispatch: (action: AppUpdateAction) => void;
  };
  ```

  `UpdateHandle` 只暴露 `currentVersion`、`version`、`body`、`downloadAndInstall`、`close`，测试不依赖 Tauri class。

- [ ] production adapter 使用：

  ```ts
  const update = await check({ timeout: 15_000 });
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") dispatch({ type: "download-started", totalBytes: event.data.contentLength ?? null });
    if (event.event === "Progress") dispatch({ type: "download-progress", chunkBytes: event.data.chunkLength });
    if (event.event === "Finished") dispatch({ type: "download-finished" });
  }, { timeout: 10 * 60_000 });
  await relaunch();
  ```

  不传 `proxy`、自定义 header、token 或 `allowDowngrades`。

- [ ] `downloadAndInstall` 顺序固定为：确认当前 handle 存在，且 handle 的 `currentVersion`／`version` 与 reducer state 完全一致（不一致则要求重新检查）→ 从 `busySnapshotRef.current` 取得最新 snapshot 并检查 frontend busy → 调用 `begin_update_install` 并保存不透明 `reservation_id` → 下载／验证／安装 → relaunch。`check()` 不传 `allowDowngrades`，不自行构造 Update handle。只要控制流继续运行（下载失败、安装失败、relaunch 抛错或 relaunch 意外返回），`finally` 都用 matching ID 尝试 cancel；安装已经成功但进程仍在时进入 `restart-ready`，提示用户手工退出并重开。Windows installer 令进程退出时后续代码自然不执行，reservation 随进程消失。

- [ ] production 当前版本必须来自 `await getVersion()`，不得硬编码 `0.4.0`。同时 invoke `app_installation_channel`，验证返回值属于 `app|nsis|msi|development|unsupported` 后初始化 state。`app`／`nsis` 才可调用 updater plugin；其余 channel 在 controller 入口 fail closed。

- [ ] `App.tsx` 用 `useRef` 保存 controller，并只构造一次；ports 的 `getState` 和 `getBusySnapshot` 从同步更新的 refs 读取，避免 render closure 过期。首屏现有初始化完成后按以下形状启动自动检查，并在 effect cleanup 调用 `controller.dispose()`：

  ```ts
  void Promise.allSettled([
    loadInitialSettings(),
    loadInitialSessions(),
    loadInitialCatalogState(),
    bootstrapUpdaterVersionAndChannel(),
  ]).then(() => {
    if (claimAutomaticUpdateCheck()) void controller.check("automatic");
  });
  ```

  这里用 `allSettled`，使某个非 updater 启动请求失败时仍能检查；自动检查失败保持 silent。controller 未完成 version／channel bootstrap 时不得 check。

- [ ] topbar 增加手动「检查更新」按钮；checking 时 spinner；available 时复用／整理 `.update-dot`；downloading／installing 时显示进度状态。手动无新版显示「当前已是最新版本」；手动网络／签名／manifest 错误显示可操作错误；自动无新版或错误不打开 modal、不覆盖当前 notice。

- [ ] 删除 `App.tsx` 中未使用的旧 `UpdateResult`／`StartupResult` 类型，并把 `styles.css` 现有两处 `.update-dot` 规则合并为一处；不要让残留命名形成第二套更新状态。

- [ ] `AppUpdaterDialog` 行为：

  - available 时展示当前版本、目标版本、`notes` 纯文本、Release 手工下载入口；
  - 使用 `white-space: pre-wrap`，禁止 `dangerouslySetInnerHTML`；
  - available 可「稍后」或「下载并安装」；
  - downloading／installing 不可关闭，按钮 disabled；
  - 字节进度未知时显示不确定进度，已知时 clamp 到 0–100%；
  - failed／blocked 显示可重试与 `https://github.com/nxxxsooo/codex-minus/releases/latest`。
  - `manual-only` 明确说明当前是 MSI 安装，自动更新只支持 NSIS；主按钮通过现有 `actions.openExternalUrl(RELEASE_URL)` 打开 Release。不得新增裸 `<a target>`、shell opener 或更宽 capability。
  - `development`／`unsupported` 的手动错误明确说明 dev／非打包环境不可测试 updater，同样可通过现有安全 opener 查看 Release。

- [ ] 把已知前端 busy flags 汇入 `updateInstallBlockReason`：settings save、relay switch、official/catalog write、archive／restore／maintenance／adaptation。read-only catalog status、session list、compatibility scan 不阻止检查；即使前端漏掉状态，Rust reservation 仍阻止安装。

- [ ] 运行 GREEN：

  ```bash
  node --test --experimental-strip-types src/app-updater-controller.test.ts
  npm test
  npm run check
  npm run vite:build
  ```

- [ ] 提交：

  ```bash
  git add -- src/app-updater-controller.ts src/app-updater-controller.test.ts src/components/AppUpdaterDialog.tsx
  git add -p -- src/App.tsx src/styles.css src/i18n-en.ts
  git diff --cached --name-only
  git diff --cached --check
  git diff --cached
  git commit -m "feat: add interactive app update flow"
  ```

### Task 5：建立版本门禁与 `latest.json` 生成器

**Files:**

- Create: `scripts/release-updater.mjs`
- Create: `scripts/release-updater.test.mjs`
- Create: `RELEASE_NOTES.md`
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces consumed:** 1 个 tag、5 个本地版本源、Tauri updater artifacts 与 `.sig`。

**Produces:** `assertLocalVersions`、`assertReleaseVersions`、`buildLatestManifest`、`validateLatestManifest`；统一 `0.4.0` 版本。

- [ ] 先写 `scripts/release-updater.test.mjs`，为以下导出逐项建临时 fixture：

  ```js
  readReleaseVersions(rootDir)
  assertLocalVersions({ rootDir })
  assertReleaseVersions({ rootDir, tag })
  buildLatestManifest({ version, tag, notes, pubDate, distDir, repository })
  validateLatestManifest({ manifest, distDir })
  ```

- [ ] `assertLocalVersions` 不需要 tag，要求 5 个本地版本源彼此一致且是有效 SemVer；`assertReleaseVersions` 先调用它，再要求 `v${localVersion}` 与 tag 精确相等。测试必须分别失败于：非法 tag；package.json、package-lock root、Cargo.toml `[package]`、Cargo.lock 中 `name = "codex-minus"` block、tauri.conf 任一版本失配；任一平台 artifact 缺失；任一 `.sig` 缺失；signature 被写成路径；HTTP URL；多余／缺失 platform key；URL basename 不存在于 dist。manifest generator 不负责「同版本／低版本」判断；该行为由 Tauri 客户端 comparator 和真实 rc 升级矩阵验证。

- [ ] 成功 fixture 使用稳定文件名：

  ```text
  Codex--Manager_0.4.0_aarch64.app.tar.gz
  Codex--Manager_0.4.0_aarch64.app.tar.gz.sig
  Codex--Manager_0.4.0_x64-setup.exe
  Codex--Manager_0.4.0_x64-setup.exe.sig
  Codex--Manager_0.4.0_arm64-setup.exe
  Codex--Manager_0.4.0_arm64-setup.exe.sig
  ```

- [ ] 运行 RED：

  ```bash
  node --test scripts/release-updater.test.mjs
  ```

- [ ] 实现精确解析：Cargo.toml 只读首个 `[package]` section；Cargo.lock 按 `[[package]]` 分块并要求恰好一个 `name = "codex-minus"`，不得搜索第一个 `version =`。

- [ ] `buildLatestManifest` 生成且只生成：

  ```json
  {
    "version": "0.4.0",
    "notes": "RELEASE_NOTES.md 的完整纯文本",
    "pub_date": "RFC3339",
    "platforms": {
      "darwin-aarch64": {
        "url": "https://github.com/nxxxsooo/codex-minus/releases/download/v0.4.0/Codex--Manager_0.4.0_aarch64.app.tar.gz",
        "signature": "对应 .sig 文件正文"
      },
      "windows-x86_64-nsis": {
        "url": "https://github.com/nxxxsooo/codex-minus/releases/download/v0.4.0/Codex--Manager_0.4.0_x64-setup.exe",
        "signature": "对应 .sig 文件正文"
      },
      "windows-aarch64-nsis": {
        "url": "https://github.com/nxxxsooo/codex-minus/releases/download/v0.4.0/Codex--Manager_0.4.0_arm64-setup.exe",
        "signature": "对应 .sig 文件正文"
      }
    }
  }
  ```

  JSON 示例中的 signature 描述在真实输出中必须由文件正文替换；validator 要拒绝空值、路径和换行外的控制字符。

- [ ] CLI 提供两个确定入口：

  ```bash
  node scripts/release-updater.mjs verify-local --root .
  node scripts/release-updater.mjs verify --root . --tag v0.4.0
  node scripts/release-updater.mjs manifest --root . --tag v0.4.0 --dist artifacts/dist --notes RELEASE_NOTES.md --output artifacts/dist/latest.json
  ```

- [ ] 把 `package.json` test 改为同时包含 `src/*.test.ts` 与 `scripts/*.test.mjs`，并增加 `release:verify`／`release:manifest` scripts；保持现有前端测试参数 `--experimental-strip-types`。

- [ ] 创建 updater-only 的 `RELEASE_NOTES.md` `0.4.0` 内容，只声明本计划实际交付：启动自动检查、手动入口、签名验证、NSIS／macOS 自动 channel、MSI 手工 channel、`v0.3.0` 用户需手工安装一次、平台代码签名限制。不得预先声称独立供应商计划已完成；正式 tag gate 可根据届时已合并且验证过的 commit 再追加其他已交付内容。

- [ ] 把五个本地版本源同步升为 `0.4.0`。使用 `npm version 0.4.0 --no-git-tag-version` 更新 package files；用小范围 patch 更新 Cargo.toml／tauri.conf；用 Cargo 命令刷新本包 lock block。不要创建 git tag。

- [ ] 运行 GREEN：

  ```bash
  node --test scripts/release-updater.test.mjs
  npm test
  node scripts/release-updater.mjs verify-local --root .
  node scripts/release-updater.mjs verify --root . --tag v0.4.0
  npm run check
  cargo check --manifest-path src-tauri/Cargo.toml
  ```

- [ ] 提交：

  ```bash
  git add -- scripts/release-updater.mjs scripts/release-updater.test.mjs RELEASE_NOTES.md
  git add -p -- package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
  git diff --cached --name-only
  git diff --cached --check
  git diff --cached
  git commit -m "build: add updater release manifest tooling"
  ```

### Task 6：重构 tag CI 的签名 artifact 与 Release 流

**Files:**

- Modify: `.github/workflows/build.yml`
- Create: `src/release-workflow.test.ts`

**Interfaces consumed:** Task 2 secrets／overlay；Task 5 generator；GitHub Actions artifacts。

**Produces:** branch 无 secret 构建；tag fail-closed 签名；完整 Release＋`latest.json`。

- [ ] 先创建始终由 `npm test` 执行的 `src/release-workflow.test.ts`，解析 workflow 文本并断言以下不可回退契约：branch／PR `macos`／`windows` jobs 依赖 `preflight`、不绑定 environment、不引用 signing secret、不使用 updater overlay；tag-only `macos-release`／`windows-release` 绑定 `environment: updater-release`、使用 updater overlay 和两个 secret；最终 `release` job 不引用 secret；macOS 不含 `codesign --force`；manifest keys 精确包含 `darwin-aarch64`、`windows-x86_64-nsis`、`windows-aarch64-nsis` 且不含通用 Windows／MSI key；release 上传 `latest.json`、三个 updater bundle／sig 和手工包。

- [ ] 运行 RED：

  ```bash
  npm test
  ```

  Expected: workflow 还不满足测试。若 `actionlint` 已在 PATH，后续把它作为补充验证；不得用 `actionlint` 取代语义测试，也不为此全局安装。

- [ ] 添加 `preflight` job：checkout＋Node setup＋`npm ci`；branch／PR 运行 `node scripts/release-updater.mjs verify-local --root .`，tag 运行 `node scripts/release-updater.mjs verify --root . --tag "$GITHUB_REF_NAME"`。所有平台 build jobs 都 `needs: preflight`。

- [ ] 把平台构建明确拆成两组：branch／PR 的 `macos`／`windows` jobs 只用 base config 且不绑定 environment；tag 的 `macos-release`／`windows-release` jobs 设 `if: startsWith(github.ref, 'refs/tags/')`、绑定受保护的 `environment: updater-release` 并读取 updater secrets。这样 branch／PR 不会触发 release environment 审批，也不可能读取私钥。

- [ ] branch／PR jobs 不注入 `TAURI_SIGNING_PRIVATE_KEY*`，不生成 updater artifact。tag jobs 在 shell 中先检查两个 secrets 非空，再运行：

  ```bash
  npx tauri build --config src-tauri/tauri.updater.conf.json --bundles app --target aarch64-apple-darwin
  ```

  Windows 同理，保留 `--bundles msi,nsis --target`。

- [ ] 只有 `macos-release`／`windows-release` 可读取两个 updater secrets；preflight、branch、PR 和最终 release metadata job 不接触 private key。最终 `release` job 的 `needs` 只指向两个 tag release build jobs。

- [ ] macOS 删除 `codesign --force --deep --sign -`；保留：

  ```bash
  codesign --verify --deep --strict --verbose=2 "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Codex-- Manager.app"
  ```

  手工 `.app.zip` 必须在验证后从同一个最终 `.app` 生成。

- [ ] tag mac artifact 上传：手工 `.app.zip`、`.app.tar.gz`、`.app.tar.gz.sig`。branch artifact 只要求手工 `.app.zip`。

- [ ] tag Windows 每个架构上传：手工 `.msi`、同时作为 updater bundle 的 NSIS `.exe`，以及该 `.exe.sig`。Tauri 可能同时产生 `.msi.sig`，但 MSI 不进入自动 channel，release job 不把它写入 manifest。

- [ ] release job 下载全部 artifact 后，要求每类数量精确为 1，再改名为 Task 5 的稳定名称；任何 `cp ... || true`、nullglob 静默缺失或模糊选第一个都改为 fail closed。

- [ ] 调用 manifest generator，随后生成 `SHA256SUMS`；checksum 覆盖手工 mac zip、两个 MSI、同时作为 updater 的两个 NSIS exe、mac updater tar、三个 sig 和 `latest.json`。

- [ ] `softprops/action-gh-release` 使用 `RELEASE_NOTES.md` 作为 body，上传 `artifacts/dist/*`。`latest.json` 内部 URL 指向不可变 `releases/download/v0.4.0/...`；客户端 endpoint 仍使用 `releases/latest/download/latest.json` 取得最新 manifest。

- [ ] tag preflight 在发布前检查 `RELEASE_NOTES.md` 只陈述当前 tag 实际包含且已验证的功能。供应商计划未合并时不得出现供应商／Doctor 完成声明；若届时已合并，以实际 commit 和验证结果为依据再加入，不让两个计划形成代码级前置依赖。

- [ ] 运行 GREEN：

  ```bash
  npm test
  if command -v actionlint >/dev/null; then
    actionlint .github/workflows/build.yml
  fi
  git diff --check
  ```

- [ ] 在不使用 secrets 的 branch／PR 路径本地执行等价 build 命令，确认 base config 仍能构建。不要用 tag overlay 冒充 branch 验证。

- [ ] 提交：

  ```bash
  git add -- src/release-workflow.test.ts
  git add -p -- .github/workflows/build.yml
  git diff --cached --name-only
  git diff --cached --check
  git diff --cached
  git commit -m "ci: publish signed updater artifacts"
  ```

### Task 7：密钥恢复、签名升级矩阵、OpenSpec 与文档收口

**Files:**

- Create: `docs/updater-release-runbook.md`
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `BOARD.md`

**Interfaces consumed:** Tasks 1–6；Bitwarden recovery item；GitHub release environment；macOS arm64、Windows x64／arm64 test surfaces。

**Produces:** 可恢复发布流程；真实签名升级证据；准确完成记录。

- [ ] 在 runbook 写清：key 生成与公钥核对、从 Bitwarden 恢复到安全临时路径、GitHub secret 名、branch／tag 命令、artifact 命名、manifest 验证、SHA 校验、失败回退、私钥轮换后果、`v0.3.0` 手工 bootstrap。

- [ ] 取得用户对 GitHub prerelease 外部写入的明确授权后，从 Bitwarden 恢复 private key 到新的 `/private/tmp/codex-minus-updater-restore.*` 临时目录。只把 key 路径／密码注入当前 staging build 进程，不回显值。不要另造无法证明客户端路径的「假验签」；下面真实 rc.1→rc.2 升级必须使用这把恢复出的 private key，而 rc.1 客户端使用仓库 committed pubkey。成功安装才算 keypair 恢复验证通过。

- [ ] 在 `/private/tmp/codex-minus-updater-staging.*` 生成两个临时 config overlay，绝不提交：rc.1 overlay 把 app version 设为 `0.4.0-rc.1`，把 updater endpoint 精确改为：

  ```text
  https://github.com/nxxxsooo/codex-minus/releases/download/updater-staging-rc2/latest.json
  ```

  rc.2 overlay 把 app version 设为 `0.4.0-rc.2` 并启用 `createUpdaterArtifacts: true`。二者沿用 production config 中 committed pubkey；构建签名只使用刚从 Bitwarden 恢复的 key。不得修改或暂存 production `tauri.conf.json`、5 个正式本地版本源或 `RELEASE_NOTES.md`。

- [ ] 为 macOS arm64、Windows x64、Windows arm64 生成 rc.2 updater artifacts 和 installer-specific staging `latest.json`，keys 仍精确为 `darwin-aarch64`、`windows-x86_64-nsis`、`windows-aarch64-nsis`，artifact URL 全部指向不可变 tag `updater-staging-rc2`。先本地运行 `validateLatestManifest`；获批后创建 GitHub prerelease `updater-staging-rc2` 并上传 rc.2 artifacts、`.sig` 和 `latest.json`。不得更新 `releases/latest` 或 production endpoint。

- [ ] 手工安装 rc.1 macOS `.app`／Windows NSIS 后启动真实客户端，等待自动检查并完成 rc.1→rc.2 下载、committed pubkey 验签、安装与重启。记录每个平台当前／目标版本和 artifact SHA-256，不记录 key。若任一平台表面不可用，标记未验证并阻止正式发布。成功后移除本地临时 key／overlay；删除远端 prerelease 是独立破坏性动作，必须再次取得用户授权，否则保留并明确标为 staging prerelease。

- [ ] macOS arm64 验收：自动检查只触发一次；手动可重复；notes 为纯文本；下载进度；签名验证；app 替换；重启；tray／单实例恢复；失败后手工 Release 链接可用。

- [ ] Windows x64 与 arm64 分别验收签名 NSIS `.exe` updater：架构匹配、passive installer、应用退出、安装完成、重启、卸载注册不损坏。另从 MSI 安装 rc.1，确认启动自动检查不调用 updater plugin、手动入口显示 manual-only 并用现有安全 opener 打开 Release，且绝不启动 NSIS 覆盖；MSI 不写入 manifest。

- [ ] 三个平台执行负向测试：篡改 updater bundle、错 `.sig`、缺 platform、signature 写路径、HTTP URL；manifest validator／客户端必须拒绝。另用 staging manifest 依次提供同版本和低版本，确认 Tauri `check()` 返回无更新且不安装；这两项是客户端 comparator 测试，不是 generator 验证。自动检查失败不得显示「已是最新」。

- [ ] 每次升级前后记录 SHA-256 与权限／ACL：

  ```text
  ~/.codex/config.toml
  ~/.codex/auth.json
  ~/.codex-session-delete/settings.json
  ~/.codex-session-delete/network-policy.json（存在时）
  ~/.codex-session-delete/session-lifecycle.json（存在时）
  ```

  内容和权限只能出现用户预期的非 updater 变化；更新器不得写这些文件。

- [ ] 专门验证 OpenSpec 交叉行为：archive maintenance／archive／restore／adaptation mutation 活跃时 install reservation 被拒绝；只读 session list／compatibility scan 不阻止检查；Manager network custom／direct 不被 updater 自动继承；server-side composite transaction 活跃时安装被拒绝且 journal 完整回滚／提交。

- [ ] 运行完整代码验证：

  ```bash
  npm test
  npm run check
  npm run vite:build
  cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
  cargo test --manifest-path src-tauri/Cargo.toml
  npm run build
  node scripts/release-updater.mjs verify --root . --tag v0.4.0
  openspec validate --all --strict --no-interactive
  ```

- [ ] 若完整 Rust 或跨平台升级矩阵不能在当前执行面完成，明确记录为「未验证」，不得发布正式 `v0.4.0` 或写「全部平台通过」。

- [ ] 重读 OpenSpec progress，并把实际状态写入本次执行报告，但不在本计划中修改或暂存 tasks 文件。network 4.2、composite 6.6 和 session lifecycle 的勾选只能在各自 OpenSpec apply／complete 工作流完成；updater gate 本身不自动完成它们。

- [ ] README 明确：启动自动检查、手动入口、签名校验、手工回退、首个 `0.4.0` 需从 `v0.3.0` 手工安装、macOS app／Windows NSIS 可自动更新、Windows MSI 永远需手工下载、updater 签名不消除 Gatekeeper／SmartScreen 提示。

- [ ] 在 `AGENTS.md` 加入短约束：任何新的持久化或 session mutation 路径必须取得 shared mutation guard，不能绕过 updater install reservation；保留现有 Context／OAuth 约束。

- [ ] 全部已声明验证真实通过后，向 `BOARD.md` 追加 updater 完成记录与版本矩阵；不改写旧历史。

- [ ] 最终 diff 与 secret 扫描：

  ```bash
  rg -n "BEGIN PRIVATE KEY|BEGIN OPENSSH PRIVATE KEY|untrusted comment: minisign encrypted secret key|TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)?=[^$]" src src-tauri scripts .github package.json package-lock.json
  rg -n "TO[D]O|TB[D]" docs/updater-release-runbook.md README.md AGENTS.md RELEASE_NOTES.md
  git diff --check
  git status --short
  ```

  Expected: 两次 `rg` 均无匹配；CI 中只允许 `${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}` 与 `${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}` 形式的引用。

- [ ] 只提交 updater 文档 hunk；不暂存任何 OpenSpec task 文件：

  ```bash
  git add -- docs/updater-release-runbook.md
  git add -p -- README.md AGENTS.md BOARD.md
  git diff --cached --name-only
  git diff --cached --check
  git diff --cached
  git commit -m "docs: add signed updater release runbook"
  ```

  `git diff --cached --name-only` 必须不含 `openspec/`。正式 tag／Release 是独立、需用户明确确认的外部发布动作，本任务不自动创建。
