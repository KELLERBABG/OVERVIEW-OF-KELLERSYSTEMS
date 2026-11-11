use rustc_middle::mir::Body;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::contract::FnContracts;
use crate::types::{FunctionEvidence, VerificationEvent};

/// Bumped whenever a cached entry stops carrying enough to reproduce the run's
/// coverage or artifacts. A cache written by another format is discarded
/// wholesale, because a half-restored entry produces a receipt whose counts
/// disagree with its findings, or an obligation manifest that indexes files no
/// longer on disk - and that receipt is the evidence, not a diagnostic.
const CACHE_VERSION: u64 = 3;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerificationCache {
    #[serde(default)]
    pub version: u64,
    pub functions: HashMap<String, CachedFunctionEntry>,
}

impl Default for VerificationCache {
    fn default() -> Self {
        Self {
            version: CACHE_VERSION,
            functions: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedFunctionEntry {
    pub fingerprint: u64,
    pub events: Vec<VerificationEvent>,
    pub evidence: Option<FunctionEvidence>,
    pub supported_assertions: usize,
    pub unsupported_assertions: usize,
    pub panic_sites: usize,
    /// The panic sites this function's analysis could decide — a safe site
    /// leaves no event, so this count has to travel with the cache entry too.
    #[serde(default)]
    pub panic_sites_decided: usize,
    /// Proofs decided by constant folding. They deliberately leave no `PASS`
    /// event, so a cache hit has to carry the count or a fully proved function
    /// is restored as undecided.
    #[serde(default)]
    pub folded_passes: usize,
    /// The exact SMT-LIB files this function's analysis wrote.
    ///
    /// `prepare_report_dir` deletes `obligations/` at the start of every run, so
    /// a cache hit that only restores events leaves the manifest indexing files
    /// that no longer exist. Keeping the bytes here lets the hit republish them
    /// unchanged, which is the point of a cache: the evidence for a cached
    /// verdict has to be the same evidence the first run produced.
    #[serde(default)]
    pub obligations: Vec<CachedObligation>,
}

/// One SMT-LIB artifact recorded alongside the function that produced it.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct CachedObligation {
    /// File name under `obligations/<crate>/`, e.g. `add_bb0.smt2`.
    pub artifact: String,
    /// The file's exact contents.
    pub contents: String,
}

impl VerificationCache {
    pub fn cache_dir_for(project: &Path) -> PathBuf {
        project.join(".keller").join("cache")
    }

    pub fn cache_path_for(project: &Path, crate_name: &str) -> PathBuf {
        Self::cache_dir_for(project).join(format!("{crate_name}.cache.json"))
    }

    pub fn load_for_crate(project: &Path, crate_name: &str) -> Self {
        let path = Self::cache_path_for(project, crate_name);
        if !path.exists() {
            return Self::default();
        }
        let loaded: Option<VerificationCache> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        match loaded {
            Some(cache) if cache.version == CACHE_VERSION => cache,
            // An unreadable cache and one from another format are the same
            // thing here: nothing in it can be trusted to add up.
            _ => Self::default(),
        }
    }

    pub fn save_for_crate(&self, project: &Path, crate_name: &str) -> Result<(), String> {
        let dir = Self::cache_dir_for(project);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = Self::cache_path_for(project, crate_name);
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get(&self, function_name: &str, fingerprint: u64) -> Option<&CachedFunctionEntry> {
        let entry = self.functions.get(function_name)?;
        if entry.fingerprint == fingerprint {
            Some(entry)
        } else {
            None
        }
    }

    pub fn insert(&mut self, function_name: String, entry: CachedFunctionEntry) {
        self.functions.insert(function_name, entry);
    }
}

pub fn compute_mir_fingerprint(mir: &Body<'_>, contracts: &FnContracts) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    // Include analysis parameters that affect precision or solver verdicts
    // so that an imprecise/cut-budget run never poisons a normal run's cache.
    std::env::var("KELLER_MAX_ROUNDS")
        .unwrap_or_default()
        .hash(&mut hasher);
    std::env::var("KELLER_SOLVER_TIMEOUT_MS")
        .unwrap_or_default()
        .hash(&mut hasher);
    std::env::var("KELLER_RELEASE")
        .unwrap_or_default()
        .hash(&mut hasher);

    mir.basic_blocks.len().hash(&mut hasher);
    mir.local_decls.len().hash(&mut hasher);
    mir.arg_count.hash(&mut hasher);

    for (bb, block) in mir.basic_blocks.iter_enumerated() {
        bb.index().hash(&mut hasher);
        block.statements.len().hash(&mut hasher);
        for stmt in &block.statements {
            format!("{:?}", stmt.kind).hash(&mut hasher);
        }
        if let Some(terminator) = &block.terminator {
            format!("{:?}", terminator.kind).hash(&mut hasher);
        }
    }

    for (req_raw, _) in &contracts.requires {
        req_raw.hash(&mut hasher);
    }
    for (ens_raw, _) in &contracts.ensures {
        ens_raw.hash(&mut hasher);
    }

    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_roundtrip() {
        let mut cache = VerificationCache::default();
        let entry = CachedFunctionEntry {
            fingerprint: 12345,
            events: vec![VerificationEvent {
                crate_name: "test_crate".to_string(),
                function: "foo".to_string(),
                block: "bb0".to_string(),
                status: "PASS".to_string(),
                kind: "overflow".to_string(),
                defect_class: String::new(),
                file: "src/lib.rs".to_string(),
                line: Some(10),
                details: "test proof".to_string(),
                witness: Vec::new(),
                witness_kind: String::new(),
            }],
            evidence: None,
            supported_assertions: 1,
            unsupported_assertions: 0,
            panic_sites: 0,
            panic_sites_decided: 0,
            folded_passes: 1,
            obligations: vec![CachedObligation {
                artifact: "foo_bb0.smt2".to_string(),
                contents: "; keller-verify proof obligation\n(check-sat)\n".to_string(),
            }],
        };

        cache.insert("foo".to_string(), entry.clone());
        assert!(cache.get("foo", 12345).is_some());
        assert!(cache.get("foo", 99999).is_none());

        let temp_dir =
            std::env::temp_dir().join(format!("keller_test_cache_{}", std::process::id()));
        cache.save_for_crate(&temp_dir, "test_crate").unwrap();
        let loaded = VerificationCache::load_for_crate(&temp_dir, "test_crate");
        assert_eq!(loaded.version, CACHE_VERSION);
        let restored = loaded.get("foo", 12345).expect("the entry round-tripped");
        // The event-less folded proof is the counter a cache hit used to lose.
        assert_eq!(restored.folded_passes, 1);
        assert_eq!(restored.supported_assertions, 1);
        // The artifact bytes travel with the entry so a hit can republish them.
        assert_eq!(restored.obligations, entry.obligations);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn a_cache_from_another_format_is_discarded() {
        let temp_dir =
            std::env::temp_dir().join(format!("keller_test_cache_version_{}", std::process::id()));
        let dir = VerificationCache::cache_dir_for(&temp_dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            VerificationCache::cache_path_for(&temp_dir, "test_crate"),
            r#"{"version":1,"functions":{}}"#,
        )
        .unwrap();
        let loaded = VerificationCache::load_for_crate(&temp_dir, "test_crate");
        assert_eq!(loaded.version, CACHE_VERSION);
        assert!(loaded.functions.is_empty());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
