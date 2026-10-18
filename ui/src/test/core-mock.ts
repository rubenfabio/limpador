// Unit-test helper: simulates the core commands with mockIPC and records every call.
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Settings } from "../core/api";

export interface Call {
  cmd: string;
  args: unknown;
}

export const freshInstall: Settings = {
  onboardingDone: false,
  roots: [],
  autoCleanSafe: false,
  startWithSystem: false,
};

export const configured: Settings = {
  ...freshInstall,
  onboardingDone: true,
  roots: ["C:\\Projetos"],
};

/**
 * Installs `responses` (a value, or a function of the arguments) and returns the calls made.
 * Events (`listen`/`emit`) are simulated too, so tests can emit analysis progress.
 */
export function simulateCore(responses: Record<string, unknown>): Call[] {
  const calls: Call[] = [];
  mockIPC(
    (cmd, args) => {
      calls.push({ cmd, args });
      if (!(cmd in responses)) throw new Error(`comando não simulado: ${cmd}`);
      const response = responses[cmd];
      return typeof response === "function" ? response(args) : response;
    },
    { shouldMockEvents: true },
  );
  return calls;
}
