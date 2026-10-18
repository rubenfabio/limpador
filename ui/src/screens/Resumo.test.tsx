import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { configured } from "../test/core-mock";
import { analysis, GB, item, typicalAnalysis } from "../test/fixtures";
import { renderApp } from "../test/render";

afterEach(() => {
  cleanup();
  clearMocks();
});

async function selectInRanking(view: ReturnType<typeof analysis>, names: string[]) {
  renderApp("/ranking", { get_settings: configured, last_analysis: view });
  await screen.findByRole("heading", { name: "Ranking" });
  for (const name of names) fireEvent.click(screen.getByRole("checkbox", { name }));
  fireEvent.click(screen.getByRole("link", { name: "Seleção" }));
  await screen.findByRole("heading", { name: "Resumo da limpeza" });
}

describe("resumo da seleção (T-08.05)", () => {
  it("t08_05_ca1_selection_made_in_the_ranking_reaches_the_summary", async () => {
    await selectInRanking(typicalAnalysis(), ["npm-cache"]);
    expect(screen.getByTestId("summary-direct").textContent).toBe(
      "Apagar agora: 1 item, 12,0 GB liberados na hora",
    );
  });

  it("t08_05_ca2_separates_direct_deletion_from_quarantine", async () => {
    await selectInRanking(typicalAnalysis(), ["npm-cache", "node_modules-loja", "volume-db"]);
    expect(screen.getByTestId("summary-direct").textContent).toBe(
      "Apagar agora: 2 itens, 32,0 GB liberados na hora",
    );
    expect(screen.getByTestId("summary-quarantine").textContent).toBe(
      "Quarentena: 1 item, 3,0 GB liberados quando a quarentena expirar",
    );
    expect(screen.getByTestId("summary-caution").textContent).toBe(
      "1 item pode conter dados que só existem ali.",
    );
  });

  it("t08_05_ca3_compaction_lists_what_it_will_interrupt", async () => {
    const disk = item({
      name: "docker-desktop (dados)",
      action: "compact",
      freeableSize: 12 * GB,
      sideEffects: [{ key: "effect.stops_docker", params: {} }],
    });
    await selectInRanking(analysis({ space: [disk] }), ["docker-desktop (dados)"]);
    const compact = screen.getByTestId("summary-compact");
    expect(compact.textContent).toContain("Compactar discos virtuais: cerca de 12,0 GB");
    expect(compact.textContent).toContain("Interrompe o Docker durante a compactação.");
    expect(screen.queryByTestId("summary-direct")).toBeNull();
  });

  it("nothing runs yet and the selection can be cleared", async () => {
    await selectInRanking(typicalAnalysis(), ["npm-cache"]);
    expect((screen.getByRole("button", { name: "Continuar" }) as HTMLButtonElement).disabled).toBe(
      true,
    );
    fireEvent.click(screen.getByRole("button", { name: "Limpar seleção" }));
    expect(await screen.findByText("Nenhum item selecionado.")).toBeDefined();
  });
});
