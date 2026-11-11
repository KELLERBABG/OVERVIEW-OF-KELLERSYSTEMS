/**
 * The licensing half of the desktop inject script: the 14-day trial, the
 * Polar key and the paywall. Assembled between shell.js and boot.js by
 * `desktop/inject/sources.mjs`.
 *
 * The decision comes from src/licensing.js — this file only reads and writes
 * the persisted state and draws what the decision says. One rule
 * implementation, because a second one here would be a second thing to keep
 * correct. It runs on window.BauLensLicensing against
 * window.BauLensLicensingConfig, both inlined at stage time by
 * scripts/build-desktop-assets.mjs; neither exists in the plain browser
 * build, and enableLicensing() degrades to a no-op without them.
 *
 * Structure, in dependency order. shell.js reaches into this file through
 * exactly two names — assertUnlocked() and gated() — and never touches
 * state or the paywall's DOM; this file reaches back through exactly three
 * shell names — invoke, announce, isWorkhorse. A structure test pins both
 * directions.
 *
 *   Data          licenseState (persisted) and licenseDecision (derived).
 *                 Both are written by setLicenseState() and by nothing
 *                 else: assign, persist, derive, draw — in that order.
 *   The gate      assertUnlocked() answers whether paid work may run;
 *                 gated() is the one place that applies it to a callable;
 *                 the camera hotkeys are its event-shaped application.
 *                 A locked app refuses instead of quietly running
 *                 underneath — and the refusal needs no drawing, because a
 *                 locked paywall is already on screen and cannot be closed
 *                 (licenseHidePaywall).
 *   Presentation  the paywall and the chip: build-once DOM (paywallEl,
 *                 chipEl, focusBeforePaywall) plus everything that draws
 *                 it. Reads state and decision, writes neither.
 *   Flows         boot and activation — the only callers of
 *                 setLicenseState(), composing data and presentation.
 *
 * Data flows one way: a user action reaches a flow, the flow transitions
 * the state, the decision is re-derived, presentation draws it. A shell
 * event goes the other way only as far as the gate, then on to the IPC.
 *
 * The IPC surface it uses (see desktop/src-tauri/src/lib.rs):
 *   license_state_read()                          -> state | null
 *   license_state_write({ state })                -> ()
 *   open_url({ url })                             -> ()   (platform opener)
 */
  /* -- Data ---------------------------------------------------------- *
   * licenseState is the persisted record, licenseDecision is derived from
   * it. setLicenseState() below is the only writer of both; everything
   * else reads.
   * ------------------------------------------------------------------ */

  var licensing = window.BauLensLicensing || null;
  var licensingConfig = window.BauLensLicensingConfig || {};
  var licenseState = null;
  var licenseDecision = null;

  /**
   * The one transition: every change to licenseState goes through here, and
   * licenseDecision is derived inside it — nothing else writes either.
   *
   * `persist` is false only where a write must not or cannot land: adopting
   * the stored state at boot (it already is the stored state) and the
   * in-memory trial when storage is unreadable.
   */
  function setLicenseState(next, persist) {
    licenseState = next;
    if (persist) {
      invoke("license_state_write", { state: next }).catch(function (error) {
        /* The decision still stands for this session; say so rather than
           pretending the write worked. */
        announce("Lizenzstatus konnte nicht gespeichert werden: " + error, "fail");
      });
    }
    licenseDecision = licensing.entitlement(next, Date.now(), {
      trialDays: licensingConfig.trialDays || undefined,
    });
    licenseBuildPaywall();
    licenseRenderChip();
    return licenseDecision;
  }

  /* -- The gate ------------------------------------------------------ *
   * The only thing sections 1–4b consult: the predicate for a mid-flow
   * step, gated() for a callable.
   * ------------------------------------------------------------------ */

  /**
   * Runs fn only while unlocked — the one place the gate is applied to a
   * function. The workspace commands and the studio's page globals register
   * through it; mid-flow steps (drop routing, print, the hotkeys) call the
   * predicate directly.
   */
  function gated(fn) {
    return function () {
      if (!assertUnlocked()) return;
      return fn.apply(this, arguments);
    };
  }

  /**
   * The entitlement gate. Returns true when paid functionality may run.
   *
   * A pure predicate: locked ⇒ the paywall is already on screen. A locked
   * paywall cannot be closed — licenseHidePaywall() refuses while the decision
   * is locked, and the launch path raises it in the same tick that decides —
   * so refusal does no drawing here, and re-showing (which would reset the key
   * field the user may be typing in) never happens.
   *
   * No decision yet (licensing still loading, or a host without the module)
   * is not a lock: those windows are milliseconds, and failing closed would
   * refuse actions with nothing on screen to explain why.
   */
  function assertUnlocked() {
    return !licenseDecision || licenseDecision.unlocked;
  }

  /**
   * The camera hotkeys (1/2/3/R) are bound by the workhorse page itself, on
   * document. Intercepting at window in the capture phase runs before every
   * document listener, so a locked app swallows the shortcut and surfaces the
   * paywall instead of moving a camera behind it. Editable targets are
   * left alone — digits typed into the license key field are input, not
   * shortcuts.
   */
  function enableLicenseHotkeys() {
    window.addEventListener(
      "keydown",
      function (event) {
        if (event.metaKey || event.ctrlKey || event.altKey) return;
        if (["1", "2", "3", "r", "R"].indexOf(event.key) === -1) return;
        var target = event.target;
        var tag = target && target.tagName ? String(target.tagName).toUpperCase() : "";
        if (tag === "INPUT" || tag === "TEXTAREA" || (target && target.isContentEditable)) return;
        /* Same gate as every other paid action: locked means swallowed —
           the paywall is already up and cannot be closed. */
        if (assertUnlocked()) return;
        event.preventDefault();
        event.stopPropagation();
      },
      true
    );
  }

  /* -- Presentation -------------------------------------------------- *
   * Draws what the decision says. Owns the view references; reads state and
   * decision, writes neither. The paywall is built once — every transition
   * asks, only the first one draws.
   * ------------------------------------------------------------------ */

  var paywallEl = null;
  var chipEl = null;
  var focusBeforePaywall = null;

  var LICENSE_CSS =
    ".lic-backdrop{position:fixed;inset:0;z-index:980;display:none;align-items:center;justify-content:center;" +
    "background:rgba(5,7,12,0.86);padding:1.5rem}" +
    ".lic-backdrop.is-on{display:flex}" +
    ".lic-panel{width:min(30rem,100%);background:var(--ink-800);border:1px solid var(--line-strong);" +
    "border-radius:var(--r-lg);box-shadow:var(--shadow-2);overflow:hidden}" +
    ".lic-head{padding:0.9rem 1.1rem;border-bottom:1px solid var(--line);background:var(--ink-700)}" +
    ".lic-eyebrow{font-family:var(--font-mono);font-size:0.62rem;letter-spacing:0.14em;" +
    "text-transform:uppercase;color:var(--fg-dim);margin:0 0 0.3rem}" +
    ".lic-title{margin:0;font-size:1.02rem;font-weight:600;color:var(--fg)}" +
    ".lic-body{padding:1rem 1.1rem;font-size:0.8rem;color:var(--fg-muted);line-height:1.55}" +
    ".lic-body p{margin:0 0 0.7rem}" +
    ".lic-body p:last-child{margin-bottom:0}" +
    ".lic-label{display:block;font-family:var(--font-mono);font-size:0.62rem;letter-spacing:0.08em;" +
    "text-transform:uppercase;color:var(--fg-dim);margin:0 0 0.3rem}" +
    ".lic-input{width:100%;box-sizing:border-box;background:var(--ink-900);color:var(--fg);" +
    "border:1px solid var(--line-strong);border-radius:var(--r-md);padding:0.45rem 0.6rem;" +
    "font-family:var(--font-mono);font-size:0.8rem}" +
    ".lic-input:focus-visible{outline:none;border-color:var(--accent);box-shadow:0 0 0 3px var(--accent-wash)}" +
    ".lic-input[aria-invalid=true]{border-color:var(--fail)}" +
    ".lic-actions{display:flex;gap:0.5rem;margin-top:0.8rem;flex-wrap:wrap}" +
    ".lic-status{min-height:1.1rem;margin-top:0.6rem;font-family:var(--font-mono);font-size:0.68rem;" +
    "color:var(--fg-dim)}" +
    ".lic-status.is-ok{color:var(--accent-hi)}" +
    ".lic-status.is-fail{color:var(--fail)}" +
    ".lic-foot{padding:0.6rem 1.1rem;border-top:1px solid var(--line);background:var(--ink-700);" +
    "font-family:var(--font-mono);font-size:0.62rem;color:var(--fg-dim);display:flex;" +
    "justify-content:space-between;gap:0.5rem;flex-wrap:wrap}" +
    ".lic-chip{cursor:pointer}" +
    ".lic-chip:hover{border-color:var(--line-strong)}" +
    ".lic-chip:focus-visible{outline:none;border-color:var(--accent);box-shadow:0 0 0 3px var(--accent-wash)}";

  function licenseBuildPaywall() {
    if (paywallEl) return;
    var style = document.createElement("style");
    style.textContent = LICENSE_CSS;
    document.head.appendChild(style);

    paywallEl = document.createElement("div");
    paywallEl.id = "licensePaywall";
    paywallEl.className = "lic-backdrop";
    paywallEl.innerHTML =
      '<div class="lic-panel" role="dialog" aria-modal="true" aria-labelledby="licTitle">' +
      '<div class="lic-head">' +
      '<p class="lic-eyebrow"></p>' +
      '<h2 class="lic-title" id="licTitle"></h2>' +
      "</div>" +
      '<div class="lic-body">' +
      '<p class="lic-copy"></p>' +
      '<label class="lic-label" for="licKeyInput">Lizenzschl\u00fcssel</label>' +
      '<input class="lic-input" id="licKeyInput" type="text" autocomplete="off" spellcheck="false" ' +
      'placeholder="z. B. 4f8c2a9e-\u2026" aria-describedby="licStatus">' +
      '<div class="lic-actions">' +
      '<button type="button" class="wh-btn wh-btn--primary lic-activate">Aktivieren</button>' +
      '<button type="button" class="wh-btn lic-buy">Lizenz kaufen</button>' +
      '<button type="button" class="wh-btn lic-dismiss">Sp\u00e4ter</button>' +
      "</div>" +
      '<p class="lic-status" id="licStatus" role="status" aria-live="polite"></p>' +
      "</div>" +
      '<div class="lic-foot">' +
      "<span>100 % lokale Pr\u00fcfung \u00b7 Gepr\u00fcft wird nur der Schl\u00fcssel, nie Ihre Planung</span>" +
      "<span>Kein Kauf n\u00f6tig zum Ansehen: Preise auf kellersystems.dev</span>" +
      "</div>" +
      "</div>";
    document.body.appendChild(paywallEl);

    paywallEl.querySelector(".lic-activate").addEventListener("click", licenseActivate);
    paywallEl.querySelector(".lic-buy").addEventListener("click", licenseOpenCheckout);
    paywallEl.querySelector(".lic-dismiss").addEventListener("click", licenseHidePaywall);
    paywallEl.querySelector(".lic-input").addEventListener("keydown", function (event) {
      if (event.key === "Enter") licenseActivate();
    });
    paywallEl.addEventListener("keydown", function (event) {
      /* Escape returns only when the state allows dismissal; a hard paywall is
         not a dialog the user can dismiss into a locked app. */
      if (event.key === "Escape") licenseHidePaywall();
    });
  }

  function licenseChipTone(decision) {
    if (decision.unlocked && decision.reason === "license") {
      return { text: "Lizenz aktiv", cls: "wh-chip wh-chip--accent lic-chip" };
    }
    if (decision.unlocked) {
      var soon = decision.trial.daysLeft <= 3;
      return {
        text: "Testphase \u00b7 noch " + decision.trial.daysLeft + " Tag(e)",
        cls: "wh-chip " + (soon ? "wh-chip--warn" : "") + " lic-chip",
      };
    }
    return { text: "Gesperrt \u00b7 Lizenz n\u00f6tig", cls: "wh-chip wh-chip--fail lic-chip" };
  }

  function licenseRenderChip() {
    var decision = licenseDecision;
    if (!decision) return;
    var tone = licenseChipTone(decision);

    if (isWorkhorse) {
      if (!chipEl) {
        chipEl = document.createElement("button");
        chipEl.type = "button";
        chipEl.id = "whLicenseBadge";
        chipEl.setAttribute("aria-haspopup", "dialog");
        chipEl.addEventListener("click", function () { licenseShowPaywall(true); });
        var anchor = document.querySelector(".wh-header-center .wh-chip");
        if (anchor && anchor.parentElement) anchor.parentElement.insertBefore(chipEl, anchor);
      }
      chipEl.className = tone.cls;
      chipEl.textContent = tone.text;
      chipEl.title = decision.reason === "license"
        ? "Lizenzschl\u00fcssel verwalten"
        : "Lizenzschl\u00fcssel eingeben \u2014 Testphase: noch " + decision.trial.daysLeft + " Tag(e)";
    } else {
      /* The studio has no header to extend: one extra segment in its strip. */
      var bar = document.getElementById("desktopStatus");
      if (!bar) return;
      if (!chipEl) {
        chipEl = document.createElement("span");
        chipEl.id = "desktopLicense";
        chipEl.style.cssText = "border-left:1px solid var(--line-strong);padding-left:0.5rem;" +
          "cursor:pointer";
        chipEl.addEventListener("click", function () { licenseShowPaywall(true); });
        bar.appendChild(chipEl);
      }
      chipEl.textContent = tone.text;
    }

  }

  function licenseLockedCopy(decision) {
    if (decision.reason === "revalidation-needed") {
      return {
        eyebrow: "Lizenzpr\u00fcfung f\u00e4llig",
        title: "Verbindung zum Aktualisieren der Lizenz erforderlich",
        body: "Die Lizenz wurde zuletzt vor mehr als " +
          (licensing.OFFLINE_GRACE_DAYS || 30) +
          " Tagen gepr\u00fcft. Bitte verbinden Sie das Internet und aktivieren Sie den Schl\u00fcssel erneut. " +
          "Ihre Projekte und Plandaten bleiben dabei unber\u00fchrt auf diesem Rechner.",
      };
    }
    if (decision.reason === "license-expired") {
      return {
        eyebrow: "Lizenz abgelaufen",
        title: "Die hinterlegte Lizenz ist abgelaufen",
        body: "Verl\u00e4ngern Sie die Lizenz \u00fcber den Kauf-Link oder pr\u00fcfen Sie einen neuen Schl\u00fcssel. " +
          "Der Projektordner und alle Pr\u00fcfergebnisse bleiben erhalten.",
      };
    }
    return {
      eyebrow: "Testphase abgelaufen",
      title: "BauLens ist jetzt lizenzpflichtig",
      body: "Die 14-t\u00e4gige Testphase ist am " +
        new Date(decision.trial.expiresAt).toLocaleDateString("de-DE") +
        " abgelaufen. Mit einer Lizenz arbeiten Sie weiter \u2014 ohne Installationsverlust, ohne Datenverlust. " +
        "Eine Nachforderung der Bauaufsicht kostet mehr.",
    };
  }

  function licenseShowPaywall(dismissible) {
    if (!paywallEl || !licenseDecision) return;
    var decision = licenseDecision;
    var locked = !decision.unlocked;
    var copy = locked
      ? licenseLockedCopy(decision)
      : {
          eyebrow: decision.reason === "license" ? "Lizenz" : "Testphase",
          title: decision.reason === "license" ? "Lizenzschl\u00fcssel verwalten" : "Lizenzschl\u00fcssel aktivieren",
          body: decision.reason === "license"
            ? "Der hinterlegte Schl\u00fcssel ist aktiv. Erneutes Aktivieren ersetzt ihn."
            : "Die Testphase l\u00e4uft noch " + decision.trial.daysLeft +
              " Tag(e). Ein Schl\u00fcssel schaltet die Anwendung dauerhaft frei \u2014 der Rest der Testphase verf\u00e4llt dabei nicht.",
        };

    paywallEl.querySelector(".lic-eyebrow").textContent = copy.eyebrow;
    paywallEl.querySelector(".lic-title").textContent = copy.title;
    paywallEl.querySelector(".lic-copy").textContent = copy.body;
    paywallEl.querySelector(".lic-buy").style.display = locked || decision.reason === "trial" ? "" : "none";
    paywallEl.querySelector(".lic-dismiss").style.display = locked ? "none" : "";
    paywallEl.querySelector(".lic-dismiss").disabled = false;
    paywallEl.querySelector(".lic-activate").disabled = false;
    paywallEl.querySelector(".lic-status").textContent = "";
    paywallEl.querySelector(".lic-status").className = "lic-status";
    var input = paywallEl.querySelector(".lic-input");
    input.value = "";
    input.removeAttribute("aria-invalid");

    if (!paywallEl.classList.contains("is-on")) {
      focusBeforePaywall = document.activeElement;
      paywallEl.classList.add("is-on");
    }
    paywallEl.setAttribute("data-dismissible", dismissible && !locked ? "true" : "false");
    input.focus();
  }

  function licenseHidePaywall() {
    if (!paywallEl) return;
    /* A locked paywall is permanent: every hide path passes through here, and
       that invariant is what lets assertUnlocked() be a pure predicate. The
       offline-activation branch below sets data-dismissible before hiding, so
       the dismissibility check alone would not hold it. */
    if (licenseDecision && !licenseDecision.unlocked) return;
    if (paywallEl.getAttribute("data-dismissible") !== "true") return;
    paywallEl.classList.remove("is-on");
    if (focusBeforePaywall && typeof focusBeforePaywall.focus === "function") {
      focusBeforePaywall.focus();
    }
  }

  function licenseSetStatus(text, tone) {
    var status = paywallEl.querySelector(".lic-status");
    status.textContent = text || "";
    status.className = "lic-status" + (tone ? " is-" + tone : "");
  }

  /* -- Flows --------------------------------------------------------- *
   * Boot and activation: the only callers of setLicenseState(), composing
   * data and presentation. Everything else above is either the state, the
   * gate, or what draws it.
   * ------------------------------------------------------------------ */

  function licenseActivate() {
    var input = paywallEl.querySelector(".lic-input");
    var button = paywallEl.querySelector(".lic-activate");
    /* Enter on the key field reaches this directly, not through the button —
       guard here so a second submit while one is in flight does nothing. */
    if (button.disabled) return;
    var key = input.value;
    button.disabled = true;
    input.removeAttribute("aria-invalid");
    licenseSetStatus("Schl\u00fcssel wird bei Polar gepr\u00fcft \u2026");

    licensing
      .validateLicenseKey(key, {
        organizationId: licensingConfig.organizationId || undefined,
      })
      .then(function (result) {
        button.disabled = false;
        if (result.valid === true) {
          /* Keep the original first-start stamp: activating on day 13 must not
             restart anything, and losing the stamp would restart the trial. */
          setLicenseState(
            Object.assign({}, licenseState, { license: result.license }),
            true
          );
          licenseSetStatus("Lizenz aktiv. Vielen Dank.", "ok");
          setTimeout(function () {
            paywallEl.setAttribute("data-dismissible", "true");
            licenseHidePaywall();
          }, 900);
          return;
        }
        if (result.valid === false) {
          input.setAttribute("aria-invalid", "true");
          licenseSetStatus(
            result.reason === "empty"
              ? "Bitte einen Schl\u00fcssel eingeben."
              : "Schl\u00fcssel unbekannt, widerrufen oder abgelaufen.",
            "fail"
          );
          return;
        }
        /* valid === null: Polar unreachable — not the key's fault. Offer the
           cached license if one exists, never a false rejection. */
        if (licenseState.license) {
          licenseSetStatus(
            "Keine Verbindung. Der zuletzt gepr\u00fcfte Schl\u00fcssel bleibt bis auf Weiteres g\u00fcltig.",
            "ok"
          );
          setTimeout(function () {
            paywallEl.setAttribute("data-dismissible", "true");
            licenseHidePaywall();
          }, 1400);
        } else {
          licenseSetStatus("Keine Verbindung zu Polar. Bitte sp\u00e4ter erneut versuchen.", "fail");
        }
      })
      .catch(function (error) {
        button.disabled = false;
        licenseSetStatus("Pr\u00fcfung fehlgeschlagen: " + error, "fail");
      });
  }

  function licenseOpenCheckout() {
    var url = licensingConfig.checkoutUrl || licensingConfig.pricingUrl;
    if (!url) return;
    invoke("open_url", { url: url })
      .catch(function () {
        /* No platform opener: show the URL so it can be copied rather than
           failing silently. */
        licenseSetStatus(url);
      });
  }

  function enableLicensing() {
    if (!licensing || typeof invoke !== "function") return;
    enableLicenseHotkeys();

    invoke("license_state_read")
      .then(function (stored) {
        var now = Date.now();
        if (stored && typeof stored === "object" && typeof stored.firstStartAt === "number") {
          setLicenseState(stored, false);
        } else {
          /* First start: stamp the trial and persist it immediately, so the
             second start cannot claim a fresh trial. */
          setLicenseState(licensing.emptyState(now), true);
        }
        if (!licenseDecision.unlocked) licenseShowPaywall(false);
      })
      .catch(function (error) {
        /* Storage unavailable: run the trial in memory, say nothing scary. */
        setLicenseState(licensing.emptyState(Date.now()), false);
        announce("Lizenzstatus nicht lesbar: " + error);
      });
  }
