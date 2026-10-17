// Mutation gate (T-00.05): turns cargo-mutants and Stryker reports into per-file scores,
// applies the thresholds of mutation/politica.json and renders the summary.

/** Converts a glob (`**`, `*`) into an anchored regular expression over `/` paths. */
export function globToRegExp(glob) {
  let source = "";
  for (let i = 0; i < glob.length; i++) {
    const char = glob[i];
    if (char === "*" && glob[i + 1] === "*") {
      // "**/" also matches zero folders.
      source += glob[i + 2] === "/" ? "(?:.*/)?" : ".*";
      i += glob[i + 2] === "/" ? 2 : 1;
    } else if (char === "*") {
      source += "[^/]*";
    } else {
      source += char.replace(/[.+?^${}()|[\]\\]/g, "\\$&");
    }
  }
  return new RegExp(`^${source}$`);
}

const matchesAny = (file, globs) => globs.some((g) => globToRegExp(g).test(file));

/** Files the policy leaves out of mutation (tests, fake machine, entry points). */
export const isExcluded = (file, policy) => matchesAny(toSlash(file), policy.foraDaMedicao);
const toSlash = (path) => path.replaceAll("\\", "/");

const CARGO_STATUS = {
  CaughtMutant: "killed",
  MissedMutant: "survived",
  Timeout: "timeout",
  Unviable: "unviable",
};

/** Mutants from cargo-mutants' outcomes.json. */
export function fromCargoMutants(outcomes) {
  return outcomes.outcomes
    .filter((o) => o.scenario !== "Baseline" && o.scenario?.Mutant)
    .map((o) => {
      const m = o.scenario.Mutant;
      return {
        file: toSlash(m.file),
        line: m.span.start.line,
        description: m.name.replace(/^[^:]+:\d+:\d+: /, ""),
        status: CARGO_STATUS[o.summary] ?? "unviable",
      };
    });
}

const STRYKER_STATUS = {
  Killed: "killed",
  Survived: "survived",
  NoCoverage: "survived",
  Timeout: "timeout",
  CompileError: "unviable",
  RuntimeError: "unviable",
  Ignored: "ignored",
};

/** Mutants from Stryker's JSON report; `prefix` maps its paths to repo paths (e.g. "ui/"). */
export function fromStryker(report, prefix) {
  return Object.entries(report.files).flatMap(([file, { mutants }]) =>
    mutants
      .filter((m) => m.status in STRYKER_STATUS)
      .map((m) => ({
        file: `${prefix}${toSlash(file)}`,
        line: m.location.start.line,
        description: `${m.mutatorName} → ${m.replacement ?? ""}`.trim(),
        status: STRYKER_STATUS[m.status],
        reason: m.statusReason,
      })),
  );
}

/** Per-file counts and score (killed+timeout over everything that ran and was not ignored). */
export function scoreByFile(mutants) {
  const files = new Map();
  for (const m of mutants) {
    const s = files.get(m.file) ?? { killed: 0, survived: 0, timeout: 0, unviable: 0, ignored: 0 };
    s[m.status] += 1;
    files.set(m.file, s);
  }
  for (const s of files.values()) {
    const detected = s.killed + s.timeout;
    const total = detected + s.survived;
    s.score = total === 0 ? null : Math.floor((detected / total) * 1000) / 10;
  }
  return files;
}

/** One row per measured file: its threshold and whether it passes. */
export function evaluate(scores, policy) {
  return [...scores.entries()]
    .filter(([file]) => !matchesAny(file, policy.foraDaMedicao))
    .map(([file, s]) => {
      const critical = matchesAny(file, policy.criticos);
      const threshold = critical ? policy.metas.critico : policy.metas.padrao;
      return { file, critical, threshold, ...s, passed: s.score === null || s.score >= threshold };
    })
    .sort((a, b) => a.file.localeCompare(b.file));
}

/** Splits changed files into what each tool must mutate in incremental mode. */
export function planIncremental(changedFiles, policy) {
  const measured = changedFiles.map(toSlash).filter((f) => !matchesAny(f, policy.foraDaMedicao));
  return {
    rust: measured.filter((f) => f.endsWith(".rs")),
    ts: measured.filter((f) => /^ui\/src\/.*\.tsx?$/.test(f)).map((f) => f.slice("ui/".length)),
  };
}

