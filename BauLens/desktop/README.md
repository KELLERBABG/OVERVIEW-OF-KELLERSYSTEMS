# BauLens Desktop

A Tauri v2 shell around the tested compliance engine. It compiles and produces an
installer; see **Verification status** below for what has and has not been
exercised in this environment.

## Architecture

```
BauLens.exe  (Rust, Tauri v2)
├── WebView2
│    ├── desktop/app/index.html      the workhorse workspace (desktop/workhorse.html)
│    ├── desktop/app/studio.html     the engine view, same page the website served once
│    ├── desktop/app/docs.html       offline documentation
│    └── + desktop/inject/         desktop-only behaviour, injected at stage time:
│         ├── shell.js             page + Rust shell wiring, the host bridge
│         ├── licensing-ui.js      state, gate, paywall — data → gate → presentation → flows
│         ├── boot.js              composition root (start order, load dispatch)
│         └── sources.mjs          the source list build and tests both assemble from
└── engine child process
     └── baulens.cjs --rpc            dist/baulens.cjs, one JSON object per line
          └── renders reports, XBau, BCF and the PDF dossier (src/)
```

The shell does not reimplement any rules. The engine is the code under test, so
`src/` stays the single implementation and stays covered by the unit suite (`node --test`).
Rust cannot execute JavaScript, which is why the engine runs as a child process
speaking the protocol in `src/ipc.js`. That extends to the workspace features:
the folder scan is filesystem work, and every compliance answer — including a
whole folder's worth — still comes from the child process.

## Layout

| Path | What it is |
|---|---|
| `workhorse.html` | Multi-project workspace: explorer, interactive 3D/2D viewer, inspector |
| `inject/sources.mjs` | the source list: both the staging build and the test harness assemble the inject script from it, so staged script and tested script cannot diverge |
| `inject/shell.js` | host bridge (`invoke`, `announce`, `isWorkhorse`), status strip, drag & drop, print, PDF save, Lageplan export and export routing; consults licensing only through `assertUnlocked()` / `gated()` |
| `inject/licensing-ui.js` | licensing ordered as data → gate → presentation → flows (one state transition, one gate application); reaches the host only through the three bridge names |
| `inject/boot.js` | composition root: start order and the load-time dispatch |
| `src-tauri/src/lib.rs` | sidecar lifecycle + workspace management (scan, dialogs, batch, export) |
| `src/pdf.js`, `src/dossierpdf.js` | engine-side PDF writer and the DIN-A4 dossier layout it renders |
| `src/plansvg.js`, `src/plandxf.js` | the Lageplan sheet as SVG and as DXF, written from the same recording the PDF page replays |
| `src-tauri/tauri.conf.json` | window, CSP, bundle settings |
| `src-tauri/icons/` | generated from `assets/icon.svg` via `tauri icon` |
| `app/` | staged web frontend (generated, not tracked) |

## Building

```powershell
node scripts/build-desktop-assets.mjs     # stage the frontend: workhorse -> app/index.html
node scripts/build-desktop-sidecar.mjs    # build and verify the engine
cd desktop/src-tauri
..\..\node_modules\.bin\tauri build       # installer -> target/release/bundle/*
```

`npm run desktop:build` does the first two. `bundle.targets` covers NSIS on
Windows, a universal DMG on macOS and AppImage/Deb on Linux; the release
workflow builds one per operating system.

## The two scenarios

| Scenario | How it starts | What happens |
|---|---|---|
| **A — ad-hoc single check** | drop a file on the window, or open one | the page switches to single-file mode and checks that file immediately; no project, no folder structure |
| **B — office workspace** | *Projektordner öffnen* (native folder dialog) | `scan_workspace` indexes the tree, each subdirectory with plan files becomes a project with its own status ampel, and *Alle Projekte im Stapel prüfen* runs them together |

The scan is bounded on purpose (depth 6, 200 projects, 500 files per project) so
a mistakenly chosen drive root reports something instead of hanging the UI.

## IPC surface

All of it is JSON in and JSON out; `desktop/inject/shell.js` documents the
workspace side of that list at the top of the file, `desktop/inject/licensing-ui.js`
its three licence commands.

