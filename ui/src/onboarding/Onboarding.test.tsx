import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { simulateCore } from "../test/core-mock";
import { Onboarding } from "./Onboarding";

afterEach(() => {
  cleanup();
  clearMocks();
});

function renderAtRoots(responses: Record<string, unknown>) {
  const onFinish = vi.fn();
  const onSkip = vi.fn();
  const calls = simulateCore(responses);
  render(<Onboarding onFinish={onFinish} onSkip={onSkip} />);
  fireEvent.click(screen.getByRole("button", { name: "Começar" }));
  return { onFinish, onSkip, calls };
}

function finish() {
  fireEvent.click(screen.getByRole("button", { name: "Continuar" }));
  fireEvent.click(screen.getByRole("button", { name: "Continuar" }));
  fireEvent.click(screen.getByRole("button", { name: "Concluir e analisar" }));
}

describe("onboarding (T-02.01)", () => {
  it("t02_01_ca1_suggested_folder_shows_how_many_projects_it_has", async () => {
    renderAtRoots({ suggest_roots: [{ path: "C:\\Projetos", projects: 12 }] });

    const checkbox = await screen.findByRole("checkbox", { name: /C:\\Projetos/ });
    expect((checkbox as HTMLInputElement).checked).toBe(true);
    expect(screen.getByText("12 projetos")).toBeDefined();
  });

  it("t02_01_ca2_without_suggestions_it_asks_for_a_folder", async () => {
    renderAtRoots({ suggest_roots: [] });
    expect(
      await screen.findByText("Nenhuma pasta com projetos foi encontrada. Adicione uma."),
    ).toBeDefined();
  });

  it("t02_01_ca3_a_folder_added_by_hand_is_saved_as_a_root", async () => {
    const { onFinish } = renderAtRoots({
      suggest_roots: [],
      "plugin:dialog|open": "D:\\Trabalho",
      validate_root: null,
    });
    fireEvent.click(await screen.findByRole("button", { name: "Adicionar outra pasta" }));
    await screen.findByText("D:\\Trabalho");

    finish();
    expect(onFinish).toHaveBeenCalledWith({
      roots: ["D:\\Trabalho"],
      autoCleanSafe: true,
      startWithSystem: true,
    });
  });

  it("an invalid folder shows why and is not added", async () => {
    renderAtRoots({
      suggest_roots: [],
      "plugin:dialog|open": "Z:\\sumiu",
      validate_root: () => {
        throw "roots.not_found";
      },
    });
    fireEvent.click(await screen.findByRole("button", { name: "Adicionar outra pasta" }));
    expect((await screen.findByRole("alert")).textContent).toBe("Essa pasta não existe.");
    expect(screen.queryByText("Z:\\sumiu")).toBeNull();
  });

  it("cancelling the folder picker changes nothing", async () => {
    const { calls } = renderAtRoots({ suggest_roots: [], "plugin:dialog|open": null });
    fireEvent.click(await screen.findByRole("button", { name: "Adicionar outra pasta" }));
    await vi.waitFor(() => expect(calls.map((c) => c.cmd)).toContain("plugin:dialog|open"));
    expect(calls.map((c) => c.cmd)).not.toContain("validate_root");
  });

  it("t02_01_ca5_unchecked_folders_and_options_are_respected", async () => {
    const { onFinish } = renderAtRoots({
      suggest_roots: [
        { path: "C:\\Projetos", projects: 3 },
        { path: "C:\\Users\\dev\\repos", projects: 1 },
      ],
    });
    fireEvent.click(await screen.findByRole("checkbox", { name: /repos/ }));
    fireEvent.click(screen.getByRole("button", { name: "Continuar" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /itens seguros/ }));
    fireEvent.click(screen.getByRole("button", { name: "Continuar" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Windows/ }));
    fireEvent.click(screen.getByRole("button", { name: "Concluir e analisar" }));

    expect(onFinish).toHaveBeenCalledWith({
      roots: ["C:\\Projetos"],
      autoCleanSafe: false,
      startWithSystem: false,
    });
  });

  it("t02_01_ca4_the_onboarding_can_be_skipped_at_any_step", async () => {
    const { onSkip } = renderAtRoots({ suggest_roots: [] });
    await screen.findByText("Nenhuma pasta com projetos foi encontrada. Adicione uma.");
    fireEvent.click(screen.getByRole("button", { name: "Pular por agora" }));
    expect(onSkip).toHaveBeenCalledOnce();
  });

  it("a failed search behaves like no suggestions", async () => {
    renderAtRoots({
      suggest_roots: () => {
        throw new Error("falhou");
      },
    });
    expect(
      await screen.findByText("Nenhuma pasta com projetos foi encontrada. Adicione uma."),
    ).toBeDefined();
  });
});
