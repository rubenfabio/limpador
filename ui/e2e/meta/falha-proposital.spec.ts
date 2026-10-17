import { expect, test } from "@playwright/test";

// Fails on purpose. Only runs with LIMPADOR_E2E_META=1, from tools/e2e-ui.test.mjs,
// to prove that a failing e2e leaves a screenshot behind (T-00.03 CA3).
test("falha proposital", async ({ page }) => {
  await page.setContent("<h1>Tela da falha</h1>");
  await expect(page.getByRole("heading")).toHaveText("texto que não existe", { timeout: 1_000 });
});
