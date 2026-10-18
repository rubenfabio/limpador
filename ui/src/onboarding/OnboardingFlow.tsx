import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
  completeOnboarding,
  type OnboardingChoices,
  type Settings,
  skipOnboarding,
} from "../core/api";
import { keys, useAnalyze } from "../core/queries";
import { Onboarding } from "./Onboarding";

/** First run and "Configurar pastas": saves the choices, then runs the first analysis. */
export function OnboardingFlow() {
  const client = useQueryClient();
  const navigate = useNavigate();
  const analyze = useAnalyze();

  const saved = (next: Settings) => {
    client.setQueryData(keys.settings, next);
    void navigate({ to: "/" });
  };
  const finish = (choices: OnboardingChoices) =>
    completeOnboarding(choices).then((next) => {
      saved(next);
      analyze.mutate();
    });
  const skip = () => skipOnboarding().then(saved);

  return <Onboarding onFinish={finish} onSkip={skip} />;
}
