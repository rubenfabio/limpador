import { Link } from "@tanstack/react-router";
import type { ItemView } from "../core/api";
import { useAnalysis } from "../core/queries";
import { formatBytes, t, translate } from "../i18n";
import { useSelection } from "../selection/Selection";

const sum = (items: ItemView[]) => items.reduce((total, i) => total + i.freeableSize, 0);

/** What a cleaning of the current selection would do, before anything runs (T-08.05). */
export function Resumo() {
  const analysis = useAnalysis();
  const selection = useSelection();
  const view = analysis.data;
  const selected = view
    ? [...view.space, ...view.organization].filter((i) => selection.isSelected(i.id))
    : [];

  if (selected.length === 0) {
    return (
      <section aria-labelledby="summary-title">
        <h2 id="summary-title">{t("summary.title")}</h2>
        <p>{t("summary.empty")}</p>
        <Link to="/ranking">{t("dashboard.see_ranking")}</Link>
      </section>
    );
  }

  const compact = selected.filter((i) => i.action === "compact");
  const direct = selected.filter((i) => i.action !== "compact" && i.destination === "direct");
  const quarantine = selected.filter(
    (i) => i.action !== "compact" && i.destination === "quarantine",
  );
  const caution = selected.filter((i) => i.risk === "caution");
  const effects = [
    ...new Map(compact.flatMap((i) => i.sideEffects).map((e) => [translate(e), e])).keys(),
  ];

  return (
    <section aria-labelledby="summary-title">
      <h2 id="summary-title">{t("summary.title")}</h2>
      <ul className="summary">
        {direct.length > 0 && (
          <li data-testid="summary-direct">
            {t("summary.direct", { count: direct.length, size: formatBytes(sum(direct)) })}
          </li>
        )}
        {quarantine.length > 0 && (
          <li data-testid="summary-quarantine">
            {t("summary.quarantine", {
              count: quarantine.length,
              size: formatBytes(sum(quarantine)),
            })}
          </li>
        )}
        {compact.length > 0 && (
          <li data-testid="summary-compact">
            {t("summary.compact", { size: formatBytes(sum(compact)) })}
            <ul>
              {effects.map((effect) => (
                <li key={effect}>{effect}</li>
              ))}
            </ul>
          </li>
        )}
      </ul>
      {caution.length > 0 && (
        <p role="alert" data-testid="summary-caution">
          {t("summary.caution", { count: caution.length })}
        </p>
      )}
      <button type="button" disabled>
        {t("summary.continue")}
      </button>
      <p className="muted">{t("summary.continue_soon")}</p>
      <button type="button" className="link" onClick={selection.clear}>
        {t("summary.clear")}
      </button>
    </section>
  );
}
