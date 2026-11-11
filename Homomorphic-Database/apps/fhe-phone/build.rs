//! Build script: deploy the WebView2 loader DLL next to the produced binary.
//!
//! `wry` needs `WebView2Loader.dll` at runtime. The `webview2-com-sys`
//! build script only copies that DLL into its *private* `OUT_DIR` for the
//! linker — it is never placed beside the built `.exe`, so launching
//! `fhe-phone.exe` fails with "WebView2Loader.dll was not found".
//!
//! This script copies the vendored loader (checked in under this crate's
//! root, `apps/fhe-phone/WebView2Loader.dll`) into the cargo target profile
//! directory where the exe is produced, on every build.

use std::{env, fs, path::PathBuf};

/// Where the built binaries land: `<target-dir>/<profile>` (e.g.
/// `C:/fhe-target/release`).
///
/// Derivation: Cargo always places this build script's `OUT_DIR` at
/// `<target-dir>/<profile>/build/<pkg>-<hash>/out`, so climbing three parents
/// from `OUT_DIR` yields the profile dir that contains the produced `.exe`.
/// This works regardless of a `target-dir` override in `.cargo/config.toml`.
fn target_bin_dir() -> PathBuf {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("cargo always sets OUT_DIR"));
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR is always <target>/<profile>/build/<pkg>/out");
    assert!(
        profile_dir.is_dir(),
        "unexpected OUT_DIR layout: {}",
        out_dir.display()
    );
    profile_dir.to_path_buf()
}

fn main() {
    println!("cargo:rerun-if-changed=WebView2Loader.dll");

    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("WebView2Loader.dll");
    assert!(
        source.is_file(),
        "vendored WebView2Loader.dll missing at {source:?}"
    );

    let dest_dir = target_bin_dir();
    fs::create_dir_all(&dest_dir).expect("failed to create target profile directory");
    let dest = dest_dir.join("WebView2Loader.dll");

    // Copy only when the payload differs so rebuilds stay fast and quiet.
    let changed = fs::read(&dest)
        .ok()
        .map(|existing| existing != fs::read(&source).expect("read vendored loader"))
        .unwrap_or(true);
    if changed {
        fs::copy(&source, &dest).expect("failed to deploy WebView2Loader.dll beside the binary");
        println!(
            "cargo:warning=deployed WebView2Loader.dll -> {}",
            dest.display()
        );
    }

    // Notify the host GUI tooling that a system DLL is required beside the exe.
    println!("cargo:rustc-env=WEBVIEW2_LOADER_PATH={}", dest.display());
}
