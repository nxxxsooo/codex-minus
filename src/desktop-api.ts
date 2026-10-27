import type { AppUpdateDownloadEvent } from "./app-update";

export type CoreConnection = { state: "starting" | "ready" | "disconnected"; code: string | null };

type DesktopBridge = {
  protocolVersion: number;
  invoke(command: string, args?: Record<string, unknown>): Promise<unknown>;
  getVersion(): Promise<string>;
  getCoreState(): Promise<CoreConnection>;
  reconnect(): Promise<CoreConnection>;
  confirm(message: string): Promise<boolean>;
  checkUpdate(): Promise<{ version: string } | null>;
  downloadUpdate(version: string): Promise<void>;
  installUpdate(): Promise<void>;
  updateOutcome(): Promise<{ status: "installed" | "unconfirmed" } | null>;
  onUpdateProgress(callback: (event: AppUpdateDownloadEvent) => void): () => void;
  onCoreState(callback: (state: CoreConnection) => void): () => void;
};

declare global {
  interface Window { codexDesktop?: DesktopBridge; }
}

function bridge(): DesktopBridge {
  if (!window.codexDesktop || window.codexDesktop.protocolVersion !== 1) {
    throw new Error("DesktopBridgeUnavailable");
  }
  return window.codexDesktop;
}

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return await bridge().invoke(command, args) as T;
}

export const getVersion = async () => bridge().getVersion();
export const getCoreState = async () => bridge().getCoreState();
export const reconnectCore = async () => bridge().reconnect();
export const subscribeCoreState = (listener: (state: CoreConnection) => void) => bridge().onCoreState(listener);
export const confirmDesktop = (message: string) => bridge().confirm(message);

export type AvailableAppUpdate = {
  version: string;
  downloadAndInstall(listener: (event: AppUpdateDownloadEvent) => void): Promise<void>;
};

export async function checkForAppUpdate(): Promise<AvailableAppUpdate | null> {
  const update = await bridge().checkUpdate();
  return update ? {
    version: update.version,
    async downloadAndInstall(listener) {
      const unsubscribe = bridge().onUpdateProgress(listener);
      try { await bridge().downloadUpdate(update.version); }
      finally { unsubscribe(); }
    },
  } : null;
}

export function relaunch(): Promise<void> {
  return bridge().installUpdate();
}

export const readUpdateOutcome = () => bridge().updateOutcome();
