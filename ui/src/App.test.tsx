import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { configured, freshInstall, simulateCore } from "./test/core-mock";
import { analysis } from "./test/fixtures";
import { renderApp } from "./test/render";

afterEach(() => {
  cleanup();
  clearMocks();
  vi.restoreAllMocks();
});

describe("App", () => {
  it("t00_01_ca1_ui_renders_app_name", async () => {
    simulateCore({ core_version: "0.1.0", get_settings: configured, last_analysis: null });
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Limpador" })).toBeDefined();
  });

  it("shows the core version reported by the backend", async () => {
    renderApp("/", { core_version: "9.8.7", get_settings: configured, last_analysis: null });
    expect((await screen.findByTestId("core-version")).textContent).toBe("v9.8.7");
  });

  it("hides the version and reports the failure when the backend fails", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    renderApp("/", {
      core_version: () => {
        throw new Error("núcleo indisponível");
      },
      get_settings: configured,
      last_analysis: null,
    });
    await vi.waitFor(() =>
      expect(consoleError).toHaveBeenCalledWith(
        "Falha ao consultar a versão do núcleo",
        expect.anything(),
      ),
    );
    expect(screen.queryByTestId("core-version")).toBeNull();
  });

  it("a fresh install opens the onboarding without the navigation", async () => {
    renderApp("/", { get_settings: freshInstall, suggest_roots: [] });
    expect(await screen.findByRole("heading", { name: "Boas-vindas ao Limpador" })).toBeDefined();
    expect(screen.queryByRole("navigation")).toBeNull();
  });

  it("t02_01_ca4_skipping_shows_the_no_folders_state_with_a_way_to_configure", async () => {
    const calls = renderApp("/", {
      get_settings: freshInstall,
      suggest_roots: [],
      skip_onboarding: { ...freshInstall, onboardingDone: true },
      last_analysis: null,
    });
    fireEvent.click(await screen.findByRole("button", { name: "Pular por agora" }));

    expect(await screen.findByRole("heading", { name: "Nenhuma pasta configurada" })).toBeDefined();
    expect(calls.map((c) => c.cmd)).toContain("skip_onboarding");
    fireEvent.click(screen.getByRole("button", { name: "Configurar pastas" }));
    expect(await screen.findByRole("heading", { name: "Boas-vindas ao Limpador" })).toBeDefined();
  });

  it("t02_01_ca5_finishing_saves_the_choices_and_starts_the_first_analysis", async () => {
    const calls = renderApp("/", {
      get_settings: freshInstall,
      suggest_roots: [{ path: "C:\\Projetos", projects: 3 }],
      complete_onboarding: configured,
      last_analysis: null,
      analyze: analysis(),
    });
    fireEvent.click(await screen.findByRole("button", { name: "Começar" }));
    await screen.findByText("C:\\Projetos");
    fireEvent.click(screen.getByRole("button", { name: "Continuar" }));
    fireEvent.click(screen.getByRole("button", { name: "Continuar" }));
    fireEvent.click(screen.getByRole("button", { name: "Concluir e analisar" }));

    expect(await screen.findByText("Tudo limpo! Nada para liberar agora.")).toBeDefined();
    const commands = calls.map((c) => c.cmd);
    expect(commands.indexOf("complete_onboarding")).toBeLessThan(commands.indexOf("analyze"));
    expect(calls.find((c) => c.cmd === "complete_onboarding")?.args).toEqual({
      choices: { roots: ["C:\\Projetos"], autoCleanSafe: true, startWithSystem: true },
    });
  });

  it("the navigation moves between dashboard, ranking and selection", async () => {
    renderApp("/", { get_settings: configured, last_analysis: analysis() });
    fireEvent.click(await screen.findByRole("link", { name: "Ranking" }));
    expect(await screen.findByRole("heading", { name: "Ranking" })).toBeDefined();
    fireEvent.click(screen.getByRole("link", { name: "Seleção" }));
    expect(await screen.findByRole("heading", { name: "Resumo da limpeza" })).toBeDefined();
    fireEvent.click(screen.getByRole("link", { name: "Painel" }));
    await waitFor(() => expect(screen.getByRole("heading", { name: "Visão geral" })).toBeDefined());
  });
});
