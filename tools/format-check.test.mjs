import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

const repoRoot = resolve(import.meta.dirname, "..");
let dir;

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "limpador-fmt-"));
});
afterEach(() => {
  rmSync(dir, { recursive: true, force: true });
});

function run(command, args, cwd = repoRoot) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  return { code: result.status, output: `${result.stdout}${result.stderr}` };
}

describe("format verification", () => {
  it("t00_01_ca4_biome_fails_and_names_badly_formatted_ts_file", () => {
    writeFileSync(join(dir, "mal_formatado.ts"), "export const   x={a:1,\n      b:2}\n");
    // The temp folder is outside the repo, so Biome runs there with the project's
    // config but without the git integration (which only accepts paths in the repo).
    const biome = join(repoRoot, "node_modules", ".bin", "biome");
    const { code, output } = run(
      biome,
      ["format", "--vcs-enabled=false", `--config-path=${repoRoot}`, "mal_formatado.ts"],
      dir,
    );
    expect(code).not.toBe(0);
    expect(output).toContain("mal_formatado.ts");
  });

  it("t00_01_ca4_rustfmt_fails_and_names_badly_formatted_rs_file", () => {
    const file = join(dir, "mal_formatado.rs");
    writeFileSync(file, 'fn   main( ){let x=1;println!("{}",x);}\n');
    const { code, output } = run("rustfmt", ["--check", "--edition", "2024", file]);
    expect(code).not.toBe(0);
    expect(output).toContain("mal_formatado.rs");
  });
});
