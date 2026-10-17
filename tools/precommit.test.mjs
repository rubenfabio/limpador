import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { planPrecommit } from "./precommit.mjs";

const repoRoot = resolve(import.meta.dirname, "..");
const labels = (steps) => steps.map(([label]) => label);

describe("pre-commit: o que roda", () => {
  it("rust changes run formatting and the Rust unit tests", () => {
    expect(labels(planPrecommit(["crates/core/src/ports.rs"]))).toEqual([
      "formatação Rust",
      "unitários Rust",
    ]);
  });

  it("interface changes run Biome on the changed files and the UI unit tests", () => {
    const steps = planPrecommit(["ui/src/App.tsx"]);
    expect(labels(steps)).toEqual(["Biome nos arquivos alterados", "unitários da interface"]);
    expect(steps[0][2]).toContain("ui/src/App.tsx");
  });

  it("tool changes run the tool tests without the slow e2e meta tests", () => {
    const steps = planPrecommit(["tools/run-steps.mjs"]);
    expect(labels(steps)).toContain("testes das ferramentas");
    expect(steps.at(-1)[2]).toContain("**/e2e-*.test.mjs");
  });

  it("documentation-only commits run nothing", () => {
    expect(planPrecommit(["docs/visao.md", "tarefas/README.md"])).toEqual([]);
  });
});

describe("pre-commit: o hook do git", () => {
  let repo;
  const git = (...args) =>
    spawnSync("git", ["-c", "user.name=T", "-c", "user.email=t@t.invalid", ...args], {
      cwd: repo,
      encoding: "utf8",
    });

  beforeEach(() => {
    repo = mkdtempSync(join(tmpdir(), "limpador-hook-"));
    execFileSync("git", ["init", "--quiet", "--initial-branch=main"], { cwd: repo });
    mkdirSync(join(repo, ".githooks"));
    mkdirSync(join(repo, "tools"));
    copyFileSync(join(repoRoot, ".githooks", "pre-commit"), join(repo, ".githooks", "pre-commit"));
    execFileSync("git", ["config", "core.hooksPath", ".githooks"], { cwd: repo });
    writeFileSync(join(repo, "a.txt"), "x");
    git("add", "a.txt");
  });
  afterEach(() => rmSync(repo, { recursive: true, force: true }));

  it("t00_06_ca1_commit_is_refused_when_the_checks_fail", () => {
    // Stand-in for tools/precommit.mjs finding a broken unit test.
    writeFileSync(join(repo, "tools", "precommit.mjs"), "process.exit(1);");

    expect(git("commit", "-m", "quebrado").status).not.toBe(0);
    expect(git("rev-parse", "--verify", "HEAD").status).not.toBe(0);
  });

  it("commit goes through when the checks pass", () => {
    writeFileSync(join(repo, "tools", "precommit.mjs"), "process.exit(0);");

    expect(git("commit", "-m", "ok").status).toBe(0);
  });

  it("pnpm install wires the hooks folder", () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8"));
    expect(pkg.scripts.prepare).toBe("git config core.hooksPath .githooks");
  });
});

describe("CI (.github/workflows/ci.yml)", () => {
  const workflow = readFileSync(join(repoRoot, ".github", "workflows", "ci.yml"), "utf8");
  const agents = readFileSync(join(repoRoot, "AGENTS.md"), "utf8");
  const runs = [...workflow.matchAll(/^\s*run: (.+)$/gm)].map((m) => m[1].trim());

  it("t00_06_ca2_failed_runs_upload_screenshots_and_logs", () => {
    expect(workflow).toMatch(
      /if: failure\(\)\s+uses: actions\/upload-artifact@v\d+\s+with:\s+name: relatorios-[^\n]+\s+path: reports\//,
    );
    expect(runs).toContain("pnpm test");
    expect(workflow).toContain("runs-on: windows-latest");
  });

  it("t00_06_ca3_pull_requests_run_incremental_mutation_against_the_base", () => {
    expect(workflow).toMatch(
      /if: github\.event_name == 'pull_request'[\s\S]+run: pnpm mutation\n\s+env:\s+LIMPADOR_MUTATION_BASE: origin\/\$\{\{ github\.base_ref \}\}/,
    );
  });

  it("t00_06_ca4_weekly_full_mutation_opens_an_issue_when_a_target_drops", () => {
    expect(workflow).toMatch(/cron: "0 6 \* \* 1"/);
    expect(workflow).toMatch(
      /run: pnpm mutation:full[\s\S]+if: failure\(\)[\s\S]+run: gh issue create/,
    );
    expect(workflow).toMatch(/issues: write/);
  });

  it("t00_06_ca5_every_ci_command_has_a_documented_local_equivalent", () => {
    const local = runs.filter((cmd) => !cmd.startsWith("gh "));
    expect(local.length).toBeGreaterThan(5);
    for (const cmd of local) {
      expect(agents, `"${cmd}" não está documentado no AGENTS.md`).toContain(`\`${cmd}\``);
    }
  });
});
