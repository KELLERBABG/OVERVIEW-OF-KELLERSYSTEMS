/**
 * One formatter for every printed price.
 *
 *   node scripts/build-site.mjs      renders prices through this
 *   node scripts/check-prices.mjs    compares the rendered pages against it
 *
 * The two scripts used to compute the string independently, which is exactly how
 * a rendered "1.490 €" and a checked "$1,499" drift apart. Prices are quoted in
 * euro with German thousands separators, matching the rest of the German UI, and
 * whole euro amounts are printed without decimals.
 */
export function formatEuro(amount) {
  if (typeof amount !== "number" || !Number.isFinite(amount)) {
    throw new TypeError(`formatEuro expects a finite number, got ${amount}`);
  }
  const rounded = Math.round(amount * 100) / 100;
  const hasCents = Math.round(rounded * 100) % 100 !== 0;
  const text = rounded.toLocaleString("de-DE", {
    minimumFractionDigits: hasCents ? 2 : 0,
    maximumFractionDigits: hasCents ? 2 : 0,
  });
  return `${text} €`;
}
