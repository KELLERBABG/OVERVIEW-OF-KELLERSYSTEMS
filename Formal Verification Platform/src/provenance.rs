//! Run evidence about what actually ran: the compiler this binary was built
//! with, the solver library loaded into this process, and a digest of the
//! settings the verdict was produced under.
//!
//! The run record already carried the worker's rustc (`coverage.compilers`) and
//! the *discovered* replay executable (`env.z3_bin`). Neither answers the
//! question an audit asks of a retained verdict: which solver library was
//! loaded, and were the settings the same as the run it is compared against.
//!
//! The version below is asked of the loaded library through its own C API
//! (`Z3_get_full_version`), so it cannot drift from the library the process is
//! actually using the way a build-script discovery could. The settings digest is
//! FNV-1a rather than `DefaultHasher`, whose output is not promised to be stable
//! across Rust releases — a digest that changes when the compiler is upgraded
//! would be useless for comparing two runs.

use std::os::raw::c_char;

use crate::report::{VerifyOptions, DEFAULT_RUN_TIMEOUT_SECONDS, DEFAULT_SOLVER_TIMEOUT_MS};

extern "C" {
    /// Asks the solver that is loaded, not the one a build script found.
    fn Z3_get_full_version() -> *const c_char;
}

/// The version string of the solver library linked into this process.
pub fn solver_version() -> String {
    // SAFETY: `Z3_get_full_version` returns a pointer to a static string owned by
    // the library; it is never null in the 4.x releases this tool supports, and
    // the pointer outlives this call.
    unsafe {
        let raw = Z3_get_full_version();
        if raw.is_null() {
            return "unknown".to_string();
        }
        std::ffi::CStr::from_ptr(raw).to_string_lossy().into_owned()
    }
}

/// Which `libz3` the loader actually resolved, where the platform exposes it.
///
/// This is the module behind the symbol the version is read from, so it names the
/// file the process is using rather than the directory a build script found. It
/// is identity, not attestation: nothing signs the library.
#[cfg(windows)]
fn loaded_library() -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> *mut std::ffi::c_void;
        fn GetModuleFileNameW(module: *mut std::ffi::c_void, buffer: *mut u16, len: u32) -> u32;
    }

    for candidate in ["libz3.dll", "z3.dll"] {
        let name: Vec<u16> = candidate.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: the name is a NUL-terminated UTF-16 string for the length of the
        // call, and the buffer is a 1024-element allocation written with its length.
        unsafe {
            let handle = GetModuleHandleW(name.as_ptr());
            if handle.is_null() {
                continue;
            }
            let mut buffer = vec![0u16; 1024];
            let len = GetModuleFileNameW(handle, buffer.as_mut_ptr(), buffer.len() as u32);
            if len == 0 {
                continue;
            }
            return Some(String::from_utf16_lossy(&buffer[..len as usize]));
        }
    }
    None
}

#[cfg(unix)]
fn loaded_library() -> Option<String> {
    // SAFETY: `dlsym` is asked for a symbol that exists (the version function is
    // linked), and `dladdr` writes into an initialised `Dl_info` for that address.
    unsafe {
        let symbol = b"Z3_get_full_version\0";
        let address = libc::dlsym(libc::RTLD_DEFAULT, symbol.as_ptr().cast());
        if address.is_null() {
            return None;
        }
        let mut info: libc::Dl_info = std::mem::zeroed();
        if libc::dladdr(address, &mut info) == 0 || info.dli_fname.is_null() {
            return None;
        }
        Some(
            std::ffi::CStr::from_ptr(info.dli_fname)
                .to_string_lossy()
                .into_owned(),
        )
    }
}

#[cfg(not(any(windows, unix)))]
fn loaded_library() -> Option<String> {
    None
}

