// Fails on purpose. Only runs with LIMPADOR_E2E_META=1, from tools/e2e-app.test.mjs,
// to prove that a failing journey leaves screenshot, app log and fake-machine listing.
describe("meta", () => {
  it("falha proposital", async () => {
    await expect($("h1")).toHaveText("texto que não existe", { wait: 1_000 });
  });
});
