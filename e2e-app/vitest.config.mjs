import { defineConfig } from "vitest/config";

// Unit tests of the e2e helpers only; the journeys themselves run through WebdriverIO.
export default defineConfig({
  test: {
    include: ["*.test.mjs"],
  },
});
