//! Spawns the BauLens engine as a child process and talks to it over the
//! line-delimited JSON protocol implemented in src/ipc.js.
//!
//! Why a child process rather than embedded rules: the engine is the code under
//! test (256 tests), and it is JavaScript. Rust cannot execute it, so the
//! alternative would be a second implementation of the rules inside this shell,
//! which is the drift this project has already been bitten by. The child speaks
//! one JSON object per line on stdin and stdout; see src/ipc.js for the schema.
//!
//! In development the CLI is launched through the system `node`. In a packaged
//! build `scripts/build-desktop-sidecar.mjs` produces a single-file
//! `baulens-sidecar.exe` next to the app binary, and that is preferred when it
//! exists.
//!
//! Beyond the engine transport this shell owns what a browser cannot do for the
//! workhorse UI:
//!   - a native folder dialog (`pick_workspace_folder`) and a bounded recursive
//!     project scan (`scan_workspace`) for the multi-project workspace,
//!   - batch checks (`batch_check`) and plain file access
//!     (`read_workspace_file`, `write_workspace_file`),
//!   - writing an engine export next to its drawing (`export_file`).
//! None of that reimplements a rule: the scan is filesystem work, and every
//! compliance answer still comes from the child process.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

/// A live engine process plus its pipes.
struct Sidecar {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    /// How the process was started, surfaced through `engine_info` so the UI can
    /// show whether it is running the bundled engine or a development one.
    origin: String,
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        // Closing stdin makes the engine's read loop end and the process exit.
        let _ = self.child.kill();
    }
}

#[derive(Default)]
struct EngineState(Mutex<Option<Sidecar>>);

/// Locates the engine and the argument style needed to run it.
///
/// Three shapes are supported, in order of preference:
///   1. a standalone `baulens-sidecar` executable beside the app (no runtime
///      needed; this is what a fully self-contained installer would ship),
///   2. `baulens.cjs` in the resource directory, which is the same tested
///      single-file build the release already publishes, run through Node,
///   3. the repository CLI through the system Node, for development.
fn resolve_engine(app: &tauri::AppHandle) -> Result<(PathBuf, Vec<String>, String), String> {
    let exe_name = if cfg!(windows) { "baulens-sidecar.exe" } else { "baulens-sidecar" };

    // 1. Beside the app binary.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let bundled = dir.join(exe_name);
            if bundled.exists() {
                let label = format!("bundled binary ({})", bundled.display());
                return Ok((bundled, vec!["--rpc".into()], label));
            }
        }
    }

    // 2. The published single-file build, staged as a Tauri resource.
    let resource_dirs = [
        app.path().resource_dir().ok(),
        app.path().resource_dir().ok().map(|d| d.join("resources")),
        app.path().resource_dir().ok().map(|d| d.join("_up_").join("dist")),
    ];
    for dir in resource_dirs.into_iter().flatten() {
        for candidate in ["baulens.cjs", "dist/baulens.cjs"] {
            let script = dir.join(candidate);
            if script.exists() {
                return Ok((
                    PathBuf::from("node"),
                    vec![script.to_string_lossy().into_owned(), "--rpc".into()],
                    format!("bundled script ({})", script.display()),
                ));
            }
        }
    }

    // 3. Development: the repository CLI through the system Node.
    if let Ok(cwd) = std::env::current_dir() {
        // Either src-tauri/ or the repository root, depending on how it was started.
        for repo in [cwd.join("..").join(".."), cwd.clone()] {
            let cli = repo.join("bin").join("baucheck.js");
            if cli.exists() {
                return Ok((
                    PathBuf::from("node"),
                    vec![cli.to_string_lossy().into_owned(), "--rpc".into()],
                    format!("development ({} via node)", cli.display()),
                ));
            }
        }
    }

    Err(
        "no engine found. Expected a bundled baulens-sidecar, a staged baulens.cjs, \
         or bin/baucheck.js in development."
            .into(),
    )
}

