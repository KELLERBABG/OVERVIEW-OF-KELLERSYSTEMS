/**
 * Composition: when the desktop inject script starts and in what order.
 * Assembled last by `desktop/inject/sources.mjs` — the only part with
 * top-level execution, and it must stay last: the dispatch below reads
 * `invoke`, which shell.js assigns.
 *
 * No Tauri bridge (a page opened outside the app): start() wires nothing at
 * all, exactly as the whole-file guard used to — no strip, no listeners, no
 * paywall, no gate.
 */
  function start() {
    /* Without a shell there is nothing to wire — same no-op the whole-file
       guard used to give us, now owned by the composition root. */
    if (!invoke) return;
    addStatusStrip();
    enableLicensing();
    enableDragAndDrop();
    enablePdfExport();
    enableWorkspaceShell();
    enableStudioActions();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start);
  } else {
    start();
  }
