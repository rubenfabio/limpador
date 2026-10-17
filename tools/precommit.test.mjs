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
