import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("App", () => {
  it("t00_01_ca1_ui_renders_app_name", () => {
    mockIPC(() => "0.1.0");
    render(<App />);
    expect(screen.getByRole("heading", { name: "Limpador" })).toBeDefined();
  });

  it("shows the core version reported by the backend", async () => {
    mockIPC((cmd) => (cmd === "core_version" ? "9.8.7" : undefined));
    render(<App />);
    expect((await screen.findByTestId("core-version")).textContent).toBe("v9.8.7");
  });

  it("hides the version and reports the failure when the backend fails", async () => {
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => {});
    mockIPC(() => {
      throw new Error("núcleo indisponível");
    });
    render(<App />);

    await vi.waitFor(() => expect(consoleError).toHaveBeenCalledOnce());
    expect(consoleError.mock.calls[0]?.[0]).toBe("Falha ao consultar a versão do núcleo");
    expect(screen.queryByTestId("core-version")).toBeNull();
    consoleError.mockRestore();
  });
});
