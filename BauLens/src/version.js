/**
 * Single source of truth for the product identity and version.
 *
 * The version is read from package.json so the CLI banner, the generated
 * dossier, the BCF export and the XBau XML can never drift apart again.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

export const PRODUCT = "BauLens";
export const VENDOR = "Keller Systems";

const here = dirname(fileURLToPath(import.meta.url));

function readVersion() {
  try {
    const pkg = JSON.parse(readFileSync(join(here, "..", "package.json"), "utf8"));
    return typeof pkg.version === "string" && pkg.version.trim() !== "" ? pkg.version : "0.0.0";
  } catch {
    return "0.0.0";
  }
}

export const VERSION = readVersion();

/** Cache-busting-free label for banners and headings, e.g. `BauLens v0.6.0`. */
export const PRODUCT_LABEL = `${PRODUCT} v${VERSION}`;

/** Engine label as printed on formal documents. */
export const ENGINE_LABEL = `${PRODUCT} Core v${VERSION}`;
