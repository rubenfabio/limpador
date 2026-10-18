import { expect, test } from "@playwright/test";
import { ipcCalls, simulateCore } from "./support/core-mock";
import { scenarios } from "./support/scenarios";

test("t02_01_ca1_first_run_suggests_folders_with_project_counts", async ({ page }) => {
  await simulateCore(page, scenarios.primeiraExecucao);
  await page.goto("/");
  await page.getByRole("button", { name: "Começar" }).click();

  await expect(page.getByRole("checkbox", { name: /C:\\Projetos/ })).toBeChecked();
  await expect(page.getByText("12 projetos", { exact: true })).toBeVisible();
  await expect(page.getByText("2 projetos", { exact: true })).toBeVisible();
});

test("t02_01_ca5_completing_saves_choices_and_runs_the_first_analysis", async ({ page }) => {
  await simulateCore(page, scenarios.primeiraExecucao);
  await page.goto("/");
  await page.getByRole("button", { name: "Começar" }).click();
  await page.getByRole("checkbox", { name: /repos/ }).uncheck();
  await page.getByRole("button", { name: "Adicionar outra pasta" }).click();
  await expect(page.getByText("D:\\Trabalho")).toBeVisible();
  await page.getByRole("button", { name: "Continuar" }).click();
  await page.getByRole("button", { name: "Continuar" }).click();
  await page.getByRole("button", { name: "Concluir e analisar" }).click();

  await expect(page.getByTestId("recoverable")).toHaveText("35,0 GB");
  const calls = await ipcCalls(page);
  const saved = calls.find((c) => c.cmd === "complete_onboarding");
  expect(saved?.args).toEqual({
    choices: {
      roots: ["C:\\Projetos", "D:\\Trabalho"],
      autoCleanSafe: true,
      startWithSystem: true,
    },
  });
  const order = calls.map((c) => c.cmd);
  expect(order.indexOf("complete_onboarding")).toBeLessThan(order.indexOf("analyze"));
});

test("t02_01_ca4_skipping_leads_to_the_no_folders_state", async ({ page }) => {
  await simulateCore(page, scenarios.primeiraExecucao);
  await page.goto("/");
  await page.getByRole("button", { name: "Pular por agora" }).click();

  await expect(page.getByRole("heading", { name: "Nenhuma pasta configurada" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Configurar pastas" })).toBeVisible();
});
