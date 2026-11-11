"""Command-line interface for the Herzschrittmacher research toolchain."""

from __future__ import annotations

import argparse
import json
import sys


def _run_demo() -> int:
    # Imported lazily so `--help` works without a solver installed.
    from .execution_gate import ExecutionGate
    from .modes import CHAMBER_FROM_STRING, Chamber, PacingMode
    from .pacing import PacingSystem
    from .verification import VerificationEngine

    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()

    # Atrial sense at t=200ms → starts AV timer.
    d1 = pace.sense(Chamber.ATRIUM, 200)

    # Ventricular escape check at t=200ms (should be inhibited: VA interval).
    d2 = pace.check_escape(Chamber.VENTRICLE, 200)

    # Ventricular escape check after AV delay (t=380ms → triggered pace).
    d3 = pace.check_escape(Chamber.VENTRICLE, 380)

    # Attempt an actual ventricular pulse at t=380ms with nominal output.
    engine = VerificationEngine()
    gate = ExecutionGate(engine)
    result = gate.release(d3, Chamber.VENTRICLE, amplitude_ma=3.5, pulse_width_ms=0.5)

    print("=== Herzschrittmacher demo (DDD pacing, formal gate) ===")
    print()
    print("1) Atrial sense @200ms        ->", d1.action, "|", d1.reason)
    print("2) Ventricular check @200ms   ->", d2.action, "|", d2.reason)
    print("3) Ventricular check @380ms   ->", d3.action, "|", d3.reason)
    print()
    print("Gate verdict for V-pulse @380ms (3.5mA, 0.5ms):")
    print("   released :", result.released)
    print("   reason   :", result.reason)
    if result.verification is not None:
        print("   checks   :", json.dumps(result.verification.checks))
        print("   runtime  : %.3f ms" % result.verification.runtime_ms)
    print()
    print("Events:", [(e.chamber.value, e.time_ms, e.kind) for e in pace.events])
    print()
    print("Demo complete. This is a research prototype — NOT a medical device.")
    return 0


def _run_verify(args: argparse.Namespace) -> int:
    from .constraints import ConstraintGenerator
    from .execution_gate import ExecutionGate
    from .modes import CHAMBER_FROM_STRING, PacingMode
    from .pacing import PacingSystem
    from .verification import VerificationEngine

    data = json.load(sys.stdin)
    chamber_str = data.get("chamber", "ventricle")
    amplitude = float(data.get("amplitude_ma", 3.5))
    pulse_width = float(data.get("pulse_width_ms", 0.5))
    time_ms = int(data.get("time_ms", 1000))

    chamber = CHAMBER_FROM_STRING[chamber_str]
    pace = PacingSystem(mode=PacingMode.DDD)
    pace.start()
    try:
        decision = pace.check_escape(chamber, time_ms)
    except ValueError as exc:
        print(json.dumps({"error": str(exc)}, indent=2))
        return 1

    gate = ExecutionGate(VerificationEngine())
    result = gate.release(decision, chamber, amplitude, pulse_width)

    print(json.dumps(result.as_dict(), indent=2))
    return 0 if result.released else 2


def _run_verify_symbolic(args: argparse.Namespace) -> int:
    from .modes import Chamber
    from .verification import VerificationEngine

    result = VerificationEngine().verify_symbolic_energy_envelope(
        chamber=Chamber.VENTRICLE,
        amplitude_range=(0.5, args.max_amplitude_ma),
        width_range=(0.1, args.max_width_ms),
    )
    print(json.dumps(result, indent=2, default=str))
    return 0


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="herzschrittmacher",
        description=(
            "Herzschrittmacher — formally verified pacemaker pacing "
            "research toolchain (Z3 SMT). NOT a medical device."
        ),
    )
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("demo", help="run the DDD pacing demo with the formal gate")

    p_verify = sub.add_parser(
        "verify", help="verify one pacing decision from JSON on stdin"
    )
    p_verify.set_defaults(func=_run_verify)

    p_sym = sub.add_parser(
        "verify-symbolic",
        help="search the output envelope for an energy-invariant violation",
    )
    p_sym.add_argument("--max-amplitude-ma", type=float, default=8.0)
    p_sym.add_argument("--max-width-ms", type=float, default=2.0)
    p_sym.set_defaults(func=_run_verify_symbolic)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)

    if args.command == "demo":
        return _run_demo()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())