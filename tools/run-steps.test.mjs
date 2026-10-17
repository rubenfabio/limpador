import { describe, expect, it } from "vitest";
import { runSteps } from "./run-steps.mjs";

const node = process.execPath;
const pass = (label) => [label, node, ["-e", "process.exit(0)"]];
const fail = (label) => [label, node, ["-e", "process.exit(3)"]];
const quiet = { log: () => {} };

describe("runSteps", () => {
  it("t00_01_ca1_returns_zero_when_every_step_passes", async () => {
    expect(await runSteps([pass("a"), pass("b")], quiet)).toBe(0);
  });

  it("t00_01_ca2_returns_error_code_when_a_step_fails", async () => {
    expect(await runSteps([pass("a"), fail("b")], quiet)).toBe(1);
  });

  it("t00_01_ca2_keeps_running_after_a_failure_and_reports_it", async () => {
    const lines = [];
    const code = await runSteps([fail("primeiro"), pass("segundo")], {
      log: (line) => lines.push(line),
    });
    expect(code).toBe(1);
    const summary = lines.join("\n");
    expect(summary).toContain("✘ primeiro (código 3)");
    expect(summary).toContain("✔ segundo");
  });

  it("t00_01_ca2_treats_missing_command_as_failure", async () => {
    const code = await runSteps([["inexistente", "comando-que-nao-existe-xyz", []]], quiet);
    expect(code).toBe(1);
  });
});
