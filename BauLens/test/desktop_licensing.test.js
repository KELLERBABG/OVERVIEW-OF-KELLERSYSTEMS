/**
 * Drives the real desktop inject script (desktop/inject/{shell,licensing-ui,boot}
 * assembled by sources.mjs) through its licensing flows against a minimal DOM,
 * using the actual src/licensing.js.
 *
 * The inject script is what ships into the staged app; these tests pin the
 * behaviour the paywall depends on: first start stamps and persists the trial,
 * a locked launch shows a non-dismissible paywall, activation unlocks it, the
 * key field cannot be double-submitted, and Escape respects dismissability.
 * On top: the entitlement gate on both hosts — the workhorse's commands, the
 * studio page's own action globals, hotkeys and drop — and the invariant that
 * makes the gate a pure predicate: a locked paywall can never be closed.
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import * as licensing from "../src/licensing.js";
import { injectScript } from "../desktop/inject/sources.mjs";

/* The very script the staging build inlines — assembled from the shared
   source list, not re-read from a file of its own, so what these tests drive
   and what ships into the app are the same bytes. */
const INJECT = injectScript();
const DAY = 24 * 60 * 60 * 1000;

/* studio.html's engine-reaching page actions, as the inject script's list must
   pin them. Both host and gate lists are exercised name by name: a name in one
   side but not the other fails loudly. */
const STUDIO_ACTIONS = [
  "runEvaluation",
  "handleStudioUpload",
  "handleBplanUpload",
  "parseTextAndRun",
  "parseJsonAndRun",
  "parseBplanTextAndApply",
  "downloadXbauXml",
  "downloadBcfJson",
  "downloadDossierHtml",
  "printDossier",
];

/* ------------------------------------------------------------------ *
 * Minimal DOM: only what the inject script touches at init and in the
 * licensing section. innerHTML is parsed flat (class/id/attrs), which
 * matches the paywall markup it is used for.
 * ------------------------------------------------------------------ */

class Elem {
  constructor(tag) {
    this.tagName = tag.toUpperCase();
    this.children = [];
    this.parent = null;
    this.attrs = {};
    this.listeners = {};
    this.style = { cssText: "" };
    this.textContent = "";
    this.value = "";
    this.disabled = false;
    this._classes = new Set();
    this._innerHTML = "";
    const self = this;
    this.classList = {
      add: (c) => { self._classes.add(c); self._syncClass(); },
      remove: (c) => { self._classes.delete(c); self._syncClass(); },
      contains: (c) => self._classes.has(c),
    };
  }
  set className(v) {
    this._classes = new Set(String(v).split(/\s+/).filter(Boolean));
    this._syncClass();
  }
  get className() { return [...this._classes].join(" "); }
  _syncClass() {}
  set innerHTML(html) {
    this._innerHTML = html;
    this.children = [];
    const tag = /<(\w+)([^>]*)>/g;
    let m;
    while ((m = tag.exec(html))) {
      const el = new Elem(m[1]);
      const attr = /(\w[\w-]*)(?:="([^"]*)")?/g;
      let a;
      const body = m[2];
      while ((a = attr.exec(body))) {
        if (a[1] === "class") el.className = a[2] || "";
        else if (a[1] === "id") el.id = a[2] || "";
        else el.attrs[a[1]] = a[2] !== undefined ? a[2] : true;
      }
      el.parent = this;
      this.children.push(el);
    }
  }
  get innerHTML() { return this._innerHTML; }
  get parentElement() { return this.parent; }
  appendChild(el) { el.parent = this; this.children.push(el); return el; }
  insertBefore(el, ref) {
    el.parent = this;
    const i = this.children.indexOf(ref);
    this.children.splice(i === -1 ? this.children.length : i, 0, el);
    return el;
  }
  setAttribute(k, v) { this.attrs[k] = String(v); if (k === "id") this.id = v; }
  getAttribute(k) { return k in this.attrs ? this.attrs[k] : null; }
  removeAttribute(k) { delete this.attrs[k]; }
  addEventListener(type, fn) { (this.listeners[type] = this.listeners[type] || []).push(fn); }
  dispatch(type, event) {
    const ev = event || { key: undefined, preventDefault() {} };
    /* Events bubble to ancestors, as in the browser: the paywall's Escape
       handler sits on the backdrop while focus is in the key field. */
    for (let el = this; el; el = el.parent) {
      (el.listeners[type] || []).forEach((fn) => fn(ev));
    }
  }
  focus() { /* activeElement tracking is not what these tests assert on */ }
  *walk() {
    for (const c of this.children) { yield c; yield* c.walk(); }
  }
  matches(sel) {
    if (sel.startsWith(".")) return this._classes.has(sel.slice(1));
    if (sel.startsWith("#")) return this.id === sel.slice(1);
    const attr = sel.match(/^(\w+)\[([\w-]+)="([^"]*)"\]$/);
    if (attr) return this.tagName === attr[1].toUpperCase() && this.attrs[attr[2]] === attr[3];
    return this.tagName === sel.toUpperCase();
  }
  querySelector(sel) {
    const chain = sel.split(/\s+/);
    let candidates = [...this.walk()];
    for (let i = 0; i < chain.length; i++) {
      const hit = candidates.find((e) => e.matches(chain[i]));
      if (!hit) return null;
      if (i === chain.length - 1) return hit;
      candidates = [...hit.walk()];
    }
    return null;
  }
}

