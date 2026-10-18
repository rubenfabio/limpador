import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup, fireEvent, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { configured } from "../test/core-mock";
import { analysis, GB, item, typicalAnalysis } from "../test/fixtures";
import { renderApp } from "../test/render";

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("painel (T-08.01)", () => {
  it("t08_01_ca1_shows_the_recoverable_total_split_by_risk", async () => {
    renderApp("/", { get_settings: configured, last_analysis: typicalAnalysis() });

    expect((await screen.findByTestId("recoverable")).textContent).toBe("35,0 GB");
    expect(screen.getByTestId("recoverable-safe").textContent).toBe("12,0 GB");
    expect(screen.getByTestId("recoverable-moderate").textContent).toBe("20,0 GB");
    expect(screen.getByTestId("recoverable-caution").textContent).toBe("3,0 GB");
  });

  it("t08_01_ca2_clean_safe_now_opens_the_summary_with_only_safe_items", async () => {
    const calls = renderApp("/", { get_settings: configured, last_analysis: typicalAnalysis() });
    fireEvent.click(await screen.findByRole("button", { name: "Limpar o seguro agora" }));

    expect(await screen.findByRole("heading", { name: "Resumo da limpeza" })).toBeDefined();
    expect(screen.getByTestId("summary-direct").textContent).toBe(
      "Apagar agora: 1 item, 12,0 GB liberados na hora",
    );
    expect(screen.queryByTestId("summary-quarantine")).toBeNull();
    expect(calls.map((c) => c.cmd)).not.toContain("clean");
  });

  const stoppedDocker = analysis({
    collectors: [
      {
        id: "docker",
        state: "unavailable",
        message: { key: "docker.stopped", params: {} },
        action: { key: "docker.start", params: {} },
        durationMs: 5,
        items: 0,
        partial: false,
      },
    ],
  });

  it("t08_01_ca3_stopped_docker_gets_a_card_with_the_start_action", async () => {
    const calls = renderApp("/", {
      get_settings: configured,
      last_analysis: stoppedDocker,
      start_docker: null,
      analyze: typicalAnalysis(),
    });
    const card = await screen.findByTestId("collector-card-docker");
    expect(card.textContent).toContain("O Docker está parado.");
    fireEvent.click(within(card).getByRole("button", { name: "Iniciar o Docker" }));

    expect((await screen.findByTestId("recoverable")).textContent).toBe("35,0 GB");
    const commands = calls.map((c) => c.cmd);
    expect(commands.indexOf("start_docker")).toBeLessThan(commands.indexOf("analyze"));
  });

  it("a failure to start Docker is shown on its card", async () => {
    const calls = renderApp("/", {
      get_settings: configured,
      last_analysis: stoppedDocker,
      start_docker: () =>
        Promise.reject({ key: "docker.start_failed", params: { detail: "sem licença" } }),
    });
    const card = await screen.findByTestId("collector-card-docker");
    fireEvent.click(within(card).getByRole("button", { name: "Iniciar o Docker" }));

    expect((await within(card).findByRole("alert")).textContent).toBe(
      "Não foi possível iniciar o Docker: sem licença",
    );
    expect(calls.map((c) => c.cmd)).not.toContain("analyze");
  });

  it("t08_06_ca3_collector_errors_are_shown_legibly", async () => {
    renderApp("/", {
      get_settings: configured,
      last_analysis: analysis({
        collectors: [
          {
            id: "docker",
            state: "error",
            message: {
              key: "docker.command_failed",
              params: { command: "docker system df", detail: "pipe fechado" },
            },
            action: null,
            durationMs: 5,
            items: 0,
            partial: false,
          },
        ],
      }),
    });
    expect((await screen.findByTestId("collector-card-docker")).textContent).toContain(
      "O comando docker system df falhou: pipe fechado",
    );
  });

  it("t08_01_ca5_without_any_analysis_it_says_so", async () => {
    renderApp("/", { get_settings: configured, last_analysis: null });
    expect(await screen.findByText("Ainda não houve nenhuma análise.")).toBeDefined();
    expect(screen.queryByTestId("recoverable")).toBeNull();
  });

  it("t08_06_ca4_an_analysis_with_nothing_to_clean_says_all_clean", async () => {
    renderApp("/", { get_settings: configured, last_analysis: analysis() });
    expect(await screen.findByText("Tudo limpo! Nada para liberar agora.")).toBeDefined();
    const button = screen.getByRole("button", {
      name: "Limpar o seguro agora",
    }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it("lists drives, the top five items and organization items", async () => {
    const space = Array.from({ length: 7 }, (_, i) =>
      item({ name: `item-${i}`, freeableSize: (7 - i) * GB }),
    );
    renderApp("/", {
      get_settings: configured,
      last_analysis: analysis({
        space,
        organization: [item({ name: "feature/x", freeableSize: 0 })],
        drives: [{ root: "C:\\", total: 100 * GB, available: 40 * GB }],
        analyzedAt: new Date(2026, 9, 17, 9, 5).getTime(),
      }),
    });

    expect((await screen.findByTestId("drive-C:\\")).textContent).toBe(
      "C:\\ — 60,0 GB usados, 40,0 GB livres",
    );
    const top = screen.getByTestId("top-items");
    expect(within(top).getAllByRole("listitem")).toHaveLength(5);
    expect(top.textContent).toContain("item-0 — 7,0 GB");
    expect(screen.getByTestId("organization-count").textContent).toContain("1 item de organização");
    expect(screen.getByText("Última análise: 17/10/2026, 09:05")).toBeDefined();
  });

  it("analysis warnings are shown translated", async () => {
    renderApp("/", {
      get_settings: configured,
      last_analysis: analysis({
        warnings: [{ key: "roots.unavailable", params: { path: "E:\\Projetos" } }],
      }),
    });
    expect((await screen.findByRole("alert")).textContent).toBe(
      "A pasta E:\\Projetos não está disponível agora (disco desconectado?).",
    );
  });

  it("the analyze button runs a new analysis and shows its totals", async () => {
    const calls = renderApp("/", {
      get_settings: configured,
      last_analysis: null,
      analyze: typicalAnalysis(),
    });
    fireEvent.click(await screen.findByRole("button", { name: "Analisar agora" }));
    expect((await screen.findByTestId("recoverable")).textContent).toBe("35,0 GB");
    expect(calls.map((c) => c.cmd)).toContain("analyze");
  });

  it("t02_01_ca4_without_roots_it_invites_to_configure", async () => {
    renderApp("/", { get_settings: { ...configured, roots: [] }, suggest_roots: [] });
    expect(await screen.findByRole("heading", { name: "Nenhuma pasta configurada" })).toBeDefined();
    expect(
      screen.getByText("Escolha onde ficam seus projetos para o Limpador começar a analisar."),
    ).toBeDefined();
    expect(screen.queryByRole("button", { name: "Analisar agora" })).toBeNull();
  });
});
