import { defineConfig, devices } from "@playwright/test";

// E2E of the interface alone: the production build is served by Vite and the Rust
// core is replaced by a simulated IPC (see e2e/support/core-mock.ts). No Tauri, no cargo.
const port = 4173;
const meta = Boolean(process.env.LIMPADOR_E2E_META);

export default defineConfig({
  testDir: "e2e",
  // Specs under e2e/meta fail on purpose; they only run when testing this setup itself.
  testIgnore: meta ? [] : ["**/meta/**"],
  outputDir: process.env.LIMPADOR_E2E_OUTPUT ?? "../reports/e2e-ui/results",
  reporter: meta
    ? "line"
    : [["list"], ["html", { outputFolder: "../reports/e2e-ui/html", open: "never" }]],
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  use: {
    baseURL: `http://localhost:${port}`,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  // Tauri renders with WebView2 on Windows, which is Chromium-based.
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: `pnpm build && pnpm exec vite preview --port ${port} --strictPort`,
    url: `http://localhost:${port}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    // Always a production bundle, like the one Tauri ships, even when the caller
    // (e.g. Vitest) has NODE_ENV=test — otherwise React runs in development mode.
    env: { NODE_ENV: "production" },
  },
});
