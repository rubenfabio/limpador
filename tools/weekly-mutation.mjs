// Weekly full mutation run (T-00.06, ADR-0017), started by a Windows scheduled task
// (registered with `pnpm mutation:agendar`). Shows a Windows notification when a target drops.
import { spawnSync } from "node:child_process";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");

/** Notification for an exit code of `pnpm mutation:full`, or null when everything passed. */
export function weeklyNotice(exitCode, summaryPath) {
  if (exitCode === 0) return null;
  return {
    title: "Limpador: mutação semanal abaixo da meta",
    message: `Veja o relatório em ${summaryPath}`,
  };
}

if (process.argv[1] && resolve(process.argv[1]) === resolve(import.meta.filename)) {
  const run = spawnSync("pnpm", ["mutation:full"], { cwd: root, stdio: "inherit", shell: true });
  const notice = weeklyNotice(run.status ?? 1, join(root, "reports", "mutation", "resumo.md"));
  if (notice) {
    spawnSync(
      "powershell",
      ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", join(root, "tools", "notificar.ps1")],
      {
        stdio: "inherit",
        env: { ...process.env, LIMPADOR_TITULO: notice.title, LIMPADOR_MENSAGEM: notice.message },
      },
    );
  }
  process.exit(run.status ?? 1);
}
