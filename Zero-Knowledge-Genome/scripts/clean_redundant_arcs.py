"""Drop the now-redundant outer Arc around AppState.

Since `AppState.inner` is now `Arc<RwLock<AppStateInner>>` and AppState is
`Clone`, every call site that wraps `Arc::new(AppState::new())` should drop
the outer Arc and just use `AppState::new()` / pass `AppState` directly.

Cleanup:
- `main.rs`: `build_router(state: Arc<AppState>, ...)` -> `build_router(state: AppState, ...)`.
- `routes/{auth,queries,proofs,stats}.rs`: `State<Arc<AppState>>` -> `State<AppState>`.
"""

from pathlib import Path
import re

ws_api = Path("zk-genome/api")

# --- main.rs ------------------------------------------------------------------

main_path = ws_api / "src" / "main.rs"
main = main_path.read_text(encoding="utf-8")

# Replace Signature/Usage of Arc<AppState> with AppState directly.
# Pattern: any `state: AppState` or `Arc<AppState>` adjustment.
# Just remove `Arc` from `Arc<AppState>` and `Arc::new(...)` reasons:
# - `let state = AppState::new();` was already set.
# - `build_router(state.clone(), ...)` works either way since AppState: Clone.
# - Drop `use std::sync::Arc;` import if unused.
# - Drop `state: Arc<AppState>` parameter type in build_router.

# If main.rs contains `Arc<AppState>`, replace with `AppState`.
main = main.replace("Arc<AppState>", "AppState", -1)
# Also drop the now-redundant `use std::sync::Arc;` import if present and only one use.
if main.count("Arc") == 0 or main.count("Arc;") + main.count("Arc }") + main.count("Arc,") == 0:
    main = re.sub(r"\nuse std::sync::Arc;\n", "\n", main, count=1)

main_path.write_text(main, encoding="utf-8")
print(f"OK main.rs: {len(main)} bytes")

# --- routes/*.rs --------------------------------------------------------------

for route_name in ("auth", "queries", "proofs", "stats"):
    rp = ws_api / "src" / "routes" / f"{route_name}.rs"
    if not rp.exists():
        continue
    text = rp.read_text(encoding="utf-8")
    text = text.replace("Arc<AppState>", "AppState", -1)
    # Also drop `use std::sync::Arc;` if no other use.
    if re.search(r"\bArc\b", text) is None or all(
        "Arc::" not in ln and not re.search(r"\bArc\b", ln.replace("AppState", ""))
        for ln in text.splitlines()
        if "Arc" in ln
    ):
        text = re.sub(r"\nuse std::sync::Arc;\n", "\n", text, count=1)
    rp.write_text(text, encoding="utf-8")
    print(f"OK routes/{route_name}.rs: {len(text)} bytes")

print("DONE")
