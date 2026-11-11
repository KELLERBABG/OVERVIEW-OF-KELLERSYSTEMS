//! The phone shell: a tao window rendered by wry.
//!
//! The webview loads the Android-style UI and talks to Rust through wry's
//! `window.ipc.postMessage`. Heavy FHE work (keygen, encryption, inference)
//! runs on worker threads; results are pushed back to the page with
//! `evaluate_script` so the UI never blocks.

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::Value;
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::http::{Request, Response};
use wry::WebView;

use crate::engine::{dispatch, SharedEngine};

/// The embedded Android-style UI.
const SHELL_HTML: &str = include_str!("../assets/index.html");

/// Custom event carrying one JSON result string back to the UI thread.
struct UiResult(String);

/// Run the phone shell until the user closes the window.
///
/// `engine` is the shared FHE session; the window is a phone-like 420×860
/// webview bound to the embedded server.
pub fn run(engine: SharedEngine) -> anyhow::Result<()> {
    let event_loop = EventLoop::new();
    let (result_tx, result_rx) = mpsc::channel::<UiResult>();

    let window = WindowBuilder::new()
        .with_title("FHE-DB — Fully Homomorphic Phone")
        .with_inner_size(LogicalSize::new(420.0, 860.0))
        .with_min_inner_size(LogicalSize::new(380.0, 700.0))
        .build(&event_loop)?;

    // WebView is neither Send nor Sync and is only ever touched from the
    // (single) event-loop thread, so an `Rc` is the right owner here — an
    // `Arc` would falsely advertise cross-thread sharing.
    let holder: Rc<RefCell<Option<WebView>>> = Rc::new(RefCell::new(None));

    let builder = wry::WebViewBuilder::new()
        .with_custom_protocol("fhe".into(), |_id, _req| {
            let mut response = Response::new(Cow::from(SHELL_HTML.as_bytes()));
            response.headers_mut().insert(
                "Content-Type",
                wry::http::HeaderValue::from_static("text/html; charset=utf-8"),
            );
            response
        })
        .with_url("fhe://index.html")
        .with_ipc_handler({
            let engine = engine.clone();
            let result_tx = result_tx.clone();
            move |request: Request<String>| {
                let body = request.body().clone();
                let engine = engine.clone();
                let result_tx = result_tx.clone();
                // Heavy FHE ops (keygen, homomorphic eval) run off the UI
                // thread; the only thing the webview thread does is spawn.
                std::thread::spawn(move || {
                    let req: Value = serde_json::from_str(&body)
                        .unwrap_or_else(|_| serde_json::json!({ "op": "unknown", "id": 0 }));
                    let payload = dispatch(&engine, &req);
                    if let Ok(json) = serde_json::to_string(&payload) {
                        let _ = result_tx.send(UiResult(json));
                    }
                });
            }
        });

    let webview = builder.build(&window)?;
    *holder.borrow_mut() = Some(webview);

    event_loop.run(move |event, _window_target, control_flow| {
        // Wake every 50 ms so completed worker results get drained into the
        // page promptly; the FHE work itself never blocks this thread.
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(50));
        match event {
            Event::NewEvents(StartCause::Init)
            | Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                // Drain any completed worker results into the page.
                while let Ok(UiResult(payload)) = result_rx.try_recv() {
                    if let Some(view) = holder.borrow().as_ref() {
                        deliver_result(view, &payload);
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

/// Deliver one JSON result payload into the page.
///
/// `assets/index.html` installs a host callback slot, `__fhe_on_result`,
/// which resolves the pending promise for the matching IPC request `id`
/// (see the `call()` helper in the page). wry 0.56's IPC handler is
/// fire-and-forget (`Fn(Request<String>)` — it cannot return a reply to
/// `window.ipc.postMessage`), so completed work must be pushed back into
/// the page from the UI thread with `evaluate_script`.
///
/// `payload` is always `serde_json::to_string` output (valid JSON), so
/// embedding it directly cannot break out of the script literal. The
/// `window.__fhe_on_result &&` guard keeps this a no-op if the page is
/// still loading or has been reloaded mid-flight.
fn deliver_result(view: &WebView, payload: &str) {
    let script = format!("window.__fhe_on_result && __fhe_on_result({payload})");
    if let Err(err) = view.evaluate_script(&script) {
        tracing::warn!("failed to deliver IPC result to the page: {err}");
    }
}
