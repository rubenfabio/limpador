import { useIsMutating } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { analyzeKey, useAnalysis, useAnalyze, useSettings, useStartDocker } from "../core/queries";
import { formatBytes, formatDateTime, t, translate } from "../i18n";
import { useSelection } from "../selection/Selection";
import { RiskBadge } from "./RiskBadge";

/** Dashboard (T-08.01): how much can be freed, how safely, and what needs attention. */
export function Painel() {
  const settings = useSettings();
  const analysis = useAnalysis();
  const analyze = useAnalyze();
  const startDocker = useStartDocker();
  const analyzing = useIsMutating({ mutationKey: analyzeKey }) > 0;
  const selection = useSelection();
  const navigate = useNavigate();

  if (settings.data?.roots.length === 0) {
    return (
      <section aria-labelledby="dashboard-title">
        <h2 id="dashboard-title">{t("home.no_roots.title")}</h2>
        <p>{t("home.no_roots.help")}</p>
        <button type="button" onClick={() => void navigate({ to: "/configurar" })}>
          {t("home.no_roots.action")}
        </button>
      </section>
    );
  }

  const view = analysis.data;
  const cleanable = view?.space.filter((i) => i.risk !== "blocked") ?? [];
  const safeIds = cleanable.filter((i) => i.risk === "safe").map((i) => i.id);
  const attention = view?.collectors.filter(
    (c) => c.state === "unavailable" || c.state === "error",
  );

  return (
    <section aria-labelledby="dashboard-title">
      <h2 id="dashboard-title">{t("home.title")}</h2>
      <p>
        {view
          ? t("dashboard.last_analysis", { when: formatDateTime(new Date(view.analyzedAt)) })
          : t("home.never_analyzed")}
      </p>
      <button type="button" onClick={() => analyze.mutate()} disabled={analyzing}>
        {t("home.analyze")}
      </button>

      {view && (
        <>
          {view.space.length === 0 && view.organization.length === 0 && (
            <p className="all-clean">{t("dashboard.all_clean")}</p>
          )}

          <dl className="totals">
            <dt>{t("home.recoverable")}</dt>
            <dd data-testid="recoverable">{formatBytes(view.totals.recoverable)}</dd>
            <dt>
              <RiskBadge risk="safe" />
            </dt>
            <dd data-testid="recoverable-safe">{formatBytes(view.totals.recoverableSafe)}</dd>
            <dt>
              <RiskBadge risk="moderate" />
            </dt>
            <dd data-testid="recoverable-moderate">
              {formatBytes(view.totals.recoverableModerate)}
            </dd>
            <dt>
              <RiskBadge risk="caution" />
            </dt>
            <dd data-testid="recoverable-caution">{formatBytes(view.totals.recoverableCaution)}</dd>
          </dl>
          <button
            type="button"
            disabled={safeIds.length === 0}
            onClick={() => {
              selection.replace(safeIds);
              void navigate({ to: "/resumo" });
            }}
          >
            {t("dashboard.clean_safe_now")}
          </button>

          {attention && attention.length > 0 && (
            <ul className="cards">
              {attention.map((c) => (
                <li key={c.id} className="card" data-testid={`collector-card-${c.id}`}>
                  <strong>{t(`collector.${c.id}`)}</strong>
                  {c.message && <p>{translate(c.message)}</p>}
                  {c.action?.key === "docker.start" && (
                    <button
                      type="button"
                      disabled={startDocker.isPending || analyzing}
                      onClick={() =>
                        startDocker.mutate(undefined, { onSuccess: () => analyze.mutate() })
                      }
                    >
                      {startDocker.isPending ? t("docker.starting") : translate(c.action)}
                    </button>
                  )}
                  {c.id === "docker" && startDocker.error && (
                    <p role="alert">{translate(startDocker.error)}</p>
                  )}
                </li>
              ))}
            </ul>
          )}

          {view.warnings.map((warning) => (
            // Stryker disable next-line StringLiteral: a chave do React só identifica a linha; mudá-la não altera o que aparece
            <p key={`${warning.key}${JSON.stringify(warning.params)}`} role="alert">
              {translate(warning)}
            </p>
          ))}

          {view.drives.length > 0 && (
            <section aria-labelledby="drives-title">
              <h3 id="drives-title">{t("dashboard.drives")}</h3>
              <ul>
                {view.drives.map((d) => (
                  <li key={d.root} data-testid={`drive-${d.root}`}>
                    {t("dashboard.drive", {
                      root: d.root,
                      used: formatBytes(d.total - d.available),
                      free: formatBytes(d.available),
                    })}
                  </li>
                ))}
              </ul>
            </section>
          )}

          {cleanable.length > 0 && (
            <section aria-labelledby="top-title">
              <h3 id="top-title">{t("dashboard.top")}</h3>
              <ol data-testid="top-items">
                {cleanable.slice(0, 5).map((item) => (
                  <li key={item.id}>
                    {item.name} — {formatBytes(item.freeableSize)} <RiskBadge risk={item.risk} />
                  </li>
                ))}
              </ol>
              <Link to="/ranking">{t("dashboard.see_ranking")}</Link>
            </section>
          )}

          {view.totals.organizationItems > 0 && (
            <p data-testid="organization-count">
              {t("dashboard.organization", { count: view.totals.organizationItems })}
            </p>
          )}
        </>
      )}
    </section>
  );
}