/* A fresh document per boot: each test drives one independent app start. */
function makeDoc(options = {}) {
  const documentElement = new Elem("html");
  const doc = {
    head: documentElement.appendChild(new Elem("head")),
    body: documentElement.appendChild(new Elem("body")),
    documentElement,
    activeElement: null,
    readyState: "complete",
    listeners: {},
    createElement: (tag) => new Elem(tag),
    addEventListener(type, fn) { (this.listeners[type] = this.listeners[type] || []).push(fn); },
    getElementById(id) {
      for (const e of this.documentElement.walk()) if (e.id === id) return e;
      return null;
    },
    querySelector(sel) { return this.documentElement.querySelector(sel); },
  };
  if (options.host === "studio") {
    /* The studio host: no workhorse root. The page's own header print button —
       the one whose inline onclick enablePdfExport re-routes through the shell. */
    const printButton = doc.body.appendChild(new Elem("button"));
    printButton.setAttribute("onclick", "window.print()");
    return { doc, printButton };
  }
  /* The host page skeleton: workhorse root, header with the engine chip the
     licensing chip inserts itself before. */
  const root$ = doc.body.appendChild(new Elem("div"));
  root$.id = "workhorse";
  const header = root$.appendChild(new Elem("header"));
  const center = header.appendChild(new Elem("div"));
  center.className = "wh-header-center";
  const engineChip = center.appendChild(new Elem("span"));
  engineChip.className = "wh-chip";
  engineChip.id = "whEngineBadge";
  return { doc, center };
}

/* ------------------------------------------------------------------ *
 * Harness
 * ------------------------------------------------------------------ */

