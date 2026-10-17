// Thin, typed wrappers over the Tauri commands exposed by src-tauri.
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { CoreMessage } from "../i18n";

export interface Settings {
  onboardingDone: boolean;
  roots: string[];
  autoCleanSafe: boolean;
  startWithSystem: boolean;
}

export interface RootSuggestion {
  path: string;
  projects: number;
}

export interface OnboardingChoices {
  roots: string[];
  autoCleanSafe: boolean;
  startWithSystem: boolean;
}

export interface AnalysisSummary {
  recoverable: number;
  recoverableSafe: number;
  spaceItems: number;
  organizationItems: number;
  protectedItems: number;
  projects: number;
  warnings: CoreMessage[];
  collectors: { id: string; state: string }[];
}

export function getCoreVersion(): Promise<string> {
  return invoke<string>("core_version");
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function suggestRoots(): Promise<RootSuggestion[]> {
  return invoke<RootSuggestion[]>("suggest_roots");
}

/** Resolves when the folder can be a root; rejects with a translation key otherwise. */
export function validateRoot(path: string): Promise<void> {
  return invoke<void>("validate_root", { path });
}

export function completeOnboarding(choices: OnboardingChoices): Promise<Settings> {
  return invoke<Settings>("complete_onboarding", { choices });
}

export function skipOnboarding(): Promise<Settings> {
  return invoke<Settings>("skip_onboarding");
}

export function runAnalysis(): Promise<AnalysisSummary> {
  return invoke<AnalysisSummary>("run_analysis");
}

/** Native folder picker; `null` when the user cancels. */
export async function pickFolder(): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false });
  return typeof picked === "string" ? picked : null;
}
