import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import {
  checkTraceability,
  criterionId,
  findTestIds,
  layerOf,
  parseCriteria,
  parseTaskIndex,
  renderTraceability,
  traceRepository,
} from "./traceability.mjs";

const index = `
| ID | Tarefa | Depende de | Status |
|---|---|---|---|
| T-99.02 | [Classificação de risco](F99/T-99.02-risco.md) | T-01.01 | concluída |
| T-99.03 | [Ganho seguro](F99/T-99.03-ganho.md) | T-99.02 | pendente |
`;

function task(id, status, count) {
  return {
    id,
    title: id,
    file: `${id}.md`,
    status,
    criteria: Array.from({ length: count }, (_, i) => i + 1),
  };
}

describe("rastreabilidade", () => {
  it("reads tasks and status from the index", () => {
    expect(parseTaskIndex(index)).toEqual([
      {
        id: "T-99.02",
        title: "Classificação de risco",
        file: "F99/T-99.02-risco.md",
        status: "concluída",
      },
      { id: "T-99.03", title: "Ganho seguro", file: "F99/T-99.03-ganho.md", status: "pendente" },
    ]);
  });

  it("reads acceptance criteria from a task file", () => {
    expect(parseCriteria("- [ ] **CA1** Dado...\n- [x] **CA2** Dado...\nver CA1")).toEqual([1, 2]);
  });

  it("builds criterion ids", () => {
    expect(criterionId("T-99.02", 3)).toBe("t99_02_ca3");
  });

  it("t00_07_ca1_done_task_with_a_criterion_without_test_fails_listing_it", () => {
    const refs = [1, 2, 3, 5].map((ca) => ({ id: `t99_02_ca${ca}`, layer: "unitário" }));
    const { missing } = checkTraceability([task("T-99.02", "concluída", 5)], refs);
    expect(missing).toEqual([{ task: "T-99.02", ca: 4, id: "t99_02_ca4", layers: [] }]);
  });

  it("t00_07_ca2_pending_task_without_tests_does_not_fail", () => {
    const tasks = [task("T-99.03", "pendente", 4), task("T-99.04", "em andamento", 2)];
    expect(checkTraceability(tasks, []).missing).toEqual([]);
  });

  it("t00_07_ca3_report_shows_every_layer_covering_a_criterion", () => {
    const refs = [
      ...findTestIds({ path: "crates/core/src/risk.rs", text: "fn t99_02_ca1_x() {}" }),
      ...findTestIds({ path: "e2e-app/specs/risk.e2e.mjs", text: 'it("t99_02_ca1_y")' }),
    ];
    const result = checkTraceability([task("T-99.02", "concluída", 1)], refs);
    expect(result.report[0].criteria[0].layers).toEqual(["e2e do app real", "unitário"]);
    expect(renderTraceability(result)).toContain("- CA1 (`t99_02_ca1`): e2e do app real, unitário");
  });

  it("classifies test layers by location", () => {
    expect(layerOf("crates/core/src/lib.rs")).toBe("unitário");
    expect(layerOf("crates/testkit/tests/fake_machine.rs")).toBe("integração");
    expect(layerOf("ui/src/App.test.tsx")).toBe("unitário");
    expect(layerOf("ui/e2e/app.spec.ts")).toBe("e2e da interface");
    expect(layerOf("e2e-app/specs/app.e2e.mjs")).toBe("e2e do app real");
    expect(layerOf("tools\\run-steps.test.mjs")).toBe("verificação do repositório");
  });

  it("this repository is fully traced", () => {
    const { missing } = traceRepository(resolve(import.meta.dirname, ".."));
    expect(missing).toEqual([]);
  });
});