async function boot(options = {}) {
  const { doc, center, printButton } = makeDoc({ host: options.host });
  const writes = [];
  const commands = [];
  const pendingTimers = [];
  const tauriListeners = {};
  const windowListeners = [];
  const invoke = async (cmd, args) => {
    commands.push(cmd);
    if (cmd === "license_state_read") {
      if (options.readError) throw new Error("config dir unreadable");
      return options.stored !== undefined ? options.stored : null;
    }
    if (cmd === "license_state_write") { writes.push(args.state); return null; }
    if (cmd === "engine_info") return { transport: "test", engine: { result: { product: "BauLens", version: "0.0.0", counts: { total: 15 } } } };
    if (cmd === "open_url") { (options.opened || []).push(args.url); return null; }
    if (cmd === "scan_workspace") return { root: args.path, projects: [], fileCount: 0 };
    if (cmd === "batch_check") return { results: [], checked: 0, passed: 0, failed: 0 };
    return null;
  };

  const window = {
    addEventListener(type, fn, capture) { windowListeners.push({ type, fn, capture: !!capture }); },
    __TAURI__: {
      core: { invoke },
      event: {
        listen: async (type, fn) => { tauriListeners[type] = fn; },
      },
    },
    BauLensLicensing: licensing,
    BauLensLicensingConfig: options.config || {
      trialDays: 14,
      organizationId: "org_test",
      checkoutUrl: null,
      pricingUrl: "https://lens.kellersystems.dev/pricing/",
    },
  };
  /* The studio page wires its actions as globals; inline handlers resolve the
     name at call time. Install the page's side of that contract before the
     inject script runs, exactly as studio.html's own script does. */
  const pageCalls = [];
  if (options.host === "studio") {
    STUDIO_ACTIONS.forEach((name) => {
      window[name] = () => { pageCalls.push(name); };
    });
  }
  const context = vm.createContext({
    window,
    document: doc,
    console,
    File: class {},
    FileReader: class {},
    setTimeout: (fn, ms) => { pendingTimers.push({ fn, ms }); return pendingTimers.length; },
    clearTimeout: () => {},
  });
  vm.runInContext(INJECT, context, { filename: "inject.js" });
  /* Let the enableLicensing promise chain settle. */
  await new Promise(setImmediate);
  await new Promise(setImmediate);
  const flushTimers = async () => {
    while (pendingTimers.length) {
      const t = pendingTimers.shift();
      t.fn();
      await new Promise(setImmediate);
    }
  };
  /* A keydown as the browser delivers it: window capture listeners first, then
     document listeners unless a capture listener stopped propagation. This is
     the model the hotkey gate relies on. */
  const pressKey = (key, target) => {
    const event = {
      key,
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      target: target || doc.body,
      defaultPrevented: false,
      propagationStopped: false,
      preventDefault() { this.defaultPrevented = true; },
      stopPropagation() { this.propagationStopped = true; },
    };
    windowListeners
      .filter((l) => l.type === "keydown" && l.capture)
      .forEach((l) => l.fn(event));
    if (!event.propagationStopped) {
      (doc.listeners.keydown || []).forEach((fn) => fn(event));
    }
    return event;
  };
  /* The page's own document-level events (workhorse:* and the workhorse page's
     keydown handlers). */
  const fire = (type, detail) => {
    (doc.listeners[type] || []).forEach((fn) => fn({ detail: detail || {} }));
  };
  /* The native tauri://drag-drop event the shell registers at start. */
  const dropPaths = (paths) => {
    const fn = tauriListeners["tauri://drag-drop"];
    if (fn) fn({ payload: { paths: paths } });
  };
  const paywall = doc.getElementById("licensePaywall");
  return {
    writes,
    commands,
    doc,
    flushTimers,
    pressKey,
    fire,
    dropPaths,
    paywall,
    panel: paywall,
    input: paywall.querySelector(".lic-input"),
    activate: paywall.querySelector(".lic-activate"),
    dismiss: paywall.querySelector(".lic-dismiss"),
    buy: paywall.querySelector(".lic-buy"),
    status: paywall.querySelector(".lic-status"),
    title: paywall.querySelector(".lic-title"),
    chip: center
      ? center.querySelector("#whLicenseBadge")
      : doc.getElementById("desktopLicense"),
    window,
    pageCalls,
    printButton,
    keydown: (el, key) => el.dispatch("keydown", { key, preventDefault() {} }),
    settle: async () => { await new Promise(setImmediate); await new Promise(setImmediate); },
  };
}

function fetchOnce(impl) {
  const calls = [];
  globalThis.fetch = async (url, init) => { calls.push({ url, init }); return impl(); };
  return calls;
}
const polarOk = () => ({ ok: true, status: 200, json: async () => ({ status: "granted", id: "lic_1" }) });
const polar404 = () => ({ ok: false, status: 404, json: async () => ({ error: "ResourceNotFound" }) });

/* ------------------------------------------------------------------ *
 * Flows
 * ------------------------------------------------------------------ */

test("First start: trial is stamped, persisted immediately and shown on the chip", async () => {
  const h = await boot();
  assert.equal(h.writes.length, 1, "first start persists the stamp once");
  assert.equal(h.writes[0].license, null);
  assert.ok(Date.now() - h.writes[0].firstStartAt < 5000);
  assert.match(h.chip.textContent, /Testphase · noch 14 Tag/);
  assert.equal(h.paywall.classList.contains("is-on"), false);
});

test("Locked launch: paywall is up, cannot be dismissed, points at the pricing fallback", async () => {
  const opened = [];
  const h = await boot({
    stored: { version: 1, firstStartAt: Date.now() - 15 * DAY, license: null },
    config: { trialDays: 14, organizationId: "org_test", checkoutUrl: null, pricingUrl: "https://lens.kellersystems.dev/pricing/" },
    opened,
  });
  assert.equal(h.paywall.classList.contains("is-on"), true);
  assert.match(h.paywall.querySelector(".lic-eyebrow").textContent, /Testphase abgelaufen/);
  assert.match(h.title.textContent, /lizenzpflichtig/);
  assert.equal(h.dismiss.style.display, "none", "no dismiss control while locked");
  h.keydown(h.input, "Escape");
  assert.equal(h.paywall.classList.contains("is-on"), true, "Escape must not dismiss a locked paywall");
  h.buy.dispatch("click");
  assert.deepEqual(opened, ["https://lens.kellersystems.dev/pricing/"]);
});

