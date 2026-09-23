import { emptyOfficialOverride, officialModelIsVisible, type CatalogModeValue, type CatalogOverlayDraft } from "./model-catalog-ui.ts";

const LONG_CONTEXT_WINDOW = 1_050_000;
const ELIGIBLE_SLUGS = new Set(["gpt-6-astra", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"]);

type OfficialModel = { slug: string; visible: boolean; contextWindow?: number | null };
type Input = { overlay: CatalogOverlayDraft; officialModels: readonly OfficialModel[]; mode: CatalogModeValue };

function contextRows({ overlay, officialModels, mode }: Input) {
  if (mode === "external") return [];
  const customSlugs = new Set(overlay.custom.map((row) => row.slug));
  const official = mode === "custom-only" ? [] : officialModels
    .filter((row) => !customSlugs.has(row.slug) && officialModelIsVisible(overlay, row))
    .map((row) => ({ slug: row.slug, custom: false, window: overlay.official[row.slug]?.contextWindow ?? row.contextWindow }));
  return [...official, ...overlay.custom.filter((row) => mode === "custom-only" ? row.visible : overlay.official[row.slug]?.visible ?? row.visible).map((row) => ({
    slug: row.slug, custom: true,
    window: mode === "custom-only" ? row.contextWindow : overlay.official[row.slug]?.contextWindow ?? row.contextWindow,
  }))].filter((row) => ELIGIBLE_SLUGS.has(row.slug));
}

export function longContextState(input: Input) {
  const rows = contextRows(input);
  const enabled = rows.filter((row) => row.window === LONG_CONTEXT_WINDOW).length;
  return { available: rows.length > 0, checked: rows.length > 0 && enabled === rows.length, mixed: enabled > 0 && enabled < rows.length };
}

/// Only the four visible, current OpenAI models are affected. This edits the catalog draft;
/// the existing commit owns materialization, context protection and writing the live config.
/// Disabling clears only the preset value, leaving independently edited windows alone.
export function setLongContext(input: Input, enabled: boolean): CatalogOverlayDraft {
  const rows = contextRows(input);
  const official = { ...input.overlay.official };
  const custom = input.overlay.custom.map((row) => ({ ...row }));
  for (const row of rows) {
    if (!enabled && row.window !== LONG_CONTEXT_WINDOW) continue;
    if (row.custom) {
      const item = custom.find((item) => item.slug === row.slug)!;
      item.contextWindow = enabled ? LONG_CONTEXT_WINDOW
        : input.officialModels.find((model) => model.slug === row.slug)?.contextWindow ?? 272_000;
      // In a composed catalog an official override takes precedence over a same-slug custom row.
      if (input.mode !== "custom-only" && official[row.slug]?.contextWindow != null) {
        official[row.slug] = { ...official[row.slug], contextWindow: enabled ? LONG_CONTEXT_WINDOW : null };
      }
    } else {
      const next = { ...(official[row.slug] ?? emptyOfficialOverride()), contextWindow: enabled ? LONG_CONTEXT_WINDOW : null };
      if (Object.values(next).every((value) => value === null)) delete official[row.slug];
      else official[row.slug] = next;
    }
  }
  return rows.length ? { official, custom } : input.overlay;
}
