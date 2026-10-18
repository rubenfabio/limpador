import { useCancelAnalysis, usePartialItems, useProgress } from "../core/queries";
import { formatBytes, t } from "../i18n";

const SHOWN = 10;

const RUNNING = new Set(["waiting", "analyzing"]);

/** Live state of each collector while an analysis runs, with a way to cancel (T-08.06). */
export function AnalysisProgress() {
  const progress = useProgress(true);
  const cancel = useCancelAnalysis();
  const collectors = Object.values(progress);
  const finished = collectors.filter((c) => !RUNNING.has(c.state)).length;
  const found = Object.values(usePartialItems(true))
    .flat()
    .filter((i) => i.risk !== "blocked")
    .sort((a, b) => b.freeableSize - a.freeableSize);
  const recoverable = found.reduce((total, i) => total + i.freeableSize, 0);

  return (
    <section className="progress" aria-labelledby="progress-title">
      <h2 id="progress-title">{t("progress.title")}</h2>
      <p data-testid="progress-overall">
        {t("progress.overall", { done: finished, total: collectors.length })}
      </p>
      <ul>
        {collectors.map((c) => (
          <li key={c.collector} data-testid={`progress-${c.collector}`}>
            {t(`collector.${c.collector}`)}: {t(`collector_state.${c.state}`)}
            {c.done !== null && c.total !== null && ` (${c.done}/${c.total})`}
          </li>
        ))}
      </ul>
      {found.length > 0 && (
        <>
          <p data-testid="progress-found">
            {t("progress.found", { count: found.length, size: formatBytes(recoverable) })}
          </p>
          <ul data-testid="progress-items">
            {found.slice(0, SHOWN).map((i) => (
              <li key={i.id}>
                {i.name} — {formatBytes(i.freeableSize)}
              </li>
            ))}
          </ul>
        </>
      )}
      <button type="button" className="secondary" onClick={() => cancel.mutate()}>
        {t("progress.cancel")}
      </button>
    </section>
  );
}
