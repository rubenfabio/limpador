import { type CoreResponses, coreError } from "./core-mock";

const freshInstall = {
  onboardingDone: false,
  roots: [],
  autoCleanSafe: false,
  startWithSystem: false,
};
const configured = { ...freshInstall, onboardingDone: true, roots: ["C:\\Projetos"] };

// Reusable simulated-core responses. Analysis scenarios (typical, 10k items, collector
// error, Docker stopped) are added by the F08 tasks that introduce those screens.
export const scenarios = {
  basico: { core_version: "0.1.0", get_settings: configured },
  nucleoIndisponivel: { core_version: coreError("núcleo indisponível"), get_settings: configured },
  primeiraExecucao: {
    core_version: "0.1.0",
    get_settings: freshInstall,
    suggest_roots: [
      { path: "C:\\Projetos", projects: 12 },
      { path: "C:\\Users\\dev\\repos", projects: 2 },
    ],
    "plugin:dialog|open": "D:\\Trabalho",
    validate_root: null,
    complete_onboarding: { ...configured, roots: ["C:\\Projetos", "D:\\Trabalho"] },
    skip_onboarding: { ...freshInstall, onboardingDone: true },
    run_analysis: {
      recoverable: 0,
      recoverableSafe: 0,
      spaceItems: 0,
      organizationItems: 0,
      protectedItems: 0,
      projects: 14,
      warnings: [],
      collectors: [],
    },
  },
} satisfies Record<string, CoreResponses>;
