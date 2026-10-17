import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import type { AnalysisSummary, Settings } from "./core/api";
import { freshInstall, simulateCore } from "./test/core-mock";

afterEach(() => {
  cleanup();
  clearMocks();
  vi.restoreAllMocks();
});

const configured: Settings = { ...freshInstall, onboardingDone: true, roots: ["C:\\Projetos"] };
const emptyAnalysis: AnalysisSummary = {
  recoverable: 0,
  recoverableSafe: 0,
  spaceItems: 0,
  organizationItems: 0,
  protectedItems: 0,
  projects: 3,
  warnings: [],
  collectors: [],
};

describe("App", () => {
  it("t00_01_ca1_ui_renders_app_name", () => {
    simulateCore({ core_version: "0.1.0", get_settings: configured });
    render(<App />);
    expect(screen.getByRole("heading", { name: "Limpador" })).toBeDefined();
  });

  it("shows the core version reported by the backend", async () => {
    simulateCore({ core_version: "9.8.7", get_settings: configured });
    render(<App />);
    expect((await screen.findByTestId("core-version")).textContent).toBe("v9.8.7");
  });

  it("hides the version and reports the failure when the backend fails", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    simulateCore({ get_settings: configured });
    render(<App />);

    await vi.waitFor(() =>
      expect(consoleError).toHaveBeenCalledWith(
        "Falha ao consultar a versão do núcleo",
        expect.anything(),
      ),
    );
    expect(screen.queryByTestId("core-version")).toBeNull();
  });

  it("a fresh install opens the onboarding", async () => {
    simulateCore({ core_version: "0.1.0", get_settings: freshInstall, suggest_roots: [] });
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Boas-vindas ao Limpador" })).toBeDefined();
  });

  it("t02_01_ca4_skipping_shows_the_no_folders_state_with_a_way_to_configure", async () => {
    const calls = simulateCore({
      core_version: "0.1.0",
      get_settings: freshInstall,
      suggest_roots: [],
      skip_onboarding: { ...freshInstall, onboardingDone: true },
    });
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Pular por agora" }));

    expect(await screen.findByRole("heading", { name: "Nenhuma pasta configurada" })).toBeDefined();
    expect(calls.map((c) => c.cmd)).toContain("skip_onboarding");
    fireEvent.click(screen.getByRole("button", { name: "Configurar pastas" }));
    expect(await screen.findByRole("heading", { name: "Boas-vindas ao Limpador" })).toBeDefined();
  });

  it("t02_01_ca5_finishing_saves_the_choices_and_starts_the_first_analysis", async () => {
    const calls = simulateCore({
      core_version: "0.1.0",
      get_settings: freshInstall,
      suggest_roots: [{ path: "C:\\Projetos", projects: 3 }],
      complete_onboarding: configured,
      run_analysis: emptyAnalysis,
    });
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Começar" }));
    await screen.findByText("C:\\Projetos");
    fireEvent.click(screen.getByRole("button", { name: "Continuar" }));
    fireEvent.click(screen.getByRole("button", { name: "Continuar" }));
    fireEvent.click(screen.getByRole("button", { name: "Concluir e analisar" }));

    expect((await screen.findByTestId("projects")).textContent).toBe("3 projetos");
    const commands = calls.map((c) => c.cmd);
    expect(commands.indexOf("complete_onboarding")).toBeLessThan(commands.indexOf("run_analysis"));
    expect(calls.find((c) => c.cmd === "complete_onboarding")?.args).toEqual({
      choices: { roots: ["C:\\Projetos"], autoCleanSafe: true, startWithSystem: true },
    });
  });

  it("the analysis button runs a new analysis and shows its totals", async () => {
    simulateCore({
      core_version: "0.1.0",
      get_settings: configured,
      run_analysis: { ...emptyAnalysis, recoverable: 2 * 1024 ** 3, recoverableSafe: 1024 ** 3 },
    });
    render(<App />);
    expect(await screen.findByText("Ainda não houve nenhuma análise.")).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Analisar agora" }));

    await waitFor(() => expect(screen.getByTestId("recoverable").textContent).toBe("2,0 GB"));
    expect(screen.getByTestId("recoverable-safe").textContent).toBe("1,0 GB");
  });

  it("analysis warnings are shown translated", async () => {
    simulateCore({
      core_version: "0.1.0",
      get_settings: configured,
      run_analysis: {
        ...emptyAnalysis,
        warnings: [{ key: "roots.unavailable", params: { path: "E:\\Projetos" } }],
      },
    });
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Analisar agora" }));
    expect((await screen.findByRole("alert")).textContent).toBe(
      "A pasta E:\\Projetos não está disponível agora (disco desconectado?).",
    );
  });
});
