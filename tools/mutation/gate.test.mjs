import { describe, expect, it } from "vitest";
import {
  evaluate,
  findUnjustifiedIgnores,
  fromCargoMutants,
  fromStryker,
  globToRegExp,
  planIncremental,
  renderSummary,
  scoreByFile,
} from "./gate.mjs";

const policy = {
  metas: { critico: 90, padrao: 70 },
  criticos: ["crates/core/src/risk/**", "src-tauri/src/sandbox.rs"],
  foraDaMedicao: ["crates/testkit/**", "ui/src/main.tsx", "**/*.test.*"],
};

/** `killed` caught mutants and `survived` missed ones in `file`. */
function mutantsOf(file, killed, survived) {
  return [
    ...Array.from({ length: killed }, (_, i) => ({
      file,
      line: i + 1,
      description: "k",
      status: "killed",
    })),
    ...Array.from({ length: survived }, (_, i) => ({
      file,
      line: 100 + i,
      description: `sobrevivente ${i}`,
      status: "survived",
    })),
  ];
}

describe("metas por criticidade", () => {
  it("t00_05_ca1_critical_module_below_90_is_blocked", () => {
    const [row] = evaluate(scoreByFile(mutantsOf("crates/core/src/risk/mod.rs", 17, 3)), policy);
    expect(row).toMatchObject({ critical: true, threshold: 90, score: 85, passed: false });
  });

  it("t00_05_ca2_common_module_at_75_passes", () => {
    const [row] = evaluate(scoreByFile(mutantsOf("crates/core/src/ports.rs", 3, 1)), policy);
    expect(row).toMatchObject({ critical: false, threshold: 70, score: 75, passed: true });
  });

  it("critical module exactly at the threshold passes", () => {
    const [row] = evaluate(scoreByFile(mutantsOf("src-tauri/src/sandbox.rs", 9, 1)), policy);
    expect(row).toMatchObject({ score: 90, passed: true });
  });

  it("timeouts count as detected and unviable/ignored mutants do not count", () => {
    const scores = scoreByFile([
      { file: "a.rs", status: "killed" },
      { file: "a.rs", status: "timeout" },
      { file: "a.rs", status: "survived" },
      { file: "a.rs", status: "unviable" },
      { file: "a.rs", status: "ignored" },
    ]);
    expect(scores.get("a.rs").score).toBe(66.6);
  });

  it("files with only unviable mutants have no score and pass", () => {
    const [row] = evaluate(scoreByFile([{ file: "a.rs", status: "unviable" }]), policy);
    expect(row).toMatchObject({ score: null, passed: true });
  });

  it("excluded files are not measured", () => {
    const rows = evaluate(scoreByFile(mutantsOf("crates/testkit/src/machine.rs", 0, 5)), policy);
    expect(rows).toEqual([]);
  });
});

describe("modo incremental", () => {
  it("t00_05_ca3_only_changed_files_are_mutated", () => {
    const plan = planIncremental(
      [
        "crates/core/src/ports.rs",
        "ui/src/App.tsx",
        "ui/src/App.test.tsx",
        "ui/src/main.tsx",
        "crates/testkit/src/machine.rs",
        "docs/visao.md",
      ],
      policy,
    );
    expect(plan).toEqual({ rust: ["crates/core/src/ports.rs"], ts: ["src/App.tsx"] });
  });

  it("windows paths are normalized", () => {
    expect(planIncremental(["crates\\core\\src\\lib.rs"], policy).rust).toEqual([
      "crates/core/src/lib.rs",
    ]);
  });
});

