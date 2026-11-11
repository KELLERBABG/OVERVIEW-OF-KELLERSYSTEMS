# Release, support and assurance baseline

**This file is the only place the support matrix lives.** What platform, toolchain and solver are
supported; which assertions are checked; what each gate promises; and what a passing run does and
does not prove. Other documents link here rather than restating any of it, because a matrix with
several owners drifts and a reader cannot tell which copy is current.

Nothing here is a claim that the analyzer is sound, complete, or that a clean exit is an all-clear.
It is the precise boundary of what has been tested, stated so it can be quoted.

## Supported baseline

The only clean-machine install and execution combination currently supported by automation:

| Area | Supported value |
|---|---|
| OS / host | Ubuntu 24.04, x86_64, GNU/Linux, glibc 2.39 |
| Rust | `nightly-2026-07-24`, host `x86_64-unknown-linux-gnu`, with `rustc-dev` and `llvm-tools-preview` |
| Z3 | Exactly 4.16.0, official `z3-4.16.0-x64-glibc-2.39` archive; its SHA-256 is checked before extraction |
| Analysis mode | The verifier runs `cargo check` with debug assertions; it is not a release-profile overflow guarantee |
| Distribution | Source build, or a tagged release archive produced by `.github/workflows/release.yml` (executables, bundled Z3, launcher). No static binary |
| Compiler identity | The build embeds the exact rustc release and commit; a run refuses a different one |

A release archive is **not** a statically linked executable. The verifier reads MIR through
`rustc_private`, so it is linked against one exact nightly and loads that toolchain's driver libraries
at run time. The archive therefore bundles the executable, the Z3 library it was built against, and a
launcher that resolves the pinned toolchain through rustup and puts the driver libraries on the loader
path. The target machine still needs the pinned Rust toolchain with `rustc-dev` and
`llvm-tools-preview`; a machine without it cannot run the verifier, and the launcher says which
`rustup toolchain install` line fixes that.

Windows and macOS are development-tested, not supported; other Linux distributions or architectures,
other Rust toolchains, other Z3 versions, cross-compilation and release-profile analysis are outside
this baseline until clean-machine tests cover them.

### What would make macOS supported

`verify.yml` now runs the fixture suite on macOS (`macos-fixtures`, `macos-13`, x86_64), so the
platform is exercised by CI instead of only by hand. That job is not a support claim, and no run of
it has been observed from this checkout. Moving macOS into the table above needs all of:

- **An observed green run** of `macos-fixtures`, enough of them that a red job means a regression
  rather than an untested setup. The job is in the tree; that is the whole of what is known here.
- **An install path.** `scripts/install-linux.sh` is Linux-only and verifies the pinned Linux Z3
  digest. macOS needs its own installer, with a pinned digest for the `*x64-osx*.zip` archive —
  today that archive's hash is only recorded in the release manifest and checksums, not checked
  before extraction.
- **A packaged archive built and smoke-tested on macOS** through `release.yml`. It has a macOS
  matrix row (`macos-13`, `x86_64-apple-darwin`), but no run of that workflow has been observed.
- **An architecture decision.** The job, the archive and this baseline are all x86_64. Apple silicon
  is untested: supporting it means either declaring Rosetta the supported path or adding
  `aarch64-apple-darwin` with its own toolchain, Z3 archive and matrix row.
- **An OS floor.** `macos-13` is the tested image; older releases are unverified, and this file
  would have to name the floor rather than imply "macOS".
- **A clean-machine install check** for macOS, mirroring the `clean-install` job: a fixture run on a
  prepared runner is not the path a new user takes.

The version this baseline describes is `1.0.0`, which is what the `v1.0.0` git tag means: a source
build at that version fixing the five assertion kinds below. The release workflow above can attach
archives to a `v*` tag, but no run of that workflow has been observed from this checkout, so this file
does not claim that binaries for `v1.0.0` exist.

### Toolchain selection

`keller verify` drives `cargo check` with the compiler this verifier was *built against*, not the
toolchain the project under verification pins. `cargo` is a rustup shim and the shim follows
`rust-toolchain.toml`, so the verifier sets `RUSTUP_TOOLCHAIN` to its own toolchain when it spawns
`cargo`; a project pinned to `stable` is still asked its question by the pinned nightly, and its own
pin keeps governing its editor, tests and builds. Before `cargo` starts, the run checks that this
toolchain is actually installed: if it is not, the command exits `3` naming the toolchain and the
`rustup toolchain install` line that fixes it, rather than letting rustc report a version mismatch on
a crate that compiles perfectly well.

