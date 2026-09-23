const MAX_MANIFEST_BYTES = 64 * 1024;
const MAX_ARTIFACT_BYTES = 2 * 1024 * 1024 * 1024;
const ASSETS = Object.freeze({
  "darwin-aarch64": version => `CodexMinus_${version}_aarch64.app.tar.gz`,
  "windows-x86_64": version => `CodexMinus_${version}_x64-setup.exe`,
  "windows-aarch64": version => `CodexMinus_${version}_arm64-setup.exe`,
});

function invalid() { throw new Error("InvalidUpdateManifest"); }
function objectWithKeys(value, keys) {
  return value && typeof value === "object" && !Array.isArray(value)
    && Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key));
}

function versionParts(version) {
  if (typeof version !== "string" || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) invalid();
  const parts = version.split(".").map(Number);
  if (parts.some(part => !Number.isSafeInteger(part))) invalid();
  return parts;
}

function signatureLooksComplete(value) {
  if (typeof value !== "string" || value.length < 100 || value.length > 4096 || !/^[A-Za-z0-9+/]+={0,2}$/.test(value)) return false;
  const decoded = Buffer.from(value, "base64");
  if (decoded.toString("base64") !== value) return false;
  const lines = decoded.toString("utf8").trimEnd().split("\n");
  return lines.length === 4 && lines[0].startsWith("untrusted comment: ")
    && /^[A-Za-z0-9+/]+={0,2}$/.test(lines[1]) && lines[1].length >= 80
    && lines[2].startsWith("trusted comment: ")
    && /^[A-Za-z0-9+/]+={0,2}$/.test(lines[3]) && lines[3].length >= 80;
}

export function selectElectronUpdate(text, currentVersion, platform, arch) {
  if (typeof text !== "string" || Buffer.byteLength(text, "utf8") > MAX_MANIFEST_BYTES) invalid();
  let feed;
  try { feed = JSON.parse(text); } catch { invalid(); }
  if (!objectWithKeys(feed, ["schemaVersion", "runtime", "version", "platforms"]) || feed.schemaVersion !== 1) invalid();
  if (feed.runtime !== "electron") throw new Error("UnsupportedUpdateRuntime");
  const next = versionParts(feed.version);
  const current = versionParts(currentVersion);
  const target = platform === "darwin" && arch === "arm64" ? "darwin-aarch64"
    : platform === "win32" && arch === "x64" ? "windows-x86_64"
      : platform === "win32" && arch === "arm64" ? "windows-aarch64" : null;
  if (!target) throw new Error("UnsupportedUpdateTarget");
  if (!objectWithKeys(feed.platforms, Object.keys(ASSETS))) invalid();
  for (const [name, filename] of Object.entries(ASSETS)) {
    const entry = feed.platforms[name];
    if (!objectWithKeys(entry, ["url", "signature", "size", "sha256"])) invalid();
    const url = `https://github.com/nxxxsooo/codex-minus/releases/download/v${feed.version}/${filename(feed.version)}`;
    if (entry.url !== url || !signatureLooksComplete(entry.signature)
      || !Number.isSafeInteger(entry.size) || entry.size < 1 || entry.size > MAX_ARTIFACT_BYTES
      || typeof entry.sha256 !== "string" || !/^[a-f0-9]{64}$/.test(entry.sha256)) invalid();
  }
  const order = next.findIndex((part, index) => part !== current[index]);
  if (order < 0) return null;
  if (next[order] < current[order]) throw new Error("UpdateDowngrade");
  return { version: feed.version, platform: target, ...feed.platforms[target] };
}
