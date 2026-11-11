/**
 * The inject script's module seam, pinned.
 *
 * desktop/inject/ is one program in three parts (shell.js, licensing-ui.js,
 * boot.js) assembled by sources.mjs into the single IIFE the staging build
 * inlines. Sharing one scope is only safe while the halves consult each
 * other through their documented names, so these tests make the seam
 * mechanical instead of documentary:
 *
 *   shell.js         may consult licensing through exactly
 *                    assertUnlocked() / gated() — never its state, DOM or
 *                    flow names.
 *   licensing-ui.js  may reach the host through exactly invoke, announce and
 *                    isWorkhorse — never a workspace command, page API or
 *                    IPC method of the shell's own.
 *   boot.js          the composition root: the only part that names the
 *                    entry points of both halves and the only one that
 *                    dispatches at load.
 *
 * Plus the one cross-file product contract: every studio page action the gate
 * wraps must exist in studio.html, or the wrapper silently skips it and the
 * paid action runs unlocked.
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { INJECT_PARTS, injectScript } from "../desktop/inject/sources.mjs";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const read = (...parts) => readFileSync(join(root, ...parts), "utf8");

const shell = read("desktop", "inject", "shell.js");
const licensingUi = read("desktop", "inject", "licensing-ui.js");
const boot = read("desktop", "inject", "boot.js");

/* Identifiers the other half owns. Matched as whole words so prose can still
   say "licensing" or "the gate" without tripping the pin; the IPC method
   names and event names are matched as substrings, which is what a stray
   reference to them looks like. */
const LICENSING_INTERNALS = [
  "licenseState", "licenseDecision", "setLicenseState", "focusBeforePaywall",
  "paywallEl", "chipEl", "LICENSE_CSS", "licenseBuildPaywall", "licenseRenderChip",
  "licenseChipTone", "licenseLockedCopy", "licenseShowPaywall", "licenseHidePaywall",
  "licenseSetStatus", "licenseActivate", "licenseOpenCheckout",
  "enableLicensing", "enableLicenseHotkeys", "BauLensLicensing", "licensingConfig",
];
const SHELL_WORD_INTERNALS = [
  "addStatusStrip", "enableDragAndDrop", "enablePdfExport", "enableWorkspaceShell",
  "enableStudioActions", "routeFile", "feedPageInput", "fileNameOf", "exportNameFor",
  "openWorkspace", "openSingleFile", "checkPaths", "exportFile", "exportPlanSet",
  "exportPlan", "exportPdf", "whCall", "STUDIO_PAID_ACTIONS", "PLAN_EXTENSIONS",
  "looksLikePlanFile", "extensionOf",
];
const SHELL_SUBSTRING_INTERNALS = [
  "workhorse:", "window.Workhorse",
  "engine_info", "print_dossier", "pick_workspace_folder", "pick_save_path",
  "scan_workspace", "read_workspace_file", "write_workspace_file", "batch_check",
  "export_file", "dossier_pdf", "plan_svg", "plan_dxf", "plan_export",
];

test("Inject: the source list names the three parts, in order, and they exist", () => {
  assert.deepEqual(INJECT_PARTS, ["shell.js", "licensing-ui.js", "boot.js"]);
  for (const part of INJECT_PARTS) {
    assert.ok(read("desktop", "inject", part).length > 0, part + " is empty");
  }
});

test("Inject: shell.js consults licensing only through the gate", () => {
  const leaked = LICENSING_INTERNALS.filter((name) =>
    new RegExp("\\b" + name + "\\b").test(shell)
  );
  assert.deepEqual(leaked, [], "shell.js names licensing internals: " + leaked.join(", "));
  /* The seam the shell is allowed to use must actually be there. */
  assert.match(shell, /\bgated\(/, "shell.js does not use gated()");
  assert.match(shell, /\bassertUnlocked\(\)/, "shell.js does not use assertUnlocked()");
});

test("Inject: licensing-ui.js reaches the host only through invoke/announce/isWorkhorse", () => {
  const leakedWords = SHELL_WORD_INTERNALS.filter((name) =>
    new RegExp("\\b" + name + "\\b").test(licensingUi)
  );
  const leakedStrings = SHELL_SUBSTRING_INTERNALS.filter((name) => licensingUi.includes(name));
  const leaked = leakedWords.concat(leakedStrings);
  assert.deepEqual(leaked, [], "licensing-ui.js names shell internals: " + leaked.join(", "));
  assert.match(licensingUi, /\binvoke\(/, "licensing-ui.js does not use the invoke bridge");
  assert.match(licensingUi, /\bannounce\(/, "licensing-ui.js does not use announce()");
  assert.match(licensingUi, /\bisWorkhorse\b/, "licensing-ui.js does not use isWorkhorse");
});

test("Inject: boot.js is the composition root — both halves' entries, dispatch exactly once", () => {
  for (const entry of [
    "addStatusStrip", "enableLicensing", "enableDragAndDrop", "enablePdfExport",
    "enableWorkspaceShell", "enableStudioActions",
  ]) {
    assert.ok(new RegExp("\\b" + entry + "\\b").test(boot), "boot.js does not call " + entry);
  }
  /* Only boot.js dispatches at load, and the assembled script does it once. */
  assert.ok(!/document\.readyState/.test(shell), "shell.js must not dispatch at load");
  assert.ok(!/document\.readyState/.test(licensingUi), "licensing-ui.js must not dispatch at load");
  assert.equal(injectScript().match(/document\.readyState/g).length, 1, "one readyState dispatch");
});

test("Inject: the assembly is one strict IIFE wrapping the parts verbatim, in order", () => {
  const script = injectScript();
  assert.ok(script.startsWith('(function () {\n"use strict";\n'), "one IIFE, one strict mode");
  assert.ok(script.endsWith("\n})();\n"), "closed exactly once");
  const at = INJECT_PARTS.map((part) => script.indexOf(read("desktop", "inject", part)));
  assert.ok(at.every((i) => i > -1), "every part is in the assembly verbatim");
  assert.deepEqual(at, [...at].sort((a, b) => a - b), "parts appear in list order");
});

test("Inject: every gated studio action exists in studio.html, so the gate cannot skip it", () => {
  const page = read("studio.html");
  const listBlock = shell.match(/var STUDIO_PAID_ACTIONS = \[([^\]]*)\]/);
  assert.ok(listBlock, "shell.js declares STUDIO_PAID_ACTIONS");
  const gated = [...listBlock[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  assert.ok(gated.length >= 10, "the paid-action list looks truncated");
  for (const name of gated) {
    assert.ok(
      new RegExp("function\\s+" + name + "\\s*\\(").test(page),
      "studio.html does not define " + name + " — the wrapper would skip it and it would run unlocked"
    );
  }
});
