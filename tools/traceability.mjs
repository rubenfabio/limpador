// Acceptance-criteria → test traceability (T-00.07).
// Every CA of a task marked `concluída` in tarefas/README.md needs at least one test whose
// name carries its id: T-99.02 CA3 → `t99_02_ca3` (made-up task).
import { readdirSync, readFileSync } from "node:fs";
import { join, relative, resolve } from "node:path";

/** Tasks from the index tables: `| T-00.01 | [Título](pasta/arquivo.md) | deps | status |`. */
export function parseTaskIndex(markdown) {
  const row = /^\|\s*(T-\d\d\.\d\d)\s*\|\s*\[([^\]]+)\]\(([^)]+)\)\s*\|[^|]*\|\s*([^|]+?)\s*\|/;
  return markdown
    .split(/\r?\n/)
    .map((line) => line.match(row))
    .filter(Boolean)
    .map(([, id, title, file, status]) => ({ id, title, file, status }));
}

/** CA numbers declared in a task file (`**CA3**`). */
export function parseCriteria(markdown) {
  return [...new Set([...markdown.matchAll(/\*\*CA(\d+)\*\*/g)].map((m) => Number(m[1])))];
}

/** `T-99.02` + 3 → `t99_02_ca3` (made-up task). */
export function criterionId(taskId, ca) {
  return `t${taskId.slice(2).replace(".", "_")}_ca${ca}`;
}

/** Where a test lives decides its layer. */
export function layerOf(path) {
  const p = path.replaceAll("\\", "/");
  if (p.startsWith("ui/e2e/")) return "e2e da interface";
  if (p.startsWith("e2e-app/specs/")) return "e2e do app real";
  if (p.startsWith("tools/") || p.startsWith("e2e-app/")) return "verificação do repositório";
  if (/^crates\/[^/]+\/tests\//.test(p)) return "integração";
  return "unitário";
}

/** Criterion ids mentioned in a test source file, with the layer of that file. */
export function findTestIds({ path, text }) {
  const layer = layerOf(path);
  return [...new Set([...text.matchAll(/\bt(\d\d)_(\d\d)_ca(\d+)/g)].map((m) => m[0]))].map(
    (id) => ({ id, layer, path: path.replaceAll("\\", "/") }),
  );
}

/**
 * Coverage of every CA of every task, and the CAs of `concluída` tasks that lack a test.
 * Tasks in any other status are reported but never fail the check.
 */
export function checkTraceability(tasks, references) {
  const layersById = new Map();
  for (const ref of references) {
    const layers = layersById.get(ref.id) ?? new Set();
    layers.add(ref.layer);
    layersById.set(ref.id, layers);
  }
  const report = tasks.map((task) => ({
    ...task,
    criteria: task.criteria.map((ca) => {
      const id = criterionId(task.id, ca);
      return { ca, id, layers: [...(layersById.get(id) ?? [])].sort() };
    }),
  }));
  const missing = report
    .filter((t) => t.status === "concluída")
    .flatMap((t) =>
      t.criteria.filter((c) => c.layers.length === 0).map((c) => ({ task: t.id, ...c })),
    );
  return { report, missing };
}

export function renderTraceability({ report, missing }) {
  const out = ["# Rastreabilidade CA → teste", ""];
  out.push(
    missing.length === 0
      ? "**Resultado: completo.** Todas as tarefas concluídas têm teste para cada critério."
      : `**Resultado: incompleto.** ${missing.length} critério(s) de tarefas concluídas sem teste.`,
    "",
  );
  for (const task of report.filter((t) => t.status === "concluída")) {
    out.push(`## ${task.id} — ${task.title}`, "");
    for (const c of task.criteria) {
      out.push(
        `- CA${c.ca} (\`${c.id}\`): ${c.layers.length ? c.layers.join(", ") : "**SEM TESTE**"}`,
      );
    }
    out.push("");
  }
  return `${out.join("\n")}\n`;
}

const TEST_ROOTS = ["crates", "src-tauri/src", "ui/src", "ui/e2e", "e2e-app", "tools"];
const TEST_FILE = /\.(rs|ts|tsx|mjs)$/;

function sourceFiles(root) {
  return TEST_ROOTS.flatMap((dir) => {
    try {
      return readdirSync(join(root, dir), { recursive: true, withFileTypes: true })
        .filter((e) => e.isFile() && TEST_FILE.test(e.name))
        .map((e) => join(e.parentPath, e.name))
        .filter((p) => !/[\\/](node_modules|target|dist)[\\/]/.test(p));
    } catch {
      return [];
    }
  });
}

/** Reads the repository and checks it. */
export function traceRepository(root) {
  const tasksDir = join(root, "tarefas");
  const tasks = parseTaskIndex(readFileSync(join(tasksDir, "README.md"), "utf8")).map((t) => ({
    ...t,
    criteria: parseCriteria(readFileSync(join(tasksDir, t.file), "utf8")),
  }));
  const references = sourceFiles(root).flatMap((path) =>
    findTestIds({ path: relative(root, path), text: readFileSync(path, "utf8") }),
  );
  return checkTraceability(tasks, references);
}

if (process.argv[1] && resolve(process.argv[1]) === resolve(import.meta.filename)) {
  const root = resolve(import.meta.dirname, "..");
  const result = traceRepository(root);
  console.log(renderTraceability(result));
  if (result.missing.length > 0) {
    for (const m of result.missing) console.error(`✘ ${m.task} CA${m.ca} sem teste (${m.id})`);
    process.exit(1);
  }
}
