import { readFileSync } from "node:fs";
import { resolve } from "node:path";

// Version declared in [workspace.package] of the root Cargo.toml, compiled into the core.
const coreVersion = readFileSync(resolve(import.meta.dirname, "..", "..", "Cargo.toml"), "utf8")
  .split("[workspace.package]")[1]
  .match(/^version = "(.+)"/m)[1];

describe("app real em sandbox (T-00.04)", () => {
  it("t00_04_ca1_app_shows_data_from_the_real_core", async () => {
    await expect($("h1")).toHaveText("Limpador");
    await expect($('[data-testid="core-version"]')).toHaveText(`v${coreVersion}`);
  });
});
