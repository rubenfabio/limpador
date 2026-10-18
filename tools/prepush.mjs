// Local CI before each push (T-00.06, ADR-0017). Git calls it through .githooks/pre-push,
// passing one line per pushed ref on stdin: "<local ref> <local sha> <remote ref> <remote sha>".
import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const NO_COMMIT = /^0+$/;

/** Pushed refs that carry commits (branch deletions are skipped). */
export function parsePushedRefs(stdin) {
  return stdin
    .split(/\r?\n/)
    .map((line) => line.trim().split(/\s+/))
    .filter((parts) => parts.length === 4 && !NO_COMMIT.test(parts[1]))
    .map(([localRef, localSha, remoteRef, remoteSha]) => ({
      localRef,
      localSha,
      remoteRef,
      remoteSha: NO_COMMIT.test(remoteSha) ? undefined : remoteSha,
    }));
}

/** Files that never affect the build or the tests. */
export function isDocumentation(file) {
  const f = file.replaceAll("\\", "/");
  return (
    /\.md$/i.test(f) ||
    f === "LICENSE" ||
    ["docs/", "tarefas/", "features/"].some((dir) => f.startsWith(dir))
  );
}

/** Whether the local CI must run for a push that changes `files`. */
export function needsCi(files) {
  return files.some((f) => !isDocumentation(f));
}

const git = (args) => execFileSync("git", args, { encoding: "utf8" }).trim();
const repoRoot = resolve(import.meta.dirname, "..");

/**
 * Environment of the CI run in the clean checkout: builds and downloads are shared with
 * the main checkout (so they are not redone), and mutation compares against `base`.
 */
export function ciEnvironment(root, base) {
  const env = {
    CARGO_TARGET_DIR: join(root, "target", "ci"),
    LIMPADOR_CACHE_DIR: join(root, ".cache"),
  };
  if (base) env.LIMPADOR_MUTATION_BASE = base;
  return env;
}

const run = (command, args, cwd, env = {}) =>
  spawnSync(command, args, {
    cwd,
    stdio: "inherit",
    shell: process.platform === "win32",
    env: { ...process.env, ...env },
  }).status ?? 1;

/**
 * Runs the local CI on exactly what is being pushed: a temporary checkout of `sha`, so
 * uncommitted work neither hides nor causes failures, and keeps being editable meanwhile.
 */
function ciOnCommit(sha, base) {
  const checkout = mkdtempSync(join(tmpdir(), "limpador-ci-"));
  rmSync(checkout, { recursive: true, force: true });
  git(["worktree", "add", "--detach", "--quiet", checkout, sha]);
  try {
    const installed = run("pnpm", ["install", "--frozen-lockfile", "--prefer-offline"], checkout);
    if (installed !== 0) return installed;
    const code = run("node", ["tools/run.mjs", "ci"], checkout, ciEnvironment(repoRoot, base));
    if (code !== 0 && existsSync(join(checkout, "reports"))) {
      cpSync(join(checkout, "reports"), join(repoRoot, "reports"), {
        recursive: true,
        force: true,
      });
    }
    return code;
  } finally {
    try {
      git(["worktree", "remove", "--force", checkout]);
    } catch {
      rmSync(checkout, { recursive: true, force: true });
      git(["worktree", "prune"]);
    }
  }
}

/** Commit the remote already has: the pushed range starts there. */
function baseOf(ref) {
  if (ref.remoteSha) return ref.remoteSha;
  try {
    return git(["merge-base", ref.localSha, "origin/main"]);
  } catch {
    return undefined; // brand-new history: check everything
  }
}

if (process.argv[1] && resolve(process.argv[1]) === resolve(import.meta.filename)) {
  const refs = process.stdin.isTTY ? [] : parsePushedRefs(readFileSync(0, "utf8"));
  const ranges = refs.map((ref) => {
    const base = baseOf(ref);
    const files = base
      ? git(["diff", "--name-only", base, ref.localSha]).split("\n").filter(Boolean)
      : ["(histórico novo)"];
    return { base, files };
  });

  const codeIndex = ranges.findIndex((r) => needsCi(r.files));
  const codeRange = ranges[codeIndex];
  if (refs.length > 0 && !codeRange) {
    console.log("Push só de documentação: CI local pulado.");
    process.exit(0);
  }
  // Pushing by hand without git (no refs on stdin) checks the current commit.
  const sha = refs[codeIndex]?.localSha ?? git(["rev-parse", "HEAD"]);
  console.log(
    `CI local sobre ${sha.slice(0, 8)} numa cópia limpa (check + test + mutação incremental)...`,
  );
  const code = ciOnCommit(sha, codeRange?.base);
  if (code !== 0) {
    console.error("\nPush recusado: o CI local falhou. Relatórios em reports/.");
  }
  process.exit(code);
}
