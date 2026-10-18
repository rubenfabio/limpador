// Dashboard on the real app (T-08.01). Each spec file gets a fresh fake machine.
describe("painel no app real (T-08.01)", () => {
  it("t08_01_ca7_without_roots_the_dashboard_shows_the_empty_state", async () => {
    await $("button=Pular por agora").click();

    await expect($("h2=Nenhuma pasta configurada")).toBeDisplayed();
    await expect($("button=Analisar agora")).not.toBeDisplayed();
  });
});
