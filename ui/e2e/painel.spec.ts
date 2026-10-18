import { expect, test } from "@playwright/test";
import { ipcCalls, simulateCore } from "./support/core-mock";
import { scenarios } from "./support/scenarios";

test("t08_01_ca6_dashboard_totals_match_the_core_response", async ({ page }) => {
  await simulateCore(page, scenarios.analiseTipica);
  await page.goto("/");

  await expect(page.getByTestId("recoverable")).toHaveText("35,0 GB");
  await expect(page.getByTestId("recoverable-safe")).toHaveText("12,0 GB");
  await expect(page.getByTestId("recoverable-moderate")).toHaveText("20,0 GB");
  await expect(page.getByTestId("recoverable-caution")).toHaveText("3,0 GB");

  await page.getByRole("button", { name: "Limpar o seguro agora" }).click();
  await expect(page.getByTestId("summary-direct")).toHaveText(
    "Apagar agora: 1 item, 12,0 GB liberados na hora",
  );
  expect((await ipcCalls(page)).map((c) => c.cmd)).not.toContain("clean");
});
