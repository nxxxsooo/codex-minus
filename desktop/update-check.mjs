import { selectElectronUpdate } from "./update-manifest.mjs";

// Separate from Tauri's latest.json; older releases deliberately answer 404 here.
export const ELECTRON_UPDATE_FEED = "https://github.com/nxxxsooo/codex-minus/releases/latest/download/electron-latest.json";

async function boundedBody(response, limit) {
  if (!response.ok || !response.body || response.url && new URL(response.url).protocol !== "https:") throw new Error("UpdateFeedUnavailable");
  let bytes = 0;
  const parts = [];
  for await (const chunk of response.body) {
    bytes += chunk.byteLength;
    if (bytes > limit) throw new Error("InvalidUpdateManifest");
    parts.push(chunk);
  }
  return Buffer.concat(parts);
}

export async function checkElectronFeed(currentVersion, platform, arch, { fetchFeed = fetch, verifyManifest } = {}) {
  const signal = AbortSignal.timeout(12_000);
  const response = await fetchFeed(ELECTRON_UPDATE_FEED, { signal, cache: "no-store" });
  if (response.status === 404) return null;
  const bytes = await boundedBody(response, 64 * 1024);
  const signature = await boundedBody(await fetchFeed(ELECTRON_UPDATE_FEED + ".sig", { signal, cache: "no-store" }), 4096);
  if (!verifyManifest || await verifyManifest(bytes, signature.toString("utf8").trim()) !== true) throw new Error("UpdateSignatureRejected");
  return selectElectronUpdate(bytes.toString("utf8"), currentVersion, platform, arch);
}
