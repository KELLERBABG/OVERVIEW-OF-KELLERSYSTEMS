//! `keller.toml`: the checked-in run policy (B8 / K19, and `keller init`).
//!
//! Before this, every setting was a flag or a `KELLER_*` environment variable, so
//! a repository could not record "these crates are gated, these are advisory"
//! anywhere a reviewer would find it — the decision lived in whatever a CI file
//! happened to pass that week. This module reads a small, explicit subset of TOML
//! from the project directory and turns it into the *base* a run starts from.
//!
//! Two rules make it predictable:
//!
//! - **The command line always wins.** The config is applied first and the flags
//!   are applied on top, so `keller verify --profile advisory` in a repository
//!   whose config says `profile = "strict"` is advisory. A config file that
//!   silently overrode a flag someone typed would be worse than no config file.
//! - **An unknown key is an error, not a default.** A typo in `strict_coverage`
//!   must not leave the gate quietly looser than the author intended — the same
//!   reason an unrecognised flag is refused rather than ignored.
//!
//! The parser is deliberately a subset: top-level `key = value`, and `[[policy]]`
//! tables. It exists so the tool does not carry a TOML dependency into a build
//! that already pins a nightly compiler and a specific Z3, and every rule it does
//! not understand is reported with a line number rather than ignored.

use std::path::{Path, PathBuf};

use crate::report::{Policy, VerifyOptions};

/// File name looked for in the verified project's directory.
pub const CONFIG_FILE: &str = "keller.toml";

/// The keys allowed in a `[[policy]]` table.
const POLICY_KEYS: [&str; 3] = ["crate", "path", "gate"];

/// What a `keller.toml` said.
///
/// Every field is `Option` so "not mentioned" and "mentioned as false" stay
/// distinguishable: only what the file actually sets is applied, and anything it
/// leaves out keeps the built-in default rather than being forced to zero.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    pub profile: Option<String>,
    pub deny_warn: Option<bool>,
    pub strict_coverage: Option<bool>,
    pub solver_timeout_ms: Option<u64>,
    pub run_timeout_seconds: Option<u64>,
    pub only: Option<String>,
    pub max_findings: Option<usize>,
    pub baseline: Option<PathBuf>,
    pub sarif: Option<PathBuf>,
    pub no_cache: Option<bool>,
    pub keep_runs: Option<usize>,
    pub verify_deps: Option<bool>,
    pub release: Option<bool>,
    pub solver: Option<String>,
    pub policies: Vec<Policy>,
}

/// Read `keller.toml` from `dir`.
///
/// `Ok(None)` means there is no config file, which is not an error: a project
/// without one behaves exactly as it did before this module existed. A file that
/// exists but cannot be understood is an error, because silently ignoring it would
/// leave the gate looser than the file says.
pub fn load(dir: &Path) -> Result<Option<Config>, String> {
    let path = dir.join(CONFIG_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => parse(&text).map(Some),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("Could not read '{}': {err}", path.display())),
    }
}

/// The [`VerifyOptions`] base a config file describes.
///
/// A profile is applied first and the individual keys after it, so
/// `profile = "strict"` followed by `deny_warn = false` means what it reads as:
/// the profile is a starting point, exactly as it is on the command line.
pub fn base_options(config: &Config) -> VerifyOptions {
    let mut options = VerifyOptions::default();
    if let Some(profile) = &config.profile {
        // The same two lines `--profile` applies, so a profile means one thing
        // whether it was typed or checked in.
        match profile.as_str() {
            "strict" => {
                options.deny_warn = true;
                options.strict_coverage = true;
            }
            _ => {
                options.deny_warn = false;
                options.strict_coverage = false;
            }
        }
        options.gate_profile = Some(profile.clone());
    }
    if let Some(value) = config.deny_warn {
        options.deny_warn = value;
    }
    if let Some(value) = config.strict_coverage {
        options.strict_coverage = value;
    }
    if let Some(value) = config.solver_timeout_ms {
        options.solver_timeout_ms = Some(value);
    }
    if let Some(value) = config.run_timeout_seconds {
        options.run_timeout_seconds = Some(value);
    }
    if let Some(value) = &config.only {
        options.only = Some(value.clone());
    }
    if let Some(value) = config.max_findings {
        options.max_findings = Some(value);
    }
    if let Some(value) = &config.baseline {
        options.baseline = Some(value.clone());
    }
    if let Some(value) = &config.sarif {
        options.sarif = Some(value.clone());
    }
    if let Some(value) = config.no_cache {
        options.no_cache = value;
    }
    if let Some(value) = config.keep_runs {
        options.keep_runs = Some(value);
    }
    if let Some(value) = config.verify_deps {
        options.verify_deps = value;
    }
    if let Some(value) = config.release {
        options.release = value;
    }
    if let Some(value) = &config.solver {
        options.solver = Some(value.clone());
    }
    options.policies = config.policies.clone();
    options
}

