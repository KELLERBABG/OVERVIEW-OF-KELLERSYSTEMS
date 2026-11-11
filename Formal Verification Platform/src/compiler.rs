//! The compiler-version adapter: one answer to "which rustc is this build for?".
//!
//! The verifier is written against `rustc_private`, which is not a stable
//! interface. A nightly is not a compatibility promise: MIR types and driver
//! entry points change shape between releases, and a binary linked against one
//! commit can fail on another in ways that look like a bug in the crate under
//! verification. The build therefore embeds the release, commit and host of the
//! compiler it was linked against (`build.rs` reads them from `rustc -vV`), and a
//! run compares that identity with the toolchain it is about to drive.
//!
//! This is the explicit retirement point M3 asks for. Supporting a new nightly is
//! an act, not an accident: change `rust-toolchain.toml`, rebuild, and the
//! embedded identity changes with the code that was compiled against it. Retiring
//! one is the reverse: the old build refuses to run against the new toolchain and
//! says so with both identities, instead of reaching an API mismatch deep in MIR
//! lowering or blaming the project's source.
//!
//! It does not abstract the MIR API itself. That remains `rustc_private` code
//! inside `verifier.rs`; what this module guarantees is that the boundary is
//! named, checked and recorded rather than assumed.

use crate::host_env::RustcIdentity;
use std::path::PathBuf;
use std::process::Command;

/// The identity compiled into this binary. Empty when `build.rs` could not ask a
/// compiler, which is also the case where there is nothing to check against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildIdentity {
    pub release: String,
    pub commit: String,
    pub host: String,
    /// The rustup toolchain name, when the build used a rustup toolchain.
    pub toolchain: Option<String>,
}

impl BuildIdentity {
    /// True when the build recorded a compiler it can compare against.
    pub fn known(&self) -> bool {
        !self.commit.is_empty()
    }

    /// The one-line form used by `--version` and the log.
    pub fn describe(&self) -> String {
        let mut text = if self.release.is_empty() {
            "rustc unknown".to_string()
        } else {
            format!("rustc {}", self.release)
        };
        if !self.commit.is_empty() {
            text.push_str(&format!(" ({})", self.commit));
        }
        if !self.host.is_empty() {
            text.push_str(&format!(" host {}", self.host));
        }
        if let Some(toolchain) = &self.toolchain {
            text.push_str(&format!(" toolchain {toolchain}"));
        }
        text
    }

    /// The record written into `report.run.json` and the environment artifact.
    pub fn as_json(&self) -> serde_json::Value {
        serde_json::json!({
            "release": self.release,
            "commit": self.commit,
            "host": self.host,
            "toolchain": self.toolchain,
        })
    }
}

/// What this binary was compiled against.
pub fn this_build() -> BuildIdentity {
    let text = |value: Option<&'static str>| value.unwrap_or_default().to_string();
    BuildIdentity {
        release: text(option_env!("KELLER_RUSTC_RELEASE")),
        commit: text(option_env!("KELLER_RUSTC_COMMIT")),
        host: text(option_env!("KELLER_RUSTC_HOST")),
        toolchain: option_env!("KELLER_DRIVER_TOOLCHAIN").map(str::to_string),
    }
}

/// Refuse a compiler whose commit or host is not the one this binary was built
/// against.
///
/// A release string is not enough: `nightly` moves and two installations can
/// still print the same number while differing in patch level. The commit hash is
/// what the linked dylibs actually are. `Ok` when the build identity is unknown
/// (a system compiler with no rustup layout), because there is then no claim to
/// enforce and the compiler itself will fail loudly if it cannot load the driver.
pub fn check_loaded(build: &BuildIdentity, loaded: &RustcIdentity) -> Result<(), String> {
    if !build.known() {
        return Ok(());
    }
    let commits_match = build.commit == loaded.commit
        || (!build.commit.is_empty()
            && !loaded.commit.is_empty()
            && (build.commit.starts_with(&loaded.commit) || loaded.commit.starts_with(&build.commit)));
    if commits_match && (build.host.is_empty() || build.host == loaded.host) {
        return Ok(());
    }

    let toolchain = build
        .toolchain
        .as_deref()
        .unwrap_or("<the pinned toolchain>");
    Err(format!(
        "The installed compiler is not the one this verifier was built against.\n\
         This build reads the MIR of one exact rustc commit and loads that \
         compiler's private dylibs, so a different nightly is not supported.\n\
         \n  built with: {}\n  installed:  {}\n\
         \nInstall the pinned toolchain with:\n\
         \x20 rustup toolchain install {toolchain} --component rustc-dev --component llvm-tools-preview\n\
         or rebuild this verifier after updating `rust-toolchain.toml`.",
        build.describe(),
        loaded.describe()
    ))
}