fn spawn_engine(app: &tauri::AppHandle) -> Result<Sidecar, String> {
    let (program, args, origin) = resolve_engine(app)?;

    let mut child = Command::new(&program)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("could not start the engine ({}): {e}", program.display()))?;

    let stdin = child.stdin.take().ok_or("engine stdin unavailable")?;
    let child_stdout = child.stdout.take().ok_or("engine stdout unavailable")?;
    let mut stdout = BufReader::new(child_stdout);

    // The engine announces itself before answering anything; consuming it here
    // means the first command the UI sends gets its own response back.
    let mut hello = String::new();
    stdout
        .read_line(&mut hello)
        .map_err(|e| format!("engine did not announce itself: {e}"))?;

    let parsed: Value =
        serde_json::from_str(hello.trim()).map_err(|e| format!("unreadable handshake: {e}"))?;
    if parsed.get("event").and_then(Value::as_str) != Some("ready") {
        return Err(format!("unexpected handshake from the engine: {}", hello.trim()));
    }

    Ok(Sidecar { child, stdin, stdout, origin })
}

/// One request/response exchange. The protocol is strictly line-oriented and
/// serialised by the mutex, so responses cannot interleave.
fn exchange(state: &EngineState, app: &tauri::AppHandle, request: Value) -> Result<Value, String> {
    let mut guard = state.0.lock().map_err(|_| "engine lock poisoned")?;

    if guard.is_none() {
        *guard = Some(spawn_engine(app)?);
    }

    let sidecar = guard.as_mut().ok_or("engine unavailable")?;

    let mut line = serde_json::to_string(&request).map_err(|e| e.to_string())?;
    line.push('\n');
    sidecar
        .stdin
        .write_all(line.as_bytes())
        .and_then(|_| sidecar.stdin.flush())
        .map_err(|e| format!("could not write to the engine: {e}"))?;

    let mut response = String::new();
    let read = sidecar
        .stdout
        .read_line(&mut response)
        .map_err(|e| format!("could not read from the engine: {e}"))?;

    if read == 0 {
        // The child died. Drop it so the next call respawns rather than failing
        // forever against a closed pipe.
        *guard = None;
        return Err("the engine exited unexpectedly".into());
    }

    serde_json::from_str(response.trim()).map_err(|e| format!("unreadable engine response: {e}"))
}

/// Reports what the shell is running so the UI can show it.
#[tauri::command]
fn engine_info(state: tauri::State<EngineState>, app: tauri::AppHandle) -> Result<Value, String> {
    let response = exchange(&state, &app, json!({ "id": 0, "method": "describe" }))?;
    let origin = {
        let guard = state.0.lock().map_err(|_| "engine lock poisoned")?;
        guard.as_ref().map(|s| s.origin.clone()).unwrap_or_default()
    };
    Ok(json!({ "transport": origin, "engine": response }))
}

/// Runs a compliance check in the engine. `params` is passed through verbatim,
/// so the schema lives in one place: src/ipc.js.
#[tauri::command]
fn engine_check(
    state: tauri::State<EngineState>,
    app: tauri::AppHandle,
    params: Value,
) -> Result<Value, String> {
    exchange(&state, &app, json!({ "id": 1, "method": "check", "params": params }))
}

/// Calls a named sidecar method (`statute`, `describe`, `check`, etc.) with
/// `params` forwarded verbatim.
#[tauri::command]
fn engine_method(
    state: tauri::State<EngineState>,
    app: tauri::AppHandle,
    method: String,
    params: Value,
) -> Result<Value, String> {
    exchange(&state, &app, json!({ "id": 1, "method": method, "params": params }))
}

/// Opens the platform print dialog for the current view.
///
/// Tauri v2 exposes `WebviewWindow::print`, not a headless PDF writer, so the
/// dossier is produced through the system dialog's "Save as PDF". An earlier
/// draft of this file claimed a headless `print_to_pdf`; it does not exist on
/// this type and the claim was wrong.
///
/// This is still worth having over the browser: the packaged app owns the
/// webview, so the page is printed from local files with no server and no
/// address bar.
#[tauri::command]
fn print_dossier(app: tauri::AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("no main window")?;
    window.print().map_err(|e| format!("print failed: {e}"))
}

/* --------------------------------------------------------------------------
 * Workspace: native folder dialog, bounded project scan, batch checks.
 *
 * None of this decides anything about compliance. The scan only tells the UI
 * which files are worth handing to the engine, and batch_check forwards each
 * file to `check_file` in src/ipc.js, which does the actual ingestion and
 * calculation. Keeping the rules in one place is the whole reason the engine is
 * a child process rather than a Rust module.
 * ------------------------------------------------------------------------ */

/// Depth, breadth and batch caps. A user who points the dialog at `C:\` should
/// get a slow answer, not a hung window.
const MAX_DEPTH: usize = 6;
const MAX_PROJECTS: usize = 200;
const MAX_FILES_PER_PROJECT: usize = 500;
const MAX_BATCH: usize = 200;

