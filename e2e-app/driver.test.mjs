import { describe, expect, it } from "vitest";
import { checkCompatibility, parseVersion } from "./driver.mjs";

describe("msedgedriver × WebView2", () => {
  it("parses the version out of command output", () => {
    expect(parseVersion("Microsoft Edge WebDriver 155.0.4283.60 (abc)")).toBe("155.0.4283.60");
    expect(parseVersion("    pv    REG_SZ    155.0.4283.60")).toBe("155.0.4283.60");
    expect(parseVersion("sem versão")).toBeUndefined();
  });

  it("accepts a driver with the same major version", () => {
    expect(checkCompatibility("155.0.4283.60", "155.0.4283.11")).toEqual({ ok: true });
  });

  it("t00_04_ca2_incompatible_driver_explains_the_mismatch_and_the_fix", () => {
    const result = checkCompatibility("155.0.4283.60", "154.0.1.2");
    expect(result.ok).toBe(false);
    expect(result.message).toContain("154.0.1.2");
    expect(result.message).toContain("155.0.4283.60");
    expect(result.message).toContain("incompatível");
    expect(result.message).toContain(".cache/msedgedriver");
  });

  it("t00_04_ca2_missing_webview2_is_reported_clearly", () => {
    const result = checkCompatibility(undefined, "155.0.4283.60");
    expect(result.ok).toBe(false);
    expect(result.message).toContain("WebView2 não encontrado");
  });

  it("unreadable driver version is reported", () => {
    expect(checkCompatibility("155.0.4283.60", undefined).ok).toBe(false);
  });
});
