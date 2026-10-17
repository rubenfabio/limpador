import { useEffect, useState } from "react";
import {
  type OnboardingChoices,
  pickFolder,
  type RootSuggestion,
  suggestRoots,
  validateRoot,
} from "../core/api";
import { t } from "../i18n";

type Step = "welcome" | "roots" | "autonomy" | "background";

interface Props {
  onFinish: (choices: OnboardingChoices) => void;
  onSkip: () => void;
}

/** First run (T-02.01): roots, autonomy and background, then the first analysis. */
export function Onboarding({ onFinish, onSkip }: Props) {
  const [step, setStep] = useState<Step>("welcome");
  const [suggestions, setSuggestions] = useState<RootSuggestion[]>();
  const [selected, setSelected] = useState<string[]>([]);
  const [manual, setManual] = useState<string[]>([]);
  const [error, setError] = useState<string>();
  const [autoCleanSafe, setAutoCleanSafe] = useState(true);
  const [startWithSystem, setStartWithSystem] = useState(true);

  useEffect(() => {
    suggestRoots()
      .then((found) => {
        setSuggestions(found);
        setSelected(found.map((s) => s.path));
      })
      .catch(() => setSuggestions([]));
  }, []);

  const toggle = (path: string) =>
    setSelected((current) =>
      current.includes(path) ? current.filter((p) => p !== path) : [...current, path],
    );

  const addFolder = async () => {
    const path = await pickFolder();
    if (!path) return;
    try {
      await validateRoot(path);
      setError(undefined);
      setManual((current) => (current.includes(path) ? current : [...current, path]));
      setSelected((current) => (current.includes(path) ? current : [...current, path]));
    } catch (key) {
      setError(String(key));
    }
  };

  const skip = (
    <button type="button" className="link" onClick={onSkip}>
      {t("onboarding.skip")}
    </button>
  );

  if (step === "welcome") {
    return (
      <section aria-labelledby="onboarding-title">
        <h2 id="onboarding-title">{t("onboarding.welcome.title")}</h2>
        <p>{t("onboarding.welcome.what")}</p>
        <p>{t("onboarding.welcome.safe")}</p>
        <button type="button" onClick={() => setStep("roots")}>
          {t("onboarding.start")}
        </button>
        {skip}
      </section>
    );
  }

  if (step === "roots") {
    return (
      <section aria-labelledby="onboarding-title">
        <h2 id="onboarding-title">{t("onboarding.roots.title")}</h2>
        <p>{t("onboarding.roots.help")}</p>
        {suggestions === undefined && <p>{t("onboarding.roots.searching")}</p>}
        {suggestions?.length === 0 && <p>{t("onboarding.roots.none_found")}</p>}
        <ul className="choices">
          {suggestions?.map((s) => (
            <li key={s.path}>
              <label>
                <input
                  type="checkbox"
                  checked={selected.includes(s.path)}
                  onChange={() => toggle(s.path)}
                />
                <span className="path">{s.path}</span>
                <span className="muted">
                  {t("onboarding.roots.projects", { count: s.projects })}
                </span>
              </label>
            </li>
          ))}
          {manual.map((path) => (
            <li key={path}>
              <label>
                <input
                  type="checkbox"
                  checked={selected.includes(path)}
                  onChange={() => toggle(path)}
                />
                <span className="path">{path}</span>
              </label>
            </li>
          ))}
        </ul>
        <button type="button" className="secondary" onClick={addFolder}>
          {t("onboarding.roots.add")}
        </button>
        {error && <p role="alert">{t(error)}</p>}
        <div className="actions">
          <button type="button" onClick={() => setStep("autonomy")}>
            {t("onboarding.next")}
          </button>
          {skip}
        </div>
      </section>
    );
  }

  if (step === "autonomy") {
    return (
      <section aria-labelledby="onboarding-title">
        <h2 id="onboarding-title">{t("onboarding.autonomy.title")}</h2>
        <p>{t("onboarding.autonomy.help")}</p>
        <label>
          <input
            type="checkbox"
            checked={autoCleanSafe}
            onChange={(e) => setAutoCleanSafe(e.target.checked)}
          />
          {t("onboarding.autonomy.option")}
        </label>
        <div className="actions">
          <button type="button" onClick={() => setStep("background")}>
            {t("onboarding.next")}
          </button>
          {skip}
        </div>
      </section>
    );
  }

  return (
    <section aria-labelledby="onboarding-title">
      <h2 id="onboarding-title">{t("onboarding.background.title")}</h2>
      <p>{t("onboarding.background.help")}</p>
      <label>
        <input
          type="checkbox"
          checked={startWithSystem}
          onChange={(e) => setStartWithSystem(e.target.checked)}
        />
        {t("onboarding.background.option")}
      </label>
      <div className="actions">
        <button
          type="button"
          onClick={() => onFinish({ roots: selected, autoCleanSafe, startWithSystem })}
        >
          {t("onboarding.finish")}
        </button>
        {skip}
      </div>
    </section>
  );
}
