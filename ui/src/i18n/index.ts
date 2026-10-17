// Minimal i18n on top of the browser's Intl APIs (ADR-0019): no dependencies, and the same
// key + params shape as the messages produced by the core (ADR-0014).
import { ptBR } from "./pt-BR";

/** Intl plural forms, plus an optional `zero` used for a count of exactly 0. */
export type PluralForms = Partial<Record<Intl.LDMLPluralRule, string>> & { other: string };
export type Dictionary = Record<string, string | PluralForms>;
export type Params = Record<string, string | number>;

/** A translatable message as the core sends it: a key plus named parameters. */
export interface CoreMessage {
  key: string;
  params: Record<string, string>;
}

const dictionaries: Record<string, Dictionary> = { "pt-BR": ptBR };
let locale = "pt-BR";

export function setLocale(next: string) {
  if (!(next in dictionaries)) throw new Error(`Idioma sem dicionário: ${next}`);
  locale = next;
}

export function getLocale() {
  return locale;
}

/** Text for `key`, with `{name}` placeholders filled from `params`. */
export function t(key: string, params: Params = {}): string {
  const entry = dictionaries[locale]?.[key];
  if (entry === undefined) {
    if (import.meta.env.DEV) console.warn(`Chave de tradução ausente: ${key}`);
    return key;
  }
  const template = typeof entry === "string" ? entry : pluralForm(entry, Number(params.count ?? 0));
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

function pluralForm(forms: PluralForms, count: number): string {
  // Portuguese treats 0 as singular ("0 item"); a dedicated zero form reads better.
  if (count === 0 && forms.zero) return forms.zero;
  return forms[new Intl.PluralRules(locale).select(count)] ?? forms.other;
}

/** Translates a message produced by the core (reasons, alerts, "como recriar"…). */
export function translate(message: CoreMessage): string {
  return t(message.key, message.params);
}

const GB = 1024 ** 3;
const MB = 1024 ** 2;

/** Sizes as Windows shows them: GB with one decimal, MB below 1 GB ("17,8 GB"). */
export function formatBytes(bytes: number): string {
  const [value, unit] = bytes >= GB ? [bytes / GB, "GB"] : [bytes / MB, "MB"];
  const number = new Intl.NumberFormat(locale, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(value);
  return `${number} ${unit}`;
}

const RELATIVE_UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 365 * 24 * 3600],
  ["month", 30 * 24 * 3600],
  ["day", 24 * 3600],
  ["hour", 3600],
  ["minute", 60],
];

/** "há 3 dias", "há 4 meses": the largest whole unit since `date`. */
export function formatRelative(date: Date, now: Date = new Date()): string {
  const seconds = Math.round((now.getTime() - date.getTime()) / 1000);
  const format = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  for (const [unit, size] of RELATIVE_UNITS) {
    if (Math.abs(seconds) >= size) return format.format(-Math.trunc(seconds / size), unit);
  }
  return format.format(0, "second");
}

/** Date and time in the user's time zone. */
export function formatDateTime(date: Date): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: "short", timeStyle: "short" }).format(date);
}
