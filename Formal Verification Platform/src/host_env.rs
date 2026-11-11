//! One decision about where the toolchain and Z3 are.
//!
//! Three consumers read this file, so the rule is written once:
//!
//!   * `build.rs` includes it — the Z3 the verifier links against is chosen by
//!     [`z3_root`], and what counts as an install root by [`looks_like_z3_root`],
//!   * `src/bin/keller-env.rs` compiles it and prints the result,
//!   * `tests/run_tests.sh` and `tests/run_tests.cmd` consume that printout and
//!     carry no lookup of their own.
//!
//! Before this file the rule existed in three places and they had drifted.
//! `build.rs` took `Z3_HOME`, then a project-local install (`.z3/current`, else
//! the *last* of several sorted releases), then the platform default. The shell
//! runner took `Z3_HOME`, then `.z3/current`, then the *first* sorted release with
//! a runnable `z3`, and put that on `PATH` itself. The Windows runner took
//! `Z3_HOME`, then `.z3/current`, then a machine-specific `C:\z3-4.16.0-x64-win`,
//! and never checked that what it found would start. So a checkout with two Z3
//! releases unpacked could *link* one and *run* the other, and a `z3.exe` that
//! existed but did not start was a silent `[SKIP]` on one host and a spurious
//! failure on the other.
//!
//! It is deliberately free of the compiler and of Z3 itself — `std` only — because
//! the binary that runs it has to start before the environment it reports exists.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Which reader the paths are rendered for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flavour {
    /// A POSIX shell: `/c/...` paths, quoted assignments ready for `eval`.
    Sh,
    /// `cmd.exe`, or any other program parsing them: native paths, bare values.
    Plain,
    /// Native path-list syntax for the process running this code.
    Native,
}

impl Flavour {
    /// What separates the directories of a `PATH` for this reader.
    pub fn separator(self) -> char {
        match self {
            Flavour::Sh => ':',
            Flavour::Plain => ';',
            Flavour::Native if cfg!(windows) => ';',
            Flavour::Native => ':',
        }
    }
}

/// Where the solver that will be used came from, for the log line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Z3Source {
    /// `KELLER_Z3`, an explicit path from the environment.
    Override,
    /// Already on `PATH`, so nothing has to be added for it to be found.
    OnPath,
    /// `Z3_HOME`.
    Home,
    /// The project-local `.z3/` install.
    ProjectLocal,
    /// The fixed per-platform fallback.
    PlatformDefault,
}

impl Z3Source {
    pub fn label(self) -> &'static str {
        match self {
            Z3Source::Override => "KELLER_Z3",
            Z3Source::OnPath => "PATH",
            Z3Source::Home => "Z3_HOME",
            Z3Source::ProjectLocal => "the project-local .z3/",
            Z3Source::PlatformDefault => "the platform default",
        }
    }
}

/// True when `dir` looks like a Z3 installation root (has `bin/` + `include/`).
///
/// This is the build's test, and it is the one used to *choose between several*
/// project-local releases — so the release the suite runs is the release the
/// verifier was linked against.
pub fn looks_like_z3_root(dir: &Path, probe: &dyn Probe) -> bool {
    let bin = dir.join("bin");
    let has_lib = probe.is_file(&bin.join("libz3.lib"))
        || probe.is_file(&bin.join("libz3.so"))
        || probe.is_file(&bin.join("libz3.dylib"))
        || probe.is_file(&bin.join("z3.lib"));
    has_lib && probe.is_file(&dir.join("include").join("z3.h"))
}

/// Look for a project-local Z3 install: `.z3/<any-dir>/` containing bin+include.
///
/// This lets a checkout build without any environment setup: drop a Z3 release
/// into `.z3/` (gitignored) and it is picked up. Keeps CI and local builds
/// reproducible without hardcoding machine paths into `.cargo/config.toml`.
pub fn project_local_z3(manifest_dir: &Path, probe: &dyn Probe) -> Option<PathBuf> {
    let base = manifest_dir.join(".z3");
    // Preferred, stable layout: `.z3/current/{bin,include}`. `.cargo/config.toml`
    // points Z3_HOME and Z3_SYS_Z3_HEADER here (with `relative = true`), which is
    // what lets z3-sys' own build script find `z3.h` — a `cargo:rustc-env` from
    // that file could never reach a dependency's build script.
    let current = base.join("current");
    if looks_like_z3_root(&current, probe) {
        return Some(current);
    }
    if looks_like_z3_root(&base, probe) {
        return Some(base);
    }
    let mut candidates: Vec<PathBuf> = probe
        .read_dir(&base)
        .into_iter()
        .filter(|path| probe.is_dir(path) && looks_like_z3_root(path, probe))
        .collect();
    // Deterministic pick when several releases are unpacked side by side.
    candidates.sort();
    candidates.pop()
}

/// The install to fall back on when nothing else answers.
pub fn platform_default() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("C:\\z3-4.16.0-x64-win")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/usr")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/usr/local")
    }
}

