import assert from "node:assert";
import { it } from "node:test";

import { defaultSettings, removeRelayProfile } from "./relay-settings.ts";

const ordinaryProfile = (id: string, baseUrl: string) => ({
  ...defaultSettings.relayProfiles[0],
  id,
  name: id,
  baseUrl,
  upstreamBaseUrl: baseUrl,
  apiKey: `key-${id}`,
  protocol: "responses" as const,
  relayMode: "official" as const,
  officialMixApiKey: true,
});

const settingsWithTwoProfiles = () => ({
  ...defaultSettings,
  relayProfiles: [
    ordinaryProfile("relay-a", "https://a.example/v1"),
    ordinaryProfile("relay-b", "https://b.example/v1"),
  ],
  activeRelayId: "relay-a",
  relayBaseUrl: "https://a.example/v1",
  relayApiKey: "key-relay-a",
});

it("removes one ordinary provider and selects the first remaining active profile", () => {
  const settings = settingsWithTwoProfiles();
  const untouched = settings.relayProfiles[1];
  const next = removeRelayProfile(settings, settings.activeRelayId);
  assert.deepEqual(next.relayProfiles, [untouched]);
  assert.equal(next.activeRelayId, untouched.id);
  assert.equal(next.relayBaseUrl, untouched.baseUrl);
});
