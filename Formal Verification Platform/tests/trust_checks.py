#!/usr/bin/env python3
"""Trust regression gate. Uses the built verifier, never mocks solver verdicts."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
VERIFIER = ROOT / "target/debug" / ("keller-verify.exe" if os.name == "nt" else "keller-verify")


def run(cwd, expected, *args, env=None):
    proc = subprocess.run([str(VERIFIER), "keller", *args], cwd=cwd,
                          capture_output=True, text=True, encoding="utf-8", env=env, timeout=60)
    assert proc.returncode == expected, (args, proc.returncode, proc.stdout, proc.stderr)
    return proc


with tempfile.TemporaryDirectory(prefix="keller-trust-") as tmp:
    # The installed CLI should show its own usage when invoked like a careless
    # first-time user, not enter rustc-wrapper mode and print rustc's help.
    no_args = subprocess.run([str(VERIFIER)], capture_output=True, text=True,
                             encoding="utf-8", timeout=10)
    assert no_args.returncode == 0, (no_args.returncode, no_args.stdout, no_args.stderr)
    assert no_args.stdout.startswith("Usage: keller verify"), no_args.stdout

    cwd = Path(tmp)
    (cwd / "src").mkdir()
    (cwd / "rust-toolchain.toml").write_text((ROOT / "rust-toolchain.toml").read_text())
    (cwd / "Cargo.toml").write_text('[package]\nname = "trust_probe"\nversion = "0.0.0"\nedition = "2021"\n[workspace]\n')
    source = cwd / "src/main.rs"
    source.write_text('fn main() { let x: u8 = 10; println!("{}", x + 1); }\n')
    for _ in range(2):
        summary = json.loads(run(cwd, 0, "verify", "--json", "--strict-coverage").stdout)
        assert json.loads((cwd / "report.json").read_text()) == []
        assert summary["coverage"]["workers"] == 1
        assert summary["coverage"]["functions"] > 0
        assert summary["coverage"]["reachable_assertions"] > 0
        record = json.loads((cwd / "report.run.json").read_text())
        assert record["completed"] and record["schema_version"] == 1
        compiler = record["coverage"]["compilers"][0]
        assert compiler["rustc"] and "host:" in compiler["version_verbose"]
        assert record["env"]["rustc"] == compiler["rustc"]
        assert record["env"]["host"] in compiler["version_verbose"]
        assert record["env"]["host"] in record["env"]["rustlib"], record["env"]
        runtime_paths = record["env"]["path_add"].split(os.pathsep)
        assert record["env"]["rustlib"] in runtime_paths, record["env"]
        assert runtime_paths[0] == record["env"]["rustlib"], record["env"]
        assert all("msvc" not in path.lower() for path in runtime_paths
                   if "rustlib" in path.lower()) or "msvc" in record["env"]["host"], record["env"]

    print("[PASS] trust: unchanged clean runs analyse and write [] plus completion evidence")

    source.write_text('pub fn add(x: u8) -> u8 { x + 1 }\nfn main() { println!("{}", add(255)); }\n')
    reports = []
    for _ in range(2):
        summary = json.loads(run(cwd, 1, "verify", "--json").stdout)
        assert summary["counts"]["fail"] > 0
        reports.append((cwd / "report.json").read_bytes())
    assert reports[0] == reports[1]
    print("[PASS] trust: unchanged failing runs cannot become cached false-clean verdicts")

    source.write_text('pub fn divided(x: i8) -> i8 { if x >= -100 && x <= 100 { x / -2 + 1 } else { 0 } }\n'
                      'pub fn remainder(x: i8) -> i8 { x % -3 + 1 }\n'
                      'fn main() { println!("{} {}", divided(-7), remainder(-7)); }\n')
    summary = json.loads(run(cwd, 0, "verify", "--json", "--strict-coverage").stdout)
    assert summary["findings"] == []
    assert summary["coverage"]["passed"] > 0

    # The obligations manifest (the last open row of "Reporting and artifacts").
    # `report.json` is a list of things to act on, so a *proved* assertion is only
    # addressable if something lists it — and the index is only worth reading if it
    # agrees with the files it names. Three things are asserted: a crate that left
    # obligations has a manifest, its entries point at files that exist, and each
    # entry's recorded verdict and kind are the ones the artifact's own header
    # carries. The last two are what make it an index rather than a second claim.
    # (Collecting the records as an `Option` once blanked the whole manifest whenever
    # any event left no file, which is why the non-empty half is asserted at all.)
    indexes = list((cwd / "obligations").rglob("index.json"))
    assert indexes, "a run that proved obligations wrote no obligations manifest"
    index = json.loads(indexes[0].read_text())
    assert index["schema_version"] == 1
    assert index["obligations"], index
    for entry in index["obligations"]:
        artifact = indexes[0].parent / entry["artifact"]
        assert artifact.is_file(), artifact
        header = artifact.read_text(encoding="utf-8")
        assert f"; verdict:  {entry['verdict']}" in header, (entry, header[:200])
        assert f"; kind:     {entry['kind']}" in header, (entry, header[:200])
    assert any(e["status"] == "PASS" for e in index["obligations"]), index
    print("[PASS] trust: division/remainder derived arithmetic is proved end-to-end, and the "
          "obligations manifest lists every proof and agrees with the files it names")

    source.write_text('pub fn add(x: u8) -> u8 { x + 1 }\nfn main() { println!("{}", add(255)); }\n')
    run(cwd, 1, "verify")

    obligation = next((cwd / "obligations").rglob("*.witness.smt2"))
    assert "encoded model" in run(cwd, 0, "explain", str(obligation)).stdout
    assert "sat" in run(cwd, 0, "replay", str(obligation)).stdout
    bad = cwd / "mismatch.smt2"
    bad.write_text('; keller-verify proof obligation\n; verdict:  FAIL (sat)\n(assert false)\n(check-sat)\n')
    run(cwd, 1, "replay", str(bad))
    bad.write_text('; keller-verify proof obligation\n; verdict:  FAIL (sat)\n(assert invalid)\n(check-sat)\n')
    run(cwd, 3, "replay", str(bad))
    print("[PASS] trust: explain/replay verify witnesses and reject mismatches and invalid SMT")

    source.write_text('fn main() { let p = 0x12345usize as *const i32; unsafe { println!("{}", *p); } }\n')
    run(cwd, 0, "verify", "--write-baseline", "baseline.json")
    findings = json.loads((cwd / "report.json").read_text())
    assert any(e["kind"] == "unsupported-assertion" for e in findings)
    assert not any(e["status"] == "FAIL" for e in findings)
    run(cwd, 0, "verify", "--baseline", "baseline.json")
    strict_baseline = run(cwd, 1, "verify", "--strict-coverage", "--baseline", "baseline.json")
    assert "--strict-coverage was given" in strict_baseline.stderr
    strict_summary = json.loads(run(cwd, 1, "verify", "--strict-coverage", "--json").stdout)
    assert strict_summary["counts"]["warn"] >= 1
    strict_record = json.loads((cwd / "report.run.json").read_text())
    assert strict_record["gate"]["strict_coverage"] is True
    assert strict_record["gate"]["status"] == "gate-failed"
    print("[PASS] trust: unsupported pointer assertions WARN and cannot be baselined out of strict coverage")

    (cwd / "report.json").unlink()
    (cwd / "report.json").mkdir()
    proc = run(cwd, 3, "verify")
    assert "compatibility publication failed" in proc.stderr
    assert "Evidence retained" in proc.stderr
    (cwd / "report.json").rmdir()
    if (cwd / "report.run.json").exists():
        (cwd / "report.run.json").unlink()
    (cwd / "report.run.json").mkdir()
    run(cwd, 3, "verify")
    print("[PASS] trust: incompatible publication paths fail with setup exit 3 and retain evidence")
    (cwd / "report.run.json").rmdir()
    source.write_text('#[cfg(not(trust_flag))] compile_error!("compiler flags were lost");\nfn main() {}\n')
    env = os.environ.copy()
    env.pop("CARGO_ENCODED_RUSTFLAGS", None)
    env["RUSTFLAGS"] = "--cfg trust_flag"
    run(cwd, 0, "verify", "--strict-coverage", env=env)
    compiler = json.loads((cwd / "report.run.json").read_text())["coverage"]["compilers"][0]
    assert "trust_flag" in compiler["arguments"]
    print("[PASS] trust: fresh analysis preserves and records caller compiler flags")

workspace = ROOT / "tests/workspace_multi"
summary = json.loads(run(workspace, 1, "verify", "--json").stdout)
# Five members, three of them clean. The count is asserted against the workspace
# manifest rather than written here, so adding a member to the fixture cannot leave
# this check asserting a number that stopped being true (D9).
members = sorted(
    name.strip().strip('"')
    for name in (workspace / "Cargo.toml").read_text().split("members = [")[1].split("]")[0].split(",")
)
assert len(members) == 5, members
assert summary["coverage"]["workers"] == len(members), summary["coverage"]["workers"]
# Every member was compiled by its own worker, and the three clean ones did not
# erase the two with findings: the same two crates, the same two findings, whatever
# order the workers finished in.
record = json.loads((workspace / "report.run.json").read_text())
workers = record["coverage"]["compilers"]
assert len(workers) == len(members), len(workers)
crates = sorted(w["arguments"][w["arguments"].index("--crate-name") + 1] for w in workers)
assert crates == members, crates
assert sorted({e["crate"] for e in summary["findings"]}) == ["alpha", "beta"]
print(f"[PASS] trust: {len(members)} workspace members each report once, and the merge keeps exactly the two findings")
