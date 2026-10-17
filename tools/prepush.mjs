// Local CI before each push (T-00.06, ADR-0017). Git calls it through .githooks/pre-push,
// passing one line per pushed ref on stdin: "<local ref> <local sha> <remote ref> <remote sha>".
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { runSteps } from "./run-steps.mjs";
import { groups } from "./steps.mjs";

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

  const codeRange = ranges.find((r) => needsCi(r.files));
  if (refs.length > 0 && !codeRange) {
    console.log("Push só de documentação: CI local pulado.");
    process.exit(0);
  }
  if (codeRange?.base) process.env.LIMPADOR_MUTATION_BASE = codeRange.base;

  console.log("CI local antes do push (check + test + mutação incremental)...");
  const code = await runSteps(groups.ci);
  if (code !== 0) {
    console.error("\nPush recusado: o CI local falhou. Relatórios em reports/.");
  }
  process.exit(code);
}
