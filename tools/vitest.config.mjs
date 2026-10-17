import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["**/*.test.mjs"],
    exclude: ["**/node_modules/**"],
    // Some tests call cargo, which can be slow on a cold cache.
    testTimeout: 300_000,
  },
});
