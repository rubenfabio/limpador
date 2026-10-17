import { useCallback, useEffect, useState } from "react";
import {
  type AnalysisSummary,
  completeOnboarding,
  getCoreVersion,
  getSettings,
  type OnboardingChoices,
  runAnalysis,
  type Settings,
  skipOnboarding,
} from "./core/api";
import { Home } from "./home/Home";
import { t } from "./i18n";
import { Onboarding } from "./onboarding/Onboarding";

export function App() {
  const [version, setVersion] = useState<string>();
  const [settings, setSettings] = useState<Settings>();
  const [onboarding, setOnboarding] = useState(false);
  const [analysis, setAnalysis] = useState<AnalysisSummary>();
  const [analyzing, setAnalyzing] = useState(false);

  useEffect(
    () => {
      getCoreVersion()
        .then(setVersion)
        .catch((error: unknown) => console.error("Falha ao consultar a versão do núcleo", error));
      getSettings()
        .then((loaded) => {
          setSettings(loaded);
          setOnboarding(!loaded.onboardingDone);
        })
        .catch((error: unknown) => console.error("Falha ao carregar as configurações", error));
    },
    // Stryker disable next-line ArrayDeclaration: qualquer lista constante roda o efeito uma vez só, então o mutante é equivalente
    [],
  );

  const analyze = useCallback(() => {
    setAnalyzing(true);
    runAnalysis()
      .then(setAnalysis)
      .catch((error: unknown) => console.error("Falha na análise", error))
      .finally(() => setAnalyzing(false));
  }, []);

  const finish = (choices: OnboardingChoices) =>
    completeOnboarding(choices).then((saved) => {
      setSettings(saved);
      setOnboarding(false);
      analyze();
    });

  const skip = () =>
    skipOnboarding().then((saved) => {
      setSettings(saved);
      setOnboarding(false);
    });

  return (
    <main>
      <header>
        <h1>{t("app.name")}</h1>
        {version && <p data-testid="core-version">{t("app.version", { version })}</p>}
      </header>
      {settings && onboarding && <Onboarding onFinish={finish} onSkip={skip} />}
      {settings && !onboarding && (
        <Home
          settings={settings}
          analysis={analysis}
          analyzing={analyzing}
          onAnalyze={analyze}
          onConfigure={() => setOnboarding(true)}
        />
      )}
    </main>
  );
}