describe("mutantes ignorados precisam de justificativa", () => {
  it("t00_05_ca4_rust_skip_without_reason_is_invalid", () => {
    const text = "#[cfg_attr(test, mutants::skip)]\npub fn run() {}\n";
    expect(findUnjustifiedIgnores({ path: "src/lib.rs", text })).toEqual([
      { file: "src/lib.rs", line: 1, text: "#[cfg_attr(test, mutants::skip)]" },
    ]);
  });

  it("t00_05_ca4_rust_skip_with_reason_is_valid", () => {
    const text =
      "// motivo: abre a janela; coberto pelo e2e\n#[cfg_attr(test, mutants::skip)]\npub fn run() {}\n";
    expect(findUnjustifiedIgnores({ path: "src/lib.rs", text })).toEqual([]);
  });

  it("t00_05_ca4_cargo_exclusion_without_reason_is_invalid", () => {
    const text = 'exclude_re = [\n  # motivo: equivalente\n  "ok",\n  "sem motivo",\n]\n';
    const problems = findUnjustifiedIgnores({ path: ".cargo/mutants.toml", text });
    expect(problems.map((p) => p.line)).toEqual([4]);
  });

  it("t00_05_ca4_stryker_disable_without_reason_is_invalid", () => {
    const text = [
      "// Stryker disable next-line ArrayDeclaration: lista constante, mutante equivalente",
      "// Stryker disable next-line all",
      "// Stryker disable StringLiteral:",
    ].join("\n");
    const problems = findUnjustifiedIgnores({ path: "ui/src/App.tsx", text });
    expect(problems.map((p) => p.line)).toEqual([2, 3]);
  });
});

describe("relatório", () => {
  it("t00_05_ca5_summary_lists_survivors_with_file_and_line", () => {
    const mutants = mutantsOf("crates/core/src/risk/mod.rs", 17, 3);
    const summary = renderSummary(evaluate(scoreByFile(mutants), policy), mutants);

    expect(summary).toContain("**Resultado: bloqueado.**");
    expect(summary).toContain(
      "| crates/core/src/risk/mod.rs | crítico | 85.0% | 90% | 17 | 3 | ✘ bloqueou |",
    );
    expect(summary).toContain("## Sobreviventes (3)");
    expect(summary).toContain("- `crates/core/src/risk/mod.rs:100` — sobrevivente 0");
  });

  it("summary lists ignored mutants with their reason", () => {
    const mutants = [{ file: "ui/src/App.tsx", line: 9, status: "ignored", reason: "equivalente" }];
    expect(renderSummary([], mutants)).toContain("- `ui/src/App.tsx:9` — equivalente");
  });
});

describe("leitura dos relatórios", () => {
  it("reads cargo-mutants outcomes", () => {
    const outcomes = {
      outcomes: [
        { scenario: "Baseline", summary: "Success" },
        {
          scenario: {
            Mutant: {
              name: 'src-tauri/src/lib.rs:9:5: replace core_version with ""',
              file: "src-tauri\\src\\lib.rs",
              span: { start: { line: 9, column: 5 } },
            },
          },
          summary: "MissedMutant",
        },
      ],
    };
    expect(fromCargoMutants(outcomes)).toEqual([
      {
        file: "src-tauri/src/lib.rs",
        line: 9,
        description: 'replace core_version with ""',
        status: "survived",
      },
    ]);
  });

  it("reads Stryker reports, counting NoCoverage as survived", () => {
    const report = {
      files: {
        "src/App.tsx": {
          mutants: [
            {
              mutatorName: "ArrowFunction",
              replacement: "() => undefined",
              status: "NoCoverage",
              location: { start: { line: 9 } },
            },
            {
              mutatorName: "ArrayDeclaration",
              replacement: "[]",
              status: "Ignored",
              statusReason: "equivalente",
              location: { start: { line: 12 } },
            },
            {
              mutatorName: "BlockStatement",
              replacement: "{}",
              status: "Pending",
              location: { start: { line: 1 } },
            },
          ],
        },
      },
    };
    expect(fromStryker(report, "ui/")).toEqual([
      {
        file: "ui/src/App.tsx",
        line: 9,
        description: "ArrowFunction → () => undefined",
        status: "survived",
        reason: undefined,
      },
      {
        file: "ui/src/App.tsx",
        line: 12,
        description: "ArrayDeclaration → []",
        status: "ignored",
        reason: "equivalente",
      },
    ]);
  });

  it("globs match nested folders and single segments", () => {
    expect(globToRegExp("crates/core/src/risk/**").test("crates/core/src/risk/a/b.rs")).toBe(true);
    expect(globToRegExp("**/*.test.*").test("ui/src/App.test.tsx")).toBe(true);
    expect(globToRegExp("**/*.test.*").test("App.test.tsx")).toBe(true);
    expect(globToRegExp("ui/src/*.tsx").test("ui/src/core/api.tsx")).toBe(false);
  });
});