/// The install roots to use, in order: explicit `Z3_HOME`, then a project-local
/// install, then the platform default.
///
/// One list, two readers. A build links against the first root; a test run takes
/// the first root that holds a solver which starts, so a broken `.z3/current`
/// falls through to the release the build actually linked rather than turning
/// every re-discharge into a failure.
pub fn z3_roots(
    z3_home: Option<&str>,
    manifest_dir: &Path,
    probe: &dyn Probe,
) -> Vec<(PathBuf, Z3Source)> {
    let mut roots = Vec::new();
    if let Some(home) = z3_home.filter(|home| !home.is_empty()) {
        roots.push((PathBuf::from(home), Z3Source::Home));
    }
    if let Some(local) = project_local_z3(manifest_dir, probe) {
        roots.push((local, Z3Source::ProjectLocal));
    }
    // Always last, so a candidate list built from this is never empty.
    roots.push((platform_default(), Z3Source::PlatformDefault));
    roots
}

/// The install root a build links against, and where that choice came from.
pub fn z3_root(
    z3_home: Option<&str>,
    manifest_dir: &Path,
    probe: &dyn Probe,
) -> (PathBuf, Z3Source) {
    z3_roots(z3_home, manifest_dir, probe)
        .into_iter()
        .next()
        .expect("the platform default is always a candidate")
}

/// The directory the active toolchain keeps its rustc driver dylibs in.
///
/// The verifier is linked against `rustc_private` and loads the toolchain's own
/// `librustc_driver-*` and `libstd-*` at run time, so this directory has to be on
/// the loader path — not on the compile path. It is derived from the rustc that
/// rustup names rather than from a hardcoded toolchain, because the host may be
/// `-msvc` or `-gnu` and `rust-toolchain.toml` can override the channel.
pub fn driver_lib_dir(rustc: &Path, host: &str) -> PathBuf {
    let bin = rustc.parent().unwrap_or_else(|| Path::new(""));
    bin.join("..")
        .join("lib")
        .join("rustlib")
        .join(host)
        .join("lib")
}

/// The rustup toolchain name a compiler belongs to, from its path.
///
/// `cargo` is resolved through the rustup shim, and the shim honours the project
/// being checked: with a `rust-toolchain.toml` that pins `stable`, `cargo -Z...`
/// fails before the verifier's own compiler is consulted. `RUSTUP_TOOLCHAIN` is
/// what makes the shim pick the toolchain the verifier was built against, and
/// this is the name to set it to. The rustc path has to sit under the rustup
/// `toolchains` directory for the answer to mean anything, which is also the test
/// for whether this is a rustup toolchain at all.
pub fn toolchain_name(rustc: &Path) -> Option<String> {
    let path_str = rustc.to_string_lossy();
    let parts: Vec<&str> = path_str
        .split(|c| c == '/' || c == '\\')
        .filter(|s| !s.is_empty())
        .collect();
    let at = parts.iter().position(|&part| part == "toolchains")?;
    let name = parts.get(at + 1)?;
    if name.is_empty() {
        None
    } else {
        Some((*name).to_string())
    }
}

