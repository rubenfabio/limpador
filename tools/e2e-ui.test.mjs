import { spawnSync } from "node:child_process";
import { mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

const uiDir = resolve(import.meta.dirname, "..", "ui");
let output;

beforeEach(() => {
  output = mkdtempSync(join(tmpdir(), "limpador-e2e-"));
});
afterEach(() => {
  rmSync(output, { recursive: true, force: true });
});

function playwright(args, env) {
  const result = spawnSync("pnpm", ["exec", "playwright", "test", ...args], {
    cwd: uiDir,
    encoding: "utf8",
    shell: process.platform === "win32",
    env: { ...process.env, LIMPADOR_E2E_OUTPUT: output, ...env },
  });
  return { code: result.status, log: `${result.stdout}${result.stderr}` };
}

function filesUnder(dir) {
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name);
}

describe("e2e da interface (T-00.03)", () => {
  it("t00_03_ca3_failing_e2e_leaves_a_screenshot", () => {
    const { code, log } = playwright(["e2e/meta"], { LIMPADOR_E2E_META: "1" });

    expect(code, log).not.toBe(0);
    expect(
      filesUnder(output).some((name) => name.endsWith(".png")),
      log,
    ).toBe(true);
  });

  it("t00_03_ca4_runs_without_rust_or_the_compiled_app", () => {
    // Remove every Rust toolchain folder from PATH: if the e2e needed cargo, it would fail.
    const path = (process.env.PATH ?? process.env.Path ?? "")
      .split(delimiter)
      .filter((dir) => !/\.cargo|rustup/i.test(dir))
      .join(delimiter);

    const { code, log } = playwright(["-g", "t00_03"], { PATH: path, Path: path });

    expect(code, log).toBe(0);
  });
});