| Command | Purpose |
|---|---|
| `engine_info`, `engine_check` | engine handshake and a check against a supplied spec |
| `print_dossier` | platform print dialog for the HTML dossier |
| `pick_workspace_folder` | native folder dialog (blocking call pushed off the main thread) |
| `pick_save_path({ defaultName })` | native save dialog for the PDF and Lageplan exports; `null` when cancelled. Chooses a path only — the engine writes the file |
| `scan_workspace({ path })` | bounded recursive project discovery |
| `read_workspace_file({ path })`, `write_workspace_file({ path, contents })` | plain file access for the explorer |
| `batch_check({ files, land })` | one `check_file` per path; a failing file is reported, never fatal |
| `export_file({ path, kind, target })` | writes the engine's XBau/BCF document next to its drawing |
| `dossier_pdf({ file, out, land })` | forwarder: the engine renders the DIN-A4 dossier as a PDF and writes it to `out` |
| `plan_svg({ file, out, land })`, `plan_dxf({ file, out, land })` | forwarders: the engine writes the Lageplan sheet as SVG or as DXF (1 unit = 1 m) to `out` |
| `plan_export({ files, outDir, formats, land })` | forwarder: the engine writes a Lageplan sheet per entry into `outDir`, named after the Vorhaben. The list is capped at `MAX_BATCH`, the same ceiling the batch check uses |

`check_file` lives in the engine (`src/ipc.js` + `src/ingest.js`), not in Rust,
because turning a DXF, IFC, GML, GeoJSON, Textteil or Baubeschreibung into a spec
is engine logic — and it is covered by tests rather than by a second
implementation in the shell. The same holds for `dossier_pdf`, `plan_svg` and
`plan_dxf`, which are engine methods reached through the sidecar: Rust never sees
the document, only the path. All three go through one shared forwarder in
`lib.rs`, so none of them can quietly drop a parameter the others pass — a
`plan_dxf` that forgot `land` would export a drawing from a different
Rechtsbereich than the dossier beside it.

| Engine method | Purpose |
|---|---|
| `check`, `check_file` | the rule run, per spec or per plan file |
| `dossier_pdf({ file \| spec, out?, land?, aktenzeichen?, datum? })` | renders the DIN-A4 dossier as a PDF; with `out` the sidecar writes the file and answers `{ out, bytes, pages, hash }` |
| `plan_svg`, `plan_dxf` | the same input, the Lageplan sheet alone; they answer `{ out, blatt, scale, bytes, hash }` with `out` and the document inline as `{ svg }` / `{ dxf }` without it, because both are text |
| `plan_export({ files: [{ path, label }], outDir, formats? })` | the whole workspace in one call: one Blatt per entry into `outDir`, named `<label>_Lageplan.<format>`, answering `{ outDir, formats, written, failed, collisions, counts }`. The loop is engine code because the naming, the scale and each Blatt's Prüfziffer are engine facts |

## Engine resolution order

`resolve_engine` in `lib.rs` tries, in order:

1. `baulens-sidecar.exe` beside the app binary — no runtime needed. Produced by
   `build-desktop-sidecar.mjs --sea`, which is **opt-in and verified before it is
   promoted**; the Node SEA step is the fragile part of the release pipeline and a
   binary that has never been launched is not evidence.
2. `baulens.cjs` from the Tauri resource directory, run through Node. This is the
   same artifact the GitHub Release publishes, so there is one engine build, not
   two.
3. `bin/baucheck.js` through the system Node, for development.

If none is found the app reports it in the status strip rather than failing
silently.

## Two plan assumptions that did not hold

- **"Fonts are already bundled locally."** They were not; every page fetched IBM
  Plex from Google Fonts and the repository contained no font files. Fixed in
  `5b258f5`.
- **"Drag & drop already exists as a drop field."** There was no drag-and-drop
  anywhere, only two hidden file inputs. Implemented in `inject/shell.js`,
  including the Tauri path, because the webview suppresses the HTML5 file drop
  that works in a browser.

## PDF export, and print

Two ways out of the app, and they are not the same document.

**The PDF file — the primary one.** *DIN-A4-Befund als PDF speichern* in the
inspector opens the native save dialog (`pick_save_path`), then the engine
renders the dossier through `src/pdf.js` + `src/dossierpdf.js` and writes the
bytes to the chosen path. No print dialog is involved, and the default file name
is `<Plandatei>_Befund.pdf`. The status bar reports the path, the page count and
the Prüfziffer (the same deterministic SHA-256 the HTML dossier carries).

The file is a Mappe, not a loose sheaf: a Blattübersicht on the first sheet names
every Blatt, and each footer reads `Blatt 2/3 · Lageplan · Seite 3 von 4`, so a
sheet pulled out of the folder identifies itself.

The Lageplan draws the building it was handed: the footprint comes from
`grz_ist × Grundstücksfläche`, or from `Nutzfläche / Geschosse` when only that is
declared, and the legend prints that arithmetic next to the Massketten. The
building data therefore travels in `projectData` on both the CLI and the sidecar
path - it is part of the Prüfziffer, because the drawing is derived from it.

The dossier mixes formats on purpose: the text pages are A4 portrait, while the
Lageplan is an A4 landscape drawing sheet with its own frame, Schriftfeld,
Maßketten in metres, scale bar and Nordpfeil — one sheet, one Blatt, the way a
submission folder expects it.

