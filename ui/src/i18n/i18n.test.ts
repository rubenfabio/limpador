import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { formatBytes, formatDateTime, formatRelative, setLocale, t, translate } from "./index";

afterEach(() => vi.restoreAllMocks());

describe("textos e tradução (T-08.07)", () => {
  it("t08_07_ca1_sizes_use_gb_with_one_decimal_in_pt_br", () => {
    expect(formatBytes(19_112_000_000)).toBe("17,8 GB");
    expect(formatBytes(1024 ** 3)).toBe("1,0 GB");
    expect(formatBytes(512 * 1024 ** 2)).toBe("512,0 MB");
    expect(formatBytes(0)).toBe("0,0 MB");
  });

  it("t08_07_ca2_core_messages_are_translated_with_their_values", () => {
    expect(translate({ key: "app.version", params: { version: "0.1.0" } })).toBe("v0.1.0");
  });

  it("byte parameters from the core are shown as sizes", () => {
    expect(
      translate({
        key: "reason.vdisk_phantom",
        params: { size: "19112000000", used: "524288000" },
      }),
    ).toBe("O arquivo ocupa 17,8 GB, mas só 500,0 MB estão em uso dentro dele.");
    expect(translate({ key: "app.version", params: { version: "123" } })).toBe("v123");
  });

  it("t08_07_ca4_plural_forms_follow_the_count", () => {
    expect(t("common.items", { count: 1 })).toBe("1 item");
    expect(t("common.items", { count: 2 })).toBe("2 itens");
    expect(t("common.items", { count: 0 })).toBe("nenhum item");
  });

  it("missing keys show the key and warn", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(t("nao.existe")).toBe("nao.existe");
    expect(warn).toHaveBeenCalledWith("Chave de tradução ausente: nao.existe");
  });

  it("unknown placeholders are left visible", () => {
    expect(t("app.version")).toBe("v{version}");
  });

  it("relative times use the largest whole unit", () => {
    const now = new Date("2026-10-17T12:00:00Z");
    expect(formatRelative(new Date("2026-10-14T12:00:00Z"), now)).toBe("há 3 dias");
    expect(formatRelative(new Date("2026-06-17T12:00:00Z"), now)).toBe("há 4 meses");
    expect(formatRelative(now, now)).toBe("agora");
  });

  it("dates are shown in the local format", () => {
    expect(formatDateTime(new Date(2026, 9, 17, 9, 5))).toBe("17/10/2026, 09:05");
  });

  it("only languages with a dictionary can be selected", () => {
    expect(() => setLocale("xx")).toThrow("Idioma sem dicionário: xx");
    setLocale("pt-BR");
  });

  it("t08_07_ca3_no_visible_text_outside_the_translation_files", () => {
    const src = join(import.meta.dirname, "..");
    const files = readdirSync(src, { recursive: true, withFileTypes: true })
      .filter((e) => e.isFile() && /\.tsx$/.test(e.name) && !/\.test\.tsx$/.test(e.name))
      .map((e) => join(e.parentPath, e.name));

    const offenders = files.flatMap((file) => {
      const code = readFileSync(file, "utf8");
      // Text between tags; `;`, `=` and parentheses only appear in code (e.g. between TS generics).
      const jsxText = [...code.matchAll(/>([^<>{}();=]*\p{L}[^<>{}();=]*)</gu)].map((m) =>
        m[1]?.trim(),
      );
      const attributes = [
        ...code.matchAll(/\b(?:aria-label|title|placeholder|alt)="([^"]*\p{L}[^"]*)"/gu),
      ].map((m) => m[1]);
      return [...jsxText, ...attributes].map((text) => `${relative(src, file)}: "${text}"`);
    });
    expect(offenders).toEqual([]);
  });
});
