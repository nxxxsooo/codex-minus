import type {
  BackendSettings,
  ProviderCommitResult,
  SettingsResult,
} from "./backend-types.ts";

export type SettingsBaselineEpochState = Readonly<{
  baselineEpoch: number;
  latestReadRevision: number;
}>;

export type SettingsReadRequest = Readonly<{
  revision: number;
  baselineEpochAtStart: number;
}>;

export type SettingsReadRegistration = Readonly<{
  state: SettingsBaselineEpochState;
  request: SettingsReadRequest;
}>;

export type LegacyModelResetNoticeState = ReadonlySet<string>;

export function createSettingsBaselineEpochState(): SettingsBaselineEpochState {
  return { baselineEpoch: 0, latestReadRevision: 0 };
}

export function registerSettingsRead(
  state: SettingsBaselineEpochState,
): SettingsReadRegistration {
  const revision = state.latestReadRevision + 1;
  return {
    state: { ...state, latestReadRevision: revision },
    request: { revision, baselineEpochAtStart: state.baselineEpoch },
  };
}

export function advanceSettingsBaselineEpoch(
  state: SettingsBaselineEpochState,
): SettingsBaselineEpochState {
  return { ...state, baselineEpoch: state.baselineEpoch + 1 };
}

export function settingsReadResponseCanAdopt(
  request: SettingsReadRequest,
  state: SettingsBaselineEpochState,
): boolean {
  return request.revision === state.latestReadRevision
    && request.baselineEpochAtStart === state.baselineEpoch;
}

export function createLegacyModelResetNoticeState(): LegacyModelResetNoticeState {
  return new Set<string>();
}

export function consumeLegacyModelResetNotice(
  state: LegacyModelResetNoticeState,
  value: string | null | undefined,
): { state: LegacyModelResetNoticeState; notice: string | null } {
  const notice = value?.trim() ?? "";
  if (!notice || state.has(notice)) return { state, notice: null };
  const next = new Set(state);
  next.add(notice);
  return { state: next, notice };
}

export function settingsBaselineFromProviderCommit(
  result: ProviderCommitResult,
  settings: BackendSettings,
  priorBaseline: SettingsResult | null,
): SettingsResult {
  return {
    status: result.status,
    message: result.message,
    settings,
    settings_path: priorBaseline?.settings_path ?? "",
    user_scripts: priorBaseline?.user_scripts ?? {},
    provider_fingerprint: result.providerFingerprint,
  };
}
