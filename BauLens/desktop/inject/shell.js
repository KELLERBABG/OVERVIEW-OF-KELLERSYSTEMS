/**
 * The shell half of the desktop inject script: what wires the page to the
 * Rust shell. Assembled with licensing-ui.js and boot.js by
 * `desktop/inject/sources.mjs` — the one description of what ships; the
 * staging script and the test suite both read it.
 *
 * It is a separate file rather than inline markup so the repository's pages
 * stay the single source: the staging copy is the only place this is added,
 * and the browser build never sees it.
 *
 * Two hosts, one script:
 *
 *   studio.html          the single-verdict workbench. Gets a status strip,
 *                        drag & drop into its own file inputs, its print
 *                        route through the shell, and the gate over the
 *                        actions its own page wires in code.
 *   workhorse.html       the multi-project workspace (staged as
 *                        desktop/app/index.html). Gets the workspace side of
 *                        the shell: native folder dialog, recursive scan,
 *                        batch checks and the export path.
 *
 * The page tells them apart by the #workhorse root element, and every entry
 * point below degrades to a no-op when its host element is missing. The
 * workhorse page exposes window.Workhorse; this file is the only thing that
 * talks to the Rust shell. It also owns the bridge the licensing half runs
 * on: invoke, announce and isWorkhorse.
 *
 * Licensing lives in licensing-ui.js. This file consults it through exactly
 * two names — assertUnlocked() and gated() — and never touches its state or
 * its DOM; a structure test pins that seam from both sides.
 *
 * The IPC surface it uses (see desktop/src-tauri/src/lib.rs):
 *   engine_info()                                  -> { transport, engine }
 *   engine_check({ params })                       -> engine response
 *   print_dossier()                                -> ()
 *   pick_workspace_folder({ title? })               -> string | null
 *   pick_save_path({ defaultName })               -> string | null
 *   scan_workspace({ path })                       -> { root, projects, fileCount }
 *   read_workspace_file({ path })                  -> string
 *   write_workspace_file({ path, contents })       -> ()
 *   batch_check({ files, land })                   -> { results, checked, passed, failed }
 *   export_file({ path, kind, target })            -> { target, bytes }  (optional)
 *
 *   dossier_pdf({ file | spec, out, land, aktenzeichen, datum, variant })
 *                                                  -> { out, bytes, pages, hash }
 *   plan_svg({ file | spec, out, land, aktenzeichen, datum })
 *   plan_dxf({ ... same params })
 *                                                  -> { out, blatt, scale, bytes, hash }
 *   plan_export({ files, outDir, formats, land })  -> { outDir, formats, written, failed,
 *                                                       collisions, counts }
 *
 * All of them are forwarders: the documents are rendered and written by the
 * engine (the sidecar), which is the only place that holds the bytes. The
 * shell contributes the native dialogs and nothing else, so there is no
 * second dossier implementation and no second drawing implementation on the
 * Rust side.
 *
 * `plan_export` is the workspace form of the two Blatt exports: a list of
 * plans and one target folder, one Lageplan-Blatt per Vorhaben, named after
 * it. The shell supplies the scan result and the folder; the loop, the file
 * names and the Prüfziffer belong to the engine, because a second
 * implementation of "which plan becomes which file" is a second thing to
 * keep correct.
 */
  var tauri = window.__TAURI__;
  var invoke =
    tauri && tauri.core && typeof tauri.core.invoke === "function" ? tauri.core.invoke : null;

  /* The workspace shell is only wired when the workhorse page is the host. */
  var isWorkhorse = !!document.getElementById("workhorse");

  function workhorse() {
    return window.Workhorse || null;
  }

  function whCall(name, args) {
    var api = workhorse();
    if (api && typeof api[name] === "function") return api[name].apply(api, args || []);
    return undefined;
  }

  /* Extensions the engine can ingest. Kept in one place because the drop
     routing and the export naming both depend on it. */
  var PLAN_EXTENSIONS = ["ifc", "dxf", "gml", "xml", "geojson", "json", "bcf", "txt", "md"];

  function extensionOf(path) {
    var name = String(path || "").split(/[\\/]/).pop() || "";
    var dot = name.lastIndexOf(".");
    return dot === -1 ? "" : name.slice(dot + 1).toLowerCase();
  }

  function looksLikePlanFile(path) {
    return PLAN_EXTENSIONS.indexOf(extensionOf(path)) !== -1;
  }

  /* ------------------------------------------------------------------ *
   * 1. Status strip: say which engine is answering, and how.
   * ------------------------------------------------------------------ */

  function addStatusStrip() {
    /* The workhorse page owns its own header and status bar, so the shell only
       reports the engine into it instead of injecting a second strip. */
    if (isWorkhorse) {
      invoke("engine_info")
        .then(function (info) {
          whCall("setEngineInfo", [info]);
          document.documentElement.setAttribute("data-desktop-engine", "ready");
        })
        .catch(function (error) {
          whCall("setStatus", ["Engine nicht verf\u00fcgbar: " + error]);
          document.documentElement.setAttribute("data-desktop-engine", "error");
        });
      return;
    }

    var bar = document.createElement("div");
    bar.id = "desktopStatus";
    bar.style.cssText =
      "position:fixed;right:12px;bottom:12px;z-index:900;display:flex;align-items:center;gap:0.5rem;" +
      "padding:0.4rem 0.7rem;background:var(--ink-700);border:1px solid var(--line-strong);" +
      "border-radius:var(--r-md);font-family:var(--font-mono);font-size:0.68rem;color:var(--fg-dim);" +
      "box-shadow:var(--shadow-2)";
    bar.innerHTML =
      '<span style="width:6px;height:6px;border-radius:50%;background:var(--fg-faint)" id="desktopDot"></span>' +
      '<span id="desktopStatusText">Desktop: Engine wird geladen …</span>';

    /* Never in a printed dossier. */
    bar.className = "no-print";
    document.body.appendChild(bar);

    invoke("engine_info")
      .then(function (info) {
        var engine = info && info.engine ? info.engine.result : null;
        var dot = document.getElementById("desktopDot");
        var text = document.getElementById("desktopStatusText");
        if (dot) dot.style.background = "var(--accent)";
        if (text) {
          text.textContent =
            "Desktop \u00b7 " +
            (engine ? engine.product + " " + engine.version : "Engine") +
            " \u00b7 " +
            (engine && engine.counts ? engine.counts.total + " Pr\u00fcfmodule" : "");
        }
        document.documentElement.setAttribute("data-desktop-engine", "ready");
      })
      .catch(function (error) {
        var text = document.getElementById("desktopStatusText");
        if (text) text.textContent = "Desktop \u00b7 Engine nicht verf\u00fcgbar: " + error;
        document.documentElement.setAttribute("data-desktop-engine", "error");
      });
  }

  /* ------------------------------------------------------------------ *
   * 2. Drag and drop from Explorer anywhere in the window.
   *
   * Tauri suppresses the webview's own file drop, so the HTML5 DataTransfer
   * route that works in a browser yields no files here. The drag events are
   * intercepted and the native event is forwarded instead.
   * ------------------------------------------------------------------ */

  function fileNameOf(path) {
    return String(path).split(/[\\/]/).pop();
  }

  /**
   * Hands a dropped file to the page's own upload handler: the extension
   * decides which one, and both drop paths (the native routeFile and the
   * HTML5 drop) come through here, so the mapping exists exactly once.
   * Returns whether the page could take the file, so the caller can report
   * the load truthfully instead of always claiming success.
   */
  function feedPageInput(name, text) {
    /* Feed the existing inputs rather than duplicating their parsing: the
       studio already knows how to read DXF, IFC, XPlanung and text. */
    var handler = /\.(gml|geojson)$/i.test(name)
      ? window.handleBplanUpload
      : window.handleStudioUpload;
    if (typeof handler !== "function") {
      announce("Keine Verarbeitung f\u00fcr " + name + " gefunden.");
      return false;
    }
    handler({
      target: { files: [new File([text], name, { type: "text/plain" })], value: "" },
    });
    return true;
  }

  function routeFile(path, name) {
    /* A dropped file is a paid action on both hosts — it opens a project or
       feeds the engine. Locked: refuse; the paywall is already up. */
    if (!assertUnlocked()) return;
    /* On the workhorse page a dropped path is a workspace entry, not a form
       input: files open in single-file mode, folders are scanned. */
    if (isWorkhorse) {
      if (looksLikePlanFile(path)) openSingleFile(path, name);
      else openWorkspace(path);
      return;
    }

    fetch("asset://localhost/" + encodeURIComponent(path))
      .then(function (response) {
        if (!response.ok) throw new Error("HTTP " + response.status);
        return response.text();
      })
      .then(function (text) {
        if (feedPageInput(name, text)) announce(name + " geladen");
      })
      .catch(function () {
        /* The asset protocol is disabled by default in a bundled app; fall back
           to telling the user rather than failing silently. */
        announce(
          name + ": Datei konnte nicht gelesen werden. Bitte \u00fcber die Schaltfl\u00e4che ausw\u00e4hlen."
        );
      });
  }

  function announce(message, tone) {
    /* No strip on the workhorse page: its status bar takes the message. The tone
       is passed on when one is given, so a batch that wrote nothing can be red
       rather than grey. */
    if (isWorkhorse || !document.getElementById("desktopStatusText")) {
      whCall("setStatus", tone ? [message, tone] : [message]);
      return;
    }
    var text = document.getElementById("desktopStatusText");
    if (text) text.textContent = "Desktop \u00b7 " + message;
  }

  function enableDragAndDrop() {
    var listen = window.__TAURI__.event && window.__TAURI__.event.listen;
    var overlay = document.createElement("div");
    overlay.className = "no-print";
    overlay.style.cssText =
      "position:fixed;inset:0;z-index:950;display:none;align-items:center;justify-content:center;" +
      "background:rgba(5,7,12,0.72);border:2px dashed var(--accent);border-radius:var(--r-lg);" +
      "font-family:var(--font-mono);font-size:0.95rem;color:var(--accent-hi);pointer-events:none";
    /* Overlay copy differs: the workhorse drops folders as well as drawings. */
    overlay.textContent = isWorkhorse
      ? "Projektordner oder Plandatei hier ablegen (.ifc, .dxf, .gml, .json, .txt)"
      : "Plan hier ablegen (.dxf, .ifc, .gml, .txt)";
  document.body.appendChild(overlay);

    var show = function (on) { overlay.style.display = on ? "flex" : "none"; };

    /* HTML5 path, which is what works when the asset protocol is enabled. */
    ["dragenter", "dragover"].forEach(function (type) {
      window.addEventListener(type, function (event) {
        event.preventDefault();
        show(true);
      });
    });
    ["dragleave", "drop"].forEach(function (type) {
      window.addEventListener(type, function (event) {
        event.preventDefault();
        show(false);
      });
    });
    window.addEventListener("drop", function (event) {
      var files = event.dataTransfer ? Array.from(event.dataTransfer.files || []) : [];
      /* The webview does not expose a filesystem path on a dropped File, so the
         workhorse page relies on the native tauri://drag-drop event below. */
      if (isWorkhorse) {
        if (files.length) {
          announce("Ordner k\u00f6nnen nur \u00fcber den nativen Drop erkannt werden \u2014 bitte den Projektordner im Explorer auf das Fenster ziehen.");
        }
        return;
      }
      files.forEach(function (file) {
        var reader = new FileReader();
        reader.onload = function (e) {
          if (feedPageInput(file.name, e.target.result)) announce(file.name + " geladen");
        };
        reader.readAsText(file);
      });
    });

    /* Native path: Tauri reports the absolute paths of the dropped files. */
    if (listen) {
      listen("tauri://drag-enter", function () { show(true); });
      listen("tauri://drag-leave", function () { show(false); });
      listen("tauri://drag-drop", function (event) {
        show(false);
        var paths = (event && event.payload && event.payload.paths) || [];
        paths.forEach(function (path) { routeFile(path, fileNameOf(path)); });
      });
    }
  }

  /* ------------------------------------------------------------------ *
   * 3. Dossier through the app's own print path.
   *
   * Tauri v2 exposes print(), not a headless PDF writer, so the PDF still comes
   * from the system dialog's "Save as PDF". Routed through the shell anyway: the
   * packaged app prints from local files with no server, and the shell can add a
   * progress hint, which the plain window.print() path cannot.
   * ------------------------------------------------------------------ */

  function enablePdfExport() {
    var button = document.querySelector('button[onclick="window.print()"]');
    if (!button) return;

    button.onclick = function (event) {
      event.preventDefault();
      if (!assertUnlocked()) return;
      announce("Druckdialog wird ge\u00f6ffnet \u2026");
      invoke("print_dossier")
        .then(function () {
          announce("Dossier gedruckt");
        })
        .catch(function (error) {
          announce("Druck fehlgeschlagen: " + error);
        });
    };
    button.title = "Dossier drucken oder als PDF sichern";
  }

  /* ------------------------------------------------------------------ *
   * 4. The workspace shell: native folder dialog, recursive scan, batch
   *    checks and exports. Only active on the workhorse page.
   *
   * The page asks for these through CustomEvents on `document` so the view
   * stays free of IPC:
   *   workhorse:open-folder
   *   workhorse:check   detail { files }
   *   workhorse:export  detail { kind, file }   pdf | svg | dxf | xbau | bcf | dossier
   *   workhorse:detail  detail { path }
   * ------------------------------------------------------------------ */

  function exportNameFor(path, kind) {
    var clean = String(path || "projekt").replace(/\.[^.\\/]+$/, "");
    if (kind === "pdf") return clean + "_Befund.pdf";
    /* Das Lageplan-Blatt heißt nach sich selbst, nicht nach dem Plan: aus einem
       Projekttree kommen mehrere Lagepläne nebeneinander. */
    if (kind === "svg") return clean + "_Lageplan.svg";
    if (kind === "dxf") return clean + "_Lageplan.dxf";
    if (kind === "xbau") return clean + "_XBau.xml";
    if (kind === "bcf") return clean + "_BCF.bcfjson";
    if (kind === "deviations") return clean + "_Abweichungsantrag.txt";
    if (kind === "dossier") return clean + "_Dossier.html";
    if (kind === "matrix") return clean + "_DACH-Matrix.pdf";
    return clean + ".out";
  }

  function openWorkspace(path) {
    if (!path) return;
    whCall("setBusy", ["Indiziere " + path + " \u2026"]);
    invoke("scan_workspace", { path: path })
      .then(function (scan) {
        if (!scan || !scan.projects || !scan.projects.length) {
          /* An empty or unreadable tree is a single-file aim, not an error the
             user can act on from here. */
          announce("Keine Plandateien in " + path + " gefunden.");
          return;
        }
        whCall("setWorkspace", [scan]);
      })
      .catch(function (error) {
        announce("Ordner konnte nicht gelesen werden: " + error);
      });
  }

  function openSingleFile(path, name) {
    if (!path) return;
    whCall("setSingleFile", [name || String(path).split(/[\\/]/).pop(), path]);
  }

  function currentJurisdiction(detail) {
    var fromPage = whCall("getJurisdiction", []) || {};
    return {
      land: (detail && detail.land !== undefined ? detail.land : fromPage.land) || null,
      stadt: (detail && detail.stadt !== undefined ? detail.stadt : fromPage.stadt) || null,
    };
  }

  function checkPaths(files, land, stadt) {
    if (!files || !files.length) return;
    var jur = currentJurisdiction({ land: land, stadt: stadt });
    whCall("setBusy", ["Pr\u00fcfe " + files.length + " Datei(en) \u2026"]);
    invoke("batch_check", { files: files, land: jur.land, stadt: jur.stadt })
      .then(function (result) {
        whCall("setResult", [result]);
      })
      .catch(function (error) {
        whCall("setResult", [{
          results: files.map(function (path) {
            return { path: path, ok: false, error: String(error) };
          }),
          checked: files.length,
          passed: 0,
          failed: files.length
        }]);
      });
  }

  /**
   * Writes the DIN-A4 dossier as a real PDF file through the engine.
   *
   * No print dialog: the sidecar renders the PDF (src/dossierpdf.js) and writes
   * it to the path this dialog returns, so the architect gets a file they can
   * attach to an E-Mail or drop into a submission folder. The shell's part is
   * only the native save dialog.
   *
   * `land` stays null on purpose: the engine resolves the profile from the file's
   * own spec, which is the same profile `batch_check` used, so the PDF cannot
   * claim a different Rechtsbereich than the check that produced it.
   */
  function exportPdf(file, land, stadt, variant) {
    if (!file) {
      announce("Keine Datei ausgew\u00e4hlt.");
      return;
    }
    var jur = currentJurisdiction({ land: land, stadt: stadt });

    announce("Erzeuge PDF \u2026");
    invoke("pick_save_path", { defaultName: exportNameFor(file, "pdf") })
      .then(function (target) {
        if (!target) {
          announce("PDF-Export abgebrochen.");
          return null;
        }
        return invoke("dossier_pdf", {
          file: file,
          out: target,
          land: jur.land,
          stadt: jur.stadt,
          variant: variant || null
        }).then(function (written) {
          var pages = written && written.pages ? written.pages : 0;
          var hash = written && written.hash ? String(written.hash).slice(0, 16) : "";
          announce(
            "PDF gespeichert: " + target +
              (pages ? " \u00b7 " + pages + (pages === 1 ? " Seite" : " Seiten") : "") +
              (hash ? " \u00b7 Pr\u00fcfziffer " + hash : "") +
              (variant ? " \u00b7 mit Vorher-/Nachher-Vergleich" : "")
          );
        });
      })
      .catch(function (error) {
        /* An engine that predates `dossier_pdf` answers "unknown method"; say so
           in plain German instead of leaving a rejected promise behind. */
        announce("PDF-Export nicht m\u00f6glich: " + error);
      });
  }

  /**
   * Writes the Lageplan-Blatt as a vector file through the engine.
   *
   * The drawing has to leave the dossier for the CAD side of a submission: it
   * goes into an existing plan set or to a plotter, where a PDF page is the wrong
   * shape. Drawing it here is not an option - the engine is the only thing that
   * knows the Maßstab, the derived Hauskörper and the Prüfziffer, and a second
   * drawing in the shell would be a second drawing to keep correct. So the shell
   * asks for a target path and the engine writes SVG or DXF.
   *
   * The `scale` the engine reports back is the Maßstab printed on the Blatt
   * ("1:250"), and the Prüfziffer ties the file to the check it came from: for
   * the DXF it is also what the drawing units mean, since one unit is one metre.
   */
  function exportPlan(format, file, land, stadt) {
    if (!file) {
      announce("Keine Datei ausgew\u00e4hlt.");
      return;
    }
    var jur = currentJurisdiction({ land: land, stadt: stadt });

    announce("Erzeuge Lageplan (" + String(format).toUpperCase() + ") \u2026");
    invoke("pick_save_path", { defaultName: exportNameFor(file, format) })
      .then(function (target) {
        if (!target) {
          announce("Export abgebrochen.");
          return null;
        }
        var method = format === "dxf" ? "plan_dxf" : "plan_svg";
        return invoke(method, { file: file, out: target, land: jur.land, stadt: jur.stadt }).then(function (written) {
          var scale = written && written.scale ? " \u00b7 Ma\u00dfstab " + written.scale : "";
          var hash = written && written.hash ? " \u00b7 Pr\u00fcfziffer " + String(written.hash).slice(0, 16) : "";
          announce("Lageplan gespeichert: " + target + scale + hash);
        });
      })
      .catch(function (error) {
        /* An engine that predates `plan_svg`/`plan_dxf` answers "unknown
           method"; say so in plain German instead of leaving a rejected promise
           behind. */
        announce("Lageplan-Export nicht m\u00f6glich: " + error);
      });
  }

  /**
   * Writes the Lageplan-Blatt for every project of the workspace in one pass.
   *
   * A submission folder holds one Blatt per Vorhaben, so this has to be one
   * action rather than one save dialog per project. What the shell contributes is
   * what only it has: the scan result that says which Vorhaben exist, and the
   * folder dialog for the target. The loop itself stays in the engine - the file
   * names, the Maßstab and the Prüfziffer are engine facts, and each plan is
   * resolved there through the same ingestion and the same run as the dossier
   * that belongs to it, so an exported Blatt cannot describe a different building
   * than the Befund beside it.
   *
   * The answer reports three things worth saying out loud: how many Blätter were
   * written, which plans failed (a broken attachment in a folder of twenty must
   * not cost the other nineteen), and whether a Blatt had to be renamed because
   * two Vorhaben wanted the same file name.
   */
  function exportPlanSet(files) {
    var list = Array.isArray(files)
      ? files.filter(function (entry) {
          return entry && typeof entry.path === "string" && entry.path;
        })
      : [];

    if (!list.length) {
      announce("Keine Projekte im Workspace.");
      return;
    }

    announce("Exportiere Lagepl\u00e4ne \u2026");
    invoke("pick_workspace_folder", { title: "Zielordner f\u00fcr die Lagepl\u00e4ne w\u00e4hlen" })
      .then(function (target) {
        if (!target) {
          announce("Export abgebrochen.");
          return null;
        }

        return invoke("plan_export", {
          files: list,
          outDir: target,
          formats: ["dxf", "svg"],
          land: null,
        }).then(function (exported) {
          var counts = (exported && exported.counts) || {};
          var drawn = counts.drawn || 0;
          var failed = (exported && exported.failed) || [];
          var collisions = (exported && exported.collisions) || [];
          var formats = (exported && exported.formats) || ["dxf", "svg"];
          var formatLabel = formats
            .map(function (format) { return String(format).toUpperCase(); })
            .join(" + ");

          var parts = [
            drawn + (drawn === 1 ? " Lageplan " : " Lagepl\u00e4ne ") + "(" + formatLabel + ") nach " + target,
          ];
          var tone = drawn === 0 ? "fail" : failed.length ? "warn" : "ok";

          if (failed.length) {
            /* Name them: "2 nicht lesbar" without the names sends somebody
               looking through a folder of twenty drawings by hand. */
            var names = failed.slice(0, 3).map(function (entry) {
              var name = String(entry.source || "").split(/[\\/]/).pop();
              return name || entry.source;
            });
            var more = failed.length > names.length ? " und " + (failed.length - names.length) + " weitere" : "";
            parts.push(failed.length + " nicht lesbar" + (names.length ? ": " + names.join(", ") + more : ""));
          }

          if (collisions.length) {
            parts.push(collisions.length + " umbenannt (gleicher Blattname)");
          }

          var summary = parts.join(" \u00b7 ");
          announce(drawn === 0 ? "Kein Lageplan geschrieben \u00b7 " + summary : summary, tone);
        });
      })
      .catch(function (error) {
        /* An engine that predates `plan_export` answers "unknown method"; say so
           in plain German instead of leaving a rejected promise behind. */
        announce("Lageplan-Export nicht m\u00f6glich: " + error, "fail");
      });
  }

  function exportFile(kind, file, land, stadt, variant) {
    if (!file) {
      announce("Keine Datei ausgew\u00e4hlt.");
      return;
    }
    var jur = currentJurisdiction({ land: land, stadt: stadt });
    if (kind === "pdf") {
      exportPdf(file, jur.land, jur.stadt, variant);
      return;
    }
    if (kind === "svg" || kind === "dxf") {
      exportPlan(kind, file, jur.land, jur.stadt);
      return;
    }
    if (kind === "dossier") {
      announce("Druckdialog wird ge\u00f6ffnet \u2026");
      invoke("print_dossier")
        .then(function () { announce("Dossier gedruckt"); })
        .catch(function (error) { announce("Druck fehlgeschlagen: " + error); });
      return;
    }

    var target = exportNameFor(file, kind);
    announce("Schreibe " + target + " \u2026");
    invoke("export_file", { path: file, kind: kind, target: target, land: jur.land, stadt: jur.stadt })
      .then(function (written) {
        announce(target + " geschrieben" + (written && written.bytes ? " (" + written.bytes + " Bytes)" : ""));
      })
      .catch(function (error) {
        /* The engine can produce the document, but only the shell can place it
           next to the drawing. Say so instead of failing silently. */
        announce("Export nicht m\u00f6glich: " + error);
      });
  }

  function enableWorkspaceShell() {
    if (!isWorkhorse) return;

    /* Every workspace command registers through gated(): a locked app refuses
       with the paywall instead of executing under it, and no handler carries
       the conditional itself — gated() is the one place the gate is applied
       to a callable (the predicate lives under Licensing). */

    document.addEventListener("workhorse:open-folder", gated(function () {
      invoke("pick_workspace_folder")
        .then(function (path) {
          if (path) openWorkspace(path);
        })
        .catch(function (error) {
          announce("Ordnerdialog nicht verf\u00fcgbar: " + error);
        });
    }));

    document.addEventListener("workhorse:check", gated(function (event) {
      var detail = event.detail || {};
      checkPaths(detail.files, detail.land, detail.stadt);
    }));

    document.addEventListener("workhorse:detail", gated(function (event) {
      var detail = event.detail || {};
      var path = detail.path;
      if (path) checkPaths([path], detail.land, detail.stadt);
    }));

    document.addEventListener("workhorse:export", gated(function (event) {
      var detail = event.detail || {};
      exportFile(detail.kind, detail.file, detail.land, detail.stadt, detail.variant);
    }));

    /* The whole workspace at once: the page sends the Vorhaben it found, the
       shell asks for a folder and the engine writes the Blätter. */
    document.addEventListener("workhorse:export-plans", gated(function (event) {
      var detail = event.detail || {};
      exportPlanSet(detail.files);
    }));

    document.addEventListener("workhorse:statute", gated(function (event) {
      var detail = event.detail || {};
      if (!detail.ref) return;
      var jur = currentJurisdiction(detail);
      invoke("engine_method", {
        method: "statute",
        params: { ref: detail.ref, land: jur.land },
      })
        .then(function (response) {
          var res = response && response.result ? response.result : null;
          whCall("setStatute", [res, detail.ref]);
        })
        .catch(function () {
          whCall("setStatute", [null, detail.ref]);
        });
    }));

    document.addEventListener("workhorse:simulate", gated(function (event) {
      var detail = event.detail || {};
      if (!detail.spec) return;
      var jur = currentJurisdiction(detail);
      invoke("engine_check", {
        params: {
          spec: detail.spec,
          land: jur.land,
          stadt: jur.stadt,
          repair: true,
        },
      })
        .then(function (response) {
          var res = response && response.result ? response.result : null;
          whCall("setSimulation", [res, detail]);
        })
        .catch(function (error) {
          announce("Simulation fehlgeschlagen: " + error, "fail");
        });
    }));

    /* The DACH matrix as a PDF table next to the dossier. The engine draws it
       and writes the file; the shell only chooses the target. */
    document.addEventListener("workhorse:lands-matrix-pdf", gated(function (event) {
      var detail = event.detail || {};
      if (!detail.file) return;
      invoke("pick_save_path", { defaultName: exportNameFor(detail.file, "matrix") })
        .then(function (target) {
          if (!target) {
            announce("Matrix-Export abgebrochen.");
            return null;
          }
          announce("Erzeuge DACH-Matrix als PDF \u2026");
          return invoke("engine_method", {
            method: "lands_matrix_pdf",
            params: { file: detail.file, country: detail.country || "DACH", out: target },
          }).then(function (response) {
            var res = response && response.result ? response.result : null;
            var pages = res && res.pages ? res.pages : 0;
            announce(
              "DACH-Matrix gespeichert: " + target +
                (pages ? " \u00b7 " + pages + (pages === 1 ? " Seite" : " Seiten") : "")
            );
          });
        })
        .catch(function (error) {
          announce("Matrix-PDF nicht m\u00f6glich: " + error, "fail");
        });
    }));

    /* The DACH matrix: the active plan against every Rechtsbereich at once. The
       engine reads the spec from the file, so the page sends only the path. */
    document.addEventListener("workhorse:lands-matrix", gated(function (event) {
      var detail = event.detail || {};
      if (!detail.file) return;
      invoke("engine_method", {
        method: "lands_matrix",
        params: { file: detail.file, country: detail.country || "DACH" },
      })
        .then(function (response) {
          var res = response && response.result ? response.result : null;
          if (res) whCall("setLandsMatrix", [res, null]);
          else whCall("setLandsMatrix", [null, "Die Engine hat keine Matrix geliefert."]);
        })
        .catch(function (error) {
          whCall("setLandsMatrix", [null, "Matrix fehlgeschlagen: " + error]);
        });
    }));
  }

  /* ------------------------------------------------------------------ *
   * 4b. The studio host: the page wires its buttons in page code.
   *
   * studio.html's inline onclick/oninput handlers call the page's own
   * globals directly — there is no event bus to tap the way the workhorse
   * asks for its commands. The gate therefore wraps those globals once at
   * start through the same gated() every command registration uses: one
   * list, no conditional in any handler, and the page itself stays untouched
   * (it ships to the website unchanged; only the staged copy carries this
   * script). Inline handlers resolve the name at call time, so they land on
   * the wrapper.
   *
   * The list is the actions that reach the engine or write an output.
   * Presets, samples and the form fields funnel into runEvaluation() and
   * are covered by it. Names the page does not define are skipped.
   * ------------------------------------------------------------------ */

  var STUDIO_PAID_ACTIONS = [
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

  function enableStudioActions() {
    STUDIO_PAID_ACTIONS.forEach(function (name) {
      if (typeof window[name] !== "function") return;
      window[name] = gated(window[name]);
    });
  }

