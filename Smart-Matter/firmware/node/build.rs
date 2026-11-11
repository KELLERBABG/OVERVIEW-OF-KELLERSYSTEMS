//! Build script: wires `memory.x` into the linker for embedded targets.
//!
//! Only applied when building for an ARM bare-metal target (`thumbv*`), so
//! host builds (`cargo build --workspace`) are unaffected.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    if !target.starts_with("thumbv") {
        return;
    }

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set by cargo"));
    fs::copy("memory.x", out.join("memory.x")).expect("copy memory.x");
    println!("cargo:rustc-link-search={}", out.display());
    // `link.x` comes from the cortex-m-rt crate (its build script registers
    // the search path) and `INCLUDE`s `memory.x` (this crate's flash/RAM
    // layout, found via the search path above). It sets ENTRY(Reset) and the
    // vector table.
    println!("cargo:rustc-link-arg=-Tlink.x");
    println!("cargo:rerun-if-changed=memory.x");
}
