import type { AnalysisSummary, Settings } from "../core/api";
import { formatBytes, t, translate } from "../i18n";

interface Props {
  settings: Settings;
  analysis?: AnalysisSummary;
  analyzing: boolean;
  onAnalyze: () => void;
  onConfigure: () => void;
}

/** Home until the full dashboard (T-08.01): empty states and the last analysis totals. */
export function Home({ settings, analysis, analyzing, onAnalyze, onConfigure }: Props) {
  if (settings.roots.length === 0) {
    return (
      <section aria-labelledby="home-title">
        <h2 id="home-title">{t("home.no_roots.title")}</h2>
        <p>{t("home.no_roots.help")}</p>
        <button type="button" onClick={onConfigure}>
          {t("home.no_roots.action")}
        </button>
      </section>
    );
  }

  return (
    <section aria-labelledby="home-title">
      <h2 id="home-title">{t("home.title")}</h2>
      {analyzing && <p role="status">{t("home.analyzing")}</p>}
      {!analyzing && !analysis && <p>{t("home.never_analyzed")}</p>}
      {analysis && (
        <dl className="totals">
          <dt>{t("home.recoverable")}</dt>
          <dd data-testid="recoverable">{formatBytes(analysis.recoverable)}</dd>
          <dt>{t("home.recoverable_safe")}</dt>
          <dd data-testid="recoverable-safe">{formatBytes(analysis.recoverableSafe)}</dd>
          <dt>{t("home.projects")}</dt>
          <dd data-testid="projects">{t("home.projects_count", { count: analysis.projects })}</dd>
        </dl>
      )}
      {analysis?.warnings.map((warning) => (
        // Stryker disable next-line StringLiteral: a chave do React só identifica a linha; mudá-la não altera o que aparece
        <p key={`${warning.key}${JSON.stringify(warning.params)}`} role="alert">
          {translate(warning)}
        </p>
      ))}
      <button type="button" onClick={onAnalyze} disabled={analyzing}>
        {t("home.analyze")}
      </button>
    </section>
  );
}
