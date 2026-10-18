import { useIsMutating, useQuery } from "@tanstack/react-query";
import { Link, Outlet } from "@tanstack/react-router";
import { getCoreVersion } from "../core/api";
import { analyzeKey, useSettings } from "../core/queries";
import { t } from "../i18n";
import { OnboardingFlow } from "../onboarding/OnboardingFlow";
import { AnalysisProgress } from "../screens/AnalysisProgress";

export function Layout() {
  const version = useQuery({
    queryKey: ["core-version"],
    queryFn: () =>
      getCoreVersion().catch((error: unknown) => {
        console.error("Falha ao consultar a versão do núcleo", error);
        return null;
      }),
  });
  const settings = useSettings();
  const analyzing = useIsMutating({ mutationKey: analyzeKey }) > 0;
  const firstRun = settings.data && !settings.data.onboardingDone;

  return (
    <main>
      <header className="app-header">
        <h1>{t("app.name")}</h1>
        {version.data && (
          <p data-testid="core-version">{t("app.version", { version: version.data })}</p>
        )}
        {settings.data && !firstRun && (
          <nav aria-label={t("nav.label")}>
            <Link to="/">{t("nav.dashboard")}</Link>
            <Link to="/ranking">{t("nav.ranking")}</Link>
            <Link to="/resumo">{t("nav.summary")}</Link>
          </nav>
        )}
      </header>
      {firstRun && <OnboardingFlow />}
      {settings.data && !firstRun && (
        <>
          {analyzing && <AnalysisProgress />}
          <Outlet />
        </>
      )}
    </main>
  );
}
