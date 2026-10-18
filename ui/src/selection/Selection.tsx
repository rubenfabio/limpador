// Items the user marked for cleaning, shared by every view (T-08.05).
import { createContext, type ReactNode, useCallback, useContext, useMemo, useState } from "react";

interface SelectionApi {
  selected: ReadonlySet<string>;
  isSelected: (id: string) => boolean;
  toggle: (id: string) => void;
  /** Adds every id (already selected ones stay). */
  add: (ids: string[]) => void;
  /** Replaces the selection. */
  replace: (ids: string[]) => void;
  clear: () => void;
}

const SelectionContext = createContext<SelectionApi | null>(null);

export function SelectionProvider({ children }: { children: ReactNode }) {
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set());

  const toggle = useCallback(
    (id: string) =>
      setSelected((current) => {
        const next = new Set(current);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      }),
    [],
  );
  const add = useCallback(
    (ids: string[]) => setSelected((current) => new Set([...current, ...ids])),
    [],
  );
  const replace = useCallback((ids: string[]) => setSelected(new Set(ids)), []);
  const clear = useCallback(() => setSelected(new Set()), []);

  const api = useMemo(
    () => ({ selected, isSelected: (id: string) => selected.has(id), toggle, add, replace, clear }),
    [selected, toggle, add, replace, clear],
  );
  return <SelectionContext.Provider value={api}>{children}</SelectionContext.Provider>;
}

export function useSelection(): SelectionApi {
  const api = useContext(SelectionContext);
  if (!api) throw new Error("useSelection fora de SelectionProvider");
  return api;
}
