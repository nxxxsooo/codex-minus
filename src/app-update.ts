// Presentation rules for the in-app update banner. The shell owns the plugin calls
// (check / downloadAndInstall / relaunch); this module owns what the banner says at
// each point of that flow. It returns Chinese source keys plus interpolation args and
// the shell applies t()/tf(), keeping the module importable from node tests.

export type AppUpdatePhase =
  | { kind: "available"; version: string }
  | { kind: "downloading"; version: string; received: number; total: number | null }
  | { kind: "installing"; version: string }
  | { kind: "failed"; version: string };

// Structural mirror of the updater plugin's DownloadEvent.
export type AppUpdateDownloadEvent =
  | { event: "Started"; data: { contentLength?: number } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

export function appUpdatePhaseAfterEvent(
  phase: AppUpdatePhase,
  event: AppUpdateDownloadEvent,
): AppUpdatePhase {
  const version = phase.version;
  if (event.event === "Started") {
    return { kind: "downloading", version, received: 0, total: event.data.contentLength ?? null };
  }
  if (event.event === "Progress") {
    const received = (phase.kind === "downloading" ? phase.received : 0) + event.data.chunkLength;
    const total = phase.kind === "downloading" ? phase.total : null;
    return { kind: "downloading", version, received, total };
  }
  return { kind: "installing", version };
}

export type AppUpdateBanner = {
  text: { key: string; args: Array<string | number> };
  action: "更新并重启" | "重试" | null;
};

export function appUpdateBanner(phase: AppUpdatePhase): AppUpdateBanner {
  const version = `v${phase.version}`;
  switch (phase.kind) {
    case "available":
      return { text: { key: "发现新版本 {0}，可以直接更新到这一版。", args: [version] }, action: "更新并重启" };
    case "downloading": {
      const percent = downloadPercent(phase.received, phase.total);
      return {
        text:
          percent === null
            ? { key: "正在下载 {0} …", args: [version] }
            : { key: "正在下载 {0} … {1}%", args: [version, percent] },
        action: null,
      };
    }
    case "installing":
      return { text: { key: "正在安装 {0}，装好后会自动重启。", args: [version] }, action: null };
    case "failed":
      return { text: { key: "更新到 {0} 没有成功，可以重试。", args: [version] }, action: "重试" };
  }
}

export function downloadPercent(received: number, total: number | null): number | null {
  if (total === null || total <= 0) return null;
  return Math.min(100, Math.floor((received / total) * 100));
}
