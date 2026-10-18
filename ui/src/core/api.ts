// Thin, typed wrappers over the Tauri commands exposed by src-tauri.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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

export type Risk = "safe" | "moderate" | "caution" | "blocked";
export type Category = "docker" | "dependencies" | "git" | "ai_editors" | "general_files";
export type ProjectState = "active" | "inactive" | "pinned";

/** One cleanable item, as src-tauri/src/view.rs sends it. */
export interface ItemView {
  id: string;
  name: string;
  kind: string;
  collector: string;
  category: Category;
  resource: string;
  isPath: boolean;
  apparentSize: number;
  freeableSize: number;
  sizeEstimated: boolean;
  risk: Risk;
  nature: "recreatable" | "user_content";
  reasons: CoreMessage[];
  recreate: CoreMessage | null;
  destination: "direct" | "quarantine" | null;
  automationEligible: boolean;
  protectedBy: CoreMessage | null;
  blockReason: CoreMessage | null;
  sideEffects: CoreMessage[];
  alerts: CoreMessage[];
  action: "remove" | "compact" | "manager_command";
  project: string | null;
  projectState: ProjectState | null;
  lastActivity: number | null;
  contains: string[];
  /** Folders the item groups (e.g. every node_modules of a monorepo). */
  parts: string[];
}

export interface CollectorView {
  id: string;
  state: string;
  message: CoreMessage | null;
  action: CoreMessage | null;
  durationMs: number;
  items: number;
  partial: boolean;
}

export interface ProjectView {
  root: string;
  name: string;
  state: ProjectState;
  idleDays: number | null;
  recoverable: number;
}

export interface DriveView {
  root: string;
  total: number;
  available: number;
}

export interface AnalysisView {
  analyzedAt: number;
  durationMs: number;
  cancelled: boolean;
  totals: {
    recoverable: number;
    recoverableSafe: number;
    recoverableModerate: number;
    recoverableCaution: number;
    byCategory: { category: Category; bytes: number }[];
    organizationItems: number;
  };
  space: ItemView[];
  organization: ItemView[];
  protected: ItemView[];
  collectors: CollectorView[];
  projects: ProjectView[];
  warnings: CoreMessage[];
  drives: DriveView[];
}

/** A collector's state change while an analysis runs. */
export interface ProgressEvent {
  collector: string;
  state: string;
  done: number | null;
  total: number | null;
}

/** Items of a collector that just finished, before the analysis ends. */
export interface ItemsEvent {
  collector: string;
  items: ItemView[];
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

export function analyze(): Promise<AnalysisView> {
  return invoke<AnalysisView>("analyze");
}

export function lastAnalysis(): Promise<AnalysisView | null> {
  return invoke<AnalysisView | null>("last_analysis");
}

export function cancelAnalysis(): Promise<void> {
  return invoke<void>("cancel_analysis");
}

/** Starts Docker Desktop; only ever called from a user's click (T-03.01). */
export function startDocker(): Promise<void> {
  return invoke<void>("start_docker");
}

export function onProgress(handler: (event: ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<ProgressEvent>("analysis-progress", (event) => handler(event.payload));
}

export function onItems(handler: (event: ItemsEvent) => void): Promise<UnlistenFn> {
  return listen<ItemsEvent>("analysis-items", (event) => handler(event.payload));
}

/** Native folder picker; `null` when the user cancels. */
export async function pickFolder(): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false });
  return typeof picked === "string" ? picked : null;
}
