// Mutation testing of the interface (T-00.05). The per-file gate lives in tools/mutation.
/** @type {import('@stryker-mutator/api/core').PartialStrykerOptions} */
export default {
  // The Vitest plugin (built for Vitest 4) cannot read Vitest 5 results: every mutant
  // "survived" with zero tests run. The command runner runs the suite per mutant and
  // judges by exit code, which works with any Vitest version.
  testRunner: "command",
  commandRunner: { command: "pnpm exec vitest run --bail 1" },
  coverageAnalysis: "off",
  mutate: ["src/**/*.{ts,tsx}", "!src/**/*.test.{ts,tsx}", "!src/main.tsx"],
  reporters: ["clear-text", "progress", "json", "html"],
  jsonReporter: { fileName: "../reports/mutation/ts/mutation.json" },
  htmlReporter: { fileName: "../reports/mutation/ts/index.html" },
  tempDirName: ".stryker-tmp",
  cleanTempDir: "always",
  thresholds: { high: 90, low: 70, break: null },
  // Stryker rewrites tsconfig `extends`/`references` through the TypeScript JS API,
  // which TypeScript 7 (native) no longer ships. Our tsconfig uses neither, so the
  // rewrite is skipped by pointing at a file that does not exist.
  tsconfigFile: "sem-reescrita-de-tsconfig.json",
};
