import { execFileSync } from "node:child_process";
import { describe, expect, it } from "vitest";

// ADR-0003: the core must build and test without any UI or Tauri dependency.
function transitiveDependencies(packageName) {
  const metadata = JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1"], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    }),
  );
  const nameById = new Map(metadata.packages.map((p) => [p.id, p.name]));
  const nodeById = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const root = metadata.packages.find((p) => p.name === packageName);

  const seen = new Set();
  const stack = [root.id];
  while (stack.length > 0) {
    const id = stack.pop();
    for (const dep of nodeById.get(id).deps) {
      if (!seen.has(dep.pkg)) {
        seen.add(dep.pkg);
        stack.push(dep.pkg);
      }
    }
  }
  return [...seen].map((id) => nameById.get(id));
}

describe("core isolation", () => {
  it("t00_01_ca3_core_has_no_tauri_or_app_dependency", () => {
    const deps = transitiveDependencies("limpador-core");
    const forbidden = deps.filter((name) => name.startsWith("tauri") || name === "limpador-app");
    expect(forbidden).toEqual([]);
  });

  it("t00_01_ca3_core_tests_run_alone", () => {
    // Throws (failing the test) if the core cannot be tested in isolation.
    execFileSync("cargo", ["test", "-p", "limpador-core", "--quiet"], { stdio: "pipe" });
  });
});
