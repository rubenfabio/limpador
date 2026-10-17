// Fast checks before each commit (T-00.06), limited to what the staged files touch.
// Installed by `pnpm install` (prepare → git config core.hooksPath .githooks).
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { runSteps } from "./run-steps.mjs";

/** Steps needed for the staged `files`; an empty list means nothing to check. */
export function planPrecommit(files) {
  const paths = files.map((f) => f.replaceAll("\\", "/"));
  const any = (test) => paths.some(test);
  const steps = [];

  if (any((f) => f.endsWith(".rs") || /(^|\/)Cargo\.(toml|lock)$/.test(f))) {
    steps.push(["formatação Rust", "cargo", ["fmt", "--all", "--check"]]);
    steps.push(["unitários Rust", "cargo", ["test", "--workspace", "--lib", "--quiet"]]);
  }
  const webFiles = paths.filter((f) => /\.(ts|tsx|mjs|js|json)$/.test(f));
  if (webFiles.length > 0) {
    steps.push([
      "Biome nos arquivos alterados",
      "pnpm",
      [
        "exec",
        "biome",
        "check",
        "--no-errors-on-unmatched",
        "--files-ignore-unknown=true",
        ...webFiles,
      ],
    ]);
  }
  if (any((f) => f.startsWith("ui/src/"))) {
    steps.push(["unitários da interface", "pnpm", ["--filter", "ui", "test"]]);
  }
  if (any((f) => f.startsWith("tools/"))) {
    // The e2e meta tests build the app and take minutes: they run in CI, not here.
    steps.push([
      "testes das ferramentas",
      "pnpm",
      ["--filter", "tools", "exec", "vitest", "run", "--exclude", "**/e2e-*.test.mjs"],
    ]);
  }
  return steps;
}

if (process.argv[1] && resolve(process.argv[1]) === resolve(import.meta.filename)) {
  const staged = execFileSync("git", ["diff", "--cached", "--name-only", "--diff-filter=ACMR"], {
    encoding: "utf8",
  })
    .split("\n")
    .filter(Boolean);
  const steps = planPrecommit(staged);
  if (steps.length === 0) process.exit(0);
  const code = await runSteps(steps);
  if (code !== 0) {
    console.error("\nCommit recusado: corrija as falhas acima (ou rode `pnpm fmt`).");
  }
  process.exit(code);
}
