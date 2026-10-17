// Command groups exposed by the root package.json (`pnpm test`, `pnpm check`).
// Each step is [label, command, args]; every step runs even if a previous one failed.

export const groups = {
  test: [
    ["core e app (cargo test)", "cargo", ["test", "--workspace"]],
    [
      "app em modo sandbox (cargo test)",
      "cargo",
      ["test", "-p", "limpador-app", "--features", "sandbox"],
    ],
    ["interface (vitest)", "pnpm", ["--filter", "ui", "test"]],
    ["e2e da interface (playwright)", "pnpm", ["--filter", "ui", "e2e"]],
    ["auxiliares do e2e do app (vitest)", "pnpm", ["--filter", "e2e-app", "test"]],
    ["e2e do app real (webdriver)", "pnpm", ["--filter", "e2e-app", "e2e"]],
    ["ferramentas (vitest)", "pnpm", ["--filter", "tools", "test"]],
    ["rastreabilidade CA → teste", "node", ["tools/traceability.mjs"]],
  ],
  check: [
    ["formatação Rust", "cargo", ["fmt", "--all", "--check"]],
    [
      "lint Rust (clippy)",
      "cargo",
      ["clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings"],
    ],
    ["lint e formatação TS (biome)", "pnpm", ["exec", "biome", "ci", "."]],
    ["tipos TS", "pnpm", ["--filter", "ui", "typecheck"]],
  ],
};
