import { typicalAnalysis } from "../../src/test/fixtures";
import { type CoreResponses, coreError } from "./core-mock";

const freshInstall = {
  onboardingDone: false,
  roots: [],
  autoCleanSafe: false,
  startWithSystem: false,
};
const configured = { ...freshInstall, onboardingDone: true, roots: ["C:\\Projetos"] };

// Reusable simulated-core responses. Screen details (10k items, collector errors, Docker
// stopped) are covered by the Vitest screen tests; e2e keeps the journeys.
export const scenarios = {
  basico: { core_version: "0.1.0", get_settings: configured, last_analysis: null },
  nucleoIndisponivel: {
    core_version: coreError("núcleo indisponível"),
    get_settings: configured,
    last_analysis: null,
  },
  analiseTipica: {
    core_version: "0.1.0",
    get_settings: configured,
    last_analysis: typicalAnalysis(),
  },
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
    last_analysis: null,
    analyze: typicalAnalysis(),
  },
} satisfies Record<string, CoreResponses>;