/** A unified diff that adds `path` with `text`, as `git diff` would show a new file. */
export function newFileDiff(path, text) {
  const lines = text.split(/\r?\n/);
  if (lines.at(-1) === "") lines.pop();
  return [
    `diff --git a/${path} b/${path}`,
    "new file mode 100644",
    "--- /dev/null",
    `+++ b/${path}`,
    `@@ -0,0 +1,${lines.length} @@`,
    ...lines.map((line) => `+${line}`),
    "",
  ].join("\n");
}

const RUST_SKIP = /mutants::skip/;
const STRYKER_DISABLE = /Stryker disable(?:[ -]next-line)?\s+[\w,]+(?::\s*(\S.*))?/;

/**
 * Ignored mutants must say why (ADR-0011). Returns the places that do not:
 * - Rust: `mutants::skip` needs a "motivo:" comment on the same line or the 2 lines above;
 * - .cargo/mutants.toml: every `exclude_re` entry needs a "# motivo:" line right above;
 * - TypeScript: `Stryker disable ...` needs ": <reason>".
 */
export function findUnjustifiedIgnores({ path, text }) {
  const lines = text.split(/\r?\n/);
  const problems = [];
  const report = (index) =>
    problems.push({ file: toSlash(path), line: index + 1, text: lines[index].trim() });

  if (path.endsWith(".rs")) {
    lines.forEach((line, i) => {
      if (
        RUST_SKIP.test(line) &&
        !lines.slice(Math.max(0, i - 2), i + 1).some((l) => /motivo:/i.test(l))
      ) {
        report(i);
      }
    });
  } else if (path.endsWith("mutants.toml")) {
    let inExcludes = false;
    lines.forEach((line, i) => {
      if (/^\s*exclude_re\s*=/.test(line)) inExcludes = true;
      if (inExcludes && /^\s*"/.test(line) && !/#\s*motivo:/i.test(lines[i - 1] ?? "")) report(i);
      if (inExcludes && line.includes("]")) inExcludes = false;
    });
  } else if (/\.(m?[jt]sx?)$/.test(path)) {
    lines.forEach((line, i) => {
      const match = line.match(STRYKER_DISABLE);
      if (match && !match[1]) report(i);
    });
  }
  return problems;
}

/** Markdown summary: per-file table, survivors with file and line, ignored mutants. */
export function renderSummary(rows, mutants) {
  const pct = (score) => (score === null ? "—" : `${score.toFixed(1)}%`);
  const out = ["# Relatório de mutação", ""];
  const blocked = rows.filter((r) => !r.passed);
  out.push(
    blocked.length === 0
      ? "**Resultado: passou.** Todas as metas foram atingidas."
      : `**Resultado: bloqueado.** ${blocked.length} arquivo(s) abaixo da meta.`,
    "",
    "| Arquivo | Tipo | Nota | Meta | Mortos | Sobreviventes | Veredito |",
    "|---|---|---|---|---|---|---|",
  );
  for (const r of rows) {
    out.push(
      `| ${r.file} | ${r.critical ? "crítico" : "padrão"} | ${pct(r.score)} | ${r.threshold}% | ` +
        `${r.killed + r.timeout} | ${r.survived} | ${r.passed ? "✔ passou" : "✘ bloqueou"} |`,
    );
  }

  const survivors = mutants.filter((m) => m.status === "survived");
  out.push("", `## Sobreviventes (${survivors.length})`, "");
  for (const m of survivors) out.push(`- \`${m.file}:${m.line}\` — ${m.description}`);

  const ignored = mutants.filter((m) => m.status === "ignored");
  if (ignored.length > 0) {
    out.push("", `## Ignorados com justificativa (${ignored.length})`, "");
    for (const m of ignored) out.push(`- \`${m.file}:${m.line}\` — ${m.reason ?? m.description}`);
  }
  return `${out.join("\n")}\n`;
}