test("Activation while locked: success unlocks, persists and closes", async () => {
  const calls = fetchOnce(polarOk);
  const h = await boot({ stored: { version: 1, firstStartAt: Date.now() - 15 * DAY, license: null } });
  h.input.value = "  KEY-1  ";
  h.activate.dispatch("click");
  await h.settle();
  assert.equal(calls.length, 1);
  assert.equal(JSON.parse(calls[0].init.body).key, "KEY-1", "trimmed before send");
  assert.equal(h.writes.at(-1).license.key, "KEY-1");
  assert.match(h.chip.textContent, /Lizenz aktiv/);
  await h.flushTimers();
  assert.equal(h.paywall.classList.contains("is-on"), false);
});

test("Activation with a rejected key: stays locked, marks the field, no state written", async () => {
  fetchOnce(polar404);
  const h = await boot({ stored: { version: 1, firstStartAt: Date.now() - 15 * DAY, license: null } });
  h.input.value = "bad";
  h.activate.dispatch("click");
  await h.settle();
  assert.equal(h.input.getAttribute("aria-invalid"), "true");
  assert.match(h.status.textContent, /unbekannt, widerrufen oder abgelaufen/);
  assert.equal(h.paywall.classList.contains("is-on"), true);
  assert.ok(!h.writes.some((w) => w.license), "no license persisted on rejection");
  await h.flushTimers();
  assert.equal(h.paywall.classList.contains("is-on"), true, "still locked after the timers");
});

test("Double submit: a second Enter while validating triggers no second request", async () => {
  let release;
  const calls = fetchOnce(() => new Promise((r) => { release = r; }));
  const h = await boot({ stored: { version: 1, firstStartAt: Date.now() - 15 * DAY, license: null } });
  h.input.value = "KEY-1";
  h.keydown(h.input, "Enter");
  await h.settle();
  h.keydown(h.input, "Enter");
  h.activate.dispatch("click");
  await h.settle();
  assert.equal(calls.length, 1, "exactly one validation in flight");
  release(polarOk());
  await h.settle();
  assert.equal(h.writes.filter((w) => w.license).length, 1, "one license write");
});

test("Offline with a cached license: honest message, paywall closes, cache kept", async () => {
  fetchOnce(() => { throw new Error("ECONNREFUSED"); });
  const cached = { key: "KEY-1", validatedAt: Date.now() - DAY, expiresAt: null };
  const h = await boot({ stored: { version: 1, firstStartAt: Date.now() - 40 * DAY, license: cached } });
  /* Valid cached license: chip licensed, paywall closed at launch. */
  assert.match(h.chip.textContent, /Lizenz aktiv/);
  assert.equal(h.paywall.classList.contains("is-on"), false);
  /* Re-activating while offline must not invalidate the cached license. */
  h.chip.dispatch("click");
  h.input.value = "KEY-OTHER";
  h.activate.dispatch("click");
  await h.settle();
  assert.match(h.status.textContent, /Keine Verbindung/);
  /* An offline attempt must neither clear nor replace the cached license. */
  assert.ok(!h.writes.some((w) => w.license && w.license.key === "KEY-OTHER"), "offline attempt never persisted");
  assert.ok(!h.writes.some((w) => w.license === null && w.firstStartAt), "cached license not cleared");
  await h.flushTimers();
  assert.equal(h.paywall.classList.contains("is-on"), false, "cached license still unlocks");
});

test("Trial chip opens a dismissible dialog; Escape closes it", async () => {
  const h = await boot({ stored: { version: 1, firstStartAt: Date.now() - 2 * DAY, license: null } });
  assert.equal(h.paywall.classList.contains("is-on"), false);
  h.chip.dispatch("click");
  assert.equal(h.paywall.classList.contains("is-on"), true);
  assert.match(h.title.textContent, /Lizenzschlüssel aktivieren/);
  h.keydown(h.input, "Escape");
  assert.equal(h.paywall.classList.contains("is-on"), false);
});