/// The `rustc` to hand to `cargo` as `RUSTC`.
///
/// The path recorded at build time is exact, and preferred when it still exists.
/// It does not exist when the binary was built on another machine — the case a
/// prebuilt release archive is in — so the fallback asks rustup for the same
/// toolchain by name. The commit check above still runs, so the fallback cannot
/// silently select a different compiler; it only turns a build-machine path into
/// a this-machine path.
pub fn resolve_driver_rustc() -> Result<PathBuf, String> {
    let compiled = option_env!("KELLER_DRIVER_RUSTC").unwrap_or_default();
    if !compiled.is_empty() {
        let path = PathBuf::from(compiled);
        if path.is_file() {
            return Ok(path);
        }
    }
    let build = this_build();
    if let Some(toolchain) = &build.toolchain {
        if let Ok(output) = Command::new("rustup")
            .args(["which", "--toolchain", toolchain, "rustc"])
            .output()
        {
            if output.status.success() {
                let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
                if path.is_file() {
                    return Ok(path);
                }
            }
        }
        return Err(format!(
            "Could not find the rustc this verifier was built against ({toolchain}).\n\
             Install it with: rustup toolchain install {toolchain} --component rustc-dev \
             --component llvm-tools-preview"
        ));
    }
    Err(format!(
        "Could not find the rustc this verifier was built against: the path recorded at build \
         time ('{compiled}') does not exist and there is no rustup toolchain name to fall back to.\n\
         Rebuild the verifier on this machine."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build() -> BuildIdentity {
        BuildIdentity {
            release: "1.93.0-nightly".to_string(),
            commit: "9c9e6c2b8d1f2a3b4c5d6e7f8091a2b3c4d5e6f7".to_string(),
            host: "x86_64-unknown-linux-gnu".to_string(),
            toolchain: Some("nightly-2026-07-24-x86_64-unknown-linux-gnu".to_string()),
        }
    }

    fn loaded(commit: &str, host: &str) -> RustcIdentity {
        RustcIdentity {
            release: "1.93.0-nightly".to_string(),
            commit: commit.to_string(),
            host: host.to_string(),
        }
    }

    #[test]
    fn the_exact_commit_is_accepted_and_a_neighbouring_nightly_is_not() {
        let build = build();
        assert_eq!(
            check_loaded(
                &build,
                &loaded(
                    "9c9e6c2b8d1f2a3b4c5d6e7f8091a2b3c4d5e6f7",
                    "x86_64-unknown-linux-gnu"
                )
            ),
            Ok(())
        );

        // Same release string, different commit: exactly the case a channel name
        // cannot detect, and the one that reaches a private-API mismatch if it is
        // allowed through.
        let err = check_loaded(
            &build,
            &loaded(
                "0000000000000000000000000000000000000000",
                "x86_64-unknown-linux-gnu",
            ),
        )
        .expect_err("a different commit is refused");
        assert!(err.contains("built with:"), "got {err}");
        assert!(err.contains("installed:"), "got {err}");
        assert!(
            err.contains("rustup toolchain install nightly-2026-07-24"),
            "the message names the command that fixes it: {err}"
        );
    }

    #[test]
    fn a_host_mismatch_is_refused_even_with_the_same_commit() {
        let err = check_loaded(
            &build(),
            &loaded(
                "9c9e6c2b8d1f2a3b4c5d6e7f8091a2b3c4d5e6f7",
                "aarch64-unknown-linux-gnu",
            ),
        )
        .expect_err("the dylibs are host-specific too");
        assert!(err.contains("aarch64-unknown-linux-gnu"), "got {err}");
    }

    #[test]
    fn a_build_without_a_recorded_compiler_makes_no_claim() {
        // A system compiler has no rustup toolchain name and build.rs may not be
        // able to ask it anything; inventing a failure here would turn an
        // unsupported layout into a false prerequisite error.
        let unknown = BuildIdentity {
            release: String::new(),
            commit: String::new(),
            host: String::new(),
            toolchain: None,
        };
        assert!(!unknown.known());
        assert_eq!(
            check_loaded(&unknown, &loaded("anything", "anywhere")),
            Ok(())
        );
    }

    #[test]
    fn the_description_carries_the_commit_and_the_toolchain() {
        let text = build().describe();
        assert!(text.contains("1.93.0-nightly"), "got {text}");
        assert!(text.contains("9c9e6c2b"), "got {text}");
        assert!(text.contains("nightly-2026-07-24"), "got {text}");
    }
}
