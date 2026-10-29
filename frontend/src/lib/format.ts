// Number/duration formatting for video metadata.

/** 1234 -> "1,234"; 1234567 -> "1.2M". YouTube-style compact counts. */
export function formatCount(n: number): string {
  if (n < 1000) return String(n);
  if (n < 1_000_000) {
    const m = n / 1000;
    return `${m >= 100 ? Math.round(m) : Math.round(m * 10) / 10}K`;
  }
  if (n < 1_000_000_000) {
    const m = n / 1_000_000;
    return `${Math.round(m * 10) / 10}M`;
  }
  const b = n / 1_000_000_000;
  return `${Math.round(b * 10) / 10}B`;
}

/** Seconds -> "1:05", "1:02:03". */
export function formatDuration(secs?: number | null): string {
  if (secs == null || secs < 0) return "0:00";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  const mm = h > 0 ? String(m).padStart(2, "0") : String(m);
  const ss = String(s).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}

/** ISO date -> "Oct 28, 2026". */
export function formatDate(iso?: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
}
