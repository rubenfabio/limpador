// Thin, typed wrappers over the Tauri commands exposed by src-tauri.
import { invoke } from "@tauri-apps/api/core";

export function getCoreVersion(): Promise<string> {
  return invoke<string>("core_version");
}
