//! Build script: which Z3 to link against, and how.
//!
//! The discovery is not implemented here. `src/host_env.rs` holds it and this
//! file includes that module, so the Z3 the verifier is linked against is chosen
//! by the same code that tells the test runners which solver to re-discharge the
//! obligations with. They used to be separate copies of the rule, which is how a
//! checkout with two Z3 releases unpacked could link one and run the other.

use std::path::PathBuf;

// Shared verbatim with `src/bin/keller-env.rs`. `dead_code` and `unused_imports`
// are allowed because a build script needs only part of the module — what an
// install root looks like and which one to use; the rest is the runtime half
// (the solver that starts, and the `PATH` the runner has to build) and would be
// a warning here for every item this file does not call.
#[allow(dead_code, unused_imports)]
#[path = "src/host_env.rs"]
mod host_env;

use host_env::{looks_like_z3_root, z3_root, RealProbe};

fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo"),
    );

    // The driver is linked to this exact nightly/host. Cargo dependencies must
    // use the same compiler, not a target project's different rustup override.
    let discovered = host_env::discover(&manifest_dir);
    if let Some(ref rustc) = discovered.rustc {
        println!("cargo:rustc-env=KELLER_DRIVER_RUSTC={}", rustc.display());
        // `cargo` itself is the rustup shim, and the shim follows the checked
        // project's `rust-toolchain.toml` unless `RUSTUP_TOOLCHAIN` says
        // otherwise. Recording the toolchain name here is what lets the verify
        // command select the compiler this driver is linked to.
        if let Some(toolchain) = host_env::toolchain_name(rustc) {
            println!("cargo:rustc-env=KELLER_DRIVER_TOOLCHAIN={toolchain}");
        }
        // The exact compiler, not just the channel: `rustc_private` changes shape
        // between nightlies, so a binary is only valid for the commit it was
        // linked against. `src/compiler.rs` reads these back and compares them to
        // the installed toolchain before a run starts.
        if let Some(identity) = host_env::rustc_identity(rustc) {
            println!("cargo:rustc-env=KELLER_RUSTC_RELEASE={}", identity.release);
            println!("cargo:rustc-env=KELLER_RUSTC_COMMIT={}", identity.commit);
            println!("cargo:rustc-env=KELLER_RUSTC_HOST={}", identity.host);
        }
    } else {
        panic!("Could not identify the compiler linked into the Keller driver");
    }
    // The target and profile this binary was built for. A verdict can only be
    // reproduced on the same triple, and "debug assertions on" is what makes the
    // encoded obligations the ones rustc emits at all.
    for (key, var) in [
        ("KELLER_BUILD_TARGET", "TARGET"),
        ("KELLER_BUILD_PROFILE", "PROFILE"),
    ] {
        if let Ok(value) = std::env::var(var) {
            println!("cargo:rustc-env={key}={value}");
        }
    }

    if let Some(ref lib_dir) = discovered.lib_dir {
        println!(
            "cargo:rustc-env=KELLER_DRIVER_RUSTLIB={}",
            lib_dir.display()
        );
    }

    // Resolution order: explicit Z3_HOME, project-local .z3/, platform default.
    let (z3_path, source) = z3_root(
        std::env::var("Z3_HOME").ok().as_deref(),
        &manifest_dir,
        &RealProbe,
    );

    if !looks_like_z3_root(&z3_path, &RealProbe) {
        println!(
            "cargo:warning=Z3 not found at: {} (from {})",
            z3_path.display(),
            source.label()
        );
        println!(
            "cargo:warning=Set Z3_HOME to your Z3 install root, or unpack a Z3 release under .z3/."
        );
        println!("cargo:warning=Expected layout: <root>/bin (libz3.dll/.so/.dylib) and <root>/include/z3.h");
        println!("cargo:warning=Download: https://github.com/Z3Prover/z3/releases");
        println!(
            "cargo:warning=The build will likely fail with a linker error until Z3 is present."
        );
    }

    let z3_lib = z3_path.join("bin");
    println!("cargo:rustc-env=KELLER_DRIVER_Z3_BIN={}", z3_lib.display());

    // Pass the library search path to the linker.
    println!("cargo:rustc-link-search=native={}", z3_lib.display());

    // Link against libz3 (the Windows release names it libz3.dll/.lib).
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-lib=libz3");
    #[cfg(not(target_os = "windows"))]
    println!("cargo:rustc-link-lib=z3");

    // NOTE: z3-sys' bindgen locates the header through the Z3_SYS_Z3_HEADER
    // environment variable. That variable has to be exported by the *build
    // environment* (or set in `.cargo/config.toml`'s `[env]` table) — a
    // `cargo:rustc-env` directive here only affects compiling this crate and
    // would never reach a dependency's build script.
    println!("cargo:rerun-if-env-changed=Z3_HOME");
    println!("cargo:rerun-if-env-changed=Z3_SYS_Z3_HEADER");
    println!("cargo:rerun-if-changed=build.rs");
    // Included above, so it is an input to the link path like any other: without
    // this, a change to the resolution order would not rebuild a thing.
    println!("cargo:rerun-if-changed=src/host_env.rs");
}
