import assert from "node:assert/strict";
import { test } from "node:test";
import { checkElectronFeed, ELECTRON_UPDATE_FEED } from "./update-check.mjs";

test("a legacy release with no Electron feed reports no update", async () => {
  let requested;
  const result = await checkElectronFeed("0.4.18", "darwin", "arm64", { fetchFeed: async url => {
    requested = url;
    return new Response(null, { status: 404 });
  } });
  assert.equal(requested, ELECTRON_UPDATE_FEED);
  assert.equal(result, null);
});

test("a large or Tauri feed never reaches update selection", async () => {
  await assert.rejects(checkElectronFeed("0.4.18", "darwin", "arm64", { fetchFeed: async () => new Response("x".repeat(65_537)) }), /InvalidUpdateManifest/);
  await assert.rejects(checkElectronFeed("0.4.18", "darwin", "arm64", {
    fetchFeed: async () => new Response(JSON.stringify({ schemaVersion: 1, runtime: "tauri", version: "0.4.19", platforms: {} })),
    verifyManifest: async () => true,
  }), /UnsupportedUpdateRuntime/);
});

test("unsigned metadata cannot relabel a previously signed Tauri asset as Electron", async () => {
  let verified = false;
  await assert.rejects(checkElectronFeed("0.4.18", "darwin", "arm64", {
    fetchFeed: async url => new Response(url.endsWith(".sig") ? "forged-signature" : '{"runtime":"electron"}'),
    verifyManifest: async (bytes, signature) => {
      verified = true;
      assert.equal(bytes.toString(), '{"runtime":"electron"}');
      assert.equal(signature, "forged-signature");
      return false;
    },
  }), /UpdateSignatureRejected/);
  assert(verified);
});
