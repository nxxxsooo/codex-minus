// Real Electron/Chromium -> preload -> IPC -> Rust acceptance harness. Uses only built-in Node
// APIs and a disposable home; no live credentials or application configuration are copied.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { createRequire } from "node:module";
import { createServer } from "node:net";
import { createServer as createHttpServer } from "node:http";
import { mkdir, mkdtemp, readFile, writeFile, rm, stat } from "node:fs/promises";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { DatabaseSync } from "node:sqlite";

const require = createRequire(import.meta.url);
const root = resolve(new URL("..", import.meta.url).pathname);
const appVersion = JSON.parse(await readFile(join(root, "package.json"), "utf8")).version;
const output = process.env.CODEX_MINUS_SMOKE_OUTPUT;
const keep = process.argv.includes("--keep");
const headless = process.argv.includes("--headless");
if (process.platform === "darwin" && !headless) {
  const { stdout } = await promisify(execFile)("system_profiler", ["SPDisplaysDataType", "-json"]);
  const displays = JSON.parse(stdout).SPDisplaysDataType.flatMap(gpu => gpu.spdisplays_ndrvs ?? []);
  if (!displays.some(display => display.spdisplays_online === "spdisplays_yes" && display.spdisplays_main !== "spdisplays_yes" && display.spdisplays_mirror === "spdisplays_off")) {
    throw new Error("Native Electron review skipped: no connected secondary display; run headless checks instead.");
  }
}
const cache = join(homedir(), ".cache");
await mkdir(cache, { recursive: true });
// A short path also keeps the legacy Unix activation socket below the platform path limit.
const home = await mkdtemp(join(cache, "cm-ui-"));
await mkdir(join(home, ".codex"));
await mkdir(join(home, ".electron"));
const auth = JSON.stringify({ auth_mode: "apikey", OPENAI_API_KEY: "official-auth-sentinel" });
const protectedContext = '\n[mcp_servers.smoke]\ncommand = "context-sentinel"\n\n[skills]\nconfig = []\n\n[plugins]\nsmoke = true\n';
const initialConfig = 'model = "gpt-5.6-terra"\n' + protectedContext;
await writeFile(join(home, ".codex", "auth.json"), auth, { mode: 0o600 });
await writeFile(join(home, ".codex", "config.toml"), initialConfig, { mode: 0o600 });
await writeFile(join(home, "external-catalog.json"), '{"sentinel":"external-owner"}\n');
const dbPath = join(home, ".codex", "state_5.sqlite");
const db = new DatabaseSync(dbPath);
db.exec("CREATE TABLE threads(id TEXT PRIMARY KEY, title TEXT, cwd TEXT, model_provider TEXT, archived INTEGER, updated_at_ms INTEGER, rollout_path TEXT)");
await mkdir(join(home, ".codex", "sessions"));
await mkdir(join(home, ".codex", "archived_sessions"));
const sessionFixtures = [
  { id: "00000000-0000-4000-8000-000000000001", title: "测试活动会话 A", archived: false },
  { id: "00000000-0000-4000-8000-000000000002", title: "测试活动会话 B", archived: false },
  { id: "00000000-0000-4000-8000-000000000003", title: "测试归档会话 A", archived: true },
  { id: "00000000-0000-4000-8000-000000000004", title: "测试归档会话 B", archived: true },
];
for (const [index, session] of sessionFixtures.entries()) {
  const path = join(home, ".codex", session.archived ? "archived_sessions" : "sessions", `${session.id}.jsonl`);
  await writeFile(path, `{"id":"${session.id}"}\n`);
  db.prepare("INSERT INTO threads VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)").run(session.id, session.title, `/fixture/${index}`, "openai", session.archived ? 1 : 0, 1_800_000_000_000 - index, path);
}
db.close();

async function freePort() {
  const server = createServer();
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  return port;
}

const port = await freePort();
const mainPort = await freePort();
const guardPort = await freePort();
const env = { ...process.env, CODEX_MINUS_TEST_HOME: home, CODEX_MINUS_AUTOMATION_REVIEW: "1", CODEX_PLUS_MANAGER_GUARD_PORT: String(guardPort) };
if (headless) env.CODEX_MINUS_HEADLESS_REVIEW = "1";
else delete env.CODEX_MINUS_HEADLESS_REVIEW;
for (const key of ["ELECTRON_RUN_AS_NODE", "CODEX_PLUS_GUARD_PORT", "CODEX_MINUS_DEV_URL", "OPENAI_API_KEY", "OPENAI_BASE_URL"]) delete env[key];
const binary = process.env.CODEX_MINUS_SMOKE_APP || require("electron");
const appArgs = process.env.CODEX_MINUS_SMOKE_APP ? [] : [root];
const launchedAt = performance.now();
const processHandle = spawn(binary, [...appArgs, `--inspect=127.0.0.1:${mainPort}`, `--remote-debugging-port=${port}`, "--remote-debugging-address=127.0.0.1"], { env, stdio: ["ignore", "ignore", "pipe"] });
let stderr = "";
processHandle.stderr.on("data", data => { stderr = (stderr + data).slice(-16_384); });

