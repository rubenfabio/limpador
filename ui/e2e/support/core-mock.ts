import type { Page } from "@playwright/test";

/** Command name → value the simulated core returns (or `coreError(...)` to reject). */
export type CoreResponses = Record<string, unknown>;

export interface IpcCall {
  cmd: string;
  args: unknown;
}

const ERROR_MARKER = "__limpadorCoreError";

/** Makes the simulated core reject the command with `message`. */
export function coreError(message: string) {
  return { [ERROR_MARKER]: message };
}

/**
 * Replaces the Rust core with fixed responses before the page loads, mirroring what
 * `mockIPC` from `@tauri-apps/api/mocks` installs, and records every call.
 * Commands without a response reject, so a missing simulation fails loudly.
 */
export async function simulateCore(page: Page, responses: CoreResponses): Promise<void> {
  await page.addInitScript(
    ({ responses, marker }) => {
      const calls: { cmd: string; args: unknown }[] = [];
      const callbacks = new Map<number, (data: unknown) => void>();
      let nextId = 1;
      const w = window as unknown as Record<string, unknown>;

      w.__LIMPADOR_IPC_CALLS__ = calls;
      w.__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: unknown) => {
          calls.push({ cmd, args });
          if (!(cmd in responses)) {
            throw new Error(`Comando não simulado no e2e: ${cmd}`);
          }
          const response = responses[cmd];
          if (response && typeof response === "object" && marker in response) {
            throw new Error(String((response as Record<string, unknown>)[marker]));
          }
          return response;
        },
        transformCallback: (callback?: (data: unknown) => void, once = false) => {
          const id = nextId++;
          callbacks.set(id, (data) => {
            if (once) callbacks.delete(id);
            callback?.(data);
          });
          return id;
        },
        unregisterCallback: (id: number) => callbacks.delete(id),
        runCallback: (id: number, data: unknown) => callbacks.get(id)?.(data),
        callbacks,
        metadata: {
          currentWindow: { label: "main" },
          currentWebview: { windowLabel: "main", label: "main" },
        },
      };
    },
    { responses, marker: ERROR_MARKER },
  );
}

/** Every call the interface made to the simulated core, in order. */
export function ipcCalls(page: Page): Promise<IpcCall[]> {
  return page.evaluate(
    () => (window as unknown as { __LIMPADOR_IPC_CALLS__: IpcCall[] }).__LIMPADOR_IPC_CALLS__,
  );
}
