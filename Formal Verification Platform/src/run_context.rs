use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub struct RunContext {
    pub id: String,
    pub root: PathBuf,
    pub target: PathBuf,
    pub wrapper: PathBuf,
    pub cache_enabled: bool,
    _cache_lock: Option<File>,
    _active_lock: File,
}

fn lock(path: &Path) -> Result<File, String> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("Could not open lock '{}': {e}", path.display()))?;
    file.lock()
        .map_err(|e| format!("Could not lock '{}': {e}", path.display()))?;
    Ok(file)
}

impl RunContext {
    pub fn create(project: &Path, exe: &Path, no_cache: bool) -> Result<Self, String> {
        let base = project.join(".keller");
        std::fs::create_dir_all(base.join("runs")).map_err(|e| e.to_string())?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let id = format!("{}-{nonce}", std::process::id());
        let root = base.join("runs").join(&id);
        std::fs::create_dir(&root).map_err(|e| e.to_string())?;
        let active_lock = lock(&root.join(".active.lock"))?;
        let cache_lock = if no_cache {
            None
        } else {
            Some(lock(&base.join("build.lock"))?)
        };
        let target = if let Ok(custom) = std::env::var("CARGO_TARGET_DIR") {
            PathBuf::from(custom).join("keller")
        } else if no_cache {
            root.join("build")
        } else {
            project.join("target").join("keller")
        };
        let _ = std::fs::create_dir_all(&target);

        #[cfg(windows)]
        let target = to_short_path(&target);

        let wrapper = root.join(if cfg!(windows) {
            "workspace-wrapper.exe"
        } else {
            "workspace-wrapper"
        });
        // Cargo fingerprints this distinct workspace-wrapper path on every run.
        // Dependencies are not instrumented and retain their normal fingerprints.
        if std::fs::hard_link(exe, &wrapper).is_err() {
            std::fs::copy(exe, &wrapper)
                .map_err(|e| format!("Could not create workspace wrapper: {e}"))?;
        }
        #[cfg(windows)]
        {
            // Ensure any DLLs in the toolchain bin directory (such as rustc_driver, std, libz3)
            // are present next to the wrapper so build scripts (e.g. autocfg) can execute the wrapper.
            if let Ok(driver_rustc) = crate::compiler::resolve_driver_rustc() {
                if let Some(toolchain_bin) = driver_rustc.parent() {
                    if let Ok(entries) = std::fs::read_dir(toolchain_bin) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.extension().map_or(false, |ext| ext == "dll") {
                                if let Some(name) = p.file_name() {
                                    let dest = root.join(name);
                                    if !dest.exists() {
                                        if std::fs::hard_link(&p, &dest).is_err() {
                                            let _ = std::fs::copy(&p, &dest);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Ensure libz3.dll is present next to wrapper if not already found in toolchain
            let dest_z3 = root.join("libz3.dll");
            if !dest_z3.exists() {
                let mut z3_candidates = Vec::new();
                if let Some(parent) = exe.parent() {
                    z3_candidates.push(parent.join("libz3.dll"));
                }
                z3_candidates.push(PathBuf::from(r"C:\z3\bin\libz3.dll"));
                for z3_path in z3_candidates {
                    if z3_path.is_file() {
                        if std::fs::hard_link(&z3_path, &dest_z3).is_err() {
                            let _ = std::fs::copy(&z3_path, &dest_z3);
                        }
                        break;
                    }
                }
            }
        }
        Ok(Self {
            id,
            root,
            target,
            wrapper,
            cache_enabled: !no_cache,
            _cache_lock: cache_lock,
            _active_lock: active_lock,
        })
    }

    pub fn finish_build(&mut self) -> Result<(), String> {
        if !self.cache_enabled && self.target.exists() {
            std::fs::remove_dir_all(&self.target).map_err(|e| e.to_string())?;
        }
        // The executable and helper DLLs are only needed while compiler processes run. Evidence stays.
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |ext| ext == "dll" || ext == "exe") {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        self._cache_lock.take();
        Ok(())
    }

    pub fn publish(&self, project: &Path) -> Result<(), String> {
        let _guard = lock(&project.join(".keller/publication.lock"))?;
        // The completion marker is absent while this compatibility view changes.
        // Authoritative retained runs are immutable and do not need this view.
        for name in [
            crate::report::RUN_FILE,
            crate::report::REPORT_FILE,
            crate::report::ENV_FILE,
            "report.html",
        ] {
            let path = project.join(name);
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Could not reset '{}': {e}", path.display())),
            }
        }
        for name in [crate::report::REPORT_DIR, crate::report::OBLIGATION_DIR] {
            let dest = project.join(name);
            match std::fs::remove_dir_all(&dest) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Could not reset '{}': {e}", dest.display())),
            }
            let source = self.root.join(name);
            if source.exists() {
                copy_tree(&source, &dest)?;
            }
        }
        for name in [crate::report::REPORT_FILE, crate::report::ENV_FILE] {
            atomic_copy(&self.root.join(name), &project.join(name))?;
        }
        let html_source = self.root.join("report.html");
        if html_source.exists() {
            let _ = atomic_copy(&html_source, &project.join("report.html"));
        }
        // Completion record is the publication marker and is always copied last.
        atomic_copy(
            &self.root.join(crate::report::RUN_FILE),
            &project.join(crate::report::RUN_FILE),
        )?;

        // Bundle all report artifacts cleanly into the dedicated folder `keller-report/`:
        let bundle_dir = project.join("keller-report");
        if let Err(e) = std::fs::create_dir_all(&bundle_dir) {
            eprintln!(
                "[Keller-Pipeline] Warning: Could not create '{}': {e}",
                bundle_dir.display()
            );
        } else {
            for name in [crate::report::REPORT_FILE, crate::report::ENV_FILE] {
                let _ = atomic_copy(&self.root.join(name), &bundle_dir.join(name));
            }
            if html_source.exists() {
                let _ = atomic_copy(&html_source, &bundle_dir.join("report.html"));
            }
            let _ = atomic_copy(
                &self.root.join(crate::report::RUN_FILE),
                &bundle_dir.join(crate::report::RUN_FILE),
            );
        }
        Ok(())
    }

    pub fn finalize_gate(
        &self,
        options: &crate::report::VerifyOptions,
        outcome: &crate::report::MergeOutcome,
        status_override: Option<&str>,
    ) -> Result<(), String> {
        let path = self.root.join(crate::report::RUN_FILE);
        let mut record: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&path).map_err(|e| format!("Could not read run record: {e}"))?,
        )
        .map_err(|e| format!("Invalid run record: {e}"))?;
        record["completed"] = serde_json::json!(true);
        record["completed_at_unix_ns"] = serde_json::json!(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .min(u64::MAX as u128)
            as u64);
        // The same keys the merge wrote, including `advisory_excluded`: this runs
        // afterwards and replaces the block wholesale, so a key it does not name
        // is not merged — it is deleted. That is how the per-crate advisory count
        // reached the run record and then vanished from the published one, which
        // left "why did this exit 0" answerable only by reading `keller.toml`.
        record["gate"] = serde_json::json!({
            "status": status_override.unwrap_or(&outcome.run_status),
            "deny_warn": options.deny_warn,
            "strict_coverage": options.strict_coverage,
            "gated_findings": outcome.gated.len(),
            "baseline_suppressed": outcome.suppressed,
            "advisory_excluded": outcome.advisory_excluded,
            "breach": outcome.breach.map(|breach| breach.to_string()),
        });
        record["sarif"] = match &options.sarif {
            Some(path) => serde_json::json!({"path": path, "written": path.is_file()}),
            None => serde_json::Value::Null,
        };
        let json = serde_json::to_vec_pretty(&record).map_err(|e| e.to_string())?;
        atomic_write(&path, &json)?;
        Ok(())
    }

    /// Record a run that was stopped by the wall-clock cap.
    ///
    /// There is no merged outcome to record, and saying so is the point: a record
    /// whose status is `run-timeout` cannot be read as a verdict, where publishing a
    /// truncated report would put "no findings" in front of a reader for a crate the
    /// analysis never finished. The names of the crates that were in flight are the
    /// part that makes the stopped run diagnosable.
    ///
    /// The merge writes this record on a run that finishes; a stopped run never
    /// reaches it, so the record is created here from what is known instead of left
    /// absent -- an absent record is indistinguishable from a run that was killed by
    /// something outside the tool.
    pub fn finalize_timeout(
        &self,
        options: &crate::report::VerifyOptions,
        cap_seconds: u64,
        in_flight: &[String],
    ) -> Result<(), String> {
        let path = self.root.join(crate::report::RUN_FILE);
        let mut record = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
                .map_err(|e| format!("Invalid run record: {e}"))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({
                "schema_version": 1,
                "run_id": self.id,
                "run_directory": self.root,
                "tool": crate::report::TOOL_NAME,
                "version": crate::report::version(),
                "settings": {
                    "strict_coverage": options.strict_coverage,
                    "deny_warn": options.deny_warn,
                    "solver_timeout_ms": options
                        .solver_timeout_ms
                        .unwrap_or(crate::report::DEFAULT_SOLVER_TIMEOUT_MS),
                    "solver_timeout_source": if options.solver_timeout_ms.is_none() {
                        "default"
                    } else {
                        "flag"
                    },
                    "run_timeout_seconds": options
                        .run_timeout_seconds
                        .unwrap_or(crate::report::DEFAULT_RUN_TIMEOUT_SECONDS),
                    "run_timeout_source": if options.run_timeout_seconds.is_none() {
                        "default"
                    } else {
                        "flag"
                    },
                    "gate_profile": options.gate_profile,
                    "only": options.only,
                    "max_findings": options.max_findings,
                    "keep_runs": options.keep_runs.unwrap_or(10),
                },
            }),
            Err(e) => return Err(format!("Could not read run record: {e}")),
        };
        record["completed"] = serde_json::json!(true);
        record["completed_at_unix_ns"] = serde_json::json!(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .min(u64::MAX as u128)
            as u64);
        // Null, not zero-filled: coverage that was never computed is unknown, and a
        // count of 0 assertions would read as "nothing needed checking".
        record["coverage"] = serde_json::Value::Null;
        record["gate"] = serde_json::json!({
            "status": "run-timeout",
            "deny_warn": options.deny_warn,
            "strict_coverage": options.strict_coverage,
            "gated_findings": 0,
            "baseline_suppressed": 0,
            "breach": "run-timeout",
        });
        record["stopped"] = serde_json::json!({
            "reason": "run-timeout",
            "cap_seconds": cap_seconds,
            "crates_without_completion": in_flight,
        });
        let json = serde_json::to_vec_pretty(&record).map_err(|e| e.to_string())?;
        atomic_write(&path, &json)?;
        Ok(())
    }

    pub fn prune(&self, project: &Path, keep: usize) -> Result<(), String> {
        let _guard = lock(&project.join(".keller/prune.lock"))?;
        let mut runs = Vec::new();
        for entry in std::fs::read_dir(project.join(".keller/runs")).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !path.is_dir() || path == self.root {
                continue;
            }
            let record = path.join(crate::report::RUN_FILE);
            let Ok(bytes) = std::fs::read(&record) else {
                continue;
            };
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                continue;
            };
            if value["completed"] != true {
                continue;
            }
            runs.push((value["completed_at_unix_ns"].as_u64().unwrap_or(0), path));
        }
        runs.sort_by_key(|(time, _)| *time);
        // Do not retire the compatibility view's currently published run.
        let current = std::fs::read(project.join(crate::report::RUN_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|value| value["run_id"].as_str().map(str::to_string));
        if let Some(current) = &current {
            runs.retain(|(_, path)| {
                path.file_name().and_then(|name| name.to_str()) != Some(current)
            });
        }
        let remove_count = runs.len().saturating_sub(keep.saturating_sub(1));
        for (_, path) in runs.into_iter().take(remove_count) {
            let active_path = path.join(".active.lock");
            let active = OpenOptions::new()
                .create(false)
                .read(true)
                .write(true)
                .open(&active_path);
            if let Ok(file) = active {
                if file.try_lock().is_ok() {
                    std::fs::remove_dir_all(path).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }
}

fn temp_sibling(path: &Path) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.with_extension(format!(
        "{}-{}-{nonce}.tmp",
        std::process::id(),
        std::thread::current().name().unwrap_or("run")
    ))
}

fn replace_file(temp: &Path, path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Could not remove old '{}': {e}", path.display())),
    }
    #[cfg(windows)]
    {
        std::fs::rename(temp, path)
            .map_err(|e| format!("Could not publish '{}': {e}", path.display()))
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(temp, path)
            .map_err(|e| format!("Could not publish '{}': {e}", path.display()))
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = temp_sibling(path);
    std::fs::write(&temp, bytes)
        .map_err(|e| format!("Could not write '{}': {e}", temp.display()))?;
    replace_file(&temp, path)
}

fn atomic_copy(source: &Path, path: &Path) -> Result<(), String> {
    let temp = temp_sibling(path);
    std::fs::copy(source, &temp)
        .map_err(|e| format!("Could not copy '{}': {e}", source.display()))?;
    replace_file(&temp, path)
}

fn copy_tree(source: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &dest.join(entry.file_name()))?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), dest.join(entry.file_name())).map_err(|e| e.to_string())?;
        } else {
            return Err("Unexpected symlink or special file in run evidence.".to_string());
        }
    }
    Ok(())
}

#[cfg(windows)]
fn to_short_path(path: &Path) -> PathBuf {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut buffer = vec![0u16; 1024];
    extern "system" {
        fn GetShortPathNameW(lpszLongPath: *const u16, lpszShortPath: *mut u16, cchBuffer: u32) -> u32;
    }
    let res = unsafe { GetShortPathNameW(wide.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
    if res > 0 && (res as usize) < buffer.len() {
        buffer.truncate(res as usize);
        PathBuf::from(OsString::from_wide(&buffer))
    } else {
        path.to_path_buf()
    }
}
