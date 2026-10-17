// Mutation testing with per-file thresholds (T-00.05).
//   node tools/mutation/run.mjs            incremental: only code changed since the base branch
//   node tools/mutation/run.mjs --full     everything
// Base branch: LIMPADOR_MUTATION_BASE (default origin/main).
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import {
  evaluate,
  findUnjustifiedIgnores,
  fromCargoMutants,
  fromStryker,
  isExcluded,
  planIncremental,
  renderSummary,
  scoreByFile,
} from "./gate.mjs";

const root = resolve(import.meta.dirname, "..", "..");
const reports = join(root, "reports", "mutation");
const policy = JSON.parse(readFileSync(join(root, "mutation", "politica.json"), "utf8"));
const full = process.argv.includes("--full");
const base = process.env.LIMPADOR_MUTATION_BASE ?? "origin/main";

const git = (args) => execFileSync("git", args, { cwd: root, encoding: "utf8" });

function run(command, args, cwd) {
  console.log(`\n▶ ${command} ${args.join(" ")}`);
  return spawnSync(command, args, { cwd, stdio: "inherit", shell: process.platform === "win32" })
    .status;
}

// 1. Every ignored mutant must be justified (in measured code; tests hold examples on purpose).
const sources = git([
  "ls-files",
  "--cached",
  "--others",
  "--exclude-standard",
  "*.rs",
  "*.ts",
  "*.tsx",
  "*.mjs",
  ".cargo/mutants.toml",
])
  .split("\n")
  .filter((path) => path && !isExcluded(path, policy));
const unjustified = sources.flatMap((path) =>
  findUnjustifiedIgnores({ path, text: readFileSync(join(root, path), "utf8") }),
);
if (unjustified.length > 0) {
  console.error('Configuração inválida: mutantes ignorados sem justificativa ("motivo:"):');
  for (const p of unjustified) console.error(`  ${p.file}:${p.line}  ${p.text}`);
  process.exit(1);
}

// 2. What to mutate.
rmSync(reports, { recursive: true, force: true });
mkdirSync(reports, { recursive: true });
let plan = { rust: true, ts: undefined };
let rustDiff;
if (!full) {
  const changed = git(["diff", "--name-only", "--merge-base", base]).split("\n").filter(Boolean);
  const incremental = planIncremental(changed, policy);
  console.log(
    `Modo incremental contra ${base}: ${incremental.rust.length} arquivo(s) Rust, ${incremental.ts.length} TS.`,
  );
  rustDiff = join(reports, "rust.diff");
  writeFileSync(rustDiff, git(["diff", "--merge-base", base, "--", "*.rs"]));
  plan = { rust: incremental.rust.length > 0, ts: incremental.ts };
}

// 3. Rust (cargo-mutants exits 2/3 when mutants are missed or time out: that is data, not an error).
const mutants = [];
if (plan.rust) {
  const args = ["mutants", "--workspace", "-o", join(reports, "rust")];
  if (rustDiff) args.push("--in-diff", rustDiff);
  const code = run("cargo", args, root);
  if (![0, 2, 3].includes(code)) {
    console.error(`cargo-mutants falhou (código ${code}).`);
    process.exit(1);
  }
  const outcomes = join(reports, "rust", "mutants.out", "outcomes.json");
  if (existsSync(outcomes))
    mutants.push(...fromCargoMutants(JSON.parse(readFileSync(outcomes, "utf8"))));
}

// 4. TypeScript.
if (plan.ts === undefined || plan.ts.length > 0) {
  const args = ["exec", "stryker", "run"];
  if (plan.ts) args.push("--mutate", plan.ts.join(","));
  if (run("pnpm", args, join(root, "ui")) !== 0) {
    console.error("Stryker falhou.");
    process.exit(1);
  }
  mutants.push(
    ...fromStryker(JSON.parse(readFileSync(join(reports, "ts", "mutation.json"), "utf8")), "ui/"),
  );
}

// 5. Gate.
const rows = evaluate(scoreByFile(mutants), policy);
const summary = renderSummary(rows, mutants);
writeFileSync(join(reports, "resumo.md"), summary);
console.log(`\n${summary}\nRelatório: ${join(reports, "resumo.md")}`);
process.exit(rows.every((r) => r.passed) ? 0 : 1);