class CDP {
  pending = new Map();
  sequence = 0;
  events = [];
  constructor(socket) {
    this.socket = socket;
    socket.addEventListener("message", event => {
      const message = JSON.parse(event.data);
      if (!message.id) { this.events.push(message); return; }
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id); clearTimeout(pending.timer);
      if (message.error) pending.reject(new Error(message.error.message));
      else pending.resolve(message.result);
    });
  }
  async send(method, params = {}) {
    const id = ++this.sequence;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { this.pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15_000);
      this.pending.set(id, { resolve, reject, timer });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async evaluate(expression) {
    const response = await this.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (response.exceptionDetails) throw new Error(response.exceptionDetails.text);
    return response.result.value;
  }
  async until(expression, timeout = 20_000) {
    const deadline = Date.now() + timeout;
    while (Date.now() < deadline) {
      if (await this.evaluate(expression)) return;
      if (processHandle.exitCode !== null) throw new Error("Electron exited before the condition became true");
      await delay(100);
    }
    throw new Error(`UI condition timed out: ${expression}`);
  }
  async click(expression) {
    const point = await this.evaluate(`(async()=>{const el=(${expression});if(!el)throw new Error('Control missing');el.scrollIntoView({block:'nearest',behavior:'instant'});await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const r=el.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);
    await this.send("Input.dispatchMouseEvent", { type: "mousePressed", ...point, button: "left", clickCount: 1 });
    await this.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...point, button: "left", clickCount: 1 });
    await this.evaluate("new Promise(r=>requestAnimationFrame(r))");
  }
  async fill(selector, value) {
    await this.click(`document.querySelector(${JSON.stringify(selector)})`);
    await this.evaluate(`document.querySelector(${JSON.stringify(selector)}).select()`);
    await this.send("Input.insertText", { text: value });
    await this.until(`document.querySelector(${JSON.stringify(selector)}).value===${JSON.stringify(value)}`);
  }
  async screenshot(name) {
    if (!output) return;
    await mkdir(output, { recursive: true });
    const result = await this.send("Page.captureScreenshot", { format: "png" });
    await writeFile(join(output, name + ".png"), Buffer.from(result.data, "base64"));
  }
}

let cdp;
let mainCdp;
try {
  let target;
  for (let i = 0; i < 200; i++) {
    if (processHandle.exitCode !== null) throw new Error(`Electron startup failed (${processHandle.exitCode}): ${stderr}`);
    try {
      const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      target = targets.find(item => item.type === "page" && item.url.startsWith("codex-minus://app/"));
      if (target) break;
    } catch {}
    await delay(100);
  }
  assert(target, "Electron renderer debug target is available");
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.addEventListener("open", resolve, { once: true }); socket.addEventListener("error", reject, { once: true }); });
  cdp = new CDP(socket);
  await cdp.send("Runtime.enable");
  await cdp.send("Page.enable");
  await cdp.until("!!window.codexDesktop && !!document.querySelector('.relay-add-row button')");
  await cdp.until("window.codexDesktop.getCoreState().then(s=>s.state==='ready')");
  const readyMs = Math.round(performance.now() - launchedAt);
  let mainTarget;
  for (let i = 0; i < 100; i++) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${mainPort}/json/list`)).json();
      mainTarget = targets.find(item => item.webSocketDebuggerUrl);
      if (mainTarget) break;
    } catch {}
    await delay(100);
  }
  assert(mainTarget, "Main-process lifecycle can be inspected in the isolated test instance");
  const mainSocket = new WebSocket(mainTarget.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { mainSocket.addEventListener("open", resolve, { once: true }); mainSocket.addEventListener("error", reject, { once: true }); });
  mainCdp = new CDP(mainSocket);
  await mainCdp.send("Runtime.enable");
  const windowState = JSON.parse(await readFile(join(home, ".electron", "review-window.json"), "utf8"));
  if (headless) assert.equal(windowState.headless, true, "Headless review never shows a native window");
  else assert.notEqual(windowState.displayId, windowState.primaryId, "Automated review runs only on the secondary display");
  assert.equal(windowState.focused, false, "Automated review does not steal keyboard focus");
  const runtime = await cdp.evaluate("({node:typeof require,process:typeof process,protocol:location.protocol,version:window.codexDesktop.protocolVersion})");
  assert.deepEqual(runtime, { node: "undefined", process: "undefined", protocol: "codex-minus:", version: 1 });
  const unauthorized = await cdp.evaluate("window.codexDesktop.invoke('save_relay_file',{}).then(()=>false,e=>e.message.includes('UnknownCommand'))");
  assert(unauthorized, "Removed write bypass remains unreachable from the renderer");
  await cdp.screenshot("providers-initial");
  console.log(JSON.stringify({ phase: "runtime-ready", home, port, pid: processHandle.pid, runtime, readyMs, packaged: !!process.env.CODEX_MINUS_SMOKE_APP }));

  await cdp.until("!document.querySelector('.relay-add-row button').disabled");
  await cdp.click("document.querySelector('.relay-add-row button')");
  await cdp.until("!!document.querySelector('#provider-name')");
  await cdp.fill("#provider-name", "Electron 验证供应商");
  // A still-unsaved provider remains brand-new after structured fields materialize TOML.
  await cdp.fill("#provider-base-url", "https://relay.example.com/v1");
  await cdp.fill("#provider-api-key", "fixture-provider-key");
  await cdp.click("document.querySelectorAll('.relay-target-option input')[1]");
  await cdp.until("document.querySelectorAll('.relay-target-option input')[1].checked");
  assert.equal(await cdp.evaluate("document.querySelector('#provider-base-url').value"), "https://relay.example.com/v1");
  assert.equal(await cdp.evaluate("document.querySelector('#provider-api-key').value"), "fixture-provider-key");
  await cdp.click("document.querySelectorAll('.relay-target-option input')[0]");
  await cdp.until("document.querySelectorAll('.relay-target-option input')[0].checked");
  assert(await cdp.evaluate("[...document.querySelectorAll('.catalog-model-row')].some(e=>e.textContent.includes('gpt-5.6-terra'))"));
  await cdp.click("document.querySelectorAll('.relay-target-option input')[1]");
  await cdp.until("document.querySelectorAll('.relay-target-option input')[1].checked");
  // The current preset survives the mixed -> custom-only target change, with real display names.
  const presetIds = ["gpt-5.6-terra", "gpt-6-astra", "gpt-6-sol", "gpt-6-luna"].sort();
  const modelRows = "[...document.querySelectorAll('.catalog-model-row:not(.catalog-model-row-head)')]";
  const visibleIds = `${modelRows}.map(e=>e.querySelector('input[placeholder="model-id"]')?.value || e.querySelector('small')?.textContent).sort()`;
  assert.deepEqual(await cdp.evaluate(visibleIds), presetIds);
  assert(await cdp.evaluate(`${modelRows}.every(e=>e.querySelector('input[placeholder="显示名"]').value!==e.querySelector('input[placeholder="model-id"]').value)`));
  assert(await cdp.evaluate("!document.querySelector('.catalog-add-models').open"));
  // Delete a current model, restore the preset through its real UI action, then enable long context.
  await cdp.click(`${modelRows}.find(e=>e.querySelector('input[placeholder="model-id"]').value==='gpt-6-luna').querySelector('button[title="删除模型"]')`);
  await cdp.until(`${modelRows}.length===3`);
  await cdp.click("[...document.querySelectorAll('.catalog-editor-actions button')].find(b=>b.textContent.includes('还原 Pro 列表'))");
  await cdp.until(`${modelRows}.length===4`);
  assert.deepEqual(await cdp.evaluate(visibleIds), presetIds);
  await cdp.click("document.querySelector('.catalog-long-context')");
  await cdp.until("document.querySelector('.catalog-long-context input').checked");
  assert(await cdp.evaluate(`${modelRows}.every(e=>e.querySelector('input[inputmode="numeric"]').value==='1050000')`));
  await cdp.screenshot("provider-create");
  const save = "document.querySelector('.relay-detail-sticky button[aria-busy]')";
  await cdp.until(`!(${save}).disabled`);
  await cdp.click(save);
  await cdp.until("!document.querySelector('#provider-name') || !!document.querySelector('.toast-card.failed')");
  const error = await cdp.evaluate("document.querySelector('.toast-card.failed')?.innerText || null");
  assert.equal(error, null, `First provider save failed: ${error}`);
  const persisted = JSON.parse(await readFile(join(home, ".codex-session-delete", "settings.json"), "utf8"));
  const profile = persisted.relayProfiles.find(item => item.name === "Electron 验证供应商");
  assert(profile, "Provider saved through the actual renderer and Rust command");
  assert(profile.configContents.includes("image_generation = true"), "The pure-API target enables the explicitly owned image-tool leaf");
  const catalogState = JSON.parse(await readFile(join(home, ".codex-session-delete", "model-catalog-state.json"), "utf8"));
  const savedCatalog = catalogState.profiles[profile.id];
  assert.equal(savedCatalog.mode, "custom-only");
  assert.deepEqual(savedCatalog.overlay.custom.map(row=>row.slug).sort(), presetIds);
  assert(savedCatalog.overlay.custom.every(row=>row.displayName!==row.slug && row.contextWindow===1050000));
  const generated = JSON.parse(await readFile(join(home, ".codex", savedCatalog.generatedPath), "utf8"));
  assert.deepEqual(generated.models.map(row=>row.slug).sort(), presetIds);
  assert(generated.models.every(row=>row.context_window===1050000 && row.max_context_window===1050000));
  assert.equal(generated.models.find(row=>row.slug==='gpt-6-sol').display_name, 'GPT-6 Sol');
  assert.equal(generated.models.find(row=>row.slug==='gpt-6-luna').display_name, 'GPT-6 Luna');
  assert.equal(await readFile(join(home, ".codex", "config.toml"), "utf8"), initialConfig, "Inactive save leaves live config untouched");
  await cdp.screenshot("provider-saved");
  console.log(JSON.stringify({ phase: "provider-saved", profileId: profile.id }));

  const editor = "[...document.querySelectorAll('.relay-profile-card')].find(e=>e.textContent.includes('Electron 验证供应商')).querySelector('button[title=\"编辑\"]')";
  await cdp.click(editor);
  await cdp.until("!!document.querySelector('#provider-name')");
  await cdp.until("!!document.querySelector('.catalog-model-list')");
  assert.deepEqual(await cdp.evaluate(visibleIds), presetIds, "The saved four-model list reopens unchanged");
  assert(await cdp.evaluate("document.querySelector('.catalog-long-context input').checked"));
  await cdp.fill("#provider-name", "Electron 日常开发");
  const activate = "[...document.querySelectorAll('.relay-editor-head button')].find(b=>b.textContent.includes('设为当前'))";
  await cdp.until(`!!(${activate}) && !(${activate}).disabled`);
  await cdp.click(activate);
  await cdp.until("document.querySelector('.relay-editor-head')?.innerText.includes('使用中')");
  const savedSettings = async () => JSON.parse(await readFile(join(home, ".codex-session-delete", "settings.json"), "utf8"));
  const active = await savedSettings();
  assert.equal(active.activeRelayId, profile.id);
  assert.equal(active.relayProfiles.find(p=>p.id===profile.id).name, "Electron 日常开发", "Set current atomically saves the dirty draft");
  const activeConfig = await readFile(join(home, ".codex", "config.toml"), "utf8");
  assert(activeConfig.includes('requires_openai_auth = false'));
  assert(activeConfig.includes('experimental_bearer_token = "fixture-provider-key"'));
  assert(activeConfig.includes('image_generation = true'));
  assert(activeConfig.includes(protectedContext.trim()), "Context tables survive activation");
  assert.equal(await readFile(join(home, ".codex", "auth.json"), "utf8"), auth);
  await cdp.until("document.querySelector('[data-live-config]')?.value.includes('image_generation = true')");
  const livePanel = await cdp.evaluate("({readOnly:document.querySelector('[data-live-config]').readOnly,text:document.querySelector('[data-live-config]').value})");
  assert.equal(livePanel.readOnly, true);
  assert(!livePanel.text.includes("fixture-provider-key"), "Live config cannot reveal the provider bearer");
  assert(livePanel.text.includes("mcp_servers.smoke"), "Protected Context remains visible without secrets");
  assert.equal(await readFile(join(home, "external-catalog.json"), "utf8"), '{"sentinel":"external-owner"}\n');
  await cdp.screenshot("provider-current");
  console.log(JSON.stringify({ phase: "dirty-draft-activated", contextPreserved: true, authUnchanged: true }));

  // Provider Doctor must keep model discovery and text Responses evidence independent. This
  // controlled fixture returns no model list but a valid text response, never reaching a real API.
  const doctorRequests = [];
  let doctorResponseMode = "discovery-empty";
  const doctorServer = createHttpServer((request, response) => {
    doctorRequests.push({ method: request.method, url: request.url });
    const body = request.method === "GET"
      ? doctorResponseMode === "discovery-empty" ? '{"data":[]}' : '{"data":[{"id":"gpt-5.6-terra"}]}'
      : doctorResponseMode === "discovery-empty" ? '{"id":"doctor-text-ok","status":"completed","output":[]}'
        : '{"error":{"message":"fixture-provider-key rejected"}}';
    response.writeHead(request.method === "POST" && doctorResponseMode === "request-rejected" ? 401 : 200, { "content-type": "application/json" });
    response.end(body);
  });
  await new Promise(resolve => doctorServer.listen(0, "127.0.0.1", resolve));
  const doctorBaseUrl = `http://127.0.0.1:${doctorServer.address().port}/v1`;
  try {
    await cdp.fill("#provider-base-url", doctorBaseUrl);
    await cdp.click("[...document.querySelectorAll('.provider-doctor-head button')][0]");
    await cdp.until("!!document.querySelector('.provider-doctor-modal')");
    await cdp.until("!!document.querySelector('.provider-doctor-modal [data-step-id=request].ok')");
    const doctorSurface = await cdp.evaluate("({steps:[...document.querySelectorAll('.provider-doctor-modal .provider-doctor-step')].map(row=>({id:row.dataset.stepId,state:row.className,detail:row.textContent})),text:document.querySelector('.provider-doctor-modal').innerText})");
    assert.equal(doctorSurface.steps.length, 4);
    assert(doctorSurface.steps[0].state.includes("ok"));
    assert(doctorSurface.steps[1].state.includes("failed"), "Empty model discovery is its own failed row");
    assert(doctorSurface.steps[2].state.includes("ok"), "A successful Responses request stays successful");
    assert.equal(doctorSurface.steps[3].id, "recommendation");
    assert(!doctorSurface.text.includes("fixture-provider-key"), "Doctor never renders the provider bearer");
    assert(doctorRequests.some(request => request.method === "GET"));
    assert(doctorRequests.some(request => request.method === "POST"));
    assert(await cdp.evaluate("document.querySelector('.provider-doctor-progress').getAttribute('aria-valuenow') === '100'"));
    await cdp.screenshot("provider-doctor-discovery-warning");
    await cdp.evaluate("document.querySelector('.provider-doctor-modal .modal-actions button').click()");
    await cdp.until("!document.querySelector('.provider-doctor-modal')");
    doctorResponseMode = "request-rejected";
    await cdp.click("[...document.querySelectorAll('.provider-doctor-head button')][0]");
    await cdp.until("!!document.querySelector('.provider-doctor-modal [data-step-id=request].failed')");
    const failedDoctor = await cdp.evaluate("({steps:[...document.querySelectorAll('.provider-doctor-modal .provider-doctor-step')].map(row=>({id:row.dataset.stepId,state:row.className})),text:document.querySelector('.provider-doctor-modal').innerText})");
    assert(failedDoctor.steps.find(step => step.id === "models")?.state.includes("ok"));
    assert(failedDoctor.steps.find(step => step.id === "request")?.state.includes("failed"));
    assert(!failedDoctor.text.includes("fixture-provider-key"), "A rejected request must not expose credentials");
    assert.match(failedDoctor.text, /Responses|Key/);
    await cdp.screenshot("provider-doctor-request-rejected");
    await cdp.evaluate("document.querySelector('.provider-doctor-modal .modal-actions button').click()");
    await cdp.until("!document.querySelector('.provider-doctor-modal')");
  } finally {
    await new Promise(resolve => doctorServer.close(resolve));
  }
  await cdp.fill("#provider-base-url", "https://relay.example.com/v1");
  console.log(JSON.stringify({ phase: "provider-doctor-rows", discoveryFailed: true, textSucceeded: true, requestRejected: true, bearerHidden: true }));

  // The already-saved image tool can be independently disabled through the focused provider
  // transform and commit. The manager still owns only that feature leaf and cannot rewrite auth.
  const imageSwitch = "[...document.querySelectorAll('.switch-row')].find(row=>row.textContent.includes('图像工具')).querySelector('input')";
  assert.equal(await cdp.evaluate(`(${imageSwitch}).disabled`), false, "Saved managed profile image tool must be editable");
  await cdp.evaluate(`(${imageSwitch}).click()`);
  await cdp.until(`!(${imageSwitch}).checked`);
  await cdp.until(`!(${save}).disabled`);
  await cdp.click(save);
  await cdp.until("!document.querySelector('#provider-name')");
  const disabledImageConfig = await readFile(join(home, ".codex", "config.toml"), "utf8");
  assert.match(disabledImageConfig, /image_generation = false/);
  assert(disabledImageConfig.includes(protectedContext.trim()));
  assert.equal(await readFile(join(home, ".codex", "auth.json"), "utf8"), auth);
  await cdp.click("[...document.querySelectorAll('.relay-profile-card')].find(e=>e.textContent.includes('Electron 日常开发')).querySelector('button[title=编辑]')");
  await cdp.until("!!document.querySelector('#provider-name')");
  assert.equal(await cdp.evaluate(`(${imageSwitch}).checked`), false);
  assert(await cdp.evaluate("document.querySelector('.relay-field-mode')?.textContent.includes('图像工具已关闭')"), "Pure API copy must reflect the disabled image tool");
  await cdp.evaluate(`(${imageSwitch}).click()`);
  await cdp.until(`(${imageSwitch}).checked`);
  await cdp.until(`!(${save}).disabled`);
  await cdp.click(save);
  await cdp.until("!document.querySelector('#provider-name')");
  const enabledImageConfig = await readFile(join(home, ".codex", "config.toml"), "utf8");
  assert.match(enabledImageConfig, /image_generation = true/);
  assert(enabledImageConfig.includes(protectedContext.trim()));
  assert.equal(await readFile(join(home, ".codex", "auth.json"), "utf8"), auth);
  await cdp.click("[...document.querySelectorAll('.relay-profile-card')].find(e=>e.textContent.includes('Electron 日常开发')).querySelector('button[title=编辑]')");
  await cdp.until("!!document.querySelector('#provider-name')");
  console.log(JSON.stringify({ phase: "provider-image-leaf", disabledAndReenabled: true, contextPreserved: true, authUnchanged: true }));

  // A real killed child must retain the draft, disable writes, and permit explicit recovery.
  await cdp.fill("#provider-name", "断线时的未保存草稿");
  const { stdout: childList } = await promisify(execFile)("ps", ["-axo", "pid=,ppid=,comm="]);
  const corePid = childList.split("\n").map(line=>line.trim().split(/\s+/)).find(parts=>Number(parts[1])===processHandle.pid && parts.slice(2).join(" ").endsWith("codex-minus-core"))?.[0];
  assert(corePid, "Find only this fixture's owned core process");
  process.kill(Number(corePid), "SIGKILL");
  await cdp.until("document.querySelector('.desktop-connection-notice.failed') !== null");
  assert.equal(await cdp.evaluate("document.querySelector('#provider-name').value"), "断线时的未保存草稿");
  assert(await cdp.evaluate(`(${save}).disabled`));
  await cdp.evaluate("document.querySelector('.provider-doctor-head button').click()");
  await cdp.until("!!document.querySelector('.provider-doctor-modal') && !document.querySelector('.provider-doctor-modal .modal-actions button').disabled");
  assert(await cdp.evaluate("document.querySelector('.provider-doctor-modal .modal-head')?.textContent.includes('诊断未完成')"));
  await cdp.evaluate("document.querySelector('.provider-doctor-modal .modal-actions button').click()");
  await cdp.until("!document.querySelector('.provider-doctor-modal')");
  await cdp.screenshot("provider-disconnected");
  await cdp.click("document.querySelector('.desktop-connection-notice button')");
  await cdp.until("window.codexDesktop.getCoreState().then(s=>s.state==='ready')");
  assert.equal(await cdp.evaluate("document.querySelector('#provider-name').value"), "断线时的未保存草稿");
  await cdp.until(`!(${save}).disabled`);
  await cdp.click(save);
  await cdp.until("!document.querySelector('#provider-name')");
  assert.equal((await savedSettings()).relayProfiles.find(p=>p.id===profile.id).name, "断线时的未保存草稿");
  console.log(JSON.stringify({ phase: "core-kill-reconnect-saved", draftRetained: true }));

  await cdp.click("[...document.querySelectorAll('.relay-profile-card')].find(e=>e.textContent.includes('断线时的未保存草稿')).querySelector('button[title=\"编辑\"]')");
  await cdp.until("!!document.querySelector('#provider-name')");
  await cdp.fill("#provider-name", "冲突时的本地草稿");
  const changed = await savedSettings();
  changed.relayProfiles.find(p=>p.id===profile.id).name = "来自另一个配置编辑器";
  await writeFile(join(home, ".codex-session-delete", "settings.json"), JSON.stringify(changed), { mode: 0o600 });
  await cdp.click(save);
  await cdp.until("document.querySelector('.toast-card')?.innerText.includes('保存') && document.querySelector('#provider-name') && !document.querySelector('.relay-detail-sticky button[aria-busy]').disabled");
  assert.equal((await savedSettings()).relayProfiles.find(p=>p.id===profile.id).name, "来自另一个配置编辑器", "Stale save must not overwrite a new generation");
  assert.equal(await cdp.evaluate("document.querySelector('#provider-name').value"), "冲突时的本地草稿");
  await cdp.screenshot("provider-conflict");
  console.log(JSON.stringify({ phase: "stale-save-rejected", draftRetained: true }));

  // Re-read through the actual navigation action, then intentionally save the retained draft.
  await cdp.click("[...document.querySelectorAll('.topbar-actions button')].find(button=>button.title.includes('刷新当前页面') || button.title.includes('Refresh current'))");
  // The existing refresh button has no busy state; let the settings/catalog reads settle.
  await delay(1000);
  await cdp.until(`!(${save}).disabled`);
  await cdp.fill("#provider-name", "Electron 日常开发");
  await cdp.click(save);
  await cdp.until("!document.querySelector('#provider-name')");

  // Disabling routing still permits saving an existing profile without a live write.
  await cdp.click("document.querySelector('.relay-master-switch input')");
  await cdp.until("!document.querySelector('.relay-master-switch input').checked");
  await cdp.click("[...document.querySelectorAll('.relay-profile-card')].find(e=>e.textContent.includes('Electron 日常开发')).querySelector('button[title=\"编辑\"]')");
  await cdp.until("!!document.querySelector('#provider-name')");
  const beforeDisabledSave = await readFile(join(home, ".codex", "config.toml"), "utf8");
  await cdp.fill("#provider-base-url", "https://edited.example.com/v1");
  await cdp.click(save);
  await cdp.until("!document.querySelector('#provider-name')");
  assert.equal(await readFile(join(home, ".codex", "config.toml"), "utf8"), beforeDisabledSave);
  assert((await savedSettings()).relayProfiles.find(p=>p.id===profile.id).configContents.includes('base_url = "https://edited.example.com/v1"'));
  console.log(JSON.stringify({ phase: "routing-disabled-save", liveConfigUnchanged: true }));

  await cdp.click("[...document.querySelectorAll('.relay-profile-card')].find(e=>e.textContent.includes('Electron 日常开发')).querySelector('button[title=\"编辑\"]')");
  await cdp.until("!!document.querySelector('#provider-name')");
  for (const [width, height, theme, language] of [[1180,820,"light","zh"],[960,720,"light","zh"],[960,720,"dark","en"]]) {
    await cdp.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
    await cdp.until(`innerWidth===${width} && innerHeight===${height}`);
    await cdp.evaluate("document.fonts.ready.then(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))");
    await cdp.evaluate(`localStorage.setItem('codex-plus-theme',${JSON.stringify(theme)});localStorage.setItem('codex-plus-lang',${JSON.stringify(language)})`);
    // Theme changes through the visible toggle; language reload also exercises backend reuse.
    if (theme === "dark") {
      await cdp.send("Page.reload");
      await cdp.until("!!document.querySelector('.relay-profile-card') && !document.querySelector('.desktop-connection-notice')");
      await cdp.click("[...document.querySelectorAll('.relay-profile-card')].find(e=>e.textContent.includes('Electron 日常开发')).querySelectorAll('.relay-card-extra button')[1]");
      await cdp.until("!!document.querySelector('#provider-name')");
    }
    const viewport = await cdp.evaluate(`({innerWidth,scrollWidth:document.documentElement.scrollWidth,bodyWidth:document.body.getBoundingClientRect().width,shellWidth:document.querySelector('.shell').getBoundingClientRect().width})`);
    assert(viewport.scrollWidth <= viewport.innerWidth, `No viewport horizontal overflow at ${width}×${height}: ${JSON.stringify(viewport)}`);
    assert(await cdp.evaluate(`(()=>{const r=(${save}).getBoundingClientRect();return r.top>=0&&r.bottom<=innerHeight&&r.right<=innerWidth})()`), "Save stays reachable");
    await cdp.screenshot(`provider-${width}-${theme}-${language}`);
  }

  // Exercise the merged permanent-cleanup route with real SQLite + rollout fixtures.
  await cdp.click("[...document.querySelectorAll('.nav-item')].find(e=>e.textContent.includes('Session management'))");
  await cdp.until("document.querySelector('.sessions-list-panel')?.textContent.includes('测试活动会话 A')");
  assert.equal(await cdp.evaluate("document.querySelectorAll('.session-row').length"), 2);
  const sessionLayout = await cdp.evaluate(`[...document.querySelectorAll('.session-row')].map(row=>{
    const actions=row.querySelector('.session-row-actions').getBoundingClientRect();
    const updated=row.querySelector('.session-updated').getBoundingClientRect();
    return { updatedRight:updated.right, actionsLeft:actions.left, actionsRight:actions.right,
      buttons:[...row.querySelectorAll('.session-row-actions button')].map(button=>({text:button.textContent,left:button.getBoundingClientRect().left,right:button.getBoundingClientRect().right})) };
  })`);
  assert(sessionLayout.every(row=>row.buttons.every(button=>button.left>=row.actionsLeft-1 && button.right<=row.actionsRight+1 && button.left>=row.updatedRight+4)),
    `Session row actions must fit without overlapping the update column at 960px: ${JSON.stringify(sessionLayout)}`);
  assert(await cdp.evaluate(`[...document.querySelectorAll('.session-row-actions button')].every(button => button.scrollWidth <= button.clientWidth + 1 && button.scrollHeight <= button.clientHeight + 1)`),
    "Session row action labels must fit inside their controls");
  await cdp.screenshot("sessions-active");
  for (const [theme, language] of [["light", "zh"], ["light", "en"]]) {
    await cdp.evaluate(`localStorage.setItem('codex-plus-theme',${JSON.stringify(theme)});localStorage.setItem('codex-plus-lang',${JSON.stringify(language)})`);
    await cdp.send("Page.reload");
    await cdp.until("document.querySelectorAll('.nav-item').length >= 2 && !document.querySelector('.desktop-connection-notice')");
    await cdp.click("document.querySelectorAll('.nav-item')[1]");
    await cdp.until("document.querySelector('.sessions-list-panel')?.textContent.includes('测试活动会话 A') && !document.querySelector('.desktop-connection-notice')");
    assert(await cdp.evaluate("document.documentElement.scrollWidth <= innerWidth"), "Session workspace fits the minimum viewport");
    assert(await cdp.evaluate(`[...document.querySelectorAll('.session-row-actions button')].every(button => button.scrollWidth <= button.clientWidth + 1 && button.scrollHeight <= button.clientHeight + 1)`),
      "Session actions fit in both languages at the minimum viewport");
    await cdp.screenshot(`sessions-active-960-${theme}-${language}`);
  }
  await cdp.evaluate("[...document.querySelectorAll('.session-list-actions button')].find(e=>e.textContent.includes('多选') || e.textContent.includes('Multi-select')).click()");
  await cdp.until("document.querySelectorAll('.session-select input').length===2");
  await cdp.evaluate("document.querySelector('.session-select input').click()");
  await cdp.until("document.querySelector('.session-selection-summary')?.textContent.includes('1')");
  await cdp.screenshot("sessions-selection");
  await cdp.evaluate("[...document.querySelectorAll('.session-list-actions button')].find(e=>e.textContent.includes('取消') || e.textContent.includes('Cancel')).click()");
  await cdp.evaluate("document.querySelectorAll('.session-view-tabs button')[1].click()");
  await cdp.until("document.querySelector('.sessions-list-panel')?.textContent.includes('测试归档会话 A')");
  assert.equal(await cdp.evaluate("document.querySelectorAll('.session-row').length"), 2);
  await cdp.screenshot("sessions-archived");
  // A previewed deletion can be cancelled without touching the selected archived session.
  const deleteTrigger = "document.querySelector('.sessions-list-panel .session-row-actions .session-delete-button')";
  await cdp.evaluate(`(${deleteTrigger}).focus()`);
  assert(await cdp.evaluate(`${deleteTrigger} === document.activeElement`));
  await cdp.evaluate(`(${deleteTrigger}).click()`);
  await cdp.until("document.querySelector('.modal-card')?.textContent.includes('Delete permanently') || document.querySelector('.modal-card')?.textContent.includes('永久删除')");
  assert(await cdp.evaluate("document.querySelector('[role=dialog][aria-labelledby=confirm-dialog-title]').contains(document.activeElement)"));
  await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Escape", code: "Escape" });
  await cdp.until("!document.querySelector('.modal-card')");
  await cdp.until(`${deleteTrigger} === document.activeElement`, 2_000);
  const afterCancel = new DatabaseSync(dbPath, { readOnly: true });
  assert.equal(afterCancel.prepare("SELECT COUNT(*) AS total FROM threads WHERE archived = 1").get().total, 2);
  afterCancel.close();
  const first = sessionFixtures.find(s => s.archived);
  assert(await readFile(join(home, ".codex", "archived_sessions", `${first.id}.jsonl`), "utf8"));
  // The UI's single-delete path must remove only the selected archived record, not its neighbour.
  await cdp.evaluate("[...document.querySelectorAll('.session-row-actions .session-delete-button')][0].click()");
  await cdp.until("!!document.querySelector('.modal-card')");
  await cdp.evaluate("document.querySelector('.modal-card .toolbar button').click()");
  await cdp.until("document.querySelectorAll('.session-row').length===1");
  const checkDb = new DatabaseSync(dbPath, { readOnly: true });
  assert.equal(checkDb.prepare("SELECT COUNT(*) AS remaining FROM threads WHERE id = ?").get(first.id).remaining, 0);
  checkDb.close();
  await assert.rejects(readFile(join(home, ".codex", "archived_sessions", `${first.id}.jsonl`)), { code: "ENOENT" });
  await assert.rejects(stat(join(home, ".codex-session-delete", "backups")), { code: "ENOENT" });
  // Direct IPC against the isolated fixture proves the new command is reachable and the retired
  // backup-creating entry point is not. The UI confirmation path remains explicitly user-owned.
  const retiredDelete = await cdp.evaluate("window.codexDesktop.invoke('delete_local_session',{}).then(()=>false,e=>e.message.includes('UnknownCommand'))");
  assert(retiredDelete);
  await cdp.evaluate("[...document.querySelectorAll('.session-list-actions button')].find(e=>e.textContent.includes('清空全部归档') || e.textContent.includes('Clear all archives')).click()");
  await cdp.until("document.querySelector('.modal-card')?.textContent.includes('Delete permanently') || document.querySelector('.modal-card')?.textContent.includes('永久删除')");
  await cdp.screenshot("sessions-clear-all-confirmation");
  await cdp.evaluate("document.querySelector('.modal-card .toolbar button').click()");
  await cdp.until("document.querySelectorAll('.session-row').length===0");
  const afterArchiveClear = new DatabaseSync(dbPath, { readOnly: true });
  assert.deepEqual(afterArchiveClear.prepare("SELECT archived, COUNT(*) AS total FROM threads GROUP BY archived").all().map(row=>({ archived: row.archived, total: row.total })), [{ archived: 0, total: 2 }]);
  afterArchiveClear.close();
  await assert.rejects(stat(join(home, ".codex-session-delete", "backups")), { code: "ENOENT" });
  console.log(JSON.stringify({ phase: "session-cleanup-integration", singleArchivedDeleted: true, clearAllArchived: true, activeUntouched: true, noBackups: true, retiredBackupRouteRejected: true }));

  // Adaptation stays active-only and uses a fresh scan generation. A stale scan may not write.
  const activeRollout = join(home, ".codex", "sessions", `${sessionFixtures[0].id}.jsonl`);
  const otherActiveRollout = join(home, ".codex", "sessions", `${sessionFixtures[1].id}.jsonl`);
  const archivedRollout = join(home, ".codex", "archived_sessions", "unrelated-history-sentinel.jsonl");
  await writeFile(archivedRollout, '{"type":"session_meta","payload":{"model_provider":"old-provider"}}\n');
  const archivedBeforeAdapt = await readFile(archivedRollout);
  const activeBeforeAdapt = await readFile(otherActiveRollout);
  const beforeAdapt = new DatabaseSync(dbPath);
  beforeAdapt.prepare("UPDATE threads SET model_provider = ? WHERE id = ?").run("old-provider", sessionFixtures[0].id);
  beforeAdapt.close();
  await writeFile(activeRollout, '{"type":"session_meta","payload":{"model_provider":"old-provider"}}\n');
  const staleAdaptation = await cdp.evaluate("window.codexDesktop.invoke('adapt_active_sessions_to_current_provider', {scanGeneration:'expired-fixture'} )");
  assert.equal(staleAdaptation.status, "failed");
  assert.equal(await readFile(activeRollout, "utf8"), '{"type":"session_meta","payload":{"model_provider":"old-provider"}}\n');
  const scan = await cdp.evaluate("window.codexDesktop.invoke('scan_provider_compatibility')");
  assert.equal(scan.status, "ok");
  assert.equal(scan.mismatchCount, 1);
  assert.equal(scan.activeCount, 2);
  const adapted = await cdp.evaluate(`window.codexDesktop.invoke('adapt_active_sessions_to_current_provider', {scanGeneration:${JSON.stringify(scan.scanGeneration)}})`);
  assert.equal(adapted.status, "ok");
  assert.equal(adapted.mismatchCount, 0);
  const afterAdapt = new DatabaseSync(dbPath, { readOnly: true });
  assert.equal(afterAdapt.prepare("SELECT model_provider FROM threads WHERE id = ?").get(sessionFixtures[0].id).model_provider, scan.currentProvider);
  afterAdapt.close();
  assert.equal(JSON.parse((await readFile(activeRollout, "utf8")).trim()).payload.model_provider, scan.currentProvider);
  assert.deepEqual(await readFile(otherActiveRollout), activeBeforeAdapt);
  assert.deepEqual(await readFile(archivedRollout), archivedBeforeAdapt);
  console.log(JSON.stringify({ phase: "session-provider-adaptation", staleRejected: true, activeRewritten: true, archivedUntouched: true }));

  // Point only the disposable settings home at a fixture CLI. It simulates the target's native
  // archive state transition so the actual Electron -> Rust command and postcondition can run.
  const fakeApp = join(home, "Codex Fixture.app");
  const fakeCliDir = join(fakeApp, "Contents", "Resources");
  await mkdir(fakeCliDir, { recursive: true });
  await writeFile(join(fakeCliDir, "codex"), `#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const { DatabaseSync } = require('node:sqlite');
const [operation, id] = process.argv.slice(2);
if (id === '--help' && ['archive', 'unarchive'].includes(operation)) process.exit(0);
const home = process.env.CODEX_HOME;
if (!home || !id || !['archive', 'unarchive'].includes(operation)) process.exit(2);
const db = new DatabaseSync(path.join(home, 'state_5.sqlite'));
const row = db.prepare('SELECT rollout_path FROM threads WHERE id = ?').get(id);
if (!row) process.exit(3);
const target = path.join(home, operation === 'archive' ? 'archived_sessions' : 'sessions', path.basename(row.rollout_path));
fs.renameSync(row.rollout_path, target);
db.prepare('UPDATE threads SET archived = ?, rollout_path = ? WHERE id = ?').run(operation === 'archive' ? 1 : 0, target, id);
db.close();
`, { mode: 0o700 });
  const cliSettingsPath = join(home, ".codex-session-delete", "settings.json");
  const cliSettings = JSON.parse(await readFile(cliSettingsPath, "utf8"));
  cliSettings.codexAppPath = fakeApp;
  await writeFile(cliSettingsPath, JSON.stringify(cliSettings), { mode: 0o600 });
  // Policy verification uses the same explicit fixture CLI, including on a clean CI host.
  await cdp.evaluate("document.querySelectorAll('.session-view-tabs button')[0].click()");
  await cdp.until("document.querySelector('.sessions-list-panel')?.textContent.includes('测试活动会话 A')");
  await cdp.click("[...document.querySelectorAll('.topbar-actions button')].find(button=>button.title.includes('刷新当前页面') || button.title.includes('Refresh current'))");
  const policySwitch = ".session-status-grid .session-status-panel:first-child .session-policy-toggle input";
  const beforePolicyRequest = await cdp.evaluate("window.codexDesktop.invoke('preview_session_archive', {request:{retentionDays:30}})");
  assert.equal(beforePolicyRequest.status, "ok");
  assert.equal(beforePolicyRequest.capability.cliPath, join(fakeCliDir, "codex"));
  // Await a controlled native-dialog response through the real IPC caller, without opening UI.
  await mainCdp.evaluate(`(()=>{
    const {dialog}=globalThis.__codexMinusReview;
    const original=dialog.showMessageBox;
    globalThis.__codexMinusDialogTest={waiting:false,cancelled:false};
    dialog.showMessageBox=()=>new Promise(resolve=>{
      globalThis.__codexMinusDialogTest.waiting=true;
      globalThis.__codexMinusDialogTest.cancel=()=>{dialog.showMessageBox=original;globalThis.__codexMinusDialogTest.cancelled=true;resolve({response:0,checkboxChecked:false});};
    });
  })()`);
  await cdp.until(`!document.querySelector(${JSON.stringify(policySwitch)}).disabled`);
  await cdp.evaluate(`document.querySelector(${JSON.stringify(policySwitch)}).click()`);
  await mainCdp.until("globalThis.__codexMinusDialogTest.waiting === true");
  const beforeConsent = await cdp.evaluate("window.codexDesktop.invoke('load_session_lifecycle_settings')");
  assert.equal(beforeConsent.archiveEnabled, false);
  assert.equal(beforeConsent.firstRunReviewed, false);
  await mainCdp.evaluate("globalThis.__codexMinusDialogTest.cancel()");
  await cdp.until("document.querySelector('.session-policy-toggle input')?.checked === false");
  const afterCancelPolicy = await cdp.evaluate("window.codexDesktop.invoke('load_session_lifecycle_settings')");
  assert.equal(afterCancelPolicy.archiveEnabled, false);
  assert.equal(await mainCdp.evaluate("globalThis.__codexMinusDialogTest.cancelled"), true);
  console.log(JSON.stringify({ phase: "session-policy-preview", candidateReadOnly: true, nativeBoundaryCancellationAwaited: true, cancelPreserved: true }));

  const archivePreview = await cdp.evaluate("window.codexDesktop.invoke('preview_session_archive', {request:{retentionDays:30}})");
  assert.equal(archivePreview.status, "ok");
  assert.equal(archivePreview.capability.available, true);
  assert.equal(archivePreview.capability.cliPath, join(fakeCliDir, "codex"));
  const archiveAttempt = await cdp.evaluate(`window.codexDesktop.invoke('archive_local_session', {request:{sessionId:${JSON.stringify(sessionFixtures[1].id)}}})`);
  assert.equal(archiveAttempt.status, "ok", `Isolated CLI archive failed: ${archiveAttempt.message}`);
  assert.equal(archiveAttempt.archived, true);
  const archivedPath = join(home, ".codex", "archived_sessions", `${sessionFixtures[1].id}.jsonl`);
  assert.deepEqual(await readFile(archivedPath), activeBeforeAdapt);
  await assert.rejects(readFile(otherActiveRollout), { code: "ENOENT" });
  const archivedRow = new DatabaseSync(dbPath, { readOnly: true });
  assert.equal(archivedRow.prepare("SELECT archived FROM threads WHERE id = ?").get(sessionFixtures[1].id).archived, 1);
  archivedRow.close();
  await cdp.evaluate("document.querySelectorAll('.session-view-tabs button')[0].click()");
  await cdp.until("document.querySelector('.sessions-topline [role=tab][aria-selected=true]')?.textContent.includes('Active')");
  await cdp.evaluate("document.querySelectorAll('.session-view-tabs button')[1].click()");
  await cdp.until("document.querySelector('.sessions-topline [role=tab][aria-selected=true]')?.textContent.includes('Archived') && document.querySelector('.sessions-list-panel')?.textContent.includes('测试活动会话 B')");
  const rowRestore = "document.querySelector('.sessions-list-panel .session-row-actions button:first-child')";
  assert.match(await cdp.evaluate(`(${rowRestore}).textContent`), /Restore|恢复/);
  await cdp.click(rowRestore);
  await cdp.until("document.querySelector('.sessions-empty')?.textContent.includes('No archived sessions') || document.querySelector('.sessions-empty')?.textContent.includes('没有已归档会话')");
  await delay(800);
  const archivedAfterRestore = await cdp.evaluate("window.codexDesktop.invoke('list_local_sessions', {request:{archived:true,pageSize:5}})");
  assert.equal(archivedAfterRestore.archivedCount, 0, `UI restore did not change inventory: ${JSON.stringify(archivedAfterRestore)}`);
  const restoredRow = new DatabaseSync(dbPath, { readOnly: true });
  assert.equal(restoredRow.prepare("SELECT archived FROM threads WHERE id = ?").get(sessionFixtures[1].id).archived, 0);
  restoredRow.close();
  assert.deepEqual(await readFile(otherActiveRollout), activeBeforeAdapt);
  assert.deepEqual(await readFile(archivedRollout), archivedBeforeAdapt);
  console.log(JSON.stringify({ phase: "session-native-lifecycle", archived: true, restoredThroughUi: true, archivedHistoryUntouched: true }));

  const preferencesTrigger = "document.querySelector('.topbar-actions button[title=Preferences]')";
  await cdp.click(preferencesTrigger);
  await cdp.until("!!document.querySelector('[role=dialog][aria-labelledby=preferences-title]')");
  assert(await cdp.evaluate("document.querySelector('[role=dialog][aria-labelledby=preferences-title]').contains(document.activeElement)"));
  assert(await cdp.evaluate(`document.querySelector('[role=dialog][aria-labelledby=preferences-title]').textContent.includes(${JSON.stringify(`v${appVersion}`)})`));
  assert(await cdp.evaluate("(()=>{const box=document.querySelector('.preferences-modal').getBoundingClientRect();return box.top>=0&&box.bottom<=innerHeight&&box.left>=0&&box.right<=innerWidth})()"), "Preferences remain reachable at 960 × 720");
  await cdp.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }] });
  assert(await cdp.evaluate("(()=>{const el=document.createElement('span');el.className='spin';document.body.appendChild(el);const duration=parseFloat(getComputedStyle(el).animationDuration);el.remove();return duration<=0.001})()"), "Reduced motion prevents sustained loading animations");
  await cdp.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "no-preference" }] });
  await cdp.evaluate("[...document.querySelectorAll('.preferences-modal button')].at(-1).focus()");
  await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9 });
  assert(await cdp.evaluate("document.activeElement === document.querySelector('.preferences-modal button')"), "Tab loops within the modal");
  await cdp.evaluate("[...document.querySelectorAll('.preferences-choice')].find(button=>button.textContent.includes('Dark mode')).click()");
  await cdp.until("document.querySelector('.shell.dark') && document.documentElement.classList.contains('dark') && document.querySelector('.preferences-choice[aria-pressed=true]')?.textContent.includes('Dark mode')");
  await delay(180);
  await cdp.screenshot("preferences-960-dark-en");
  await cdp.evaluate("[...document.querySelectorAll('.preferences-choice')].find(button=>button.textContent.includes('Light mode')).click()");
  await cdp.until("document.querySelector('.shell.light') && document.documentElement.classList.contains('light') && document.querySelector('.preferences-choice[aria-pressed=true]')?.textContent.includes('Light mode')");
  await delay(180);
  await cdp.screenshot("preferences-960-light-en");
  await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Escape", code: "Escape" });
  await cdp.until("!document.querySelector('[role=dialog][aria-labelledby=preferences-title]')");
  await cdp.until(`${preferencesTrigger} === document.activeElement`, 2_000);
  await cdp.click(preferencesTrigger);
  await cdp.until("!!document.querySelector('[role=dialog][aria-labelledby=preferences-title]')");
  await cdp.evaluate("[...document.querySelectorAll('.preferences-choice')].find(button=>button.textContent.includes('简体中文')).click()");
  await cdp.until("localStorage.getItem('codex-plus-lang')==='zh' && document.querySelector('.topbar h1')?.textContent.includes('供应商配置')");
  await cdp.click("document.querySelector('.topbar-actions button[title=偏好设置]')");
  await cdp.until("!!document.querySelector('.preferences-modal')");
  assert(await cdp.evaluate("document.querySelector('.preferences-modal h2')?.textContent === '偏好设置'"));
  await cdp.screenshot("preferences-960-light-zh");
  await cdp.evaluate("document.querySelector('.preferences-modal .modal-head button').click()");
  await cdp.until("!document.querySelector('.preferences-modal')");
  console.log(JSON.stringify({ phase: "preferences-dialog", opened: true, themeSwitched: true, languagePersisted: true, escapeClosed: true, focusReturned: true }));

  await cdp.click("document.querySelectorAll('.nav-item')[0]");
  await cdp.until("[...document.querySelectorAll('.relay-profile-card')].some(e=>e.textContent.includes('Electron 日常开发'))");
  await cdp.click("[...document.querySelectorAll('.relay-profile-card')].find(e=>e.textContent.includes('Electron 日常开发')).querySelectorAll('.relay-card-extra button')[1]");
  await cdp.until("document.querySelector('#provider-name')?.value === 'Electron 日常开发'");
  const mainWindow = "globalThis.__codexMinusReview.window";
  const beforeClose = await mainCdp.evaluate(`(async()=>{const w=${mainWindow};return {id:w.id,visible:w.isVisible(),focused:w.isFocused()}})()`);
  assert.equal(beforeClose.visible, !headless);
  assert.equal(beforeClose.focused, false);
  await mainCdp.evaluate(`(async()=>{${mainWindow}.close();return true})()`);
  await mainCdp.until(`(async()=>{const w=${mainWindow};return !!w && !w.isDestroyed() && !w.isVisible()})()`);
  assert.equal(await cdp.evaluate("document.querySelector('#provider-name')?.value"), "Electron 日常开发", "Close-to-hide retains the editor");
  const second = spawn(binary, appArgs, { env, stdio: ["ignore", "ignore", "pipe"] });
  const secondExit = await Promise.race([
    new Promise(resolve => second.once("exit", resolve)),
    delay(10_000).then(() => { second.kill("SIGKILL"); throw new Error("Second launch did not settle"); }),
  ]);
  assert.equal(secondExit, 0);
  await mainCdp.until(`(async()=>${mainWindow}.isVisible()===${!headless})()`);
  const afterReopen = await mainCdp.evaluate(`(async()=>{const w=${mainWindow};return {id:w.id,focused:w.isFocused()}})()`);
  assert.equal(afterReopen.id, beforeClose.id);
  assert.equal(afterReopen.focused, false, "Second launch must not take native keyboard focus during isolated review");
  const reopenBounds = await mainCdp.evaluate(`(async()=>{const w=${mainWindow};return w.getBounds()})()`);
  if (!headless) {
    assert.equal(reopenBounds.x, windowState.bounds.x);
    assert.equal(reopenBounds.y, windowState.bounds.y);
  }
  assert.equal(await cdp.evaluate("document.querySelector('#provider-name')?.value"), "Electron 日常开发");
  console.log(JSON.stringify({ phase: "close-hide-second-launch", sameWindow: true, editorRetained: true, headless, nativeShowVerified: !headless, remainedUnfocused: true }));

  const exit = new Promise(resolve => processHandle.once("exit", resolve));
  await mainCdp.evaluate("(()=>{setTimeout(()=>globalThis.__codexMinusReview.app.quit(),100);return true})()");
  mainCdp.socket.close();
  mainCdp = null;
  const exitCode = await Promise.race([
    exit,
    delay(18_000).then(async () => {
      const { stdout: processState } = await promisify(execFile)("ps", ["-p", String(processHandle.pid), "-o", "pid=,ppid=,stat=,comm="]).catch(() => ({ stdout: "not running" }));
      processHandle.kill("SIGKILL");
      throw new Error(`App did not quit after core drain: ${processState.trim()}`);
    }),
  ]);
  assert.equal(exitCode, 0);
  console.log(JSON.stringify({ phase: "explicit-quit", exited: true }));

  if (output) await writeFile(join(output, headless ? "headless-runtime-evidence.json" : "runtime-evidence.json"), JSON.stringify({ phase: "provider-and-session-workflow-passed", home, pid: processHandle.pid, debugPort: port, profileId: profile.id, runtime, headless, readyMs, packaged: !!process.env.CODEX_MINUS_SMOKE_APP, scenarios: ["current-preset-names","delete-restore","long-context-materialized","image-generation-leaf","catalog-reopened","inactive-save","dirty-set-current","context-auth-preservation","core-death-reconnect","stale-save","routing-disabled-save","small-window-dark-english","single-session-permanent-cleanup","all-archived-cleanup","no-backups","active-only-provider-adaptation","archive-policy-cancel","fixture-cli-archive-and-restore","close-hide-second-launch","explicit-quit"] }, null, 2));
} catch (error) {
  if (cdp) {
    await cdp.screenshot("smoke-failure").catch(() => {});
    console.error(await cdp.evaluate("document.body.innerText").catch(() => "Renderer unavailable"));
  }
  console.error(error);
  if (keep) console.log(JSON.stringify({ phase: "inspect-failure", home, port, pid: processHandle.pid }));
  process.exitCode = 1;
} finally {
  cdp?.socket.close();
  mainCdp?.socket.close();
  if (keep) {
    processHandle.stderr.removeAllListeners("data"); processHandle.stderr.unref(); processHandle.unref();
    console.log("Review window retained with isolated fixture data.");
  } else {
    processHandle.kill();
    await Promise.race([new Promise(resolve => processHandle.once("exit", resolve)), delay(15_000)]);
    if (processHandle.exitCode === null) processHandle.kill("SIGKILL");
    await rm(home, { recursive: true, force: true });
  }
}
