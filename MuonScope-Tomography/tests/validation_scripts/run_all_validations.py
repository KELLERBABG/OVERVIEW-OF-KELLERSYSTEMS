import os
import subprocess
import sys
import time

def run_step(name: str, cmd: list, cwd: str = ".") -> bool:
    print(f"\n[RUNNING] {name}...")
    t0 = time.time()
    res = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    dt = time.time() - t0
    if res.returncode == 0:
        print(f"[PASSED]  {name} ({dt:.2f}s)")
        return True
    else:
        print(f"[FAILED]  {name} ({dt:.2f}s)")
        print("--- STDERR ---")
        print(res.stderr)
        print("--- STDOUT ---")
        print(res.stdout)
        return False

def main():
    print("=" * 65)
    print("  MUONSCOPE COMPREHENSIVE VALIDATION & TEST SUITE")
    print("=" * 65)

    root_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    python_exe = sys.executable

    results = []

    # 1. Python Physics, Reconstruction & System Tests
    p1 = run_step(
        "Python Inversion, Physics & System Test Suite (pytest)",
        [python_exe, "-m", "pytest", "tests", "-q"],
        cwd=os.path.join(root_dir, "src", "inversion")
    )
    results.append(("Python Unit & Integration Suite", p1))

    # 2. Instrument software audit: bytes in, density map out, every number checked
    p2 = run_step(
        "MuonScope System Audit (wire -> calibration -> tracking -> reconstruction)",
        [python_exe, os.path.join("tests", "validation_scripts", "automated_system_audit.py")],
        cwd=root_dir
    )
    results.append(("System Audit (real gates)", p2))

    # 3. ISO/ASTM Calibration Rig
    p3 = run_step(
        "ISO/ASTM E1441 Reference Phantom Benchmark",
        [python_exe, "benchmarks/ground_truth_rig.py"],
        cwd=root_dir
    )
    results.append(("ASTM E1441 Calibration Rig", p3))

    # 4. Legacy accuracy harness: reconstruction against the phantom it came from
    p4 = run_step(
        "Accuracy Harness (CNR / RMSE against phantom truth)",
        [python_exe, "benchmarks/accuracy_harness.py", "--events", "20000", "--grid", "16"],
        cwd=root_dir
    )
    results.append(("Accuracy Harness", p4))

    # 5. Absolute-units gate: units bugs are invisible to CNR and must be gated
    p5 = run_step(
        "Accuracy Gate (absolute cm^-1, no momentum oracle)",
        [python_exe, os.path.join("tests", "validation_scripts", "accuracy_gate.py")],
        cwd=root_dir
    )
    results.append(("Accuracy Gate (absolute units)", p5))

    # 6. End-to-End Simulation & Reconstruction Pipeline
    p6 = run_step(
        "End-to-End MAP-EM & BIM Export Pipeline",
        [python_exe, "src/inversion/run_pipeline.py", "--events", "1000", "--grid-size", "32",
         "--iterations", "10", "--skip-exports"],
        cwd=root_dir
    )
    results.append(("End-to-End Pipeline & Exporters", p6))

    # 7. Full instrument run from simulator bytes
    p7 = run_step(
        "Instrument Run (run_system.py, simulator source)",
        [python_exe, "src/inversion/run_system.py", "--source", "sim",
         "--events", "4000", "--grid", "16", "--iterations", "5"],
        cwd=root_dir
    )
    results.append(("Instrument Run (simulator source)", p7))

    # 8. Contract documents must be regenerable from the code. The pytest suite
    #    asserts they match; this proves the generator still runs.
    p8 = run_step(
        "Contract Document Generation (API + wire protocol)",
        [python_exe, os.path.join("tools", "generate_contract_docs.py")],
        cwd=root_dir
    )
    results.append(("Contract Documents Regenerate", p8))

    # Summary
    print("\n" + "=" * 65)
    print("  VALIDATION SUMMARY REPORT")
    print("=" * 65)
    all_passed = True
    for name, passed in results:
        status = "PASSED" if passed else "FAILED"
        print(f"  {name:<45} [{status}]")
        if not passed:
            all_passed = False
    print("=" * 65)

    sys.exit(0 if all_passed else 1)

if __name__ == "__main__":
    main()
