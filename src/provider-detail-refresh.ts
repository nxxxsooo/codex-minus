type DraftRevision = { profileId: string; profile: string; catalog: string };

export function preserveEditedProviderDraft(
  baseline: DraftRevision | null,
  current: DraftRevision,
  incomingProfileId: string,
): boolean {
  return baseline !== null
    && baseline.profileId === incomingProfileId
    && current.profileId === incomingProfileId
    && (baseline.profile !== current.profile || baseline.catalog !== current.catalog);
}

export type ProviderDraftBaseline = DraftRevision;

type ConfirmationState = { lifecycle: string; sessionToken: unknown; pendingConfirmation: unknown };

export function providerConfirmationStillCurrent(current: ConfirmationState, shown: ConfirmationState): boolean {
  return current.lifecycle === "active"
    && current.sessionToken === shown.sessionToken
    && !!current.pendingConfirmation
    && current.pendingConfirmation === shown.pendingConfirmation;
}
