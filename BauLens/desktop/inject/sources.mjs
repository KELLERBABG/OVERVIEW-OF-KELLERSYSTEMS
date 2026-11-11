/**
 * The inject script's source list — the one place that says what the
 * desktop-only behaviour shipped into the staged pages is made of.
 *
 * The parts are plain script fragments, not modules: they share one IIFE
 * scope exactly as the single file they came from did, so `shell.js` can
 * call gated() while `licensing-ui.js` calls announce() without any of the
 * ceremony an import graph would add. What makes that safe is that the seam
 * is pinned from both sides by test/inject-structure.test.js: the shell half
 * consults licensing through exactly assertUnlocked()/gated(), the licensing
 * half reaches back through exactly invoke/announce/isWorkhorse.
 *
 * Order matters twice over, and is part of the contract:
 *
 *   shell.js         host bridge and page wiring. First, because boot.js
 *                    reads `invoke`, which this part assigns.
 *   licensing-ui.js  state, gate, presentation, flows. Middle: its functions
 *                    are hoisted over the whole assembled scope either way,
 *                    but its top-level `var licensing = window…` must run
 *                    before any flow can consult it.
 *   boot.js          composition root: start() plus the readyState dispatch.
 *                    Last — the only part with top-level execution.
 *
 * Both consumers of the assembled script — scripts/build-desktop-assets.mjs
 * (what ships) and test/desktop_licensing.test.js (what is driven) — must
 * import this list rather than re-listing files, so the tested script and
 * the staged script cannot silently diverge.
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** The parts, in assembly order. */
export const INJECT_PARTS = ["shell.js", "licensing-ui.js", "boot.js"];

/**
 * Assembles the parts into the single IIFE that gets inlined into the staged
 * pages — one strict-mode scope, exactly as desktop.js was before the split.
 */
export function injectScript() {
  return (
    '(function () {\n"use strict";\n' +
    INJECT_PARTS.map((part) => readFileSync(join(dirname(fileURLToPath(import.meta.url)), part), "utf8")).join(
      "\n"
    ) +
    "\n})();\n"
  );
}
