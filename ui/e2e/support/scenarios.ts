import { type CoreResponses, coreError } from "./core-mock";

// Reusable simulated-core responses. Analysis scenarios (empty, typical, 10k items,
// collector error, Docker stopped) are added by the F08 tasks that introduce those screens.
export const scenarios = {
  basico: { core_version: "0.1.0" },
  nucleoIndisponivel: { core_version: coreError("núcleo indisponível") },
} satisfies Record<string, CoreResponses>;
