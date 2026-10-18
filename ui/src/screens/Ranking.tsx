import { useVirtualizer } from "@tanstack/react-virtual";
import { useMemo, useRef, useState } from "react";
import type { Category, ItemView, Risk } from "../core/api";
import { useAnalysis } from "../core/queries";
import { formatBytes, formatRelative, t, translate } from "../i18n";
import { useSelection } from "../selection/Selection";
import { RiskBadge } from "./RiskBadge";

export type SortBy = "gain" | "size" | "idle";
export type Activity = "all" | "active" | "inactive";

export interface Filters {
  risks: Risk[];
  category: Category | "all";
  activity: Activity;
  text: string;
  showBlocked: boolean;
}

export const ALL_RISKS: Risk[] = ["safe", "moderate", "caution", "blocked"];
const CATEGORIES: Category[] = ["docker", "dependencies", "git", "ai_editors", "general_files"];
const ROW_HEIGHT = 44;

/** Items that pass the filters, in the chosen order. `items` arrive in safe-gain order. */
export function visibleItems(items: ItemView[], filters: Filters, sortBy: SortBy): ItemView[] {
  const text = filters.text.trim().toLowerCase();
  const kept = items.filter(
    (item) =>
      filters.risks.includes(item.risk) &&
      (filters.showBlocked || item.risk !== "blocked") &&
      (filters.category === "all" || item.category === filters.category) &&
      (filters.activity === "all" ||
        (filters.activity === "active"
          ? item.projectState === "active" || item.projectState === "pinned"
          : item.projectState === "inactive")) &&
      (text === "" ||
        item.name.toLowerCase().includes(text) ||
        item.resource.toLowerCase().includes(text)),
  );
  if (sortBy === "size") return [...kept].sort((a, b) => b.freeableSize - a.freeableSize);
  if (sortBy === "idle") {
    return [...kept].sort(
      (a, b) =>
        (a.lastActivity ?? Number.MAX_SAFE_INTEGER) - (b.lastActivity ?? Number.MAX_SAFE_INTEGER),
    );
  }
  return kept;
}

/** Full ranking (T-08.02): filters, alternative orders, selection, organization tab. */
export function Ranking() {
  const analysis = useAnalysis();
  const selection = useSelection();
  const [tab, setTab] = useState<"space" | "organization">("space");
  const [sortBy, setSortBy] = useState<SortBy>("gain");
  const [filters, setFilters] = useState<Filters>({
    risks: ALL_RISKS,
    category: "all",
    activity: "all",
    text: "",
    showBlocked: true,
  });
  const view = analysis.data;
  const items = useMemo(
    () => visibleItems(view ? view[tab] : [], filters, sortBy),
    [view, tab, filters, sortBy],
  );

  const toggleRisk = (risk: Risk) =>
    setFilters((f) => ({
      ...f,
      risks: f.risks.includes(risk) ? f.risks.filter((r) => r !== risk) : [...f.risks, risk],
    }));

  if (!view) return <p>{t("home.never_analyzed")}</p>;

  return (
    <section aria-labelledby="ranking-title">
      <h2 id="ranking-title">{t("ranking.title")}</h2>
      <div role="tablist">
        <button
          type="button"
          role="tab"
          aria-selected={tab === "space"}
          onClick={() => setTab("space")}
        >
          {t("ranking.tab_space", { count: view.space.length })}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={tab === "organization"}
          onClick={() => setTab("organization")}
        >
          {t("ranking.tab_organization", { count: view.organization.length })}
        </button>
      </div>

      <fieldset className="filters">
        <legend>{t("ranking.filters")}</legend>
        {ALL_RISKS.map((risk) => (
          <label key={risk}>
            <input
              type="checkbox"
              checked={filters.risks.includes(risk)}
              onChange={() => toggleRisk(risk)}
            />
            <RiskBadge risk={risk} />
          </label>
        ))}
        <label>
          {t("ranking.category")}
          <select
            value={filters.category}
            onChange={(e) =>
              setFilters((f) => ({ ...f, category: e.target.value as Filters["category"] }))
            }
          >
            <option value="all">{t("ranking.all")}</option>
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>
                {t(`category.${c}`)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t("ranking.activity")}
          <select
            value={filters.activity}
            onChange={(e) => setFilters((f) => ({ ...f, activity: e.target.value as Activity }))}
          >
            <option value="all">{t("ranking.all")}</option>
            <option value="active">{t("ranking.active")}</option>
            <option value="inactive">{t("ranking.inactive")}</option>
          </select>
        </label>
        <label>
          {t("ranking.search")}
          <input
            type="search"
            value={filters.text}
            onChange={(e) => setFilters((f) => ({ ...f, text: e.target.value }))}
          />
        </label>
        <label>
          <input
            type="checkbox"
            checked={filters.showBlocked}
            onChange={(e) => setFilters((f) => ({ ...f, showBlocked: e.target.checked }))}
          />
          {t("ranking.show_blocked")}
        </label>
        <label>
          {t("ranking.sort")}
          <select value={sortBy} onChange={(e) => setSortBy(e.target.value as SortBy)}>
            <option value="gain">{t("ranking.sort_gain")}</option>
            <option value="size">{t("ranking.sort_size")}</option>
            <option value="idle">{t("ranking.sort_idle")}</option>
          </select>
        </label>
      </fieldset>

      <button
        type="button"
        className="secondary"
        onClick={() => selection.add(items.filter((i) => i.risk !== "blocked").map((i) => i.id))}
      >
        {t("ranking.select_filtered")}
      </button>
      <p data-testid="ranking-count">{t("common.items", { count: items.length })}</p>
      <ItemList items={items} />
    </section>
  );
}

function ItemList({ items }: { items: ItemView[] }) {
  const parent = useRef<HTMLDivElement>(null);
  const selection = useSelection();
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parent.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 8,
    // Before the first measurement (and in tests, where nothing is laid out).
    initialRect: { width: 1000, height: 600 },
  });

  return (
    <div ref={parent} className="item-list" data-testid="item-list">
      <ul style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((row) => {
          const item = items[row.index];
          if (!item) return null;
          const blocked = item.risk === "blocked";
          return (
            <li
              key={item.id}
              data-testid="item-row"
              data-item={item.name}
              style={{ position: "absolute", top: row.start, height: ROW_HEIGHT, width: "100%" }}
            >
              <label>
                <input
                  type="checkbox"
                  aria-label={item.name}
                  checked={selection.isSelected(item.id)}
                  disabled={blocked}
                  onChange={() => selection.toggle(item.id)}
                />
                {blocked && <span aria-hidden>🔒</span>}
                <span className="name">{item.name}</span>
              </label>
              <span>{t(`category.${item.category}`)}</span>
              <span className="muted">{item.project ?? t("ranking.no_project")}</span>
              <span>{formatBytes(item.freeableSize)}</span>
              <RiskBadge risk={item.risk} />
              <span className="muted">
                {item.lastActivity !== null && formatRelative(new Date(item.lastActivity))}
              </span>
              <span className="muted">{item.reasons[0] && translate(item.reasons[0])}</span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
