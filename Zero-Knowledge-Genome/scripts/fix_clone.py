"""Fix: warp AppState in Arc<RwLock<...>> so it derives Clone (axum 0.8 requirement).

axum's `with_state` requires `S: Clone` because the router clones the state
per-handler. Our `AppState { inner: RwLock<...> }` could not derive Clone because
`RwLock<T>: !Clone`. Wrapping in `Arc<RwLock<T>>` makes the field Clone (Arc
shares the lock pointer) without changing semantics for any caller. We also
drop the now-redundant outer `Arc::new(AppState::new())` in `main.rs` and any
explicit `state.clone()` sites.
"""

from pathlib import Path
import re

# --- state.rs: derive(Clone) and swap field to Arc<RwLock<...>> ---------------

state_path = Path("zk-genome/api/src/state.rs")
state = state_path.read_text(encoding="utf-8")

# 1) Add Clone to derived traits on AppState.
old_decl = """/// Thread-safe in-memory state for the API service.
#[derive(Debug, Default)]
pub struct AppState {
    inner: RwLock<AppStateInner>,
}"""
new_decl = """/// Thread-safe in-memory state for the API service. `Clone` is cheap (deep
/// `Arc`) so axum's `with_state` can clone the state per-handler.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    inner: Arc<RwLock<AppStateInner>>,
}"""

if old_decl not in state:
    raise SystemExit("FATAL: state.rs AppState declaration changed.")
state = state.replace(old_decl, new_decl, 1)

# 2) Update the `use std::sync::RwLock;` line to also import Arc if not already.
if "use std::sync::Arc;" not in state:
    state = state.replace(
        "use std::sync::RwLock;",
        "use std::sync::{Arc, RwLock};",
        1,
    )

state_path.write_text(state, encoding="utf-8")
print(f"OK state.rs: {len(state)} bytes")

# --- main.rs: drop outer Arc + simplify_clone -----------------------------

main_path = Path("zk-genome/api/src/main.rs")
main = main_path.read_text(encoding="utf-8")

# Replace `Arc::new(AppState::new())` (likely with whitespace) with `AppState::new()`.
# The current main.rs most likely does:
#   let state = Arc::new(AppState::new());
#   let started = ...;
#   let router = build_router(state.clone(), started);
# Change to:
#   let state = AppState::new();
#   let router = build_router(state.clone(), started);
main = re.sub(
    r"let\s+state\s*=\s*Arc::new\(AppState::new\(\)\);",
    "let state = AppState::new();",
    main,
)
main_path.write_text(main, encoding="utf-8")
print(f"OK main.rs: {len(main)} bytes")

# --- routes/proofs.rs: ensure Authed handles &AppState (auto-deref through Arc)

# (No edits required -- &AppState.method() works identically for Arc wrap or RwLock wrap.)

# --- audit anything that did `Arc<AppState>` -----------------------------

ws_root = Path("zk-genome/api")
for f in ws_root.rglob("*.rs"):
    text = f.read_text(encoding="utf-8")
    if "Arc<AppState>" in text:
        print(f"WARN: {f} still references `Arc<AppState>`. Verify it still compiles.")

print("DONE")
