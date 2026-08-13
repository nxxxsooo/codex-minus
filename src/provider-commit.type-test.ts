import type {
  ProviderOwnedTopologyDraft,
  ProviderRelayProfileSource,
} from "./provider-commit";

declare const sourceProfile: ProviderRelayProfileSource;

const completeProjectedProfile: ProviderOwnedTopologyDraft["relayProfiles"][number] = {
  ...sourceProfile,
  modelInsertMode: sourceProfile.modelInsertMode ?? "patch",
};

void completeProjectedProfile;

// @ts-expect-error canonical projected profiles require modelInsertMode even when source profiles omit it
const incompleteProjectedProfile: ProviderOwnedTopologyDraft["relayProfiles"][number] = sourceProfile;

void incompleteProjectedProfile;