/// Directories that never hold plan data and are expensive to walk.
const SKIP_DIRS: [&str; 4] = ["node_modules", "target", "dist", ".git"];

/// Classifies a file for the explorer's icon and for the engine's ingestion.
/// `.json` is reported as `spec`; whether it is a spec or a GeoJSON B-Plan is
/// decided by the engine, which can actually parse it.
fn file_kind(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "ifc" => "ifc",
        "dxf" => "dxf",
        "gml" | "xml" => "xplan",
        "geojson" => "bplan",
        "bcf" | "bcfjson" => "bcf",
        "json" => "spec",
        "txt" | "md" => "text",
        _ => "other",
    }
}

/// Anything the engine can turn into a spec, plus the drawings themselves.
fn is_plan_file(path: &Path) -> bool {
    matches!(file_kind(path), "ifc" | "dxf" | "xplan" | "bplan" | "bcf" | "spec" | "text")
}

fn describe_file(path: &Path) -> Option<Value> {
    let name = path.file_name().and_then(|n| n.to_str())?.to_string();
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    Some(json!({
        "name": name,
        "path": path.display().to_string(),
        "kind": file_kind(path),
        "size": size,
    }))
}

/// Files directly inside `dir`, no recursion. Used for the root folder, whose
/// own files belong to the ad-hoc project rather than to a subdirectory.
fn shallow_plan_files(dir: &Path) -> Vec<Value> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return out,
    };

    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();

    for path in paths {
        if out.len() >= MAX_FILES_PER_PROJECT {
            break;
        }
        if path.is_dir() {
            continue;
        }
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with('.'))
            .unwrap_or(true)
        {
            continue;
        }
        if !is_plan_file(&path) {
            continue;
        }
        if let Some(file) = describe_file(&path) {
            out.push(file);
        }
    }

    out
}

/// Recursive plan-file collection for one project directory.
fn collect_plan_files(dir: &Path, depth: usize, out: &mut Vec<Value>) {
    if depth > MAX_DEPTH || out.len() >= MAX_FILES_PER_PROJECT {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        // A directory we cannot read is skipped: a locked folder inside a
        // project must not fail the whole scan.
        Ok(entries) => entries,
        Err(_) => return,
    };

    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();

    for path in paths {
        if out.len() >= MAX_FILES_PER_PROJECT {
            return;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            collect_plan_files(&path, depth + 1, out);
        } else if is_plan_file(&path) {
            if let Some(file) = describe_file(&path) {
                out.push(file);
            }
        }
    }
}

/// Recursively finds the projects under `path`.
///
/// One project is one directory that holds plan files: a subdirectory with a
/// drawing is a Vorhaben, and files sitting directly in the chosen folder are
/// the ad-hoc case from the plan — the colleague's mail attachment dropped into
/// a folder of its own.
#[tauri::command]
async fn scan_workspace(path: String) -> Result<Value, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("\"{path}\" ist kein lesbarer Ordner."));
    }

    let mut projects: Vec<Value> = Vec::new();

    let root_files = shallow_plan_files(&root);
    if !root_files.is_empty() {
        let name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Workspace")
            .to_string();
        projects.push(json!({
            "name": name,
            "path": root.display().to_string(),
            "files": root_files,
        }));
    }

    let entries = std::fs::read_dir(&root).map_err(|e| format!("Ordner \"{path}\" ist nicht lesbar: {e}"))?;
    let mut dirs: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    dirs.sort();

    for dir in dirs {
        if projects.len() >= MAX_PROJECTS {
            break;
        }
        if !dir.is_dir() {
            continue;
        }
        let name = match dir.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }

        let mut files = Vec::new();
        collect_plan_files(&dir, 0, &mut files);
        if files.is_empty() {
            continue;
        }

        projects.push(json!({
            "name": name,
            "path": dir.display().to_string(),
            "files": files,
        }));
    }

    let file_count: usize = projects
        .iter()
        .map(|p| p.get("files").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0))
        .sum();

    Ok(json!({
        "root": root.display().to_string(),
        "fileCount": file_count,
        "projects": projects,
    }))
}