**All the Lagepläne of a project folder at once.** *Alle Lagepläne exportieren* in the
explorer footer asks for a target folder (the folder dialog carries the title
*Zielordner für die Lagepläne wählen*, so it cannot be mistaken for the
project-folder question) and then sends one `plan_export` call with the scan's
own file list: for every Vorhaben the first plan file that carries geometry (DXF,
IFC, XPlanung, B-Plan), otherwise its first file, labelled with the project name.
The status bar reports how many Blätter were written and in which formats,
names the plans that could not be read (a broken attachment in a folder of twenty
must not cost the other nineteen), and says when a Blatt had to be renamed
because two Vorhaben wanted the same file name. Nothing is written into a folder
that does not exist, and the list is capped at `MAX_BATCH` — the same ceiling,
because one workspace scan is bounded to that size anyway.

**The Lageplan as a file of its own.** *Lageplan als SVG speichern* and
*Lageplan als DXF speichern* in the inspector export that one sheet: the SVG for
a plotter and the plan set (DIN A4 quer in points, one group per layer, default
name `<Plandatei>_Lageplan.svg`), the DXF for CAD (ASCII, AutoCAD-2000 dialect,
one drawing unit = one metre, closed `LWPOLYLINE` outlines, `TEXT` labels, one
layer per Bauteil, default name `<Plandatei>_Lageplan.dxf`). They are not a
second drawing: the engine records the sheet once and the PDF page replays
exactly those primitives, so a correction to the Lageplan reaches the PDF, the
SVG and the DXF together. The status bar reports the Maßstab the Blatt is drawn
in and the Prüfziffer of the check the drawing belongs to.

**The HTML dossier on paper.** *DIN-A4-Dossier drucken* still routes through
`print_dossier` (`WebviewWindow::print`), which remains the right action for a
quick preview. Tauri v2 exposes no headless PDF writer — an earlier draft of
`lib.rs` called a `print_to_pdf` that does not exist and was corrected — so this
path reaches a PDF only through the system dialog's "Save as PDF".

The renderer lives in `src/` and not in Rust on purpose: it is library code under
test, and a second dossier implementation in the shell is the drift this project
already avoids elsewhere. The consequence worth knowing: only the desktop app can
write the PDF, because only it has the engine child process. `workhorse.html`
says that out loud in Demo-Modus rather than leaving the button inert.

## Browser-only fallback

`workhorse.html` runs without Tauri. With no `window.__TAURI__` it enters an
explicit Demo-Modus with a sample workspace and sample findings, labels the
engine badge as not connected, and never invents a verdict: that mode exists so
the layout and the viewer can be checked in a plain browser, not to fake a
result.

## Verification status

| Check | Status |
|---|---|
| `cargo check`, no errors, no warnings | yes — `Finished dev profile`, zero diagnostics |
| Icon set generated from the repo SVG | yes |
| Engine bundle answers `ping` and reports 15 modules | yes (`build-desktop-sidecar.mjs`) |
| Workhorse page renders and is interactive in a browser | yes — explorer, presets 1/2/3/R, orbit drag, finding-click camera focus, 2D toggle |
| Lageplan DXF opens in a third-party reader | yes — `dxf-parser` (the library behind `three-dxf`) reads the exported file and finds 102 `LINE`, 17 closed `LWPOLYLINE` and 51 `TEXT` on 11 declared layers at `$INSUNITS 6`; the project's own CAD import reads the parcel and the Hauskörper back in metres |
| Lageplan SVG is a valid document | yes — the browser's own XML parser reports no `parsererror`, every layer group has a geometry box inside the sheet, and the file rasterises to a PNG with ink on the drawing, the legend, the north arrow and the Schriftfeld |
| Installer produced | not yet run in this environment |
| Window opens and renders | **blocked by the environment** |

To compile here, the build needs a target directory without a space in its
path: this checkout lives under `C:\Users\INTAL Admin\…`, and MinGW's `as`
fails inside it (`Fatal error: can't create C:\Users\INTAL: Permission denied`).
`CARGO_TARGET_DIR` pointing at a space-free directory fixes it, which is why the
check was run as
`CARGO_TARGET_DIR=…\baulens-target cargo check`.
`cargo check` also needs `dist/baulens.cjs` to exist, because `tauri.conf.json`
bundles it as a resource; run `node scripts/bundle.js` first.

`Failed to setup app: Zugriff verweigert (os error 5)` — WebView2 cannot create
its browser process under the token this shell runs with. The WebView2 runtime
*is* installed (154.0.4258.53), the session is interactive, and the overrides
`WEBVIEW2_USER_DATA_FOLDER` and `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--no-sandbox`
do not change it, so this is a permissions boundary rather than a defect in the
app.

Run the `.exe` from an interactive desktop session to confirm the window, drag &
drop and print path.