/// FNV-1a (64-bit) over `bytes`, as lowercase hex.
///
/// Stable by construction and small enough to read, which is what a digest that
/// two machines compare needs to be; `DefaultHasher` is neither promised nor
/// fixed.
pub fn fnv1a_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The `provenance` object of a run record: build identity, loaded solver, and a
/// digest over the effective settings with the canonical string it was computed
/// from (a digest nobody can reproduce is a claim, not evidence).
pub fn evidence(options: &VerifyOptions) -> serde_json::Value {
    let solver_timeout_ms = options
        .solver_timeout_ms
        .unwrap_or(DEFAULT_SOLVER_TIMEOUT_MS);
    let run_timeout_seconds = options
        .run_timeout_seconds
        .unwrap_or(DEFAULT_RUN_TIMEOUT_SECONDS);
    let keep_runs = options.keep_runs.unwrap_or(10);
    let max_rounds = crate::path_state::round_budget();
    let emit_smt2 = std::env::var("KELLER_EMIT_SMT2")
        .map(|v| v != "0")
        .unwrap_or(true);
    let dependency_cache = !options.no_cache;
    let rustflags = std::env::var("RUSTFLAGS").ok();
    let encoded_rustflags = std::env::var("CARGO_ENCODED_RUSTFLAGS").ok();
    // A baseline changes the gate without changing the analysis, so a run cannot
    // be compared to another by settings alone unless the baseline's bytes are in
    // the digest too.
    let baseline_digest = options
        .baseline
        .as_ref()
        .and_then(|path| std::fs::read(path).ok())
        .map(|bytes| fnv1a_hex(&bytes));

    let canonical = format!(
        "strict_coverage={};deny_warn={};solver_timeout_ms={solver_timeout_ms};\
run_timeout_seconds={run_timeout_seconds};gate_profile={:?};only={:?};max_findings={:?};\
keep_runs={keep_runs};max_rounds={max_rounds};emit_smt2={emit_smt2};\
dependency_cache={dependency_cache};release={};rustflags={rustflags:?};\
encoded_rustflags={encoded_rustflags:?};baseline_fnv1a={baseline_digest:?}",
        options.strict_coverage,
        options.deny_warn,
        options.gate_profile,
        options.only,
        options.max_findings,
        options.release,
    );

    serde_json::json!({
        "build": {
            "rustc_release": option_env!("KELLER_RUSTC_RELEASE"),
            "rustc_commit": option_env!("KELLER_RUSTC_COMMIT"),
            "rustc_host": option_env!("KELLER_RUSTC_HOST"),
            "target": option_env!("KELLER_BUILD_TARGET"),
            "profile": option_env!("KELLER_BUILD_PROFILE"),
            "debug_assertions": cfg!(debug_assertions),
        },
        "solver": {
            "loaded_version": solver_version(),
            "loaded_library": loaded_library(),
            "built_against": option_env!("KELLER_DRIVER_Z3_BIN"),
            "identity_source": "Z3_get_full_version() called on the library this process loaded; the path is the module the loader resolved, where the platform exposes it",
        },
        "config_hash": fnv1a_hex(canonical.as_bytes()),
        "config_hash_algorithm": "fnv1a-64 over config_canonical",
        "config_canonical": canonical,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_the_published_vectors() {
        assert_eq!(fnv1a_hex(b""), "cbf29ce484222325");
        assert_eq!(fnv1a_hex(b"a"), "af63dc4c8601ec8c");
    }

    /// The point of the row this closes: the version comes from the library the
    /// process loaded, so it can be asked at run time and must answer.
    #[test]
    fn the_loaded_solver_answers_with_its_own_version() {
        let version = solver_version();
        assert_ne!(version, "unknown");
        assert!(version.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn the_digest_changes_when_a_verdict_affecting_setting_changes() {
        let base = VerifyOptions::default();
        let other = VerifyOptions {
            deny_warn: true,
            ..Default::default()
        };
        assert_ne!(
            evidence(&base)["config_hash"],
            evidence(&other)["config_hash"]
        );
        assert_eq!(
            evidence(&base)["config_hash"],
            evidence(&base)["config_hash"]
        );
    }
}