/// Native folder dialog. Returns `None` when the user cancels.
///
/// The title is a parameter because this dialog is now used twice and the two
/// uses are not the same question: once for the project folder to scan, once for
/// the target folder the Lagepläne are written into. An untitled folder dialog
/// cannot tell the user which of the two it is asking for, and picking the
/// workspace folder when the export wanted a target is the kind of mistake that
/// ends with fifteen drawings in the wrong place. Missing title means no title,
/// so the original caller is unaffected.
///
/// `blocking_pick_folder` must not be called on the main thread, and a Tauri
/// command declared `async` runs on the async runtime rather than the main
/// thread — hence the spawn_blocking, which keeps the dialog call synchronous
/// while the command itself stays awaitable.
#[tauri::command]
async fn pick_workspace_folder(
    app: tauri::AppHandle,
    title: Option<String>,
) -> Result<Option<String>, String> {
    let handle = app.clone();

    let picked = tauri::async_runtime::spawn_blocking(move || {
        let dialog = handle.dialog().file();
        let dialog = match title {
            Some(title) => dialog.set_title(title),
            None => dialog,
        };
        dialog.blocking_pick_folder()
    })
    .await
    .map_err(|e| format!("Ordnerdialog konnte nicht geöffnet werden: {e}"))?;

    Ok(picked
        .and_then(|file| file.into_path().ok())
        .map(|p| p.display().to_string()))
}

/// Native save dialog for the PDF dossier. Returns `None` when the user
/// cancels.
///
/// Structured exactly like `pick_workspace_folder`, for the same reason: the
/// blocking dialog call must not run on the main thread, and an `async` command
/// runs on the async runtime, so the synchronous picker goes to the blocking
/// pool while the command stays awaitable.
///
/// This command only chooses a path. It deliberately does **not** write the PDF:
/// the engine renders the bytes, so the engine writes them too. A second write
/// path in Rust is a second implementation of the same document.
#[tauri::command]
async fn pick_save_path(
    app: tauri::AppHandle,
    default_name: String,
) -> Result<Option<String>, String> {
    let handle = app.clone();

    let picked = tauri::async_runtime::spawn_blocking(move || {
        let ext = Path::new(&default_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("pdf")
            .to_ascii_lowercase();
        let (label, filter_ext): (&str, &[&str]) = match ext.as_str() {
            "svg" => ("SVG-Lageplan", &["svg"]),
            "dxf" => ("DXF-Lageplan", &["dxf"]),
            "xml" => ("XBau XML", &["xml"]),
            "bcfjson" | "bcf" => ("BCF Issues", &["bcfjson", "bcf"]),
            "txt" => ("Textdokument", &["txt"]),
            _ => ("PDF-Dossier", &["pdf"]),
        };
        handle
            .dialog()
            .file()
            .set_file_name(default_name.clone())
            .add_filter(label, filter_ext)
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("Speicherdialog konnte nicht geöffnet werden: {e}"))?;

    Ok(picked
        .and_then(|file| file.into_path().ok())
        .map(|p| p.display().to_string()))
}

/// Reads a plan file as text for the viewer. The engine reads its own files for
/// checks; this is for the explorer's preview pane.
#[tauri::command]
async fn read_workspace_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path)
        .map_err(|e| format!("\"{path}\" konnte nicht gelesen werden: {e}"))
}

/// Writes an export (XBau XML, BCF JSON, dossier HTML) to the path the UI
/// passed. The caller chooses the path — deliberately, because only the UI knows
/// which project's folder the export belongs in. The parent directory must
/// already exist: an export should not silently create a folder tree.
#[tauri::command]
async fn write_workspace_file(path: String, contents: String) -> Result<(), String> {
    let target = PathBuf::from(&path);

    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            return Err(format!("Zielordner \"{}\" existiert nicht.", parent.display()));
        }
    }

    std::fs::write(&target, contents)
        .map_err(|e| format!("\"{}\" konnte nicht geschrieben werden: {e}", target.display()))
}