test("Inject-Skript schreibt 'Schlüssel' korrekt (Regression: ö statt ü)", () => {
  /* The UI strings ship as \u-escapes; \u00f6 (ö) instead of \u00fc (ü) reads as
     "Lizenzschlössel" and no compiler catches it. */
  assert.ok(!/Schl\\u00f6/.test(INJECT), "kein Schl\u00f6 statt Schl\u00fcssel im Inject-Skript");
  assert.ok(INJECT.includes("Schl\\u00fcssel"), "Schl\u00fcssel kommt vor");
});

/* ------------------------------------------------------------------ *
 * The entitlement gate: a locked app refuses work, it does not hide it
 * ------------------------------------------------------------------ */

const LOCKED = () => ({ version: 1, firstStartAt: Date.now() - 15 * 86400000, license: null });
const TRIAL = () => ({ version: 1, firstStartAt: Date.now() - 2 * 86400000, license: null });

test("Gate, locked boot: open-folder, check and export never reach the shell", async () => {
  const h = await boot({ stored: LOCKED() });
  h.fire("workhorse:open-folder");
  h.fire("workhorse:check", { files: ["C:\\Buero\\a.dxf"] });
  h.fire("workhorse:detail", { path: "C:\\Buero\\a.dxf" });
  h.fire("workhorse:export", { kind: "pdf", file: "C:\\Buero\\a.dxf" });
  h.fire("workhorse:export-plans", { files: ["C:\\Buero\\lageplan.dxf"] });
  h.fire("workhorse:statute", { ref: "§6 MBO" });
  h.fire("workhorse:simulate", { spec: { gebaeude: { hoehe_m: 7 } } });
  h.fire("workhorse:lands-matrix", { file: "C:\\Buero\\a.dxf" });
  await h.settle();
  for (const banned of ["pick_workspace_folder", "batch_check", "pick_save_path", "dossier_pdf", "plan_export", "export_file", "engine_method", "engine_check"]) {
    assert.ok(!h.commands.includes(banned), banned + " must not run while locked");
  }
  assert.equal(h.paywall.classList.contains("is-on"), true, "paywall is up");
});

test("Gate, trial boot: statute lookup, live repair simulation, and deviations export reach the shell", async () => {
  const h = await boot({ stored: TRIAL() });
  h.fire("workhorse:statute", { ref: "§6 Abs. 2 MBO", land: "BY" });
  h.fire("workhorse:simulate", { spec: { gebaeude: { hoehe_m: 7 } }, land: "BY", stadt: "MUENCHEN" });
  h.fire("workhorse:export", { kind: "deviations", file: "C:\\Buero\\a.dxf", land: "BY", stadt: "MUENCHEN" });
  await h.settle();
  assert.ok(h.commands.includes("engine_method"), "statute lookup uses engine_method");
  assert.ok(h.commands.includes("engine_check"), "live simulation uses engine_check");
  assert.ok(h.commands.includes("export_file"), "deviations export uses export_file");
});

test("Gate, locked boot: a dropped folder is not scanned", async () => {
  const h = await boot({ stored: LOCKED() });
  h.dropPaths(["C:\\Buero\\Projekte 2026\\2401_MFH"]);
  await h.settle();
  assert.ok(!h.commands.includes("scan_workspace"), "scan_workspace must not run while locked");
  assert.equal(h.paywall.classList.contains("is-on"), true);
});

test("Gate, locked boot: hotkeys are swallowed, the page never sees them", async () => {
  const h = await boot({ stored: LOCKED() });
  let pageHandled = 0;
  h.doc.listeners.keydown = (h.doc.listeners.keydown || []).concat(() => pageHandled++);
  const event = h.pressKey("1");
  assert.equal(event.defaultPrevented, true, "hotkey prevented");
  assert.equal(event.propagationStopped, true, "hotkey stopped before document");
  assert.equal(pageHandled, 0, "the page's own keydown handler must not run");
  assert.equal(h.paywall.classList.contains("is-on"), true);
});

test("Gate: typing the key field is input, not a shortcut", async () => {
  const h = await boot({ stored: LOCKED() });
  const input = h.paywall.querySelector(".lic-input");
  const event = h.pressKey("r", input);
  assert.equal(event.defaultPrevented, false, "letters in the license field must not be swallowed");
  assert.equal(event.propagationStopped, false);
});

