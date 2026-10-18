// Analysis results shaped like src-tauri/src/view.rs, for screen tests.
import type { AnalysisView, Category, ItemView, Risk } from "../core/api";

export const GB = 1024 ** 3;

let counter = 0;

export function item(overrides: Partial<ItemView> & { name: string }): ItemView {
  counter += 1;
  return {
    id: `id-${counter}-${overrides.name}`,
    kind: "cache",
    collector: "teste",
    category: "dependencies" as Category,
    resource: `C:\\Projetos\\${overrides.name}`,
    isPath: true,
    apparentSize: GB,
    freeableSize: GB,
    sizeEstimated: false,
    risk: "safe" as Risk,
    nature: "recreatable",
    reasons: [{ key: "reason.image_dangling", params: {} }],
    recreate: null,
    destination: "direct",
    automationEligible: true,
    protectedBy: null,
    blockReason: null,
    sideEffects: [],
    alerts: [],
    action: "remove",
    project: null,
    projectState: null,
    lastActivity: null,
    contains: [],
    parts: [],
    ...overrides,
  };
}

export function analysis(overrides: Partial<AnalysisView> = {}): AnalysisView {
  const space = overrides.space ?? [];
  const sumRisk = (risk: Risk) =>
    space.filter((i) => i.risk === risk).reduce((t, i) => t + i.freeableSize, 0);
  return {
    analyzedAt: new Date(2026, 9, 17, 9, 5).getTime(),
    durationMs: 1500,
    cancelled: false,
    totals: {
      recoverable: sumRisk("safe") + sumRisk("moderate") + sumRisk("caution"),
      recoverableSafe: sumRisk("safe"),
      recoverableModerate: sumRisk("moderate"),
      recoverableCaution: sumRisk("caution"),
      byCategory: [],
      organizationItems: overrides.organization?.length ?? 0,
    },
    space,
    organization: [],
    protected: [],
    collectors: [],
    projects: [],
    warnings: [],
    drives: [],
    ...overrides,
  };
}

/** The "análise típica" of T-08.01: 12 GB 🟢, 20 GB 🟡, 3 GB 🔴 and a blocked item. */
export function typicalAnalysis(): AnalysisView {
  return analysis({
    space: [
      item({ name: "npm-cache", freeableSize: 12 * GB, risk: "safe" }),
      item({
        name: "node_modules-loja",
        freeableSize: 20 * GB,
        risk: "moderate",
        project: "C:\\Projetos\\loja",
        projectState: "inactive",
      }),
      item({ name: "volume-db", freeableSize: 3 * GB, risk: "caution", destination: "quarantine" }),
      item({ name: "em-uso", freeableSize: 50 * GB, risk: "blocked", destination: null }),
    ],
  });
}