/// Checks many files in one call, for the "Ordner prüfen" action.
///
/// Each file goes through the engine's `check_file`, which ingests DXF, IFC,
/// XPlanung, B-Plan GeoJSON/Text or a JSON spec and then runs the rule set. A
/// file that fails — unreadable, unparseable, invalid spec — is recorded with
/// its error and the batch continues: one broken attachment in a folder of
/// fifteen must not lose the other fourteen results.
#[tauri::command]
fn batch_check(
    state: tauri::State<EngineState>,
    app: tauri::AppHandle,
    files: Vec<String>,
    land: Option<String>,
    stadt: Option<String>,
) -> Result<Value, String> {
    let mut results: Vec<Value> = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut single: Option<Value> = None;

    for (index, path) in files.iter().take(MAX_BATCH).enumerate() {
        let request = json!({
            "id": index + 1,
            "method": "check_file",
            "params": { "path": path, "land": land, "stadt": stadt, "repair": true },
        });

        match exchange(&state, &app, request) {
            Ok(response) if response.get("ok").and_then(Value::as_bool) == Some(true) => {
                let result = response.get("result").cloned().unwrap_or(Value::Null);
                let verdict = result.get("verdict").cloned().unwrap_or(Value::Null);
                let counts = result.get("counts").cloned().unwrap_or(Value::Null);

                // The engine's three verdicts: "nicht bestanden" is a hard
                // failure, "bestanden" and "mit Auflagen" are not.
                if verdict.as_str() == Some("nicht bestanden") {
                    failed += 1;
                } else {
                    passed += 1;
                }

                // A single-file call is the inspector's "show me this plan" path,
                // so it carries the full engine result (findings, profile,
                // notes). A batch of fifty does not: the explorer needs the
                // verdict for its ampel, and the detail is fetched per file when
                // one is selected, which is again a single-file call.
                if files.len() == 1 {
                    single = Some(result.clone());
                }

                results.push(json!({
                    "path": path,
                    "ok": true,
                    "verdict": verdict,
                    "counts": counts,
                    "error": Value::Null,
                    "detail": if files.len() == 1 { result } else { Value::Null },
                }));
            }
            Ok(response) => {
                let message = response
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .unwrap_or("unbekannter Fehler")
                    .to_string();
                failed += 1;
                results.push(json!({
                    "path": path,
                    "ok": false,
                    "verdict": Value::Null,
                    "counts": Value::Null,
                    "error": message,
                }));
            }
            Err(error) => {
                failed += 1;
                results.push(json!({
                    "path": path,
                    "ok": false,
                    "verdict": Value::Null,
                    "counts": Value::Null,
                    "error": error,
                }));
            }
        }
    }

    let mut aggregate = json!({
        "checked": results.len(),
        "passed": passed,
        "failed": failed,
        "results": results,
    });

    /* For one file the engine's own result is spread over the answer, so the UI
       renders the findings straight from what it asked for. */
    if let Some(detail) = single {
        if let (Some(target), Value::Object(fields)) = (aggregate.as_object_mut(), detail) {
            for (key, value) in fields {
                target.insert(key, value);
            }
        }
    }

    Ok(aggregate)
}

/// Writes an engine export next to the drawing it belongs to.
///
///   kind "xbau" -> XBau 2.3 XML     (engine field `xbau`, a string)
///   kind "bcf"  -> BCF 2.1 issues   (engine field `bcf`, an object)
///
/// The engine builds both documents during a check, so this reuses the same
/// `check_file` path as the batch run: read the plan, check it, write what came
/// back. No document is assembled here, which is the point — a second XBau
/// writer in Rust is exactly the drift this project has already been bitten by.
#[tauri::command]
fn export_file(
    state: tauri::State<EngineState>,
    app: tauri::AppHandle,
    path: String,
    kind: String,
    target: String,
    land: Option<String>,
    stadt: Option<String>,
) -> Result<Value, String> {
    let flag = match kind.as_str() {
        "xbau" => "xbau".to_string(),
        "bcf" => "bcf".to_string(),
        "abweichung" | "deviations" => "abweichung".to_string(),
        other => return Err(format!("Unbekannter Exporttyp \"{other}\"")),
    };

    let mut params = serde_json::Map::new();
    params.insert("path".to_string(), json!(path.clone()));
    params.insert(flag.clone(), json!(true));
    if let Some(land) = land {
        params.insert("land".to_string(), json!(land));
    }
    if let Some(stadt) = stadt {
        params.insert("stadt".to_string(), json!(stadt));
    }

    let response = exchange(
        &state,
        &app,
        json!({ "id": 1, "method": "check_file", "params": Value::Object(params) }),
    )?;

    if response.get("ok").and_then(Value::as_bool) != Some(true) {
        let message = response
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("unbekannter Fehler");
        return Err(format!("Die Engine konnte {path} nicht prüfen: {message}"));
    }

    let document = response
        .pointer(&format!("/result/{flag}"))
        .cloned()
        .ok_or_else(|| format!("Die Engine hat kein {flag}-Dokument geliefert."))?;

    let contents = match document {
        Value::String(text) => text,
        other => serde_json::to_string_pretty(&other)
            .map_err(|e| format!("Das {flag}-Dokument ließ sich nicht serialisieren: {e}"))?,
    };

    /* The UI passes a file name, because it is the UI that knows which project
       the export belongs to; the folder is the drawing's own. */
    let source = PathBuf::from(&path);
    let out = match source.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.join(&target),
        _ => PathBuf::from(&target),
    };

    std::fs::write(&out, contents.as_bytes())
        .map_err(|e| format!("\"{}\" konnte nicht geschrieben werden: {e}", out.display()))?;

    Ok(json!({ "target": out.display().to_string(), "bytes": contents.len() }))
}

