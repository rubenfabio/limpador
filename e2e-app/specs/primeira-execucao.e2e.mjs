// Journey J1 (docs/estrategia-de-testes.md): first run suggests folders, the user confirms,
// the first analysis runs on the real core and the result appears.
describe("primeira execução (J1, T-02.01)", () => {
  it("t02_01_ca1_real_app_suggests_projetos_with_its_project_count", async () => {
    await $("button=Começar").click();

    const suggestion = await $("//label[contains(., 'Projetos')]");
    await expect(suggestion).toHaveText(expect.stringContaining("3 projetos"));
    await expect(suggestion.$("input")).toBeSelected();
  });

  it("t02_01_ca5_finishing_saves_and_runs_the_first_analysis_on_the_real_core", async () => {
    await $("button=Continuar").click();
    await $("button=Continuar").click();
    await $("button=Concluir e analisar").click();

    // The fake machine's projects have nothing to clean and it has no Docker or WSL.
    await expect($('[data-testid="recoverable"]')).toHaveText("0,0 MB");
    await expect($(".all-clean")).toHaveText("Tudo limpo! Nada para liberar agora.");
  });
});
