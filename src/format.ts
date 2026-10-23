import { locale, t } from "./i18n";

const oneDecimal = new Intl.NumberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const whole = new Intl.NumberFormat(locale, { maximumFractionDigits: 0 });

/** Explorer ile aynı hesap: 1024 tabanlı, "GB" etiketiyle. */
export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < t.units.length - 1) {
    value /= 1024;
    unit++;
  }
  const number = unit === 0 || value >= 100 ? whole.format(value) : oneDecimal.format(value);
  return `${number} ${t.units[unit]}`;
}

export const formatCount = (n: number) => whole.format(n);

const percent = new Intl.NumberFormat(locale, { style: "percent", maximumFractionDigits: 0 });

/** %0'a yuvarlanacak küçük paylar "<%1" olarak gösterilir. */
export const formatPercent = (ratio: number) =>
  ratio > 0 && ratio < 0.005 ? `<${percent.format(0.01)}` : percent.format(ratio);

export const formatSeconds = (ms: number) => t.seconds(oneDecimal.format(ms / 1000));

export function formatDate(ms: number): string {
  if (ms <= 0) return "—";
  return new Date(ms).toLocaleDateString(locale, { year: "numeric", month: "short", day: "numeric" });
}

export function joinPath(base: string, names: string[]): string {
  if (names.length === 0) return base;
  return (base.endsWith("\\") ? base : base + "\\") + names.join("\\");
}
