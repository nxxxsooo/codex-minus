import { app, BrowserWindow, Menu, Tray, dialog, ipcMain, nativeImage, net, protocol, screen, session } from "electron";
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, realpathSync, writeFileSync } from "node:fs";
import { rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, extname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { CoreClient, CoreTransportError } from "./core-client.mjs";
import { allowedCommand, trustedSender, validTrayLabels } from "./command-policy.mjs";
import { reviewWindowBounds } from "./review-window.mjs";
import { checkElectronFeed } from "./update-check.mjs";
import { downloadVerifiedUpdate } from "./update-download.mjs";
import { verifyWithCore, verifyUpdateMetadata } from "./update-core-verify.mjs";
import { inspectMacBundle, stageVerifiedMacApp } from "./update-install.mjs";
import { launchUpdateHelper } from "./update-handoff.mjs";
import { consumeUpdateOutcomes } from "./update-outcome.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const testHome = process.env.CODEX_MINUS_TEST_HOME;
const automatedReview = process.env.CODEX_MINUS_AUTOMATION_REVIEW === "1";
const headlessReview = automatedReview && process.env.CODEX_MINUS_HEADLESS_REVIEW === "1";
if (automatedReview && !testHome) throw new Error("Automated review requires an isolated test home");
if (automatedReview && process.platform === "darwin") app.setActivationPolicy("prohibited");
const devUrl = app.isPackaged ? undefined : process.env.CODEX_MINUS_DEV_URL;
if (devUrl && new URL(devUrl).origin !== "http://127.0.0.1:1420") throw new Error("Invalid development origin");
if (testHome) {
  const reviewData = join(realpathSync(testHome), ".electron");
  mkdirSync(reviewData, { recursive: true, mode: 0o700 });
  app.setPath("userData", reviewData);
}
app.setName("Codex Minus");
app.setAppUserModelId("fun.mjshao.codex-minus");
protocol.registerSchemesAsPrivileged([{ scheme: "codex-minus", privileges: { standard: true, secure: true, supportFetchAPI: true } }]);
const allowedOrigin = devUrl ? new URL(devUrl).origin : "codex-minus://app";
// The test fixture owns its own home and must not wake or block the installed manager.
if (testHome) app.setName(`Codex Minus Review ${realpathSync(testHome).split("/").at(-1)}`);
const locked = app.requestSingleInstanceLock();
let window, tray, core, reconnecting, quitting = false, exitReady = false;
let reconciled = false;
let availableUpdate = null, preparedUpdate = null, preparingUpdate = false;
let coreStatus = { state: "starting", code: null };
let labels = { showLabel: "显示主窗口", quitLabel: "退出程序", windowTitle: "Codex Minus" };

function showWindow() {
  if (!window || window.isDestroyed()) return;
  if (headlessReview) return;
  window.restore();
  if (automatedReview) window.showInactive();
  else { window.show(); window.focus(); }
}

async function stopCore() {
  if (!core) return;
  const client = core;
  core = null;
  await client.stop();
}

function broadcast(state, code = null) {
  coreStatus = { state, code };
  if (window && !window.isDestroyed()) window.webContents.send("codex:core-state", coreStatus);
}

function executable() {
  const name = process.platform === "win32" ? "codex-minus-core.exe" : "codex-minus-core";
  return app.isPackaged ? join(process.resourcesPath, "core", name) : join(root, "src-tauri", "target", "debug", name);
}

async function connectCore() {
  if (reconnecting) return reconnecting;
  reconnecting = (async () => {
    if (core) await stopCore();
    if (quitting) return coreStatus;
    reconciled = false;
    broadcast("starting");
    const child = spawn(executable(), ["--stdio"], {
      stdio: ["pipe", "pipe", "pipe"], windowsHide: true,
      env: { ...process.env, ...(testHome ? { HOME: testHome, USERPROFILE: testHome, CODEX_HOME: join(testHome, ".codex") } : {}) },
    });
    const client = new CoreClient(child, { expectedVersion: app.getVersion() });
    core = client;
    client.on("state", (state, code) => {
      if (core !== client) return;
      if (state === "disconnected") reconciled = false;
      broadcast(state === "ready" && !reconciled ? "starting" : state, code);
    });
    client.on("focus", showWindow);
    try { await client.ready(); }
    catch (error) { broadcast("disconnected", error instanceof CoreTransportError ? error.code : "CoreUnavailable"); }
    return coreStatus;
  })().finally(() => { reconnecting = null; });
  return reconnecting;
}

function setTrayMenu() {
  tray?.setContextMenu(Menu.buildFromTemplate([
    { label: labels.showLabel, click: showWindow },
    { type: "separator" },
    { label: labels.quitLabel, click: () => app.quit() },
  ]));
  window?.setTitle(labels.windowTitle);
}

function assertSender(event) {
  if (!trustedSender(event, window, allowedOrigin)) throw new Error("UntrustedDesktopSender");
}

function registerIpc() {
  ipcMain.handle("codex:invoke", async (event, command, args = {}) => {
    assertSender(event);
    if (!args || typeof args !== "object" || Array.isArray(args)) throw new Error("InvalidArguments");
    if (command === "update_tray_labels") {
      if (!validTrayLabels(args)) throw new Error("InvalidArguments");
      labels = args; setTrayMenu(); return null;
    }
    if (!allowedCommand(command)) throw new Error("UnknownCommand");
    if (coreStatus.state === "starting" && reconnecting) await reconnecting;
    if (!core || core.state !== "ready") throw new Error("CoreDisconnected");
    if (!reconciled && command !== "load_settings") throw new Error("CoreReconciliationRequired");
    try {
      const result = await core.request(command, args);
      if (command === "load_settings" && result?.status === "ok" && result.provider_fingerprint) {
        reconciled = true; broadcast("ready");
      }
      return result;
    }
    catch (error) { throw new Error(error instanceof CoreTransportError ? error.code : "CoreCommandFailed"); }
  });
  ipcMain.handle("codex:version", (event) => { assertSender(event); return app.getVersion(); });
  ipcMain.handle("codex:core-state", (event) => { assertSender(event); return coreStatus; });
  ipcMain.handle("codex:reconnect", async (event) => { assertSender(event); return connectCore(); });
  ipcMain.handle("codex:confirm", async (event, message) => {
    assertSender(event);
    if (typeof message !== "string" || message.length > 16_384) throw new Error("InvalidArguments");
    const result = await dialog.showMessageBox(window, {
      type: "question", title: "Codex Minus", message, buttons: ["取消 / Cancel", "确认 / Confirm"],
      defaultId: 0, cancelId: 0, noLink: true,
    });
    return result.response === 1;
  });
  ipcMain.handle("codex:update-check", async (event) => {
    assertSender(event);
    if (!app.isPackaged || testHome) throw new Error("ReviewBuildUpdatesUnavailable");
    if (preparingUpdate || preparedUpdate) throw new Error("UpdateInProgress");
    availableUpdate = null;
    availableUpdate = await checkElectronFeed(app.getVersion(), process.platform, process.arch, {
      verifyManifest: (bytes, signature) => verifyUpdateMetadata(executable(), bytes, signature),
    });
    return availableUpdate ? { version: availableUpdate.version } : null;
  });
  ipcMain.handle("codex:update-download", async (event, version) => {
    assertSender(event);
    if (!app.isPackaged || testHome || preparingUpdate) throw new Error("VerifiedUpdateRequired");
    if (preparedUpdate?.version === version) return null;
    if (!availableUpdate || version !== availableUpdate.version || preparedUpdate) throw new Error("VerifiedUpdateRequired");
    preparingUpdate = true;
    let stage;
    try {
      stage = await downloadVerifiedUpdate(availableUpdate, {
        tempRoot: tmpdir(), fetchArtifact: fetch,
        verifyArtifact: (artifact, signature, digest, size) => verifyWithCore(executable(), artifact, signature, digest, size),
        onProgress: payload => { if (!window.isDestroyed()) window.webContents.send("codex:update-progress", payload); },
      });
      if (process.platform === "darwin") await stageVerifiedMacApp(stage.artifact, stage.directory, version);
      preparedUpdate = { ...stage, mode: process.platform === "darwin" ? "mac" : "windows" };
      availableUpdate = null;
      return null;
    } catch (error) {
      if (stage) await rm(stage.directory, { recursive: true, force: true });
      throw new Error(error instanceof Error && /^Update[A-Za-z]+$/.test(error.message) ? error.message : "UpdatePreparationFailed");
    } finally { preparingUpdate = false; }
  });
  ipcMain.handle("codex:update-install", async event => {
    assertSender(event);
    if (!app.isPackaged || testHome || !preparedUpdate || preparingUpdate) throw new Error("VerifiedUpdateRequired");
    preparingUpdate = true;
    const update = preparedUpdate;
    const target = process.platform === "darwin" ? dirname(dirname(dirname(process.execPath))) : process.execPath;
    try {
      if (process.platform === "darwin" && (await inspectMacBundle(target)).id !== "fun.mjshao.codex-minus") throw new Error("InvalidUpdateIdentity");
      const receipts = join(app.getPath("userData"), "update-outcomes");
      mkdirSync(receipts, { recursive: true, mode: 0o700 });
      const commit = await launchUpdateHelper(update, target, join(receipts, `${update.stageId}.json`), executable());
      commit();
      preparedUpdate = null;
    } catch (error) {
      preparingUpdate = false;
      throw new Error(error?.message === "UpdateLocationNotWritable" ? "UpdateLocationNotWritable" : "UpdateHandoffFailed");
    }
    // Only now has an external helper started. The manager exits; that helper alone replaces it.
    setTimeout(() => app.quit(), 100);
    return null;
  });
  ipcMain.handle("codex:update-outcome", async event => {
    assertSender(event);
    return consumeUpdateOutcomes(join(app.getPath("userData"), "update-outcomes"), app.getVersion());
  });
}

async function registerAssets() {
  const dist = realpathSync(join(root, "dist"));
  await protocol.handle("codex-minus", async (request) => {
    const url = new URL(request.url);
    if (url.hostname !== "app" || request.method !== "GET") return new Response(null, { status: 403 });
    let path;
    try {
      path = realpathSync(join(dist, decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname)));
      const part = relative(dist, path);
      if (part.startsWith("..") || isAbsolute(part) || ![".html", ".js", ".css", ".svg", ".png", ".woff", ".woff2", ".ico"].includes(extname(path))) {
        return new Response(null, { status: 403 });
      }
    } catch { return new Response(null, { status: 404 }); }
    return net.fetch(pathToFileURL(path).href);
  });
}

