import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { configured } from "../test/core-mock";
import { analysis, GB, item, typicalAnalysis } from "../test/fixtures";
import { renderApp } from "../test/render";
import { ALL_RISKS, type Filters, visibleItems } from "./Ranking";

afterEach(() => {
  cleanup();
  clearMocks();
});

const rows = () => screen.getAllByTestId("item-row").map((r) => r.getAttribute("data-item"));
const allFilters: Filters = {
  risks: ALL_RISKS,
  category: "all",
  activity: "all",
  text: "",
  showBlocked: true,
};

function openRanking(view = typicalAnalysis()) {
  return renderApp("/ranking", { get_settings: configured, last_analysis: view });
}

describe("ranking (T-08.02)", () => {
  it("keeps the safe-gain order, with blocked items last", async () => {
    openRanking();
    await screen.findByRole("heading", { name: "Ranking" });
    expect(rows()).toEqual(["npm-cache", "node_modules-loja", "volume-db", "em-uso"]);
  });

  it("t08_02_ca1_risk_and_category_filters_combine", async () => {
    openRanking(
      analysis({
        space: [
          item({ name: "imagem", category: "docker", risk: "safe" }),
          item({ name: "cache", category: "dependencies", risk: "safe" }),
          item({ name: "volume", category: "docker", risk: "caution" }),
        ],
      }),
    );
    await screen.findByRole("heading", { name: "Ranking" });
    for (const risk of ["Moderado", "Cuidado", "Bloqueado"]) {
      fireEvent.click(screen.getByRole("checkbox", { name: new RegExp(risk) }));
    }
    fireEvent.change(screen.getByRole("combobox", { name: "Categoria" }), {
      target: { value: "docker" },
    });
    expect(rows()).toEqual(["imagem"]);
  });

  it("t08_02_ca2_selecting_all_filtered_skips_blocked_items", async () => {
    openRanking();
    await screen.findByRole("heading", { name: "Ranking" });
    fireEvent.click(screen.getByRole("button", { name: "Selecionar todos os filtrados" }));

    expect((screen.getByRole("checkbox", { name: "npm-cache" }) as HTMLInputElement).checked).toBe(
      true,
    );
    const blocked = screen.getByRole("checkbox", { name: "em-uso" }) as HTMLInputElement;
    expect(blocked.checked).toBe(false);
    expect(blocked.disabled).toBe(true);
  });

  it("t08_02_ca3_ten_thousand_items_render_only_what_is_visible", async () => {
    const space = Array.from({ length: 10_000 }, (_, i) => item({ name: `item-${i}` }));
    const started = performance.now();
    openRanking(analysis({ space }));
    await screen.findByText("10000 itens");
    expect(performance.now() - started).toBeLessThan(5_000);
    expect(screen.getAllByTestId("item-row").length).toBeLessThan(60);
  });

  it("t08_02_ca4_sorting_by_size_puts_the_biggest_first_whatever_the_risk", async () => {
    openRanking();
    await screen.findByRole("heading", { name: "Ranking" });
    fireEvent.change(screen.getByRole("combobox", { name: "Ordenar por" }), {
      target: { value: "size" },
    });
    expect(rows()[0]).toBe("em-uso");
  });

  it("t08_02_ca5_search_matches_name_or_path", async () => {
    openRanking(
      analysis({
        space: [
          item({ name: "node_modules", resource: "C:\\Projetos\\fix-INC-2033\\node_modules" }),
          item({ name: "outro", resource: "C:\\Projetos\\outro" }),
        ],
      }),
    );
    await screen.findByRole("heading", { name: "Ranking" });
    fireEvent.change(screen.getByRole("searchbox", { name: "Buscar" }), {
      target: { value: "fix-INC" },
    });
    expect(rows()).toEqual(["node_modules"]);
  });

  it("blocked items can be hidden and organization items have their own tab", async () => {
    openRanking(
      analysis({
        space: typicalAnalysis().space,
        organization: [item({ name: "feature/x", freeableSize: 0, category: "git" })],
      }),
    );
    await screen.findByRole("heading", { name: "Ranking" });
    fireEvent.click(screen.getByRole("checkbox", { name: "Mostrar bloqueados" }));
    expect(rows()).not.toContain("em-uso");

    fireEvent.click(screen.getByRole("tab", { name: "Organização (1)" }));
    expect(rows()).toEqual(["feature/x"]);
  });

  it("without an analysis it says so", async () => {
    renderApp("/ranking", { get_settings: configured, last_analysis: null });
    expect(await screen.findByText("Ainda não houve nenhuma análise.")).toBeDefined();
  });
});

describe("visibleItems", () => {
  const items = [
    item({ name: "ativo", projectState: "active", lastActivity: 300, freeableSize: 1 }),
    item({ name: "fixado", projectState: "pinned", lastActivity: null, freeableSize: 3 }),
    item({ name: "parado", projectState: "inactive", lastActivity: 100, freeableSize: 2 }),
  ];

  it("filters active (including pinned) and inactive projects", () => {
    const names = (activity: Filters["activity"]) =>
      visibleItems(items, { ...allFilters, activity }, "gain").map((i) => i.name);
    expect(names("active")).toEqual(["ativo", "fixado"]);
    expect(names("inactive")).toEqual(["parado"]);
    expect(names("all")).toEqual(["ativo", "fixado", "parado"]);
  });

  it("sorts by idle time (oldest first, undated last) and by size", () => {
    expect(visibleItems(items, allFilters, "idle").map((i) => i.name)).toEqual([
      "parado",
      "ativo",
      "fixado",
    ]);
    expect(visibleItems(items, allFilters, "size").map((i) => i.freeableSize)).toEqual([3, 2, 1]);
  });

  it("search ignores case and surrounding spaces", () => {
    expect(
      visibleItems(items, { ...allFilters, text: "  PARADO " }, "gain").map((i) => i.name),
    ).toEqual(["parado"]);
  });

  it("does not reorder the original list", () => {
    const copy = [...items];
    visibleItems(items, allFilters, "size");
    expect(items).toEqual(copy);
    expect(GB).toBeGreaterThan(0);
  });
});
