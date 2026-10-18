import { emit } from "@tauri-apps/api/event";
import { clearMocks } from "@tauri-apps/api/mocks";
import { act, cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { configured } from "../test/core-mock";
import { analysis, GB, item, typicalAnalysis } from "../test/fixtures";
import { renderApp } from "../test/render";

afterEach(() => {
  cleanup();
  clearMocks();
});

/** An analysis that only ends when the test says so. */
function pendingAnalysis() {
  let finish: (value: unknown) => void = () => {};
  const promise = new Promise((resolve) => {
    finish = resolve;
  });
  return { respond: () => promise, finish: (value: unknown) => act(() => finish(value)) };
}

describe("progresso da análise (T-08.06)", () => {
  it("shows each collector's live state while analyzing", async () => {
    const pending = pendingAnalysis();
    renderApp("/", { get_settings: configured, last_analysis: null, analyze: pending.respond });
    fireEvent.click(await screen.findByRole("button", { name: "Analisar agora" }));
    await screen.findByRole("heading", { name: "Analisando…" });

    await act(async () => {
      await emit("analysis-progress", {
        collector: "docker",
        state: "done",
        done: null,
        total: null,
      });
      await emit("analysis-progress", {
        collector: "wsl-disks",
        state: "analyzing",
        done: 1,
        total: 2,
      });
    });

    expect((await screen.findByTestId("progress-docker")).textContent).toBe("Docker: concluído");
    expect(screen.getByTestId("progress-wsl-disks").textContent).toBe(
      "Discos virtuais (WSL e Docker): analisando (1/2)",
    );
    expect(screen.getByTestId("progress-overall").textContent).toBe(
      "1 de 2 verificações concluídas",
    );
    await pending.finish(analysis());
    await waitFor(() => expect(screen.queryByRole("heading", { name: "Analisando…" })).toBeNull());
  });

  it("t08_06_ca1_a_finished_collectors_items_appear_before_the_analysis_ends", async () => {
    const pending = pendingAnalysis();
    renderApp("/", { get_settings: configured, last_analysis: null, analyze: pending.respond });
    fireEvent.click(await screen.findByRole("button", { name: "Analisar agora" }));
    await screen.findByRole("heading", { name: "Analisando…" });

    await act(async () => {
      await emit("analysis-items", {
        collector: "docker",
        items: [
          item({ name: "imagem-antiga", freeableSize: 2 * GB, risk: "moderate" }),
          item({ name: "em-uso", freeableSize: 9 * GB, risk: "blocked" }),
        ],
      });
      await emit("analysis-items", {
        collector: "wsl-disks",
        items: [item({ name: "Ubuntu", freeableSize: 3 * GB })],
      });
    });

    expect((await screen.findByTestId("progress-found")).textContent).toBe(
      "Até agora: 2 itens, 5,0 GB recuperáveis",
    );
    expect(screen.getByTestId("progress-items").textContent).toBe(
      "Ubuntu — 3,0 GB" + "imagem-antiga — 2,0 GB",
    );
    await pending.finish(analysis());
  });

  it("t08_06_ca2_cancelling_keeps_the_previous_result", async () => {
    const pending = pendingAnalysis();
    const calls = renderApp("/", {
      get_settings: configured,
      last_analysis: typicalAnalysis(),
      analyze: pending.respond,
      cancel_analysis: null,
    });
    fireEvent.click(await screen.findByRole("button", { name: "Analisar agora" }));
    fireEvent.click(await screen.findByRole("button", { name: "Cancelar análise" }));
    await pending.finish(analysis({ cancelled: true }));

    expect(calls.map((c) => c.cmd)).toContain("cancel_analysis");
    expect((await screen.findByTestId("recoverable")).textContent).toBe("35,0 GB");
  });

  it("the analyze button is disabled while an analysis runs", async () => {
    const pending = pendingAnalysis();
    renderApp("/", { get_settings: configured, last_analysis: null, analyze: pending.respond });
    const button = (await screen.findByRole("button", {
      name: "Analisar agora",
    })) as HTMLButtonElement;
    fireEvent.click(button);
    await screen.findByRole("heading", { name: "Analisando…" });
    expect(button.disabled).toBe(true);
    await pending.finish(analysis());
  });
});