test("Gate, trial boot: commands and hotkeys execute, paywall stays down", async () => {
  const h = await boot({ stored: TRIAL() });
  h.fire("workhorse:check", { files: ["C:\\Buero\\a.dxf"] });
  h.dropPaths(["C:\\Buero\\Projekte 2026\\2401_MFH"]);
  await h.settle();
  assert.ok(h.commands.includes("batch_check"), "check runs during the trial");
  assert.ok(h.commands.includes("scan_workspace"), "drop-scan runs during the trial");

  let pageHandled = 0;
  h.doc.listeners.keydown = (h.doc.listeners.keydown || []).concat(() => pageHandled++);
  const event = h.pressKey("1");
  assert.equal(event.defaultPrevented, false, "hotkey passes during the trial");
  assert.equal(pageHandled, 1, "the page receives the hotkey");
  assert.equal(h.paywall.classList.contains("is-on"), false);
});

test("Gate: an unlocked license also passes the gate", async () => {
  const h = await boot({
    stored: { version: 1, firstStartAt: Date.now() - 40 * 86400000, license: { key: "K", validatedAt: Date.now() - DAY, expiresAt: null } },
  });
  h.fire("workhorse:check", { files: ["C:\\Buero\\a.dxf"] });
  await h.settle();
  assert.ok(h.commands.includes("batch_check"));
  assert.equal(h.paywall.classList.contains("is-on"), false);
});

test("Unreadable storage: trial still starts in memory and the app stays usable", async () => {
  const h = await boot({ readError: true });
  assert.match(h.chip.textContent, /Testphase · noch 14 Tag/);
  assert.equal(h.paywall.classList.contains("is-on"), false);
});

test("Gate: a locked paywall cannot be closed — offline activation leaves it up", async () => {
  /* The invariant assertUnlocked() relies on: a cached-but-stale license means
     reason revalidation-needed (locked), and the offline activation branch is
     the one path that sets data-dismissible and hides by hand. It must not
     close a locked paywall, or refusals would become silent. */
  fetchOnce(() => { throw new Error("ECONNREFUSED"); });
  const h = await boot({
    stored: {
      version: 1,
      firstStartAt: Date.now() - 40 * DAY,
      license: { key: "KEY-OLD", validatedAt: Date.now() - 40 * DAY, expiresAt: null },
    },
  });
  assert.equal(h.paywall.classList.contains("is-on"), true, "locked boot shows the paywall");
  h.input.value = "KEY-NEW";
  h.activate.dispatch("click");
  await h.settle();
  await h.flushTimers();
  assert.match(h.status.textContent, /Keine Verbindung/);
  assert.equal(h.paywall.classList.contains("is-on"), true, "the offline path must not close a locked paywall");
  h.fire("workhorse:check", { files: ["C:\\Buero\\a.dxf"] });
  await h.settle();
  assert.ok(!h.commands.includes("batch_check"), "still refused after the offline attempt");
  assert.equal(h.paywall.classList.contains("is-on"), true);
});

/* ------------------------------------------------------------------ *
 * The studio host: its page wires buttons in page code, so the gate
 * wraps those globals — same predicate, one list, no per-handler branch
 * ------------------------------------------------------------------ */

test("Gate, studio locked boot: page actions and print never run, paywall is up", async () => {
  const h = await boot({ host: "studio", stored: LOCKED() });
  /* What every inline onclick/oninput handler in studio.html does: resolve the
     name at call time — here against the wrapper the inject script installed. */
  STUDIO_ACTIONS.forEach((name) => h.window[name]());
  h.printButton.onclick({ preventDefault() {} });
  await h.settle();
  assert.deepEqual(h.pageCalls, [], "no studio page action ran while locked");
  assert.ok(!h.commands.includes("print_dossier"), "print must not reach the shell");
  assert.equal(h.paywall.classList.contains("is-on"), true, "paywall is up");
});

test("Gate, studio trial boot: every page action runs and print reaches the shell", async () => {
  const h = await boot({ host: "studio", stored: TRIAL() });
  STUDIO_ACTIONS.forEach((name) => h.window[name]());
  h.printButton.onclick({ preventDefault() {} });
  await h.settle();
  assert.deepEqual(h.pageCalls, STUDIO_ACTIONS, "every studio action runs during the trial");
  assert.ok(h.commands.includes("print_dossier"), "print reaches the shell during the trial");
  assert.equal(h.paywall.classList.contains("is-on"), false);
});