/// The starter file `keller init` writes.
///
/// Every line that would change behaviour is commented out, so an initialised
/// project runs exactly as it did before the file existed. A generated config that
/// silently started gating is a trap; this one is a place to write decisions down.
pub fn default_text() -> String {
    format!(
        "# {CONFIG_FILE} - checked-in run policy for keller-verify.\n\
         #\n\
         # This file supplies the *defaults* for a run; anything given on the command\n\
         # line still wins, so a flag typed by hand is never overridden by a file.\n\
         # An unknown key here is an error rather than a silent default, so a typo\n\
         # cannot leave the gate looser than it looks.\n\
         \n\
         # The named gate contract: `advisory` (default) or `strict`.\n\
         # profile = \"advisory\"\n\
         \n\
         # Individual settings. `strict` above is exactly `deny_warn` plus\n\
         # `strict_coverage`, so use whichever reads better.\n\
         # deny_warn = true\n\
         # strict_coverage = true\n\
         \n\
         # Budgets. `0` means no limit for both, which is only for a crate that has\n\
         # been measured; the default is 30000 ms per obligation, while run_timeout\n\
         # defaults to 0 (disabled).\n\
         # solver_timeout = 30000\n\
         # run_timeout = 0\n\
         \n\
         # Narrow a run while adopting the tool on an existing codebase.\n\
         # only = \"crate::function_substring\"\n\
         # max_findings = 50\n\
         # baseline = \".keller-baseline.json\"\n\
         # sarif = \"keller.sarif\"\n\
         \n\
         # Per-crate policy: which crates are gated and which are advisory.\n\
         # An advisory crate's findings are still reported - they are in report.json,\n\
         # on the console and in SARIF - but they are not what fails the build.\n\
         #\n\
         # [[policy]]\n\
         # crate = \"legacy_crypto\"\n\
         # gate = \"advisory\"\n\
         \n\
         # [[policy]]\n\
         # crate = \"core_math\"\n\
         # gate = \"strict\"\n"
    )
}

/// Parse the subset of TOML this module understands.
///
/// Deliberately strict: `key = value` at the top level, `[[policy]]` tables, and
/// nothing else. Anything unrecognised is an error naming the line, because the
/// whole value of a checked-in policy is that what it says is what runs.
pub fn parse(text: &str) -> Result<Config, String> {
    let mut config = Config::default();
    let mut in_policy = false;
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = strip_comment(raw);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("[[") {
            if line != "[[policy]]" {
                return Err(format!(
                    "{CONFIG_FILE}:{line_no}: unknown table '{line}'; only [[policy]] is supported"
                ));
            }
            config.policies.push(Policy {
                crate_name: String::new(),
                path: None,
                // Gated unless the entry says otherwise, so a half-written entry
                // reports a problem rather than hiding one.
                advisory: false,
            });
            in_policy = true;
            continue;
        }
        if line.starts_with('[') {
            return Err(format!(
                "{CONFIG_FILE}:{line_no}: unknown table '{line}'; only [[policy]] is supported"
            ));
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("{CONFIG_FILE}:{line_no}: expected `key = value`"))?;
        let key = key.trim();
        let value = value.trim();

        if in_policy {
            if !POLICY_KEYS.contains(&key) {
                return Err(format!(
                    "{CONFIG_FILE}:{line_no}: unknown key '{key}' in [[policy]]; top-level keys must appear before the first [[policy]] table"
                ));
            }
            let entry = config
                .policies
                .last_mut()
                .expect("a [[policy]] header pushed an entry");
            match key {
                "crate" => entry.crate_name = parse_string(value, line_no)?,
                "path" => entry.path = Some(parse_string(value, line_no)?),
                "gate" => {
                    let gate = parse_string(value, line_no)?;
                    entry.advisory = match gate.as_str() {
                        "advisory" => true,
                        "strict" => false,
                        other => {
                            return Err(format!(
                                "{CONFIG_FILE}:{line_no}: gate must be `advisory` or `strict`, got '{other}'"
                            ))
                        }
                    };
                }
                _ => unreachable!("the key was checked against POLICY_KEYS above"),
            }
            continue;
        }

        match key {
            "profile" => {
                let profile = parse_string(value, line_no)?;
                if profile != "advisory" && profile != "strict" {
                    return Err(format!(
                        "{CONFIG_FILE}:{line_no}: profile must be `advisory` or `strict`, got '{profile}'"
                    ));
                }
                config.profile = Some(profile);
            }
            "deny_warn" => config.deny_warn = Some(parse_bool(value, line_no)?),
            "strict_coverage" => config.strict_coverage = Some(parse_bool(value, line_no)?),
            "solver_timeout" => config.solver_timeout_ms = Some(parse_u64(value, line_no)?),
            "run_timeout" => config.run_timeout_seconds = Some(parse_u64(value, line_no)?),
            "only" => config.only = Some(parse_string(value, line_no)?),
            "max_findings" => {
                let count = parse_u64(value, line_no)?;
                if count == 0 {
                    return Err(format!(
                        "{CONFIG_FILE}:{line_no}: max_findings must be at least 1; leave it out to list every finding"
                    ));
                }
                config.max_findings = Some(count as usize);
            }
            "baseline" => config.baseline = Some(PathBuf::from(parse_string(value, line_no)?)),
            "sarif" => config.sarif = Some(PathBuf::from(parse_string(value, line_no)?)),
            "no_cache" => config.no_cache = Some(parse_bool(value, line_no)?),
            "keep_runs" => config.keep_runs = Some(parse_u64(value, line_no)? as usize),
            "verify_deps" => config.verify_deps = Some(parse_bool(value, line_no)?),
            "release" => config.release = Some(parse_bool(value, line_no)?),
            "solver" => config.solver = Some(parse_string(value, line_no)?),
            other => return Err(format!("{CONFIG_FILE}:{line_no}: unknown key '{other}'")),
        }
    }

    for policy in &config.policies {
        if policy.crate_name.is_empty() && policy.path.is_none() {
            return Err(format!(
                "{CONFIG_FILE}: a [[policy]] table needs a `crate` name or `path` pattern"
            ));
        }
    }
    Ok(config)
}