A channel name is not enough, because `rustup` can update the contents under the same name. The build
embeds the `release`, `commit-hash` and `host` that `build.rs` read from the compiler it linked
against, and the run compares them with the installed toolchain before starting. A different commit is
refused with exit `3` and both identities in the message. Supporting a new nightly is therefore an
explicit change (`rust-toolchain.toml`, rebuild, the embedded identity moves with the code); retiring
one is the reverse. This is the compiler-version adapter boundary (M3): it names and checks
the `rustc_private` dependency rather than abstracting every MIR API. The MIR lowering itself is still
written directly against `rustc_private`.

### Release artifacts

`scripts/package-release.sh` (Linux/macOS) and `scripts/package-release.ps1` (Windows) stage an
archive containing:

```text
keller-verify-<version>-<target>/
  bin/keller-verify         the verifier
  bin/keller-env            std-only environment reporter
  bin/cargo-keller          the `cargo keller` entry point
  bin/keller[.cmd]          launcher resolving the pinned toolchain
  z3/bin, z3/include        the bundled Z3 the binary linked against
  docs/RELEASE.md           this file
  manifest.json             version, target, rustc commit, Z3 version and digest
```

The release workflow runs those scripts on Ubuntu, Windows (GNU) and macOS, smoke-tests the staged
launcher with `--version`, and attaches the archives plus a combined `SHA256SUMS` to the tag. Both
packagers resolve the toolchain `cargo` actually used — the active toolchain in the repository — and
refuse to package when it is not the pinned channel; a bare channel name can select a different host
when several are installed. The Windows launcher has the target triple embedded instead of parsing
`rustc -vV`, so it cannot follow a different host's driver directory by accident. The checksums are
produced by the workflow; there is no signature, SBOM, reproducible-build claim, or clean-machine
install test for a *prebuilt* archive yet. The workflow itself has not been observed running from the
development checkout, so the first manual dispatch is the test.

## Install from a clean checkout

The install script is deliberately source-based and non-root. It downloads the pinned Z3 archive,
verifies its published digest, installs the pinned Rust components, builds with the committed
`Cargo.lock` in a temporary Cargo target, and installs a launcher plus the executable under
`~/.local` by default. It only supports the baseline above.

On Ubuntu 24.04, install the build prerequisites if needed (`git`, `curl`, `unzip`, `clang`,
`libclang-dev`, and `rustup`), then:

```bash
sudo apt-get update
sudo apt-get install -y build-essential clang libclang-dev curl unzip git
```

