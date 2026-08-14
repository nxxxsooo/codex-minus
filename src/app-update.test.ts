import assert from "node:assert";
import { describe, it } from "node:test";

import {
  appUpdateBanner,
  appUpdatePhaseAfterEvent,
  downloadPercent,
  type AppUpdatePhase,
} from "./app-update.ts";

describe("the update download phase reducer", () => {
  const available: AppUpdatePhase = { kind: "available", version: "0.4.9" };

  it("starts downloading with the reported size", () => {
    assert.deepEqual(appUpdatePhaseAfterEvent(available, { event: "Started", data: { contentLength: 1000 } }), {
      kind: "downloading",
      version: "0.4.9",
      received: 0,
      total: 1000,
    });
  });

  it("tolerates a start without a size", () => {
    assert.deepEqual(appUpdatePhaseAfterEvent(available, { event: "Started", data: {} }), {
      kind: "downloading",
      version: "0.4.9",
      received: 0,
      total: null,
    });
  });

  it("accumulates progress chunks", () => {
    let phase = appUpdatePhaseAfterEvent(available, { event: "Started", data: { contentLength: 1000 } });
    phase = appUpdatePhaseAfterEvent(phase, { event: "Progress", data: { chunkLength: 300 } });
    phase = appUpdatePhaseAfterEvent(phase, { event: "Progress", data: { chunkLength: 450 } });
    assert.deepEqual(phase, { kind: "downloading", version: "0.4.9", received: 750, total: 1000 });
  });

  it("moves to installing when the download finishes", () => {
    const downloading: AppUpdatePhase = { kind: "downloading", version: "0.4.9", received: 1000, total: 1000 };
    assert.deepEqual(appUpdatePhaseAfterEvent(downloading, { event: "Finished" }), {
      kind: "installing",
      version: "0.4.9",
    });
  });
});

describe("the update banner", () => {
  it("offers the install action while the update is only available", () => {
    assert.deepEqual(appUpdateBanner({ kind: "available", version: "0.4.9" }), {
      text: { key: "发现新版本 {0}，可以直接更新到这一版。", args: ["v0.4.9"] },
      action: "更新并重启",
    });
  });

  it("shows progress with a percentage when the size is known", () => {
    assert.deepEqual(appUpdateBanner({ kind: "downloading", version: "0.4.9", received: 250, total: 1000 }), {
      text: { key: "正在下载 {0} … {1}%", args: ["v0.4.9", 25] },
      action: null,
    });
  });

  it("shows indeterminate progress when the size is unknown", () => {
    assert.deepEqual(appUpdateBanner({ kind: "downloading", version: "0.4.9", received: 250, total: null }), {
      text: { key: "正在下载 {0} …", args: ["v0.4.9"] },
      action: null,
    });
  });

  it("offers no action while installing", () => {
    assert.deepEqual(appUpdateBanner({ kind: "installing", version: "0.4.9" }), {
      text: { key: "正在安装 {0}，装好后会自动重启。", args: ["v0.4.9"] },
      action: null,
    });
  });

  it("offers a retry after a failure", () => {
    assert.deepEqual(appUpdateBanner({ kind: "failed", version: "0.4.9" }), {
      text: { key: "更新到 {0} 没有成功，可以重试。", args: ["v0.4.9"] },
      action: "重试",
    });
  });
});

describe("the download percentage", () => {
  it("floors and caps the ratio", () => {
    assert.equal(downloadPercent(999, 1000), 99);
    assert.equal(downloadPercent(1001, 1000), 100);
    assert.equal(downloadPercent(0, 1000), 0);
  });

  it("is indeterminate without a positive total", () => {
    assert.equal(downloadPercent(500, null), null);
    assert.equal(downloadPercent(500, 0), null);
  });
});
