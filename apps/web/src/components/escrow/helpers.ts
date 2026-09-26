/**
 * CanguPay — Escrow UI Formatting Helpers
 */

/**
 * Truncates an address or cryptographic hash for concise display.
 */
export function truncateHash(hash: string, start = 8, end = 6): string {
  if (!hash) return "—";
  if (hash.length <= start + end) return hash;
  return `${hash.slice(0, start)}...${hash.slice(-end)}`;
}

/**
 * Formats seconds difference into a human-readable countdown string.
 */
export function formatCountdown(diff: number): string {
  if (diff <= 0) return "0h 0m 0s";
  const hours = Math.floor(diff / 3600);
  const minutes = Math.floor((diff % 3600) / 60);
  const seconds = diff % 60;
  return `${hours}h ${minutes}m ${seconds}s`;
}

/** Keep significant decimals, but don't let an all-zero fractional part dominate the amount. */
export function splitDisplayAmount(amount: string): { whole: string; fraction: string | null } {
  const [whole, fraction] = amount.split(".");
  return { whole, fraction: fraction && !/^0+$/.test(fraction) ? fraction : null };
}
