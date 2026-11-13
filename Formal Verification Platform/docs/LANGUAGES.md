# Multi-language frontends (experimental)

> **Status:** The eight language engines — TypeScript/JavaScript, SQL, C, C++, Assembly, Go, Java, and C# — are **hand-written static scanners**. They are feature-gated (`lang-c`, `lang-cpp`, `lang-asm`, `lang-go`, `lang-java`, `lang-csharp`, `lang-ts`, `lang-sql`; aggregate: `all-langs`), **OFF in the default build**, and are **NOT part of `keller verify`** or its formal SMT path. They perform **heuristic analysis only**: no Keller-VIR lowering, no SMT obligation discharge, no native test generation, and no CI coverage today. Their results are **advisory, not proofs**.

## Build

The language frontends are excluded from the default build. Enable them with Cargo features:

```bash
# All eight engines
cargo build --features all-langs

# A single engine (example)
cargo build --features lang-c
```

After building, the `keller-verify` binary gains the `audit` subcommand for each enabled language.

## Usage

```text
keller-verify audit --lang <c|cpp|asm|go|java|csharp|ts|sql> <path> [--out report.json]
```

| Argument / flag | Meaning |
|---|---|
| `--lang` | One of the eight language IDs: `c`, `cpp`, `asm`, `go`, `java`, `csharp`, `ts`, `sql`. |
| `<path>` | A source file or directory to scan. Directories are walked recursively; VCS and build directories (`.git`, `node_modules`, `target`, `build`, `dist`, `bin`, `obj`, `.venv`, `vendor`) are skipped. |
| `--out` | Optional path for the JSON report. Must not lie inside the scanned tree. |

### Exit codes

| Code | Meaning |
|---|---|
| `0` | Audit completed. Findings in the report are **data**, not verdicts of unsoundness. |
| `2` | The audit could not run (unknown language, missing feature, unreadable path, or output path inside the scanned tree). |

### Report format

The report is a `LangAudit` JSON object with these fields:

| Field | Type | Description |
|---|---|---|
| `language` | string | The language ID that was audited. |
| `root` | string | Path that was scanned. |
| `files_scanned` | number | Count of source files the frontend attempted to parse. |
| `files_unparsed` | number | Files the frontend could not parse. Reported, never silently dropped. |
| `functions_lowered` | number | Functions the scanner recognized and analyzed. |
| `obligations_total` | number | Total heuristic checks emitted. |
| `obligations_proved_safe` | number | Heuristic checks that passed. **This counts heuristic pattern matches, not solver proofs.** |
| `findings` | array of `Finding` | One entry per defect or unproven obligation (see below). |
| `notes` | array of string | Honest limits of this run: unsupported constructs, partial parses, etc. |

Each `Finding` object contains:

| Field | Type | Description |
|---|---|---|
| `file` | string | Path of the file where the finding was located. |
| `line` | number | 1-based line number. |
| `class` | string | Stable defect class, e.g. `c-buffer-overflow`, `go-nil-deref`. |
| `verdict` | string | `proved-defect`, `possible-defect`, or `info`. These are the frontend's own labels; the engines are heuristic scanners and no solver runs, so none of them is a formal proof. |
| `message` | string | Human-readable description. |

> **Note:** `obligations_proved_safe` counts heuristic checks that matched a safe pattern. It does **not** indicate that a solver discharged an SMT obligation. No language frontend in this project performs SMT solving.

## Language support matrix

| ID | Fixture directory | Defect classes the scanner attempts (heuristic) |
|---|---|---|
| `c` | `tests/c_cases/` | `buffer-overflow`, `div-by-zero`, `null-deref`, `signed-overflow`, `use-after-free`, `double-free` |
| `cpp` | `tests/cpp_cases/` | `vector-oob`, `null-deref`, `div-by-zero`, `use-after-delete`, `iterator-invalidation`, `pure-virtual-call`, `double-delete` |
| `asm` | `tests/asm_cases/` | `stack-smash`, `stack-imbalance`, `div-by-zero`, `callee-saved-clobber`, `unaligned-simd` |
| `go` | `tests/go_cases/` | `div-by-zero`, `data-race`, `channel-deadlock`, `slice-oob`, `nil-deref` |
| `java` | `tests/java_cases/` | `npe`, `div-by-zero`, `array-oob`, `negative-array-size`, `class-cast` |
| `csharp` | `tests/csharp_cases/` | `null-reference`, `div-by-zero`, `index-out-of-range`, `span-oob`, `invalid-cast` |
| `ts` | `tests/typescript_cases/` | `null-undefined-deref`, `prototype-pollution`, `div-by-zero`, `array-oob`, `nan-propagation`, `type-error-call` |
| `sql` | `tests/sql_cases/` | `unguarded-update-delete`, `null-comparison`, `division-by-zero`, `not-null-violation`, `recursive-cte-unbounded` |

All defect classes are **heuristic pattern matches**. A finding with verdict `possible-defect` means the scanner recognized a suspicious pattern; it does not mean a bug is proven to exist.

## Running the smoke test

A local end-to-end audit smoke exercises all eight engines against their fixture directories and verifies that each produces a well-formed `LangAudit` report.

**Windows (PowerShell):**

```powershell
powershell -ExecutionPolicy Bypass -File tests/lang_audit_smoke.ps1
```

The smoke script requires the `all-langs` build:

```bash
cargo build --features all-langs
```

**CI:** The workflow `.github/workflows/languages.yml` runs the same eight audits on Linux in every pull request and push to the default branch. It builds with `--features all-langs`, invokes `keller-verify audit` for each language against its fixture directory, and fails if any engine exits with a non-zero code or produces a malformed report.

## Limitations and roadmap

- **Feature-gated off by default.** The language frontends are not compiled unless the corresponding `lang-*` feature (or `all-langs`) is enabled. A default `cargo build` produces a binary without the `audit` subcommand for these languages.
- **Not dispatched by `keller verify`.** The `keller verify` command operates exclusively on Rust MIR via the Keller-VIR lowering and SMT obligation pipeline. Language frontends are invoked only through the separate `keller-verify audit` subcommand.
- **No VIR / SMT / testgen.** None of the eight engines lower to Keller-VIR, discharge SMT obligations, or generate native tests. They are static pattern scanners.
- **Benchmark corpora and reports are not committed.** Large-scale benchmark inputs and their output reports are excluded from version control; see the development notes in the repository for how to regenerate them locally.
- **Authoritative support matrix.** The definitive statement of which languages, constructs, and defect classes are supported (and which are not) is maintained in `docs/RELEASE.md`. This document describes the current experimental state; `RELEASE.md` governs release commitments.

## Related documents

- [Release baseline](RELEASE.md)
- [Modular architecture plan](MODULAR-ARCHITECTURE.md)
- [Usage guide](USAGE.md)
