import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

const repoRoot = resolve(import.meta.dirname, "..");
// Folders where Tauri and WebView2 would keep data for this app on a real machine.
const realDataDirs = [process.env.LOCALAPPDATA, process.env.APPDATA]
  .filter(Boolean)
  .map((base) => join(base, "com.rubenfabio.limpador"));

let output;
beforeEach(() => {
  output = mkdtempSync(join(tmpdir(), "limpador-e2e-app-relatorio-"));
});
afterEach(() => {
  rmSync(output, { recursive: true, force: true });
});

function wdio(env) {
  const result = spawnSync("pnpm", ["--filter", "e2e-app", "e2e"], {
    cwd: repoRoot,
    encoding: "utf8",
    shell: process.platform === "win32",
    env: { ...process.env, LIMPADOR_E2E_OUTPUT: output, ...env },
  });
  return { code: result.status, log: `${result.stdout}${result.stderr}`.slice(-4_000) };
}

/** Path → mtime of everything under `dir` ("absent" if the folder does not exist). */
function snapshot(dir) {
  if (!existsSync(dir)) return "absent";
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .map((e) => join(e.parentPath, e.name))
    .map((path) => `${path} ${statSync(path).mtimeMs}`)
    .sort();
}

function leftoverSandboxes() {
  return readdirSync(tmpdir()).filter((name) => /^limpador-e2e-app-(?!relatorio)/.test(name));
}

describe("e2e do app real (T-00.04)", () => {
  it("t00_04_ca3_failing_journey_leaves_screenshot_app_log_and_machine_listing", () => {
    const { code, log } = wdio({ LIMPADOR_E2E_META: "1" });

    expect(code, log).not.toBe(0);
    const files = readdirSync(output, { recursive: true });
    for (const artifact of ["tela.png", "app.log", "maquina-falsa.txt"]) {
      expect(
        files.some((f) => f.endsWith(artifact)),
        `${artifact} ausente\n${log}`,
      ).toBe(true);
    }
  });

  it("t00_04_ca4_nothing_outside_the_temp_folder_changes", () => {
    const before = realDataDirs.map(snapshot);
    const sandboxesBefore = leftoverSandboxes();

    const { code, log } = wdio({});

    expect(code, log).toBe(0);
    expect(realDataDirs.map(snapshot)).toEqual(before);
    expect(leftoverSandboxes()).toEqual(sandboxesBefore);
  });
});
