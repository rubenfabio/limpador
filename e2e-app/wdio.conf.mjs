// E2E of the real app: the Tauri binary (built with the `sandbox` feature) runs against
// a fake machine in a temp folder, driven through tauri-driver + msedgedriver (WebView2).
import { spawn, spawnSync } from "node:child_process";
import {
  copyFileSync,
  createWriteStream,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { connect } from "node:net";
import { homedir, tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";
import { SevereServiceError } from "webdriverio";
import { ensureDriver } from "./driver.mjs";

const repoRoot = resolve(import.meta.dirname, "..");
const meta = Boolean(process.env.LIMPADOR_E2E_META);
const reports = resolve(process.env.LIMPADOR_E2E_OUTPUT ?? join(repoRoot, "reports", "e2e-app"));
const application = join(repoRoot, "target", "debug", "limpador-app.exe");
const tauriDriverExe = join(homedir(), ".cargo", "bin", "tauri-driver.exe");

let tauriDriver;
let sandboxRoot;
let appLog;
let closing = false;

function safeName(text) {
  return text.replace(/[^\w.-]+/g, "_").slice(0, 120);
}

function listing(dir) {
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .map((e) => `${e.isDirectory() ? "[d]" : "   "} ${relative(dir, join(e.parentPath, e.name))}`)
    .sort()
    .join("\n");
}

/** Resolves once something accepts TCP connections on `port`, or rejects after `timeoutMs`. */
async function waitForPort(port, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const open = await new Promise((resolve) => {
      const socket = connect(port, "127.0.0.1");
      socket.once("connect", () => resolve(socket.end() && true));
      socket.once("error", () => resolve(false));
    });
    if (open) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(
    `tauri-driver não abriu a porta ${port} em ${timeoutMs / 1000}s. Veja ${appLog}.`,
  );
}

function stopDriver() {
  closing = true;
  tauriDriver?.kill();
  tauriDriver = undefined;
}

export const config = {
  runner: "local",
  host: "127.0.0.1",
  port: 4444,
  specs: [meta ? "./specs/meta/**/*.e2e.mjs" : "./specs/*.e2e.mjs"],
  maxInstances: 1,
  capabilities: [{ maxInstances: 1, "tauri:options": { application } }],
  logLevel: "warn",
  reporters: ["spec"],
  framework: "mocha",
  mochaOpts: { ui: "bdd", timeout: 60_000 },

  onPrepare: async () => {
    // WebdriverIO only logs errors thrown by hooks and carries on, which hides the real
    // cause behind "unable to connect". SevereServiceError aborts the run instead.
    try {
      mkdirSync(reports, { recursive: true });
      // Workers are forked after onPrepare and inherit this variable.
      process.env.LIMPADOR_NATIVE_DRIVER = await ensureDriver(
        join(repoRoot, ".cache", "msedgedriver"),
      );

      // Production UI bundle, even when the caller (e.g. Vitest) has NODE_ENV=test.
      const build = spawnSync(
        "pnpm",
        ["tauri", "build", "--debug", "--no-bundle", "--features", "sandbox"],
        {
          cwd: repoRoot,
          stdio: "inherit",
          shell: true,
          env: { ...process.env, NODE_ENV: "production" },
        },
      );
      if (build.status !== 0) {
        throw new Error("Falha ao compilar o app para o e2e (pnpm tauri build --debug).");
      }
    } catch (error) {
      throw new SevereServiceError(error.message);
    }
  },

  beforeSession: async () => {
    // A fresh fake machine per spec file; the app finds it through LIMPADOR_SANDBOX.
    sandboxRoot = mkdtempSync(join(tmpdir(), "limpador-e2e-app-"));
    appLog = `${sandboxRoot}.log`;
    const log = createWriteStream(appLog);

    tauriDriver = spawn(tauriDriverExe, ["--native-driver", process.env.LIMPADOR_NATIVE_DRIVER], {
      env: { ...process.env, LIMPADOR_SANDBOX: sandboxRoot },
      stdio: ["ignore", "pipe", "pipe"],
    });
    tauriDriver.stdout.pipe(log);
    tauriDriver.stderr.pipe(log);
    tauriDriver.on("exit", (code) => {
      if (!closing) console.error(`tauri-driver terminou inesperadamente (código ${code}).`);
    });
    await waitForPort(4444, 15_000);
  },

  afterTest: async (test, _context, { passed }) => {
    if (passed) return;
    // Everything needed to understand a failure without re-running it.
    const dir = join(reports, safeName(`${test.parent} ${test.title}`));
    mkdirSync(dir, { recursive: true });
    await browser.saveScreenshot(join(dir, "tela.png"));
    writeFileSync(join(dir, "maquina-falsa.txt"), listing(sandboxRoot));
    copyFileSync(appLog, join(dir, "app.log"));
  },

  afterSession: cleanUp,
};

/** Stops the driver and deletes this worker's fake machine. Safe to call twice. */
function cleanUp() {
  stopDriver();
  for (const path of [sandboxRoot, appLog]) {
    if (!path) continue;
    try {
      rmSync(path, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
    } catch {
      // The log may still be held briefly by the dying driver; it lives in the temp folder.
    }
  }
  sandboxRoot = undefined;
  appLog = undefined;
}

// afterSession does not run when the session fails to start; exit always does.
process.on("exit", cleanUp);