/// Sends one engine method and unwraps its answer — the one place where the
/// protocol's envelope is understood.
///
/// Every command that is a bridge rather than a renderer goes through here, so
/// the `ok`/`error` unwrapping and the "no result" fallback exist once. A second
/// copy of this is a second place to forget that an engine error must reach the
/// UI as its own message rather than as a silent empty answer.
fn exchange_method(
    state: &tauri::State<EngineState>,
    app: &tauri::AppHandle,
    method: &str,
    noun: &str,
    params: serde_json::Map<String, Value>,
) -> Result<Value, String> {
    let response = exchange(
        state,
        app,
        json!({ "id": 1, "method": method, "params": Value::Object(params) }),
    )?;

    if response.get("ok").and_then(Value::as_bool) != Some(true) {
        let message = response
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("unbekannter Fehler");
        return Err(format!("{noun} konnte nicht erzeugt werden: {message}"));
    }

    response
        .get("result")
        .cloned()
        .ok_or_else(|| "Die Engine hat kein Ergebnis geliefert.".to_string())
}

/// Forwards one document request to the engine and unwraps its answer.
///
/// Shared by the three document commands below, because they differ in exactly
/// two things: the engine method and the noun in the error message. Repeating
/// forty lines of parameter plumbing three times would be three chances for the
/// three commands to drift apart — and a `plan_dxf` that forgot `land` would
/// export a drawing from a different Rechtsbereich than the Dossier beside it,
/// which is precisely the disagreement the engine's single implementation
/// exists to prevent.
///
/// There is no renderer here: the engine child process builds and writes the
/// bytes because it is the only place that holds them. A command is still
/// needed, because `engine_check` is hardcoded to the `check` method and the
/// frontend cannot reach another engine method without a bridge. Same rule
/// `export_file` follows for XBau and BCF.
///
/// `file` is a plan file (the engine ingests and checks it first, which is how
/// the document cannot disagree with the check that produced it); `out` is the
/// target path from `pick_save_path`. Everything is forwarded verbatim, so the
/// engine's own answer is what the UI reports.
#[allow(clippy::too_many_arguments)]
fn request_document(
    state: &tauri::State<EngineState>,
    app: &tauri::AppHandle,
    method: &str,
    noun: &str,
    file: Option<String>,
    spec: Option<Value>,
    out: Option<String>,
    land: Option<String>,
    stadt: Option<String>,
    aktenzeichen: Option<String>,
    datum: Option<String>,
    variant: Option<Value>,
) -> Result<Value, String> {
    if file.is_none() && spec.is_none() {
        return Err("Weder Plandatei noch Spezifikation übergeben.".to_string());
    }

    let mut params = serde_json::Map::new();
    if let Some(path) = file {
        params.insert("file".to_string(), json!(path));
    }
    if let Some(spec) = spec {
        params.insert("spec".to_string(), spec);
    }
    if let Some(path) = out {
        params.insert("out".to_string(), json!(path));
    }
    if let Some(land) = land {
        params.insert("land".to_string(), json!(land));
    }
    if let Some(stadt) = stadt {
        params.insert("stadt".to_string(), json!(stadt));
    }
    if let Some(aktenzeichen) = aktenzeichen {
        params.insert("aktenzeichen".to_string(), json!(aktenzeichen));
    }
    if let Some(datum) = datum {
        params.insert("datum".to_string(), json!(datum));
    }
    if let Some(variant) = variant {
        params.insert("variant".to_string(), variant);
    }

    exchange_method(state, app, method, noun, params)
}

