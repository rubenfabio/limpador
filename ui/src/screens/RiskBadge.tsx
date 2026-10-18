import type { Risk } from "../core/api";
import { t } from "../i18n";

const ICONS: Record<Risk, string> = {
  safe: "🟢",
  moderate: "🟡",
  caution: "🔴",
  blocked: "⛔",
};

export function RiskBadge({ risk }: { risk: Risk }) {
  return (
    <span className={`risk risk-${risk}`} data-risk={risk}>
      {ICONS[risk]} {t(`risk.${risk}`)}
    </span>
  );
}
