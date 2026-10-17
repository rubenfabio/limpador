// Finds the installed WebView2 runtime and provides a matching msedgedriver,
// downloaded from Microsoft and cached per version.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const WEBVIEW2_CLIENT = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
const REGISTRY_KEYS = [
  `HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\${WEBVIEW2_CLIENT}`,
  `HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\${WEBVIEW2_CLIENT}`,
  `HKCU\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\${WEBVIEW2_CLIENT}`,
];

/** Extracts "155.0.4283.60" from any text that contains a dotted version. */
export function parseVersion(text) {
  return text.match(/(\d+\.\d+\.\d+\.\d+)/)?.[1];
}

/**
 * Chromium drivers only work with a browser of the same major version.
 * Returns `{ ok: true }` or `{ ok: false, message }` explaining how to fix it.
 */
export function checkCompatibility(webviewVersion, driverVersion) {
  if (!webviewVersion) {
    return {
      ok: false,
      message:
        "WebView2 não encontrado. Instale o Microsoft Edge WebView2 Runtime para rodar o e2e do app.",
    };
  }
  if (!driverVersion) {
    return { ok: false, message: "Não foi possível ler a versão do msedgedriver." };
  }
  const major = (v) => v.split(".")[0];
  if (major(webviewVersion) !== major(driverVersion)) {
    return {
      ok: false,
      message:
        `msedgedriver ${driverVersion} é incompatível com o WebView2 ${webviewVersion} ` +
        "(as versões principais precisam ser iguais). Apague a pasta .cache/msedgedriver " +
        "e rode o e2e de novo para baixar o driver da versão certa.",
    };
  }
  return { ok: true };
}

export function installedWebView2Version() {
  for (const key of REGISTRY_KEYS) {
    try {
      const out = execFileSync("reg", ["query", key, "/v", "pv"], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      });
      const version = parseVersion(out);
      if (version && version !== "0.0.0.0") return version;
    } catch {
      // Key absent: try the next location.
    }
  }
  return undefined;
}

export function driverVersion(driverPath) {
  try {
    return parseVersion(execFileSync(driverPath, ["--version"], { encoding: "utf8" }));
  } catch {
    return undefined;
  }
}

/** Returns the path of an msedgedriver compatible with the installed WebView2. */
export async function ensureDriver(cacheRoot) {
  const webview = installedWebView2Version();
  if (!webview) throw new Error(checkCompatibility(undefined, undefined).message);

  const dir = join(cacheRoot, webview);
  const exe = join(dir, "msedgedriver.exe");
  if (!existsSync(exe)) {
    await download(webview, dir);
  }

  const compatibility = checkCompatibility(webview, driverVersion(exe));
  if (!compatibility.ok) throw new Error(compatibility.message);
  return exe;
}

async function download(version, dir) {
  const url = `https://msedgedriver.microsoft.com/${version}/edgedriver_win64.zip`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(
      `Falha ao baixar o msedgedriver ${version} de ${url} (HTTP ${response.status}).`,
    );
  }
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const zip = join(dir, "edgedriver.zip");
  writeFileSync(zip, Buffer.from(await response.arrayBuffer()));
  try {
    // Windows' own bsdtar extracts zip files; a GNU tar earlier in PATH (Git Bash) cannot.
    const tar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
    execFileSync(tar, ["-xf", zip, "-C", dir]);
  } catch (error) {
    rmSync(dir, { recursive: true, force: true });
    throw new Error(`Falha ao extrair o msedgedriver ${version}: ${error.message}`);
  }
  rmSync(zip);
}