Install `rustup` from [rustup.rs](https://rustup.rs/) if it is not already present; the installer
adds the pinned toolchain and components but does not install `rustup` itself.

```bash
git clone https://github.com/KELLERBABG/Formal-Verification.git
cd Formal-Verification
bash scripts/install-linux.sh
```

The script checks out no code and changes no system-wide settings. It installs Z3 under
`~/.local/share/keller-verify/`, the executable under `~/.local/lib/keller-verify/`, and the
`keller-verify` launcher under `~/.local/bin/`. Add `~/.local/bin` to `PATH` if it is not already
there. To use a different prefix, set `KELLER_PREFIX=/absolute/path` before running the script.
Re-running the script updates this tool's executable and launcher; a matching Z3 installation is
reused only when its pinned digest marker, version, and required files match. An existing but
unverified Z3 directory is refused rather than deleted; move it aside explicitly before retrying.

The launcher adds the pinned rustc driver libraries and Z3 library directory to the runtime search
paths and selects the same pinned nightly for Cargo in the project being checked. It does not edit
the checked-out project or its `rust-toolchain.toml`. Invoked with no arguments it prints Keller's
usage; it never falls through to rustc's.

## What is checked, and what is not modelled

The assertions that are turned into obligations, by the compiler's own `AssertKind`:

| Checked | Runtime meaning in a debug build |
|---|---|
| `BoundsCheck` | A slice, array or string index leaves the bounds of the value |
| `Overflow(Add/Sub/Mul)` | An arithmetic operation overflows its integer type |
| `OverflowNeg` | Negating an integer overflows (the signed minimum has no positive counterpart) |
| `DivisionByZero` | An integer division divides by zero |
| `RemainderByZero` | An integer remainder divides by zero |
| `ResumedAfterReturn` / `ResumedAfterPanic` | An `async fn`'s coroutine is resumed after it returned or panicked |
| `NullPointerDereference` / `NullReferenceConstructed` / `MisalignedPointerDereference` | A dereference of an address that is null, or not a multiple of the type's alignment |

The pointer checks are decided from the *origin* of the address, not from its value. `rustc` emits
them as one integer: the address of a place, masked by `ALIGN - 1` and compared with zero, or
compared with zero for null. When that address comes from a `&T`, a `Box<T>`, a `NonNull<T>` or a
`Unique<T>` — including a field of one, a `ptr as usize` conversion of one, or a raw pointer cast
from one — then it is non-null and a multiple of `align_of(T)` by construction, and both checks
are proved. The model follows a MIR temp through its definitions, accepts a temp reused by
several sites only when every definition agrees, and never uses a value it cannot so derive: a
raw pointer from an `extern` call, an offset address, or any arithmetic over the address stays a
`WARN`. It also only *proves* — an address whose origin is known but which the model cannot show
to be valid stays unsupported rather than being called a counterexample, because a `FAIL` has to
carry a ground obligation and a witness.

Any other assertion kind the compiler emits is reported as an `unsupported-assertion` **`WARN`**
naming the kind — never skipped silently, because an assertion that was not checked is not an
assertion that holds. The kinds encountered in a run are listed by name on the console coverage
line and in the `--json` summary at `coverage_summary.unsupported_kinds`.

`ResumedAfterReturn` and `ResumedAfterPanic` are decided under the coroutine ABI rather than from the body alone: a resume
arrives with state `0` (not started) or a suspension state, never `1` (returned) or `2` (poisoned),
so the branches the transform dispatches those two states to are unreachable and the assertions in
them are `PASS` (`kind: "coroutine-resume"`). The argument is made from the CFG — every edge into
the block selects a forbidden state — so it does not depend on the body reaching its fixpoint. That
invariant is also the boundary: code that resumes a completed coroutine itself, by polling a finished
`async fn` again, can still reach the panic, and that path is exactly what this mode assumes away.

`ResumedAfterDrop` is not decided by that argument. With `async_drop` in scope the transform reaches
it through the coroutine's *suspension* states, which a resume may legitimately arrive in, so the
resume-entry invariant says nothing about those blocks. `tests/async_drop_state` is the fixture that
showed this — its async-drop guards were reported as counterexamples when the three kinds were treated
alike. The kind is therefore reported as an `unsupported-assertion` **`WARN`** naming it, never as a
`PASS` and never as a `FAIL`: named and counted, not guessed at. A guard that can never fall through —
`assert(const false, …)`, which is how the transform spells those two blocks — is a terminal in
the walk rather than a self-edge, so the resume dispatch settles inside the round budget instead
of collapsing the whole body to the path-insensitive state.

Panic call sites are not `AssertKind`s, but two are decided the same way: a site reachable only under
an unsatisfiable guard is a ground `PASS`, and an `unwrap`/`expect` whose receiver's enum variant is
known is a `PASS` or a ground `FAIL` (`kind: "decided-panic"`, `defect_class: "known-panic"`). A site
whose variant is not known stays an `unmodelled-panic` WARN counted in `coverage.panic_sites`; the
decided ones are counted in `coverage.panic_sites_decided`.

A private, cycle-free helper too small to be a unit of its own is checked against its callers rather
than against every possible argument (S7). When its body bounds a parameter by a constant — an addition
or a subtraction — that bound is derived (`x + 1` on `u8` implies `x <= 254`), and it is assumed inside
the body **only after every call site in this crate was shown to pass a constant satisfying it**: the
proof of the body's check then carries the bound as its reachability guard, and each call site is a
`helper-precondition` `PASS`. An argument the analysis does not bound, a `pub` helper whose callers are
not all in the crate, or a `fn` item whose address is taken abandons the summary, and the helper is then
reported on its own body exactly as before — so an `unguarded` finding for a helper is not, on its own,
evidence that a caller violates it.

Operations that are outside the encoding even when an assertion exists:

- The unchecked operators (`unchecked_add` and friends) are not modelled: they carry validity
  requirements rather than a defined result, so wrapping them would invent semantics for UB.
  `PtrMetadata` is modelled only where it is a slice length on a place the body does not write.
  Everything else with a defined result is modelled — bit-twiddling (`&`, `|`, `^`), shifts,
  `wrapping_*`, `saturating_*`, `checked_*` and `as` narrowing casts. A value that is unmodelled —
  one from an unmodelled call, or from an operator above — becomes a free symbol bounded by its
  type, so downstream checks are sound but pessimistic.
- Division and remainder are checked on the **divisor** and on `MIN / -1`; the quotient itself is
  not propagated, so a value derived from `/` becomes a free symbol as above.
- Loop-carried values are widened past a bounded number of trips (`MAX_LOOP_TRIPS`): the guards
  carry a proof across a loop, not the values.
- Only the assertions the compiler emits are checked, plus `unwrap`/`expect` on a receiver whose enum
  variant is known. Explicit `panic!`, slice indexing on types that emit no assertion, allocation
  failure, an `unwrap`/`expect` with an unknown variant and custom invariants are not modelled at all,
  so absence of findings is not absence of panics.
- The analysis is of the **debug** profile, because that is where rustc emits these assertions.

## Gate profiles and exit codes

`--profile` names the contract; the individual flags remain available and anything given after a
profile still wins, so a profile is a starting point rather than a mode that ignores what is typed
next to it.

| Profile | Equivalent to | Promise |
|---|---|---|
| `advisory` (default) | neither flag | Fails on a counterexample. A `WARN` — an undecided verdict or a coverage gap — is recorded and tolerated. **Exit `0` here is not an all-clear.** |
| `strict` | `--deny-warn --strict-coverage` | Fails on a counterexample, on an encountered unsupported or undecided assertion, and on a losing path-sensitivity. Nothing was left unanswered by design. |

`--strict-coverage` is the flag that cannot be laundered by a baseline: it fails with exit `1` on
any encountered unsupported or undecided assertion, including unsupported `WARN`s listed in a
baseline. `--deny-warn` is what makes an undecided *solver* verdict fatal.

| Exit code | Meaning |
|---|---|
| `0` | Analysis completed and the selected gate passed; advisory WARNs may remain |
| `1` | The gate failed: a counterexample, or a WARN under `--deny-warn`/`--strict-coverage` |
| `2` | The command line was not understood, including an unreadable or unknown-version baseline |
| `3` | Setup, analysis or evidence failure: compiler or Cargo failed, worker receipts are incomplete or corrupt, a required artifact could not be written, or the run was stopped by `--run-timeout` (a stopped run reaches no verdict, which is neither a `0` nor a `1`) |

Exit `3` exists because "your code does not compile" and "your code has a counterexample" send the
reader to different places, and both used to be reported as `1`.

```bash
keller-verify keller verify --profile strict
keller-verify keller verify --strict-coverage --baseline .keller-baseline.json
```

The installed-runner smoke test pins both outcomes: a `vec!` pointer-assertion WARN is tolerated by
the default gate and rejected by the strict one.

## What a passing run proves, and what it does not

A `PASS` is a solver proof that the encoded obligation's negation is unsatisfiable under the guards
that reach it. That is a statement about the encoded model of a debug build, and it is bounded by:

- **Coverage.** Only encountered assertions of the five kinds above were decided. The coverage line
  states the proportion that was: `decided` of `encountered`, the unsupported kinds by name, and the
  count left undecided.
- **Model fidelity.** A SAT witness satisfies the model; widening and unmodelled operations can make
  it impossible to reproduce as an execution. Replay validates the SMT, not the MIR-to-Rust
  translation. The MIR-to-SMT encoder is not independently verified.
- **Abstraction.** Loop-carried values are widened, so a proof across a loop rests on its guards.
- **Profile.** Nothing here says a release build wraps instead of panicking, or that some
  uninstrumented panic path does not exist.
- **The solver's identity.** The run records the solver library this process loaded — the version
  asked of the loaded library through `Z3_get_full_version`, and the module path the loader
  resolved where the platform exposes it (`provenance.solver`) — beside the discovered replay
  executable. That is identity, not attestation: nothing signs the library, so it says which solver
  answered, not that the answer came from an uncompromised build. The compiler this binary was built
  with and a digest of the effective settings are recorded with it (`provenance.build`,
  `provenance.config_hash`).

Every verdict writes the obligation it was decided from under `obligations/<crate>/`, so a third
party can re-discharge it rather than trust this binary: a `FAIL` writes `FAIL (sat)` or
`FAIL (folded)` plus a pinned `*.witness.smt2` that must answer `sat`, a `PASS` writes
`PASS (unsat)` and must answer `unsat`, and a `WARN` writes `WARN (unknown)`. Each header carries
the crate, function, block, obligation kind, `file:line`, the report's `details` line, the verdict
and the `; reachable:` guard. Review `report.run.json` coverage and the documented
[model boundaries](USAGE.md#reportrunjson-completion-and-coverage) alongside the exit status: an
exit `0` without a strict profile is advisory when WARNs are present, and even a strict exit `0` is
limited to the observed coverage and the current model.

## Clean-machine release/install check

The `clean-install` CI job runs on a fresh Ubuntu 24.04 runner without Cargo or Z3 caches. It
invokes `scripts/install-linux.sh`, then `tests/release_install_smoke.py` through the installed
launcher from temporary projects outside the checkout. The smoke test checks that the launcher with
no arguments prints Keller usage (not rustc's help), the strict clean path, and that an unsupported
`vec!` assertion is tolerated by the default gate but rejected by the strict gate. This is a
repeatable source-install check; it is not a binary release, signature, SBOM, or byte-for-byte
reproducible-build claim.

The `release` workflow is the separate prebuilt-binary path: it packages and checksums archives for
Linux x86_64, Windows x86_64 GNU and macOS x86_64, and publishes them on a `v*` tag. It has no
clean-machine test of the archive itself, and no run of it has been observed from this checkout.
Neither path has been exercised on Windows or macOS CI, and the macOS fixture job now in
`verify.yml` runs the suite rather than the installer. Treat the source-install job as the tested one
and the release workflow as an unverified first dispatch.