/// The filesystem and process questions the decision asks.
///
/// A trait so the decision can be tested without a machine that has two Z3
/// releases unpacked and one of them broken.
pub trait Probe {
    fn is_dir(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    /// True when `path` is a program that actually starts.
    fn runs(&self, path: &Path) -> bool;
    /// The entries of `path`, or nothing when it cannot be listed.
    fn read_dir(&self, path: &Path) -> Vec<PathBuf>;
}

/// The real machine.
pub struct RealProbe;

impl Probe for RealProbe {
    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    /// A solver that exists but does not start is not a solver. Installing one
    /// into `PATH` because its file is present would turn a missing Z3 into a
    /// spurious failure instead of the `[SKIP]` the runners report for it.
    fn runs(&self, path: &Path) -> bool {
        Command::new(path)
            .arg("-version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn read_dir(&self, path: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(path)
            .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default()
    }
}

/// What the machine looks like, before any decision is made about it.
pub struct Facts {
    pub manifest_dir: PathBuf,
    /// `KELLER_Z3`: an explicit solver, or an install root holding one.
    pub z3_override: Option<String>,
    /// `Z3_HOME`: an install root. Named by the build too.
    pub z3_home: Option<String>,
    /// Every `z3`/`z3.exe` on `PATH`, in `PATH` order.
    pub z3_on_path: Vec<PathBuf>,
    /// The rustc rustup names, or the one on `PATH`.
    pub rustc: Option<PathBuf>,
    /// That rustc's host triple.
    pub host: Option<String>,
}

/// What the runners need to know.
pub struct Discovered {
    pub rustc: Option<PathBuf>,
    pub host: Option<String>,
    /// The toolchain's driver dylib directory, when it is where it should be.
    pub lib_dir: Option<PathBuf>,
    pub z3_bin: Option<PathBuf>,
    pub z3_source: Option<Z3Source>,
    /// Directories to prepend to `PATH`, highest priority first.
    pub path_additions: Vec<PathBuf>,
    /// What could not be found, in the words the runner prints.
    pub warnings: Vec<String>,
}

/// A candidate solver, before it is known to start.
enum Candidate {
    /// A program this exact path names.
    Exe(PathBuf, Z3Source),
    /// An install root with a `bin/` to look in.
    Root(PathBuf, Z3Source),
}

/// Read the machine.
pub fn gather(manifest_dir: &Path, probe: &dyn Probe) -> Facts {
    let rustc = rustup_rustc(manifest_dir).or_else(|| first_on_path(&["rustc"], probe));
    let host = rustc.as_deref().and_then(host_triple);
    Facts {
        manifest_dir: manifest_dir.to_path_buf(),
        z3_override: named_env("KELLER_Z3"),
        z3_home: named_env("Z3_HOME"),
        z3_on_path: on_path(&["z3"]),
        rustc,
        host,
    }
}

/// Decide, on the real machine.
pub fn discover(manifest_dir: &Path) -> Discovered {
    let probe = RealProbe;
    decide(&gather(manifest_dir, &probe), &probe)
}

/// Decide what to put on `PATH`, and which solver to use, from what was read.
pub fn decide(facts: &Facts, probe: &dyn Probe) -> Discovered {
    let mut warnings: Vec<String> = Vec::new();
    let mut path_additions: Vec<PathBuf> = Vec::new();

    // The toolchain first: everything in this suite runs the verifier, which
    // cannot start without its own toolchain's driver dylibs.
    let lib_dir = match (&facts.rustc, &facts.host) {
        (Some(rustc), Some(host)) => {
            let dir = driver_lib_dir(rustc, host);
            if probe.is_dir(&dir) {
                if cfg!(windows) {
                    if let Some(bin_dir) = rustc.parent() {
                        if probe.is_dir(bin_dir) {
                            path_additions.push(bin_dir.to_path_buf());
                        }
                    }
                }
                path_additions.push(dir.clone());
                Some(dir)
            } else {
                // No path in the text: this line is echoed by a `cmd.exe` runner,
                // where an `&` in a path would end the command it is part of.
                warnings.push(
                    "the active toolchain's rustc driver dylibs are not where its layout puts \
                     them (lib/rustlib/<host>/lib), so the wrapper cannot start"
                        .to_string(),
                );
                None
            }
        }
        _ => {
            warnings.push(
                "rustup did not name a rustc and none is on PATH; is the pinned nightly installed?"
                    .to_string(),
            );
            None
        }
    };

    // The solver. `PATH` comes before the install directories so a Z3 a person
    // put in front of themselves is the one that answers; the rest is the order
    // the build resolves the library in, so a checkout that built against one
    // release does not get re-discharged by another.
    let mut candidates: Vec<Candidate> = Vec::new();
    if let Some(path) = &facts.z3_override {
        let path = PathBuf::from(path);
        if probe.is_dir(&path) {
            candidates.push(Candidate::Root(path, Z3Source::Override));
        } else {
            candidates.push(Candidate::Exe(path, Z3Source::Override));
        }
    }
    candidates.extend(
        facts
            .z3_on_path
            .iter()
            .cloned()
            .map(|path| Candidate::Exe(path, Z3Source::OnPath)),
    );
    if let Some(home) = &facts.z3_home {
        candidates.push(Candidate::Root(PathBuf::from(home), Z3Source::Home));
    }
    // Then the roots a build resolves, in the same order — `None` is not a dropped
    // setting: `Z3_HOME` is already a candidate, pushed just above in its own right,
    // and `z3_roots` answers exactly what `build.rs` links against.
    candidates.extend(
        z3_roots(None, &facts.manifest_dir, probe)
            .into_iter()
            .map(|(root, source)| Candidate::Root(root, source)),
    );

    let mut z3_bin = None;
    let mut z3_source = None;
    for candidate in candidates {
        let resolved = match candidate {
            Candidate::Exe(path, source) => {
                solver_at_exe(&path, probe).map(|exe| (exe, source, None))
            }
            Candidate::Root(root, source) => match solver_in_root(&root, probe) {
                Some((exe, bin)) => Some((exe, source, Some(bin))),
                None => None,
            },
        };
        if let Some((exe, source, extra)) = resolved {
            // What has to be on `PATH` for `libz3` to load — and for anything
            // spawned later to find `z3`: the `bin/` of an install root, or the
            // directory a named solver lives in. A solver that came from `PATH`
            // is already there, so it adds nothing.
            let addition = extra.or_else(|| match source {
                Z3Source::Override => exe.parent().map(Path::to_path_buf),
                _ => None,
            });
            if let Some(dir) = addition {
                path_additions.push(dir);
            }
            z3_bin = Some(exe);
            z3_source = Some(source);
            break;
        }
    }

    if z3_bin.is_none() {
        warnings.push(
            "no runnable z3 was found (looked at KELLER_Z3, PATH, Z3_HOME, the project-local \
             .z3/ and the platform default), so obligations cannot be re-discharged"
                .to_string(),
        );
    }

    Discovered {
        rustc: facts.rustc.clone(),
        host: facts.host.clone(),
        lib_dir,
        z3_bin,
        z3_source,
        path_additions,
        warnings,
    }
}

/// The program `path` names, or the same name with the `.exe` Windows adds.
fn solver_at_exe(path: &Path, probe: &dyn Probe) -> Option<PathBuf> {
    let already_exe = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
    let mut candidates = vec![path.to_path_buf()];
    // `C:\...\z3.exe` names one file; `C:\...\z3` needs the extension on Windows.
    if !already_exe {
        candidates.push(PathBuf::from(format!("{}.exe", path.display())));
    }
    candidates
        .into_iter()
        .find(|candidate| probe.is_file(candidate) && probe.runs(candidate))
}

/// The solver inside an install root, and the `bin/` it lives in.
fn solver_in_root(root: &Path, probe: &dyn Probe) -> Option<(PathBuf, PathBuf)> {
    let bin = root.join("bin");
    solver_at_exe(&bin.join("z3"), probe).map(|exe| (exe, bin))
}

/// The first of `stems` found on `PATH`, in `PATH` order.
fn first_on_path(stems: &[&str], probe: &dyn Probe) -> Option<PathBuf> {
    on_path(stems).into_iter().find(|path| probe.is_file(path))
}

/// Every `PATH` entry that could hold one of `stems`, in `PATH` order.
fn on_path(stems: &[&str]) -> Vec<PathBuf> {
    let path = match std::env::var_os("PATH") {
        Some(path) => path,
        None => return Vec::new(),
    };
    let mut found = Vec::new();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        for stem in stems {
            // The compiler and the solver are `.exe` on Windows; the bare name is
            // tried too, because a `-gnu` toolchain may install one without it.
            #[cfg(target_os = "windows")]
            let names = vec![format!("{stem}.exe"), (*stem).to_string()];
            #[cfg(not(target_os = "windows"))]
            let names = vec![(*stem).to_string()];
            for name in names {
                let candidate = dir.join(&name);
                if !found.contains(&candidate) {
                    found.push(candidate);
                }
            }
        }
    }
    found
}

/// `rustup which rustc`, asked with the checkout as the working directory so the
/// pinned toolchain of *this* project answers.
fn rustup_rustc(manifest_dir: &Path) -> Option<PathBuf> {
    // 1. Direct RUSTC environment variable (set by cargo during build script execution)
    if let Ok(rustc) = std::env::var("RUSTC") {
        let path = PathBuf::from(&rustc);
        if path.is_file() && toolchain_name(&path).is_some() {
            return Some(path);
        }
    }

    // 2. Resolve via rustup command
    if let Ok(output) = Command::new("rustup")
        .args(["which", "rustc"])
        .current_dir(manifest_dir)
        .stdin(Stdio::null())
        .output()
    {
        if output.status.success() {
            if let Ok(path) = String::from_utf8(output.stdout) {
                let path = path.trim();
                if !path.is_empty() {
                    let pb = PathBuf::from(path);
                    if pb.is_file() {
                        return Some(pb);
                    }
                }
            }
        }
    }

    // 3. Fallback: inspect RUSTUP_HOME and RUSTUP_TOOLCHAIN / rust-toolchain.toml directly
    let home_dir = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    let rustup_home = std::env::var("RUSTUP_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| home_dir.map(|h| h.join(".rustup")))?;

    let tc_name = std::env::var("RUSTUP_TOOLCHAIN").ok().or_else(|| {
        let content = std::fs::read_to_string(manifest_dir.join("rust-toolchain.toml")).ok()?;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("channel") {
                let channel = line.split('"').nth(1)?;
                let host = std::env::var("HOST").or_else(|_| std::env::var("TARGET")).unwrap_or_else(|_| "x86_64-pc-windows-msvc".to_string());
                return Some(format!("{}-{}", channel, host));
            }
        }
        None
    })?;

    let rustc_name = if cfg!(windows) { "rustc.exe" } else { "rustc" };
    let rustc_candidate = rustup_home.join("toolchains").join(tc_name).join("bin").join(rustc_name);
    if rustc_candidate.is_file() {
        Some(rustc_candidate)
    } else {
        None
    }
}

/// The exact compiler a build was linked against, from its own `-vV`.
///
/// `rustc_private` is not a stable interface: the MIR types this verifier reads
/// change shape between nightlies, and a binary built against one compiler cannot
/// be assumed to work with another. The commit hash is the identity that matters,
/// because a channel name such as `nightly` moves while the hash does not. This
/// struct is what the build embeds and what the run compares the installed
/// toolchain against (`src/compiler.rs`), so "which compiler is this build for?"
/// has one answer rather than a version string parsed in several places.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustcIdentity {
    pub release: String,
    pub commit: String,
    pub host: String,
}

