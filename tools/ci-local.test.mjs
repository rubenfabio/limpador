import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ciEnvironment, isDocumentation, needsCi, parsePushedRefs } from "./prepush.mjs";
import { groups } from "./steps.mjs";
import { weeklyNotice } from "./weekly-mutation.mjs";

const repoRoot = resolve(import.meta.dirname, "..");
const labels = (steps) => steps.map(([label]) => label);

describe("CI local: o que roda antes do push (ADR-0017)", () => {
  it("t00_06_ca2_local_ci_runs_every_check_and_every_test_including_e2e", () => {
    const ci = labels(groups.ci);
    for (const step of [...labels(groups.check), ...labels(groups.test)]) {
      expect(ci).toContain(step);
    }
    expect(ci).toContain("e2e do app real (webdriver)");
    expect(ci).toContain("e2e da interface (playwright)");
  });

  it("t00_06_ca3_local_ci_runs_the_incremental_mutation_gate", () => {
    const [, command, args] = groups.ci.at(-1);
    expect([command, ...args]).toEqual(["node", "tools/mutation/run.mjs"]);
    expect(args).not.toContain("--full");
  });

  it("reads the refs git is about to push, ignoring branch deletions", () => {
    const zero = "0".repeat(40);
    const stdin = [
      `refs/heads/main ${"a".repeat(40)} refs/heads/main ${"b".repeat(40)}`,
      `refs/heads/nova ${"c".repeat(40)} refs/heads/nova ${zero}`,
      `(delete) ${zero} refs/heads/velha ${"d".repeat(40)}`,
      "",
    ].join("\n");
    expect(parsePushedRefs(stdin)).toEqual([
      {
        localRef: "refs/heads/main",
        localSha: "a".repeat(40),
        remoteRef: "refs/heads/main",
        remoteSha: "b".repeat(40),
      },
      {
        localRef: "refs/heads/nova",
        localSha: "c".repeat(40),
        remoteRef: "refs/heads/nova",
        remoteSha: undefined,
      },
    ]);
  });

  it("the clean checkout shares builds and downloads and compares against the pushed base", () => {
    expect(ciEnvironment("C:\repo", "abc123")).toEqual({
      CARGO_TARGET_DIR: join("C:\repo", "target", "ci"),
      LIMPADOR_CACHE_DIR: join("C:\repo", ".cache"),
      LIMPADOR_MUTATION_BASE: "abc123",
    });
    expect(ciEnvironment("C:\repo", undefined)).not.toHaveProperty("LIMPADOR_MUTATION_BASE");
  });

  it("documentation-only pushes skip the local CI", () => {
    expect(needsCi(["docs/visao.md", "tarefas/README.md", "README.md", "LICENSE"])).toBe(false);
    expect(needsCi(["docs/visao.md", "crates/core/src/lib.rs"])).toBe(true);
    expect(isDocumentation("features/F01-motor-de-analise/processo.md")).toBe(true);
    expect(isDocumentation("tools/steps.mjs")).toBe(false);
  });
});

describe("CI local: o hook pre-push", () => {
  let dir;
  let repo;
  const git = (...args) =>
    spawnSync("git", ["-c", "user.name=T", "-c", "user.email=t@t.invalid", ...args], {
      cwd: repo,
      encoding: "utf8",
    });

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), "limpador-prepush-"));
    repo = join(dir, "repo");
    const remote = join(dir, "remoto.git");
    execFileSync("git", ["init", "--quiet", "--bare", "--initial-branch=main", remote]);
    execFileSync("git", ["init", "--quiet", "--initial-branch=main", repo]);
    mkdirSync(join(repo, ".githooks"));
    mkdirSync(join(repo, "tools"));
    copyFileSync(join(repoRoot, ".githooks", "pre-push"), join(repo, ".githooks", "pre-push"));
    git("config", "core.hooksPath", ".githooks");
    git("remote", "add", "origin", remote);
    writeFileSync(join(repo, "a.txt"), "x");
    git("add", "a.txt");
    git("commit", "--quiet", "--no-verify", "-m", "c1");
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it("t00_06_ca2_push_is_refused_when_the_local_ci_fails", () => {
    // Stand-in for tools/prepush.mjs with a failing e2e.
    writeFileSync(join(repo, "tools", "prepush.mjs"), "process.exit(1);");

    expect(git("push", "origin", "main").status).not.toBe(0);
    expect(git("ls-remote", "origin", "main").stdout.trim()).toBe("");
  });

  it("push goes through when the local CI passes", () => {
    writeFileSync(join(repo, "tools", "prepush.mjs"), "process.exit(0);");

    expect(git("push", "origin", "main").status).toBe(0);
    expect(git("ls-remote", "origin", "main").stdout).toContain("refs/heads/main");
  });
});

describe("mutação completa semanal", () => {
  it("t00_06_ca4_a_target_dropping_raises_a_notification", () => {
    const notice = weeklyNotice(1, "C:\\Projetos\\limpador-\\reports\\mutation\\resumo.md");
    expect(notice.title).toContain("mutação semanal abaixo da meta");
    expect(notice.message).toContain("resumo.md");
  });

  it("passing runs stay silent", () => {
    expect(weeklyNotice(0, "resumo.md")).toBeNull();
  });

  it("the scheduled task runs weekly and catches up when the computer was off", () => {
    const script = readFileSync(join(repoRoot, "tools", "agendar-mutacao-semanal.ps1"), "utf8");
    expect(script).toMatch(/-Weekly -DaysOfWeek Monday/);
    expect(script).toContain("-StartWhenAvailable");
    expect(script).toContain("tools\\weekly-mutation.mjs");
    // Windows PowerShell 5.1 reads BOM-less files as ANSI: the script must be pure ASCII.
    expect([...script].every((char) => char.charCodeAt(0) < 128)).toBe(true);
  });
});

describe("comandos documentados", () => {
  const agents = readFileSync(join(repoRoot, "AGENTS.md"), "utf8");

  it("t00_06_ca5_every_ci_command_has_a_documented_local_equivalent", () => {
    const workflow = readFileSync(join(repoRoot, ".github", "workflows", "ci.yml"), "utf8");
    const runs = [...workflow.matchAll(/^\s*run: (.+)$/gm)].map((m) => m[1].trim());
    expect(runs.length).toBeGreaterThan(5);
    for (const cmd of [...runs, "pnpm ci", "pnpm mutation:agendar"]) {
      expect(agents, `"${cmd}" não está documentado no AGENTS.md`).toContain(`\`${cmd}\``);
    }
  });
});