if (!locked) {
  app.quit();
} else {
  app.on("second-instance", showWindow);
  app.on("activate", showWindow);
  app.on("window-all-closed", () => {});
  app.on("before-quit", (event) => {
    if (exitReady) return;
    event.preventDefault();
    if (quitting) return;
    quitting = true;
    void stopCore().catch(() => {}).finally(() => {
      exitReady = true;
      app.exit(0);
    });
  });
  app.whenReady().then(async () => {
    if (automatedReview && process.platform === "darwin") app.dock.hide();
    const reviewBounds = automatedReview && !headlessReview
      ? reviewWindowBounds(screen.getAllDisplays(), screen.getPrimaryDisplay().id, 1180, 820)
      : null;
    if (automatedReview && !headlessReview && !reviewBounds) throw new Error("No secondary display available for native review");
    session.defaultSession.setPermissionRequestHandler((_contents, _permission, callback) => callback(false));
    session.defaultSession.setPermissionCheckHandler(() => false);
    session.defaultSession.webRequest.onHeadersReceived((details, callback) => callback({ responseHeaders: {
      ...details.responseHeaders,
      "Content-Security-Policy": ["default-src 'self'; script-src 'self'" + (devUrl ? " 'unsafe-inline'" : "") + "; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'" + (devUrl ? " ws://127.0.0.1:1420" : "") + "; object-src 'none'; frame-src 'none'; base-uri 'none'"],
    } }));
    if (!devUrl) await registerAssets();
    const iconPath = app.isPackaged ? join(process.resourcesPath, "icon.png") : join(root, "src-tauri/icons/icon.png");
    window = new BrowserWindow({
      title: "Codex Minus", ...(reviewBounds ?? { width: 1180, height: 820 }), minWidth: 960, minHeight: 720, useContentSize: true,
      icon: existsSync(iconPath) ? iconPath : undefined, backgroundColor: "#f5f0e8", show: false, focusable: !automatedReview,
      webPreferences: { preload: join(here, "preload.cjs"), nodeIntegration: false, contextIsolation: true, sandbox: true, webSecurity: true, offscreen: headlessReview, backgroundThrottling: !headlessReview },
    });
    // Available only to the isolated main-process debugger used by the native lifecycle test.
    if (automatedReview) globalThis.__codexMinusReview = { window, app, dialog };
    window.on("close", (event) => { if (!exitReady) { event.preventDefault(); window.hide(); } });
    window.on("minimize", () => { if (process.platform === "win32") window.hide(); });
    window.webContents.on("will-navigate", (event, url) => {
      try { const target = new URL(url); if (`${target.protocol}//${target.host}` !== allowedOrigin) event.preventDefault(); }
      catch { event.preventDefault(); }
    });
    window.webContents.on("will-attach-webview", (event) => event.preventDefault());
    window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
    window.webContents.on("render-process-gone", () => { broadcast("disconnected", "RendererUnavailable"); });
    const recordReviewWindow = () => {
      if (automatedReview) {
        const bounds = window.getBounds();
        writeFileSync(join(testHome, ".electron", "review-window.json"), JSON.stringify({
          bounds,
          displayId: screen.getDisplayMatching(bounds).id,
          primaryId: screen.getPrimaryDisplay().id,
          focused: window.isFocused(),
          headless: headlessReview,
        }));
      }
    };
    window.once("ready-to-show", () => { showWindow(); recordReviewWindow(); });
    registerIpc();
    const image = nativeImage.createFromPath(iconPath);
    if (!image.isEmpty()) {
      tray = new Tray(image.resize({ width: 18, height: 18 }));
      tray.setToolTip("Codex Minus"); tray.on("click", showWindow); setTrayMenu();
    }
    Menu.setApplicationMenu(Menu.buildFromTemplate([
      ...(process.platform === "darwin" ? [{ label: "Codex Minus", submenu: [{ role: "about" }, { role: "hide" }, { role: "quit" }] }] : []),
      { role: "editMenu" }, { role: "viewMenu" }, { role: "windowMenu" },
    ]));
    void connectCore();
    await window.loadURL(devUrl || "codex-minus://app/index.html");
    if (headlessReview) recordReviewWindow();
  }).catch(() => { exitReady = true; app.exit(1); });
}