impl RustcIdentity {
    /// The one-line description a log line or `--version` can carry.
    pub fn describe(&self) -> String {
        let mut text = format!("rustc {} ({})", self.release, self.commit);
        if !self.host.is_empty() {
            text.push_str(" host ");
            text.push_str(&self.host);
        }
        text
    }
}

/// Read one `key: value` field from `rustc -vV` output.
///
/// Kept as a free function so the same parser answers for the `host` field that
/// the loader path needs and the `release`/`commit` fields the compiler adapter
/// needs. Regexes are not worth a dependency for a four-line format.
pub fn version_field(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}: ");
    text.lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Parse a `rustc -vV` listing. `None` unless the release and commit are both
/// present, because a partial identity cannot be checked against anything.
pub fn parse_rustc_identity(text: &str) -> Option<RustcIdentity> {
    Some(RustcIdentity {
        release: version_field(text, "release")?,
        commit: version_field(text, "commit-hash")?,
        host: version_field(text, "host").unwrap_or_default(),
    })
}

/// Ask a compiler for its own identity.
pub fn rustc_identity(rustc: &Path) -> Option<RustcIdentity> {
    if let Ok(output) = Command::new(rustc)
        .arg("-vV")
        .stdin(Stdio::null())
        .output()
    {
        if output.status.success() {
            if let Ok(text) = String::from_utf8(output.stdout) {
                if let Some(id) = parse_rustc_identity(&text) {
                    return Some(id);
                }
            }
        }
    }

    // Fallback: read directly from rustlib/multirust-channel-manifest.toml when child processes cannot run rustc directly
    let bin_dir = rustc.parent()?;
    let root = bin_dir.parent()?;
    let rustlib = root.join("lib").join("rustlib");
    let manifest = rustlib.join("multirust-channel-manifest.toml");
    if let Ok(content) = std::fs::read_to_string(&manifest) {
        if let Some(pos) = content.find("[pkg.rustc]") {
            let section = &content[pos..];
            if let Some(v_line) = section.lines().find(|l| l.trim_start().starts_with("version =")) {
                if let Some(val) = v_line.split('"').nth(1) {
                    let mut parts = val.split_whitespace();
                    let release = parts.next().unwrap_or("").to_string();
                    let commit_part = parts.next().unwrap_or("");
                    let commit = commit_part.trim_matches(|c| c == '(' || c == ')').to_string();
                    let host = toolchain_name(rustc)
                        .and_then(|tc| {
                            let parts: Vec<&str> = tc.split('-').collect();
                            if parts.len() >= 4 {
                                Some(parts[parts.len() - 4..].join("-"))
                            } else {
                                None
                            }
                        })
                        .or_else(|| std::env::var("HOST").ok())
                        .or_else(|| std::env::var("TARGET").ok())
                        .unwrap_or_default();
                    if !release.is_empty() && !commit.is_empty() {
                        return Some(RustcIdentity { release, commit, host });
                    }
                }
            }
        }
    }
    None
}