/// Renders the DIN-A4 dossier as a PDF through the engine and writes it where
/// the save dialog pointed.
///
/// The engine's answer — `{ out, bytes, pages, hash }` — is what the UI reports.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn dossier_pdf(
    state: tauri::State<EngineState>,
    app: tauri::AppHandle,
    file: Option<String>,
    spec: Option<Value>,
    out: Option<String>,
    land: Option<String>,
    stadt: Option<String>,
    aktenzeichen: Option<String>,
    datum: Option<String>,
    variant: Option<Value>,
) -> Result<Value, String> {
    request_document(
        &state,
        &app,
        "dossier_pdf",
        "Das PDF",
        file,
        spec,
        out,
        land,
        stadt,
        aktenzeichen,
        datum,
        variant,
    )
}

/// Writes the Lageplan-Blatt (the dossier's drawing sheet) as an SVG file.
///
/// A plotter and a plan set need vectors, not page five of a PDF, so the same
/// sheet the dossier prints leaves as a file of its own. The engine renders it
/// from the same run and the same `projectData` as the PDF, which is why the
/// exported Blatt cannot be a different drawing — and why it comes back with the
/// Maßstab ("1:250") and the Prüfziffer of the check it belongs to.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn plan_svg(
    state: tauri::State<EngineState>,
    app: tauri::AppHandle,
    file: Option<String>,
    spec: Option<Value>,
    out: Option<String>,
    land: Option<String>,
    stadt: Option<String>,
    aktenzeichen: Option<String>,
    datum: Option<String>,
) -> Result<Value, String> {
    request_document(
        &state,
        &app,
        "plan_svg",
        "Das Lageplan-SVG",
        file,
        spec,
        out,
        land,
        stadt,
        aktenzeichen,
        datum,
        None,
    )
}

/// Writes the Lageplan-Blatt of every given plan into one folder — the workspace
/// export: a scanned project folder, one button, a folder full of Blätter.
///
/// The loop itself stays in the engine, and that is the point of this being a
/// forwarder: which file a Blatt is called, and that a collision between two
/// Vorhaben must not silently overwrite one drawing, are engine facts (see
/// `planOutputNames` in `src/planexport.js`), and the Prüfziffer of each Blatt
/// has to be the one of the plan it was drawn from. Rust knows the folder and
/// nothing else. `files` therefore travels as opaque JSON: the frontend hands
/// over the scan's own descriptors plus the label the Blatt is filed under, and
/// nothing here has an opinion about their shape.
///
/// The list is capped at `MAX_BATCH`, the same limit the batch check uses: one
/// list from one workspace scan, one ceiling. The scan is already bounded to
/// that size, so the cap only ever catches a caller that built its own list.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn plan_export(
    state: tauri::State<EngineState>,
    app: tauri::AppHandle,
    files: Vec<Value>,
    out_dir: String,
    formats: Vec<String>,
    land: Option<String>,
    stadt: Option<String>,
    aktenzeichen: Option<String>,
    datum: Option<String>,
) -> Result<Value, String> {
    if files.is_empty() {
        return Err("Es wurde kein Plan für den Lageplan-Export übergeben.".to_string());
    }

    let mut selected = files;
    selected.truncate(MAX_BATCH);

    let mut params = serde_json::Map::new();
    params.insert("files".to_string(), Value::Array(selected));
    params.insert("outDir".to_string(), json!(out_dir));
    if !formats.is_empty() {
        params.insert("formats".to_string(), json!(formats));
    }
    if let Some(land) = land {
        params.insert("land".to_string(), json!(land));
    }
    if let Some(stadt) = stadt {
        params.insert("stadt".to_string(), json!(stadt));
    }
    if let Some(aktenzeichen) = aktenzeichen {
        params.insert("aktenzeichen".to_string(), json!(aktenzeichen));
    }
    if let Some(datum) = datum {
        params.insert("datum".to_string(), json!(datum));
    }

    exchange_method(&state, &app, "plan_export", "Der Lageplan-Export", params)
}

/// Writes the Lageplan-Blatt as DXF, one drawing unit to the metre.
///
/// The same forwarder as `plan_svg`: the file is what a CAD practice inserts into
/// an existing plan set, so the geometry has to be in real drawing units and on
/// named layers, and only the engine knows the scale that turns paper into
/// terrain.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn plan_dxf(
    state: tauri::State<EngineState>,
    app: tauri::AppHandle,
    file: Option<String>,
    spec: Option<Value>,
    out: Option<String>,
    land: Option<String>,
    stadt: Option<String>,
    aktenzeichen: Option<String>,
    datum: Option<String>,
) -> Result<Value, String> {
    request_document(
        &state,
        &app,
        "plan_dxf",
        "Das Lageplan-DXF",
        file,
        spec,
        out,
        land,
        stadt,
        aktenzeichen,
        datum,
        None,
    )
}

