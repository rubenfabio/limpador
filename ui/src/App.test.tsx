import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("t00_01_ca1_ui_renders_app_name", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: "Limpador" })).toBeDefined();
  });
});