/// The host triple of a rustc, from its own `-vV`.
fn host_triple(rustc: &Path) -> Option<String> {
    rustc_identity(rustc).map(|identity| identity.host).or_else(|| std::env::var("HOST").ok()).or_else(|| std::env::var("TARGET").ok())
}

/// An environment variable, treated as absent when it is set but empty.
fn named_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Render a path the way `flavour`'s reader needs it.
pub fn render(path: &Path, flavour: Flavour) -> String {
    let text = path.to_string_lossy().into_owned();
    match flavour {
        Flavour::Plain | Flavour::Native => text,
        // Only on Windows: elsewhere a backslash is a legal character in a file
        // name, and rewriting one would corrupt the path.
        Flavour::Sh if cfg!(windows) => windows_to_posix(&text),
        Flavour::Sh => text,
    }
}

/// `C:\a b\c` -> `/c/a b/c`, the form an MSYS shell can both test and pass on.
///
/// A Windows form is not usable by the shell directly: exporting one and spawning
/// a native process through it leaves the directory unresolvable, so the directory
/// is put into `PATH` in the form the shell itself uses. `cygpath` does this too,
/// but requiring it would be one more thing the environment has to have.
pub fn windows_to_posix(path: &str) -> String {
    // Windows APIs hand back the extended-length form; the shell wants the plain
    // one, and a UNC path has its own spelling under it.
    let trimmed = path.strip_prefix("\\\\?\\").unwrap_or(path);
    if let Some(unc) = trimmed.strip_prefix("UNC\\") {
        return format!("//{}", unc.replace('\\', "/"));
    }
    let mut chars = trimmed.chars();
    if let (Some(drive), Some(':')) = (chars.next(), chars.next()) {
        if drive.is_ascii_alphabetic() {
            let rest = trimmed[2..].replace('\\', "/");
            let rest = rest.trim_start_matches('/');
            let drive = drive.to_ascii_lowercase();
            return if rest.is_empty() {
                format!("/{drive}")
            } else {
                format!("/{drive}/{rest}")
            };
        }
    }
    // A UNC share (`\\server\share`) is a network root to the shell: two slashes.
    trimmed.replace('\\', "/")
}

/// Join directories the way `flavour`'s reader separates a `PATH`.
pub fn join_paths(dirs: &[PathBuf], flavour: Flavour) -> String {
    dirs.iter()
        .map(|dir| render(dir, flavour))
        .collect::<Vec<String>>()
        .join(&flavour.separator().to_string())
}