/// Check if a source file path matches a glob pattern or prefix (B8).
///
/// Supports `*` (matching non-separator characters) and `**` (matching across directories).
/// Path separators `\` and `/` are normalized.
pub fn path_matches(pattern: &str, file: &str) -> bool {
    let norm_pat = pattern.replace('\\', "/");
    let norm_file = file.replace('\\', "/");
    glob_match_inner(norm_pat.as_bytes(), norm_file.as_bytes())
}

fn glob_match_inner(pat: &[u8], text: &[u8]) -> bool {
    if pat.is_empty() {
        return text.is_empty();
    }
    if pat.starts_with(b"**/") {
        let rest_pat = &pat[3..];
        if glob_match_inner(rest_pat, text) {
            return true;
        }
        for i in 0..text.len() {
            if text[i] == b'/' && glob_match_inner(pat, &text[i + 1..]) {
                return true;
            }
        }
        return false;
    }
    if pat == b"**" {
        return true;
    }
    if pat.starts_with(b"/**") && pat.len() == 3 {
        return true;
    }
    if pat[0] == b'*' {
        let rest_pat = &pat[1..];
        if glob_match_inner(rest_pat, text) {
            return true;
        }
        if !text.is_empty() && text[0] != b'/' {
            return glob_match_inner(pat, &text[1..]);
        }
        return false;
    }
    if pat[0] == b'?' {
        if !text.is_empty() && text[0] != b'/' {
            return glob_match_inner(&pat[1..], &text[1..]);
        }
        return false;
    }
    if !text.is_empty() && pat[0] == text[0] {
        return glob_match_inner(&pat[1..], &text[1..]);
    }
    false
}

/// Drop a `#` comment, respecting quoted strings so a value may contain `#`.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..index],
            _ => {}
        }
    }
    line
}

fn parse_string(value: &str, line_no: usize) -> Result<String, String> {
    let inner = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| {
            format!("{CONFIG_FILE}:{line_no}: expected a quoted string, got '{value}'")
        })?;
    Ok(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn parse_bool(value: &str, line_no: usize) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!(
            "{CONFIG_FILE}:{line_no}: expected `true` or `false`, got '{other}'"
        )),
    }
}

