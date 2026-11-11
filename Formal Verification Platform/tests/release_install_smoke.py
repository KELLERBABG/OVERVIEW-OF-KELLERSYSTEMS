#!/usr/bin/env python3
"""Exercise the installed release launcher from projects outside the checkout."""
import json
import os
from pathlib import Path
import subprocess
import tempfile


LAUNCHER = Path(os.environ["KELLER_LAUNCHER"]).resolve()


# Exercise the generated installed launcher itself, not the build-tree binary.
no_args = subprocess.run(
    [str(LAUNCHER)],
    capture_output=True,
    text=True,
    encoding="utf-8",
    timeout=10,
)
if no_args.returncode != 0:
    raise SystemExit(
        f"installed launcher with no arguments exited {no_args.returncode}: "
        f"stdout={no_args.stdout!r} stderr={no_args.stderr!r}"
    )
if not no_args.stdout.startswith("Usage: keller verify") or "Usage: rustc" in no_args.stdout:
    raise SystemExit(
        "installed launcher with no arguments must print Keller usage, not rustc help; "
        f"got stdout={no_args.stdout!r} stderr={no_args.stderr!r}"
    )
print("[PASS] release install: no-argument launcher prints Keller usage")


def run(cwd: Path, expected: int, *args: str) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(
        [str(LAUNCHER), "keller", *args],
        cwd=cwd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=180,
    )
    assert proc.returncode == expected, (args, proc.returncode, proc.stdout, proc.stderr)
    return proc


def new_crate(root: Path, name: str, source: str) -> Path:
    crate = root / name
    (crate / "src").mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n'
    )
    (crate / "src/main.rs").write_text(source)
    return crate


with tempfile.TemporaryDirectory(prefix="keller-release-smoke-") as tmp:
    root = Path(tmp)
    clean = new_crate(
        root,
        "release_clean",
        "#[inline(never)] fn guarded_increment(x: u8) -> u8 { if x < 100 { x + 1 } else { x } }\n"
        "fn main() { println!(\"{}\", guarded_increment(std::hint::black_box(1))); }\n",
    )
    clean_run = run(clean, 0, "verify", "--strict-coverage", "--json")
    assert json.loads(clean_run.stdout)["counts"]["warn"] == 0
    clean_record = json.loads((clean / "report.run.json").read_text())
    assert clean_record["completed"] is True
    assert clean_record["gate"]["strict_coverage"] is True
    assert clean_record["coverage"]["reachable_assertions"] >= 1
    assert clean_record["coverage"]["supported_assertions"] >= 1
    assert clean_record["coverage"]["unsupported_assertions"] == 0
    print("[PASS] release install: strict coverage accepts an exercised clean project")

    gaps = new_crate(
        root,
        "release_coverage_gap",
        'fn main() { let p = 0x12345usize as *const i32; unsafe { println!("{}", *p); } }\n',
    )
    default = run(gaps, 0, "verify", "--json")
    summary = json.loads(default.stdout)
    assert summary["counts"]["warn"] >= 1
    assert any(item["kind"] == "unsupported-assertion" for item in summary["findings"])
    print("[PASS] release install: default gate reports unsupported checks as WARN")

    strict = run(gaps, 1, "verify", "--strict-coverage", "--json")
    strict_summary = json.loads(strict.stdout)
    assert strict_summary["counts"]["warn"] >= 1
    strict_record = json.loads((gaps / "report.run.json").read_text())
    assert strict_record["completed"] is True
    assert strict_record["gate"]["strict_coverage"] is True
    assert strict_record["gate"]["status"] == "gate-failed"
    assert strict_record["coverage"]["unsupported_assertions"] >= 1
    print("[PASS] release install: strict coverage rejects unsupported checks with gate exit 1")

    run(gaps, 0, "verify", "--write-baseline", "coverage-baseline.json")
    run(gaps, 1, "verify", "--strict-coverage", "--baseline", "coverage-baseline.json")
    baseline_record = json.loads((gaps / "report.run.json").read_text())
    assert baseline_record["gate"]["strict_coverage"] is True
    assert baseline_record["gate"]["status"] == "gate-failed"
    print("[PASS] release install: baseline cannot suppress strict-coverage gaps")
