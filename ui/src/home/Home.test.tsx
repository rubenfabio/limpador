import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AnalysisSummary, Settings } from "../core/api";
import { Home } from "./Home";

afterEach(cleanup);

const withRoots: Settings = {
  onboardingDone: true,
  roots: ["C:\\Projetos"],
  autoCleanSafe: false,
  startWithSystem: false,
};
const analysis: AnalysisSummary = {
  recoverable: 3 * 1024 ** 3,
  recoverableSafe: 1024 ** 3,
  spaceItems: 2,
  organizationItems: 0,
  protectedItems: 0,
  projects: 1,
  warnings: [],
  collectors: [],
};

function renderHome(props: Partial<Parameters<typeof Home>[0]> = {}) {
  const handlers = { onAnalyze: vi.fn(), onConfigure: vi.fn() };
  render(<Home settings={withRoots} analyzing={false} {...handlers} {...props} />);
  return handlers;
}

describe("Home", () => {
  it("t02_01_ca4_without_roots_it_invites_to_configure", () => {
    const { onConfigure } = renderHome({ settings: { ...withRoots, roots: [] } });

    expect(screen.getByRole("heading", { name: "Nenhuma pasta configurada" })).toBeDefined();
    expect(
      screen.getByText("Escolha onde ficam seus projetos para o Limpador começar a analisar."),
    ).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Configurar pastas" }));
    expect(onConfigure).toHaveBeenCalledOnce();
    expect(screen.queryByRole("button", { name: "Analisar agora" })).toBeNull();
  });

  it("before any analysis it says so and offers to analyze", () => {
    const { onAnalyze } = renderHome();

    expect(screen.getByRole("heading", { name: "Visão geral" })).toBeDefined();
    expect(screen.getByText("Ainda não houve nenhuma análise.")).toBeDefined();
    expect(screen.queryByTestId("recoverable")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Analisar agora" }));
    expect(onAnalyze).toHaveBeenCalledOnce();
  });

  it("while analyzing it shows the status and disables the button", () => {
    renderHome({ analyzing: true });

    expect(screen.getByRole("status").textContent).toBe("Analisando…");
    expect(screen.queryByText("Ainda não houve nenhuma análise.")).toBeNull();
    const button = screen.getByRole("button", { name: "Analisar agora" }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it("after an analysis it shows the labelled totals", () => {
    renderHome({ analysis });

    expect(screen.queryByText("Ainda não houve nenhuma análise.")).toBeNull();
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.getByText("Pode ser liberado")).toBeDefined();
    expect(screen.getByText("Disso, seguro")).toBeDefined();
    expect(screen.getByText("Projetos encontrados")).toBeDefined();
    expect(screen.getByTestId("recoverable").textContent).toBe("3,0 GB");
    expect(screen.getByTestId("recoverable-safe").textContent).toBe("1,0 GB");
    expect(screen.getByTestId("projects").textContent).toBe("1 projeto");
    const button = screen.getByRole("button", { name: "Analisar agora" }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);
  });

  it("each warning is listed", () => {
    renderHome({
      analysis: {
        ...analysis,
        warnings: [
          { key: "roots.unavailable", params: { path: "E:\\a" } },
          { key: "roots.unavailable", params: { path: "F:\\b" } },
        ],
      },
    });
    expect(screen.getAllByRole("alert").map((a) => a.textContent)).toEqual([
      "A pasta E:\\a não está disponível agora (disco desconectado?).",
      "A pasta F:\\b não está disponível agora (disco desconectado?).",
    ]);
  });
});