fn parse_u64(value: &str, line_no: usize) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{CONFIG_FILE}:{line_no}: expected a whole number, got '{value}'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_or_commented_file_changes_nothing() {
        // The starter file `keller init` writes has to be inert: it is a place to
        // write decisions down, not a file that starts gating on its own.
        let config = parse(&default_text()).expect("the generated file must parse");
        assert_eq!(
            config,
            Config::default(),
            "every example line is commented out"
        );
        assert_eq!(base_options(&config), VerifyOptions::default());
    }

    #[test]
    fn a_profile_is_applied_the_same_way_the_flag_is() {
        let config = parse("profile = \"strict\"\n").unwrap();
        let options = base_options(&config);
        assert!(options.deny_warn && options.strict_coverage);
        assert_eq!(options.gate_profile.as_deref(), Some("strict"));

        // And an individual key after it wins, which is what makes a profile a
        // starting point rather than a mode.
        let config = parse("profile = \"strict\"\ndeny_warn = false\n").unwrap();
        let options = base_options(&config);
        assert!(!options.deny_warn);
        assert!(options.strict_coverage, "the profile still set this one");
    }

    #[test]
    fn per_crate_policy_is_read_and_defaults_to_gated() {
        let config = parse(
            "[[policy]]\ncrate = \"legacy\"\ngate = \"advisory\"\n\n[[policy]]\ncrate = \"core\"\ngate = \"strict\"\n",
        )
        .unwrap();
        assert_eq!(
            config.policies,
            vec![
                Policy {
                    crate_name: "legacy".to_string(),
                    path: None,
                    advisory: true
                },
                Policy {
                    crate_name: "core".to_string(),
                    path: None,
                    advisory: false
                },
            ]
        );
    }

    #[test]
    fn per_path_policy_and_globs_are_supported() {
        let config = parse(
            "[[policy]]\npath = \"tests/**\"\ngate = \"advisory\"\n\n[[policy]]\ncrate = \"legacy\"\npath = \"src/old/*.rs\"\ngate = \"advisory\"\n",
        )
        .unwrap();
        assert_eq!(
            config.policies,
            vec![
                Policy {
                    crate_name: String::new(),
                    path: Some("tests/**".to_string()),
                    advisory: true
                },
                Policy {
                    crate_name: "legacy".to_string(),
                    path: Some("src/old/*.rs".to_string()),
                    advisory: true
                },
            ]
        );

        assert!(path_matches("tests/**", "tests/cases/foo.rs"));
        assert!(path_matches("tests/**", "tests\\cases\\foo.rs"));
        assert!(path_matches("src/old/*.rs", "src/old/main.rs"));
        assert!(!path_matches("src/old/*.rs", "src/old/nested/main.rs"));
        assert!(path_matches("**/main.rs", "src/nested/main.rs"));
    }

    #[test]
    fn an_unknown_key_or_malformed_value_is_refused_with_its_line() {
        // A typo that silently left the gate looser would be the worst outcome, so
        // every unrecognised thing names the line it is on.
        let err = parse("strict_covrage = true\n").unwrap_err();
        assert!(err.contains("keller.toml:1"), "{err}");
        assert!(err.contains("strict_covrage"), "{err}");

        assert!(parse("deny_warn = yes\n")
            .unwrap_err()
            .contains("true` or `false`"));
        assert!(parse("profile = \"nope\"\n")
            .unwrap_err()
            .contains("advisory"));
        assert!(parse("max_findings = 0\n")
            .unwrap_err()
            .contains("at least 1"));
        assert!(parse("[other]\n").unwrap_err().contains("unknown table"));
        assert!(parse("[[policy]]\ngate = \"strict\"\n")
            .unwrap_err()
            .contains("needs a `crate` name"));
    }

    #[test]
    fn verify_deps_can_be_configured() {
        let config = parse("verify_deps = true\n").unwrap();
        assert_eq!(config.verify_deps, Some(true));
        let options = base_options(&config);
        assert!(options.verify_deps);
    }

    #[test]
    fn release_can_be_configured() {
        let config = parse("release = true\n").unwrap();
        assert_eq!(config.release, Some(true));
        let options = base_options(&config);
        assert!(options.release);
    }

    #[test]
    fn comments_and_strings_with_hashes_survive() {
        let config =
            parse("# a comment\nonly = \"a#b\"  # trailing comment\nsarif = \"out.sarif\"\n")
                .unwrap();
        assert_eq!(config.only.as_deref(), Some("a#b"));
        assert_eq!(config.sarif.as_deref(), Some(Path::new("out.sarif")));
    }

    #[test]
    fn a_top_level_key_after_a_policy_table_is_an_error_not_a_silent_default() {
        // TOML puts top-level keys above the tables; accepting one here would mean
        // guessing whether it was meant for the policy or the run.
        let err =
            parse("[[policy]]\ncrate = \"a\"\ngate = \"advisory\"\nonly = \"x\"\n").unwrap_err();
        assert!(err.contains("before the first [[policy]] table"), "{err}");
    }

    #[test]
    fn a_missing_file_is_not_an_error_and_a_broken_one_is() {
        let dir = std::env::temp_dir().join(format!("keller-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(load(&dir).unwrap(), None, "no config file is not a failure");

        std::fs::write(dir.join(CONFIG_FILE), "nonsense\n").unwrap();
        assert!(load(&dir).unwrap_err().contains("keller.toml:1"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
