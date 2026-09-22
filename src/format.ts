let currency = "USD";

export function setCurrency(c: string) {
  currency = c || "USD";
}

const fmtCache = new Map<string, Intl.NumberFormat>();
function fmt(key: string, opts: Intl.NumberFormatOptions): Intl.NumberFormat {
  const k = currency + key;
  let f = fmtCache.get(k);
  if (!f) {
    try {
      f = new Intl.NumberFormat(undefined, { style: "currency", currency, ...opts });
    } catch {
      f = new Intl.NumberFormat(undefined, { style: "currency", currency: "USD", ...opts });
    }
    fmtCache.set(k, f);
  }
  return f;
}

/** Negative zero (Rust's empty f64 sum) would otherwise print as "-$0". */
const nz = (n: number) => (Object.is(n, -0) || !Number.isFinite(n) ? 0 : n);

/** Whole-unit currency: $1,234 */
export const money = (n: number) => fmt("0", { maximumFractionDigits: 0 }).format(nz(n));
/** Currency with cents: $1,234.56 */
export const moneyC = (n: number) => fmt("2", { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(nz(n));
/** Compact: $1.2K, $3.4M */
export const moneyK = (n: number) => {
  n = nz(n);
  const a = Math.abs(n);
  if (a >= 1_000_000) return fmt("m", { maximumFractionDigits: 1, notation: "compact" }).format(n);
  if (a >= 10_000) return fmt("k", { maximumFractionDigits: 0, notation: "compact" }).format(n);
  return money(n);
};
export const pct = (n: number, d = 0) => `${n.toFixed(d)}%`;

export function monthLabel(key: string): string {
  const [y, m] = key.split("-").map(Number);
  const mon = new Date(y, m - 1, 1).toLocaleString(undefined, { month: "short" });
  return m === 1 ? `${mon} '${String(y).slice(2)}` : mon;
}

export function monthLong(key: string): string {
  const [y, m] = key.split("-").map(Number);
  return new Date(y, m - 1, 1).toLocaleString(undefined, { month: "long", year: "numeric" });
}

export function todayISO(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export function monthsToText(m: number | null): string {
  if (m == null) return "never";
  if (m <= 0) return "done";
  const y = Math.floor(m / 12);
  const r = m % 12;
  if (y === 0) return `${r} mo`;
  if (r === 0) return `${y} yr`;
  return `${y} yr ${r} mo`;
}
