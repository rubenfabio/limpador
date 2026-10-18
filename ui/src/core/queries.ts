// Data from the core through TanStack Query (ADR-0020).
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { CoreMessage } from "../i18n";
import {
  type AnalysisView,
  analyze,
  cancelAnalysis,
  getSettings,
  type ItemsEvent,
  type ItemView,
  lastAnalysis,
  onItems,
  onProgress,
  type ProgressEvent,
  startDocker,
} from "./api";

/** Key of the "analyze" mutation, so any screen can tell an analysis is running. */
export const analyzeKey = ["analyze"] as const;

export const keys = {
  settings: ["settings"] as const,
  analysis: ["analysis"] as const,
};

export function useSettings() {
  return useQuery({ queryKey: keys.settings, queryFn: getSettings });
}

export function useAnalysis() {
  return useQuery({ queryKey: keys.analysis, queryFn: lastAnalysis });
}

/** Runs an analysis. A cancelled one never replaces the previous result (T-08.06). */
export function useAnalyze() {
  const client = useQueryClient();
  return useMutation({
    mutationKey: analyzeKey,
    mutationFn: analyze,
    onSuccess: (view: AnalysisView) => {
      if (!view.cancelled) client.setQueryData(keys.analysis, view);
    },
  });
}

export function useCancelAnalysis() {
  return useMutation({ mutationFn: cancelAnalysis });
}

/** Fails with the core's message (e.g. `docker.start_failed`). */
export function useStartDocker() {
  return useMutation<void, CoreMessage>({ mutationFn: startDocker });
}

/**
 * Events of the running analysis while `active`, folded into a state that starts over with
 * each new analysis. `initial`, `subscribe` and `fold` must be stable (module level).
 */
function useLiveEvents<E, S>(
  active: boolean,
  initial: S,
  subscribe: (handler: (event: E) => void) => Promise<UnlistenFn>,
  fold: (state: S, event: E) => S,
): S {
  const [state, setState] = useState(initial);
  useEffect(() => {
    if (!active) return;
    setState(initial);
    let unlisten: (() => void) | undefined;
    let disposed = false;
    subscribe((event) => setState((current) => fold(current, event)))
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [active, initial, subscribe, fold]);
  return state;
}

const NO_PROGRESS: Record<string, ProgressEvent> = {};
const NO_ITEMS: Record<string, ItemView[]> = {};

const latestProgress = (current: Record<string, ProgressEvent>, event: ProgressEvent) => ({
  ...current,
  [event.collector]: event,
});

const collectorItems = (current: Record<string, ItemView[]>, event: ItemsEvent) => ({
  ...current,
  [event.collector]: event.items,
});

/** Latest state of each collector while `active`. */
export function useProgress(active: boolean): Record<string, ProgressEvent> {
  return useLiveEvents(active, NO_PROGRESS, onProgress, latestProgress);
}

/** Items of each collector that already finished, while `active` (T-08.06). */
export function usePartialItems(active: boolean): Record<string, ItemView[]> {
  return useLiveEvents(active, NO_ITEMS, onItems, collectorItems);
}
