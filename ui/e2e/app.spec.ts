import { expect, test } from "@playwright/test";
import { ipcCalls, simulateCore } from "./support/core-mock";
import { scenarios } from "./support/scenarios";

test("t00_03_ca1_interface_shows_data_from_the_simulated_core", async ({ page }) => {
  await simulateCore(page, scenarios.basico);
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Limpador" })).toBeVisible();
  await expect(page.getByTestId("core-version")).toHaveText("v0.1.0");
});

test("t00_03_ca2_test_sees_which_calls_the_interface_made", async ({ page }) => {
  await simulateCore(page, scenarios.basico);
  await page.goto("/");
  await expect(page.getByTestId("core-version")).toBeVisible();

  expect(await ipcCalls(page)).toEqual([{ cmd: "core_version", args: {} }]);
});

test("core failure keeps the interface usable", async ({ page }) => {
  await simulateCore(page, scenarios.nucleoIndisponivel);
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "Limpador" })).toBeVisible();
  await expect(page.getByTestId("core-version")).toHaveCount(0);
});