/// `KEY='value'`, safe to hand to `eval`: any quote in the value is closed and
/// reopened so it cannot end the string early and be read as a command.
pub fn shell_assignment(key: &str, value: &str) -> String {
    format!("{key}='{}'", value.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// A machine described by hand, so the decision can be tested without one.
    #[derive(Default)]
    struct Fake {
        dirs: HashSet<PathBuf>,
        files: HashSet<PathBuf>,
        runnable: HashSet<PathBuf>,
    }

    impl Fake {
        fn dir(mut self, path: &str) -> Self {
            self.dirs.insert(PathBuf::from(path));
            self
        }

        /// A plain file: present, but not a program that starts.
        fn file(mut self, path: &str) -> Self {
            self.files.insert(PathBuf::from(path));
            self
        }

        fn runnable(mut self, path: &str) -> Self {
            self.files.insert(PathBuf::from(path));
            self.runnable.insert(PathBuf::from(path));
            self
        }

        /// A complete install root: the library, the header and a working solver.
        fn root(self, dir: &str) -> Self {
            self.dir(dir)
                .file(&format!("{dir}/bin/libz3.so"))
                .file(&format!("{dir}/include/z3.h"))
                .runnable(&format!("{dir}/bin/z3"))
        }
    }

    impl Probe for Fake {
        fn is_dir(&self, path: &Path) -> bool {
            self.dirs.contains(path)
        }

        fn is_file(&self, path: &Path) -> bool {
            self.files.contains(path)
        }

        fn runs(&self, path: &Path) -> bool {
            self.runnable.contains(path)
        }

        fn read_dir(&self, path: &Path) -> Vec<PathBuf> {
            self.dirs
                .iter()
                .chain(self.files.iter())
                .filter(|entry| entry.parent() == Some(path))
                .cloned()
                .collect()
        }
    }

    /// Facts with a working toolchain and nothing else, so each test changes one
    /// thing.
    fn facts() -> Facts {
        Facts {
            manifest_dir: PathBuf::from("/repo"),
            z3_override: None,
            z3_home: None,
            z3_on_path: Vec::new(),
            rustc: Some(PathBuf::from("/tc/bin/rustc")),
            host: Some("x86_64-unknown-linux-gnu".to_string()),
        }
    }

    fn toolchain() -> Fake {
        Fake::default().dir("/tc/bin/../lib/rustlib/x86_64-unknown-linux-gnu/lib")
    }

    #[test]
    fn the_driver_dylibs_sit_next_to_rustc_under_the_host_triple() {
        let dir = driver_lib_dir(Path::new("/tc/bin/rustc"), "x86_64-unknown-linux-gnu");
        assert!(
            dir.ends_with("lib/rustlib/x86_64-unknown-linux-gnu/lib"),
            "{}",
            dir.display()
        );
        assert_eq!(
            dir.parent().expect("the directory has a parent"),
            Path::new("/tc/bin/../lib/rustlib/x86_64-unknown-linux-gnu")
        );
    }

    #[test]
    fn the_toolchain_directory_goes_on_path() {
        let probe = toolchain();
        let discovered = decide(&facts(), &probe);
        assert_eq!(
            discovered.lib_dir,
            Some(PathBuf::from(
                "/tc/bin/../lib/rustlib/x86_64-unknown-linux-gnu/lib"
            ))
        );
        assert_eq!(discovered.path_additions.len(), 1);
        assert!(
            !discovered
                .warnings
                .iter()
                .any(|warning| warning.contains("driver dylibs")),
            "{:?}",
            discovered.warnings
        );
    }

    #[test]
    fn a_missing_driver_directory_is_a_warning_and_not_a_path_entry() {
        // The failure this catches is the one that made every case look like a
        // broken tool: a `PATH` that never got the dylibs, so each run died at
        // load with the status Windows gives an unresolvable import.
        let probe = Fake::default();
        let discovered = decide(&facts(), &probe);
        assert_eq!(discovered.lib_dir, None);
        assert!(discovered.path_additions.is_empty());
        assert!(
            discovered
                .warnings
                .iter()
                .any(|warning| warning.contains("driver dylibs")),
            "{:?}",
            discovered.warnings
        );
    }

    #[test]
    fn no_rustc_at_all_is_a_warning() {
        let mut facts = facts();
        facts.rustc = None;
        facts.host = None;
        let discovered = decide(&facts, &Fake::default());
        assert_eq!(discovered.lib_dir, None);
        assert!(
            discovered
                .warnings
                .iter()
                .any(|warning| warning.contains("rustup")),
            "{:?}",
            discovered.warnings
        );
    }

    #[test]
    fn a_solver_already_on_path_is_used_and_needs_no_addition() {
        let mut facts = facts();
        facts.z3_on_path = vec![PathBuf::from("/usr/bin/z3")];
        let probe = toolchain().runnable("/usr/bin/z3");
        let discovered = decide(&facts, &probe);
        assert_eq!(discovered.z3_bin, Some(PathBuf::from("/usr/bin/z3")));
        assert_eq!(discovered.z3_source, Some(Z3Source::OnPath));
        // Only the toolchain, because the solver is already findable.
        assert_eq!(discovered.path_additions.len(), 1);
    }

    #[test]
    fn z3_home_wins_over_a_project_local_install() {
        let mut facts = facts();
        facts.z3_home = Some("/opt/z3".to_string());
        let probe = toolchain().root("/repo/.z3/current").root("/opt/z3");
        let discovered = decide(&facts, &probe);
        assert_eq!(discovered.z3_bin, Some(PathBuf::from("/opt/z3/bin/z3")));
        assert_eq!(discovered.z3_source, Some(Z3Source::Home));
        assert_eq!(
            discovered.path_additions.last(),
            Some(&PathBuf::from("/opt/z3/bin"))
        );
    }

    #[test]
    fn the_override_names_the_solver_and_supplies_its_library_directory() {
        let mut facts = facts();
        facts.z3_override = Some("/opt/other/z3".to_string());
        let probe = toolchain().runnable("/opt/other/z3");
        let discovered = decide(&facts, &probe);
        assert_eq!(discovered.z3_source, Some(Z3Source::Override));
        assert_eq!(
            discovered.path_additions.last(),
            Some(&PathBuf::from("/opt/other"))
        );
    }

    #[test]
    fn a_project_local_install_is_the_release_the_build_linked() {
        // Two releases unpacked side by side: the build takes the last of the
        // sorted candidates, so the suite has to run that one and not the first
        // one a directory listing happens to offer.
        let probe = toolchain()
            .root("/repo/.z3/z3-4.15.0-x64")
            .root("/repo/.z3/z3-4.16.0-x64");
        let discovered = decide(&facts(), &probe);
        assert_eq!(
            discovered.z3_bin,
            Some(PathBuf::from("/repo/.z3/z3-4.16.0-x64/bin/z3"))
        );
        assert_eq!(discovered.z3_source, Some(Z3Source::ProjectLocal));
    }

    #[test]
    fn current_wins_over_the_siblings() {
        let probe = toolchain()
            .root("/repo/.z3/current")
            .root("/repo/.z3/z3-4.16.0-x64");
        let discovered = decide(&facts(), &probe);
        assert_eq!(
            discovered.z3_bin,
            Some(PathBuf::from("/repo/.z3/current/bin/z3"))
        );
    }

    #[test]
    fn a_solver_that_does_not_start_is_skipped_for_the_next_candidate() {
        // Present but broken: used, it would turn every re-discharge into a
        // failure. Skipped, the next candidate answers — and if none does, the
        // runner reports the `[SKIP]` it always did.
        let mut facts = facts();
        facts.z3_on_path = vec![PathBuf::from("/bad/z3"), PathBuf::from("/good/z3")];
        let probe = toolchain().file("/bad/z3").runnable("/good/z3");
        let discovered = decide(&facts, &probe);
        assert_eq!(discovered.z3_bin, Some(PathBuf::from("/good/z3")));
        assert_eq!(discovered.z3_source, Some(Z3Source::OnPath));
    }

    #[test]
    fn a_broken_install_is_skipped_rather_than_replaced_by_another_release() {
        // `.z3/current` is the root the build linked against, so its `z3` being
        // missing is a broken install and not an invitation to answer with a
        // sibling release's solver — that is the mismatch this module exists to
        // remove. Reporting no solver lets the runners say `[SKIP]` honestly.
        let probe = toolchain()
            .file("/repo/.z3/current/bin/libz3.so")
            .file("/repo/.z3/current/include/z3.h")
            .file("/repo/.z3/current/bin/z3")
            .root("/repo/.z3/z3-4.16.0-x64");
        let discovered = decide(&facts(), &probe);
        assert_eq!(discovered.z3_bin, None);
        assert!(discovered.warnings[0].contains("no runnable z3"));
    }

    #[test]
    fn no_solver_anywhere_is_one_warning_and_no_solver() {
        let discovered = decide(&facts(), &toolchain());
        assert_eq!(discovered.z3_bin, None);
        assert_eq!(discovered.z3_source, None);
        assert_eq!(discovered.warnings.len(), 1, "{:?}", discovered.warnings);
        assert!(discovered.warnings[0].contains("no runnable z3"));
    }

    #[test]
    fn the_platform_default_answers_last() {
        let probe = toolchain().root(&platform_default().to_string_lossy());
        let discovered = decide(&facts(), &probe);
        assert_eq!(discovered.z3_source, Some(Z3Source::PlatformDefault));
    }

    #[test]
    fn windows_paths_become_the_form_the_shell_can_use() {
        assert_eq!(windows_to_posix(r"C:\a b\c"), "/c/a b/c");
        assert_eq!(windows_to_posix(r"C:\"), "/c");
        assert_eq!(windows_to_posix(r"D:\Users\x"), "/d/Users/x");
        assert_eq!(windows_to_posix(r"\\?\C:\x"), "/c/x");
        assert_eq!(windows_to_posix(r"\\srv\share\x"), "//srv/share/x");
        assert_eq!(windows_to_posix(r"\\?\UNC\srv\share\x"), "//srv/share/x");
        // A path that is already in the shell's own form is left alone.
        assert_eq!(windows_to_posix("/already/posix"), "/already/posix");
    }

    #[test]
    fn a_shell_assignment_cannot_be_talked_into_ending_early() {
        assert_eq!(shell_assignment("Z3_BIN", "/a b/c"), "Z3_BIN='/a b/c'");
        assert_eq!(
            shell_assignment("Z3_BIN", "/a/don't"),
            r"Z3_BIN='/a/don'\''t'"
        );
        assert_eq!(shell_assignment("Z3_BIN", ""), "Z3_BIN=''");
    }

    /// This module removes the drift only while it is the only implementation.
    ///
    /// A runner that grows a lookup of its own again would break nothing
    /// visible: the cases would still pass, decided by whichever toolchain and
    /// solver that lookup happened to find. So the primitives are scanned for in
    /// the two runners — comments excluded, because several name them to explain
    /// what was removed — and the patterns live here, in a file that is not one
    /// of the two being scanned.
    #[test]
    fn the_runners_do_not_look_for_the_toolchain_or_z3_themselves() {
        const PRIMITIVES: [&str; 5] = [
            "rustup which",
            "lib/rustlib",
            "lib\\rustlib",
            "Z3_HOME",
            "z3.exe",
        ];
        let mut offenders = Vec::new();
        for (name, text) in [
            ("tests/run_tests.sh", include_str!("../tests/run_tests.sh")),
            (
                "tests/run_tests.cmd",
                include_str!("../tests/run_tests.cmd"),
            ),
        ] {
            for (index, line) in text.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with('#')
                    || trimmed.starts_with("rem")
                    || trimmed.starts_with("::")
                {
                    continue;
                }
                if let Some(primitive) = PRIMITIVES
                    .iter()
                    .find(|primitive| line.contains(**primitive))
                {
                    offenders.push(format!(
                        "{name}:{}: `{}` in `{trimmed}`",
                        index + 1,
                        primitive
                    ));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "a runner is looking for the toolchain or Z3 itself; that decision belongs in this \
             module, and the readers should use what `keller-env` prints:\n{}",
            offenders.join("\n")
        );
    }

    /// How many parentheses the `cmd` parser would act on in one line, and the
    /// column of the first of them. A parenthesis inside double quotes, or one
    /// escaped as `^(`/`^)`, is text rather than structure and is not counted.
    fn cmd_parentheses(line: &str) -> (i32, Option<usize>) {
        let mut depth = 0i32;
        let mut first = None;
        let mut quoted = false;
        let mut escaped = false;
        for (index, ch) in line.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '^' => escaped = true,
                '"' => quoted = !quoted,
                '(' | ')' if !quoted => {
                    depth += if ch == '(' { 1 } else { -1 };
                    first.get_or_insert(index + 1);
                }
                _ => {}
            }
        }
        (depth, first)
    }

    /// A `cmd` parenthesised block ends at the first `)` its parser meets, so an
    /// `echo` inside one that carries a bare parenthesis closes the block early —
    /// and every line after it is then a top-level command that runs whatever
    /// the condition said. That is not hypothetical: `set "FAILED=1"` in the
    /// safe-project check escaped its condition that way, and the Windows runner
    /// then reported every check as passing, printed no `[FAIL]` at all, and
    /// still exited 1 — a runner failing without saying what failed, which reads
    /// as a broken fixture. The file states the rule in prose beside the
    /// environment block; this asserts it against the file itself.
    #[test]
    fn no_echo_inside_a_cmd_block_carries_a_bare_parenthesis() {
        let mut depth = 0i32;
        let mut offenders = Vec::new();
        for (index, line) in include_str!("../tests/run_tests.cmd").lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("rem") || trimmed.starts_with("::") {
                continue;
            }
            let (delta, first) = cmd_parentheses(trimmed);
            if depth > 0 && trimmed.starts_with("echo") {
                if let Some(column) = first {
                    offenders.push(format!(
                        "tests/run_tests.cmd:{}: the parenthesis at column {column} ends the block \
                         before the line after it: `{trimmed}`",
                        index + 1
                    ));
                }
            }
            depth += delta;
        }
        assert_eq!(
            depth, 0,
            "the parentheses in tests/run_tests.cmd do not balance, so this check cannot tell \
             which lines are inside a block"
        );
        assert!(
            offenders.is_empty(),
            "an `echo` inside a cmd block carries a bare parenthesis, which closes the block early \
             and lets the commands after it run unconditionally; write it `^(like this^)` or word \
             it without one:\n{}",
            offenders.join("\n")
        );
    }

    #[test]
    fn the_path_list_is_joined_with_the_separator_its_reader_uses() {
        let dirs = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        assert_eq!(join_paths(&dirs, Flavour::Sh), "/a:/b");
        assert_eq!(join_paths(&dirs, Flavour::Plain), "/a;/b");
        assert_eq!(
            join_paths(&dirs, Flavour::Native),
            if cfg!(windows) { "/a;/b" } else { "/a:/b" }
        );
    }

    #[test]
    fn the_toolchain_name_comes_from_the_path_rustup_installed_it_under() {
        let rustc = PathBuf::from(
            "/home/x/.rustup/toolchains/nightly-2026-07-24-x86_64-unknown-linux-gnu/bin/rustc",
        );
        assert_eq!(
            toolchain_name(&rustc).as_deref(),
            Some("nightly-2026-07-24-x86_64-unknown-linux-gnu")
        );
        // A Windows form, which is the shape this was found on.
        let rustc = PathBuf::from(
            r"C:\Users\x\.rustup\toolchains\nightly-2026-07-24-x86_64-pc-windows-gnu\bin\rustc.exe",
        );
        assert_eq!(
            toolchain_name(&rustc).as_deref(),
            Some("nightly-2026-07-24-x86_64-pc-windows-gnu")
        );
    }

    #[test]
    fn a_compiler_rustup_did_not_install_has_no_toolchain_to_select() {
        // Selecting a toolchain for a system rustc would be a guess, and a wrong
        // one means cargo refuses to run at all.
        assert_eq!(toolchain_name(&PathBuf::from("/usr/bin/rustc")), None);
        assert_eq!(toolchain_name(&PathBuf::from("/opt/toolchains")), None);
    }

    #[test]
    fn the_compiler_identity_is_the_release_and_the_commit() {
        // A real `rustc -vV` shape, abbreviated the way the driver prints it.
        let listing = "rustc 1.93.0-nightly (9c9e6c2b8 2026-07-23)\n\
                       binary: rustc\n\
                       commit-hash: 9c9e6c2b8d1f2a3b4c5d6e7f8091a2b3c4d5e6f7\n\
                       commit-date: 2026-07-23\n\
                       host: x86_64-unknown-linux-gnu\n\
                       release: 1.93.0-nightly\n\
                       LLVM version: 21.1.0\n";
        let identity = parse_rustc_identity(listing).expect("a complete listing parses");
        assert_eq!(identity.release, "1.93.0-nightly");
        assert_eq!(identity.commit, "9c9e6c2b8d1f2a3b4c5d6e7f8091a2b3c4d5e6f7");
        assert_eq!(identity.host, "x86_64-unknown-linux-gnu");

        // A listing without a commit is not an identity: accepting it would let a
        // build claim it was checked against a compiler it cannot name.
        let partial = "release: 1.93.0-nightly\nhost: x86_64-unknown-linux-gnu\n";
        assert_eq!(parse_rustc_identity(partial), None);
        // `commit-date` must not be mistaken for `commit-hash`.
        assert_eq!(version_field(partial, "commit-hash"), None);
    }
}
