const { contextBridge, ipcRenderer } = require("electron");

// No raw ipcRenderer, event objects, fs or child-process APIs cross this bridge.
contextBridge.exposeInMainWorld("codexDesktop", {
  protocolVersion: 1,
  invoke: (command, args) => ipcRenderer.invoke("codex:invoke", command, args),
  getVersion: () => ipcRenderer.invoke("codex:version"),
  getCoreState: () => ipcRenderer.invoke("codex:core-state"),
  reconnect: () => ipcRenderer.invoke("codex:reconnect"),
  confirm: (message) => ipcRenderer.invoke("codex:confirm", message),
  checkUpdate: () => ipcRenderer.invoke("codex:update-check"),
  downloadUpdate: (version) => ipcRenderer.invoke("codex:update-download", version),
  installUpdate: () => ipcRenderer.invoke("codex:update-install"),
  updateOutcome: () => ipcRenderer.invoke("codex:update-outcome"),
  onUpdateProgress: (callback) => {
    const listener = (_event, payload) => callback(payload);
    ipcRenderer.on("codex:update-progress", listener);
    return () => ipcRenderer.removeListener("codex:update-progress", listener);
  },
  onCoreState: (callback) => {
    const listener = (_event, state) => callback(state);
    ipcRenderer.on("codex:core-state", listener);
    return () => ipcRenderer.removeListener("codex:core-state", listener);
  },
});