/// The desktop app's licensing state: when the trial started and which Polar
/// license key (if any) was last validated, with the time of that validation.
///
/// Kept in the app config directory rather than the webview's localStorage for
/// the same reason native apps keep their settings outside the browser profile:
/// the file survives a webview data clear, its location is documented, and the
/// shell — not the page — owns the write. The schema lives in src/licensing.js;
/// this file is storage, and it never interprets what it holds.
const LICENSE_STATE_FILE: &str = "license-state.json";

fn license_state_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("App-Konfigurationsverzeichnis nicht verfügbar: {e}"))?;
    Ok(dir.join(LICENSE_STATE_FILE))
}

/// Returns the stored licensing state, or `null` on first start. A corrupt
/// file answers `null` rather than an error: losing the file must not brick
/// the app, and the JS side re-creates it (fresh trial stamp) in that case.
#[tauri::command]
fn license_state_read(app: tauri::AppHandle) -> Result<Value, String> {
    let path = license_state_path(&app)?;
    if !path.exists() {
        return Ok(Value::Null);
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("Lizenzstatus \"{}\" konnte nicht gelesen werden: {e}", path.display()))?;
    Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
}

/// Persists the licensing state written by src/licensing.js. The payload is
/// size-capped and must be a JSON object, so a misbehaving caller cannot fill
/// the config directory with garbage.
#[tauri::command]
fn license_state_write(app: tauri::AppHandle, state: Value) -> Result<(), String> {
    if !state.is_object() {
        return Err("Lizenzstatus muss ein JSON-Objekt sein.".into());
    }
    let text = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
    if text.len() > 4096 {
        return Err("Lizenzstatus ist zu groß.".into());
    }

    let path = license_state_path(&app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Konfigurationsverzeichnis konnte nicht angelegt werden: {e}"))?;
    }
    std::fs::write(&path, text)
        .map_err(|e| format!("Lizenzstatus \"{}\" konnte nicht geschrieben werden: {e}", path.display()))
}

/// Opens a URL in the platform browser. Only https URLs on hosts the app
/// actually points at are allowed — this command exists so the paywall's
/// „Lizenz kaufen" reaches the configured checkout, not so arbitrary page
/// code can launch a browser at any address.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    const ALLOWED_HOSTS: [&str; 3] = ["buy.polar.sh", "polar.sh", "lens.kellersystems.dev"];

    let trimmed = url.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .ok_or("Nur https-Links sind erlaubt.")?;
    let host = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ALLOWED_HOSTS.contains(&host.as_str()) {
        return Err(format!("Link \"{host}\" ist nicht freigegeben."));
    }

    let mut command = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", trimmed]);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(trimmed);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(trimmed);
        c
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Browser konnte nicht geöffnet werden: {e}"))
}

#[cfg(windows)]
fn show_fatal_error(title: &str, message: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    let wide_title: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();
    let wide_msg: Vec<u16> = OsStr::new(message).encode_wide().chain(Some(0)).collect();
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            std::ptr::null_mut(),
            wide_msg.as_ptr(),
            wide_title.as_ptr(),
            windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_fatal_error(title: &str, message: &str) {
    eprintln!("[{}] {}", title, message);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(EngineState::default())
        .invoke_handler(tauri::generate_handler![
            engine_info,
            engine_check,
            engine_method,
            print_dossier,
            pick_workspace_folder,
            pick_save_path,
            scan_workspace,
            read_workspace_file,
            write_workspace_file,
            batch_check,
            export_file,
            dossier_pdf,
            plan_svg,
            plan_dxf,
            plan_export,
            license_state_read,
            license_state_write,
            open_url
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        let err_str = err.to_string();
        let detail = if err_str.contains("Zugriff verweigert") || err_str.contains("os error 5") {
            format!(
                "BauLens konnte nicht gestartet werden:\n\n{}\n\n\
                Ursache: Microsoft WebView2 hat in dieser Umgebung keine ausreichenden Ausführungsrechte.\n\
                Tipp: Starten Sie die Anwendung direkt per Doppelklick in Ihrer normalen Windows-Benutzersitzung.",
                err_str
            )
        } else {
            format!("BauLens konnte nicht gestartet werden:\n\n{}", err_str)
        };
        show_fatal_error("BauLens – Startfehler", &detail);
    }
}
