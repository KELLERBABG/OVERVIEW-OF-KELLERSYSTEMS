import { readFileSync } from "node:fs";
import { formatEuro } from "./pricing-format.mjs";

const cfg = JSON.parse(readFileSync("site.config.json", "utf8"));
const index = readFileSync("index.html", "utf8");
const pricing = readFileSync("pricing/index.html", "utf8");

const block = index.slice(index.indexOf("PRICING:START"), index.indexOf("PRICING:END"));

console.log("Config vs rendered surfaces\n");

let bad = 0;
for (const tier of cfg.tiers) {
  /* The euro string comes from the same formatter build-site.mjs renders with,
     so this compares the printed price, not a hand-built lookalike. */
  const price = formatEuro(tier.price);
  const inLanding = tier.price === 0 || block.includes(price);
  const inPricing = tier.price === 0 || pricing.includes(price);
  const annual = tier.annualPrice ? formatEuro(tier.annualPrice) : null;
  const annualOk = !annual || (block.includes(annual) && pricing.includes(annual));
  const checkoutOk = !tier.checkoutUrl
    || (block.includes(tier.checkoutUrl) && pricing.includes(tier.checkoutUrl));

  const ok = inLanding && inPricing && annualOk && checkoutOk;
  if (!ok) bad++;
  console.log(
    `${ok ? "ok  " : "FAIL"} ${tier.name.padEnd(18)} ${tier.price === 0 ? "kostenlos" : price.padEnd(8)}` +
    ` landing=${inLanding} pricing=${inPricing}` +
    ` jahr=${annual ? `${annual}=${annualOk}` : "n/a"}` +
    ` checkout=${tier.checkoutUrl ? checkoutOk : "n/a"}`
  );
}

/* No retired price or quota may survive anywhere. The values below are the ones
   the old USD model printed; a stale one means a surface was hand-edited instead
   of being regenerated from site.config.json. */
const stale = [
  "1.499 $", "$1,499", "1499", "2.750 $", "$2,750", "2750",
  "180 Projektprüfungen", "drei vollständige Projektprüfungen",
  "Testlizenz kostenlos", "20 Prüfungen", "15 Prüfungen",
  "US-Dollar", "149 €", "299 €", "€149", "€299", "149&nbsp;€", "299&nbsp;€",
];
console.log("\nStale price and quota strings:");
for (const s of stale) {
  const inIndex = index.includes(s);
  const inPricing = pricing.includes(s);
  if (inIndex || inPricing) { bad++; console.log(`  FAIL "${s}" still present (index=${inIndex} pricing=${inPricing})`); }
  else console.log(`  ok   "${s}" gone`);
}

/* The currency must be euro everywhere a price is printed. */
console.log("\nCurrency:");
for (const [label, text] of [["index.html", block], ["pricing/index.html", pricing]]) {
  const dollars = [...text.matchAll(/\$\s?\d/g)].map((m) => m[0]);
  if (dollars.length) { bad++; console.log(`  FAIL ${label} still prints ${dollars.length} dollar amount(s): ${[...new Set(dollars)].join(", ")}`); }
  else console.log(`  ok   ${label} prints euro only`);
}

console.log(`\n${bad === 0 ? "PRICES CONSISTENT" : bad + " PROBLEM(S)"}`);
process.exit(bad === 0 ? 0 : 1);

