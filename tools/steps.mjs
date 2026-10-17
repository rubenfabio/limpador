// Command groups exposed by the root package.json (`pnpm test`, `pnpm check`).
// Each step is [label, command, args]; every step runs even if a previous one failed.

export const groups = {
  test: [
    ["core e app (cargo test)", "cargo", ["test", "--workspace"]],
    ["interface (vitest)", "pnpm", ["--filter", "ui", "test"]],
    ["ferramentas (vitest)", "pnpm", ["--filter", "tools", "test"]],
  ],
  check: [
    ["formatação Rust", "cargo", ["fmt", "--all", "--check"]],
    [
      "lint Rust (clippy)",
      "cargo",
      ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    ],
    ["lint e formatação TS (biome)", "pnpm", ["exec", "biome", "ci", "."]],
    ["tipos TS", "pnpm", ["--filter", "ui", "typecheck"]],
  ],
};
