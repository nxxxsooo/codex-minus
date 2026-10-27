import { Plus } from "lucide-react";
import { Button } from "./components/ui/button";
import { t } from "./i18n";
import { catalogModelDisplayName } from "./model-catalog-ui";

export function CatalogModelAdditions({ candidates, officialModels, onAdd }: {
  candidates: readonly string[];
  officialModels: readonly { slug: string; displayName: string }[];
  onAdd: (slug?: string) => void;
}) {
  return (
    <div className="catalog-list-foot"><details className="catalog-add-models">
      <summary><Plus className="h-4 w-4" />{t("添加模型")}</summary>
      <div className="catalog-candidates">
        {candidates.map((slug) => (
          <button key={slug} onClick={() => onAdd(slug)} title={slug} type="button">
            <Plus className="h-3 w-3" />{catalogModelDisplayName(slug, officialModels)}
          </button>
        ))}
        <Button onClick={() => onAdd()} size="sm" variant="secondary"><Plus className="h-4 w-4" />{t("自定义模型")}</Button>
      </div>
    </details></div>
  );
}
