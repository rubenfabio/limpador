import { spawn } from "node:child_process";

/**
 * Runs every step in order, even after failures, and reports a summary.
 * Resolves to the process exit code: 0 when all steps pass, 1 otherwise.
 */
export async function runSteps(steps, { log = console.log } = {}) {
  const results = [];
  for (const [label, command, args] of steps) {
    log(`\n▶ ${label}: ${command} ${args.join(" ")}`);
    const code = await runCommand(command, args);
    results.push({ label, code });
  }

  log("\nResumo:");
  for (const { label, code } of results) {
    log(`  ${code === 0 ? "✔" : "✘"} ${label}${code === 0 ? "" : ` (código ${code})`}`);
  }
  return results.every((r) => r.code === 0) ? 0 : 1;
}

function runCommand(command, args) {
  return new Promise((resolve) => {
    // shell: true lets Windows resolve .cmd shims such as pnpm.cmd.
    const child = spawn(command, args, { stdio: "inherit", shell: process.platform === "win32" });
    child.on("error", () => resolve(127));
    child.on("close", (code) => resolve(code ?? 1));
  });
}
