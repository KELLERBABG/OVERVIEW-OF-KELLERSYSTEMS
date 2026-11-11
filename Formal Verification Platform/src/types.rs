use serde;

/// One concrete value Z3 assigned to a variable of a failing obligation.
///
/// A SAT verdict says a failing input exists; a verdict without the input is a
/// bug report nobody can act on. These values turn a FAIL into a specific input
/// a reader can paste back into the program.
///
/// Every entry is either an operand of the operation that would panic or a value
/// the obligation *pins* — the model's arbitrary choices are left out, so a
/// witness can be read as evidence rather than as the solver's mood (see
/// `verifier::value_is_pinned`).
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Witness {
    /// The source operand the value belongs to: a local's name plus its MIR
    /// local (e.g. `a (_1)`), a projection (e.g. `_3.0`), or a constant
    /// (e.g. `100_u8`).
    pub input: String,
    /// The Z3 symbol the value was read from (e.g. `overflow_l`). Pinning the
    /// value back into the obligation needs the symbol, not the label.
    pub symbol: String,
    /// The value Z3 chose, as an exact decimal string. A string rather than an
    /// integer so a 128-bit value survives the round-trip through JSON.
    ///
    /// Chosen among the values the obligation allows, which for a pinned symbol
    /// is exactly one — see the struct's own note.
    pub value: String,
}

/// A single verification event captured during MIR analysis.
///
/// `PartialEq`/`Eq` back the de-duplication in `report::merge_events`.
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct VerificationEvent {
    /// Which crate the event came from, serialised as `crate`.
    ///
    /// Each worker analyses exactly one crate, so this is stamped when the
    /// per-crate report is written. Without it a merged workspace report could
    /// not say whose finding each entry is. `default` keeps reports written
    /// before this field existed readable.
    #[serde(rename = "crate", default)]
    pub crate_name: String,
    pub function: String,
    pub block: String,
    pub status: String,
    /// Machine-readable class of the obligation this event is about:
    /// `bounds`, `overflow`, `overflow-neg`, `divzero` or `remainder` — or
    /// `path-insensitive`, which is not an obligation class at all: it says the
    /// function's dataflow fell back to the path-insensitive state, so the
    /// guards that lead to its obligations were missing and a `FAIL` among them
    /// may be a false positive. Kept in the same field as the obligation classes
    /// so one token identifies what a finding is, and distinct from them so
    /// nothing has to string-match prose to tell the two apart.
    ///
    /// `details` is prose for a person; anything that wants to group, route or
    /// de-duplicate findings needs a stable token instead of a substring match
    /// on that prose. `default` keeps reports written before the field existed
    /// readable.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
    /// Why this finding is a finding, in the terms a reader can act on
    /// (K6): `guard-limited`, `widening-limited`, `unguarded` or
    /// `constant-folded`. Empty for anything that is not a FAIL, and for reports
    /// written before the field existed.
    ///
    /// The `kind` above says *what* can go wrong; this says what stands between
    /// the verdict and the program. A `guard-limited` failure is stated under
    /// guards the analysis did express, so a stronger guard is what would settle
    /// it. A `widening-limited` one leans on a value the loop abstraction
    /// generalised, which is where a residual false positive would come from —
    /// the two send a reader to different places, which is the whole reason the
    /// field exists.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub defect_class: String,
    /// The source file the function was defined in, when the compiler gave a
    /// usable span for it. Omitted rather than guessed when it did not.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file: String,
    /// The line the function starts on. Needed by a reader, by SARIF annotations
    /// and by anything that wants to jump to the finding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub details: String,
    /// Concrete failing inputs: the values that trigger this FAIL.
    ///
    /// Filled in for every FAIL, whether the obligation was handed to the solver
    /// or decided by constant folding, so a consumer never has to parse `details`
    /// to find the input. Empty for PASS/WARN and for reports written before this
    /// field existed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub witness: Vec<Witness>,
    /// What kind of claim the `witness` above makes, when there is one
    /// (K10 / A14 residual).
    ///
    /// `pinned` means every reported value is one the obligation *forces*: the
    /// same run of the same check would print the same numbers, so the witness is
    /// a fact about the program. `one-of-many` means at least one entry is an
    /// operand the obligation only bounds — `a + b` overflowing names a pair the
    /// solver chose out of the range the guard allows — so re-running can print a
    /// different, equally valid instance. Without this a reader could not tell a
    /// deterministic witness from one of many, which is exactly the ambiguity the
    /// row was written about.
    ///
    /// Empty for anything that is not a FAIL with a witness, and for reports
    /// written before the field existed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub witness_kind: String,
}

/// Thread-safe accumulator for verification results.
#[derive(Default)]
pub struct SharedReport {
    pub events: Vec<VerificationEvent>,
    pub analysis_completed: bool,
    pub coverage: Coverage,
    pub compiler: Option<serde_json::Value>,
    pub evidence: Vec<FunctionEvidence>,
}

/// Conservative function-level disclosure, not precise per-value provenance.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionEvidence {
    pub function: String,
    pub disclosures: Vec<String>,
}

/// Counts describe the analysed workspace targets, not every possible panic.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub functions: usize,
    pub reachable_assertions: usize,
    pub supported_assertions: usize,
    pub unsupported_assertions: usize,
    /// The assert kinds that were skipped, by name, de-duplicated and sorted.
    ///
    /// A count says how much was not checked; only the names say what was not
    /// checked, and which one to model next. Recorded here rather than derived
    /// from the findings because a run with no findings at all still has coverage
    /// to state.
    #[serde(default)]
    pub unsupported_kinds: Vec<String>,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub without_verdict: usize,
    /// The call sites where the program can panic through a callee this analysis
    /// does not read: `unwrap`, `expect`, `panic!`, `unreachable!`, the failure
    /// path of a user `assert!`, and the allocation-failure abort.
    ///
    /// Counted apart from the assertions because a panic site is not one: it is
    /// the admission that the run says nothing about that path, which is what
    /// stops a report with no findings being read as "this cannot panic". Its
    /// events carry `kind = unmodelled-panic` and leave no obligation file.
    /// This is the *undecided* half; sites this analysis can settle are counted
    /// in [`Self::panic_sites_decided`] and leave this field.
    #[serde(default)]
    pub panic_sites: usize,
    /// The call sites above whose panic behaviour was decided rather than
    /// disclosed: a site no input reaches, or an `unwrap`-family call whose
    /// receiver variant is known.
    ///
    /// Kept beside [`Self::panic_sites`] rather than replacing it, because the
    /// two answer different questions: one says how much of the panic surface
    /// was proved, the other how much was admitted. A decided site that panics
    /// is a FAIL event with `kind = decided-panic`; a decided site that cannot
    /// panic leaves no event, only this count.
    #[serde(default)]
    pub panic_sites_decided: usize,
    /// Primary obligation queries; reachability and witness probes are excluded.
    pub obligation_solver_calls: usize,
    /// Total wall-clock time spent inside Z3 solver checks in milliseconds (D6).
    #[serde(default)]
    pub solver_elapsed_ms: u64,
}

impl Coverage {
    pub fn add(&mut self, other: &Self) {
        self.functions += other.functions;
        self.reachable_assertions += other.reachable_assertions;
        self.supported_assertions += other.supported_assertions;
        self.unsupported_assertions += other.unsupported_assertions;
        for kind in &other.unsupported_kinds {
            if !self.unsupported_kinds.contains(kind) {
                self.unsupported_kinds.push(kind.clone());
            }
        }
        self.unsupported_kinds.sort();
        self.passed += other.passed;
        self.failed += other.failed;
        self.warnings += other.warnings;
        self.without_verdict += other.without_verdict;
        self.panic_sites += other.panic_sites;
        self.panic_sites_decided += other.panic_sites_decided;
        self.obligation_solver_calls += other.obligation_solver_calls;
        self.solver_elapsed_ms += other.solver_elapsed_ms;
    }
}

/// Rust integer types and their bit widths for overflow checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntType {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    Isize,
}

impl IntType {
    /// True for unsigned integer types (min bound is 0).
    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            IntType::U8
                | IntType::U16
                | IntType::U32
                | IntType::U64
                | IntType::U128
                | IntType::Usize
        )
    }

    /// Bit width of this integer type (8, 16, 32, 64 or 128; pointer-width for usize/isize).
    pub fn bit_width(&self) -> u32 {
        match self {
            IntType::I8 | IntType::U8 => 8,
            IntType::I16 | IntType::U16 => 16,
            IntType::I32 | IntType::U32 => 32,
            IntType::I64 | IntType::U64 => 64,
            IntType::I128 | IntType::U128 => 128,
            IntType::Usize | IntType::Isize => {
                #[cfg(target_pointer_width = "64")]
                {
                    64
                }
                #[cfg(target_pointer_width = "32")]
                {
                    32
                }
            }
        }
    }

    /// Return the (min, max) bounds for this integer type.
    ///
    /// Note: `u128::MAX` (2^128 - 1) does not fit in `i128`, so `U128` is
    /// clamped to `i128::MAX` here. Code that needs the *exact* bound (e.g. Z3
    /// constraint construction) must use [`IntType::bounds_str`] instead, which
    /// returns the exact decimal literals.
    pub fn bounds(&self) -> (i128, i128) {
        match self {
            IntType::I8 => (i8::MIN as i128, i8::MAX as i128),
            IntType::I16 => (i16::MIN as i128, i16::MAX as i128),
            IntType::I32 => (i32::MIN as i128, i32::MAX as i128),
            IntType::I64 => (i64::MIN as i128, i64::MAX as i128),
            IntType::I128 => (i128::MIN, i128::MAX),
            IntType::U8 => (0, u8::MAX as i128),
            IntType::U16 => (0, u16::MAX as i128),
            IntType::U32 => (0, u32::MAX as i128),
            IntType::U64 => (0, u64::MAX as i128),
            IntType::U128 => (0, i128::MAX), // exact bound is 2^128-1, see bounds_str()
            // Pointer-width types must match [`IntType::bounds_str`], which is
            // already target-aware. These used to be hardcoded to the 64-bit
            // extremes, so on a 32-bit target the constant-folding path (which
            // uses `bounds`) and the Z3 path (which uses `bounds_str`) disagreed
            // about the range of `usize`/`isize`.
            IntType::Usize => {
                #[cfg(target_pointer_width = "64")]
                {
                    (0, u64::MAX as i128)
                }
                #[cfg(target_pointer_width = "32")]
                {
                    (0, u32::MAX as i128)
                }
            }
            IntType::Isize => {
                #[cfg(target_pointer_width = "64")]
                {
                    (i64::MIN as i128, i64::MAX as i128)
                }
                #[cfg(target_pointer_width = "32")]
                {
                    (i32::MIN as i128, i32::MAX as i128)
                }
            }
        }
    }

    /// Return the exact (min, max) bounds as decimal string literals.
    ///
    /// Unlike [`IntType::bounds`], this is exact for every type including
    /// `u128` (whose max does not fit in `i128`). These strings are fed
    /// directly to Z3, which uses unbounded integers.
    pub fn bounds_str(&self) -> (String, String) {
        // Exact decimal literals for the 128-bit extremes (2^127, 2^127-1, 2^128-1).
        const I128_MIN: &str = "-170141183460469231731687303715884105728";
        const I128_MAX: &str = "170141183460469231731687303715884105727";
        const U128_MAX: &str = "340282366920938463463374607431768211455";
        match self {
            IntType::I8 => ("-128".into(), "127".into()),
            IntType::I16 => ("-32768".into(), "32767".into()),
            IntType::I32 => ("-2147483648".into(), "2147483647".into()),
            IntType::I64 => ("-9223372036854775808".into(), "9223372036854775807".into()),
            IntType::I128 => (I128_MIN.into(), I128_MAX.into()),
            IntType::U8 => ("0".into(), "255".into()),
            IntType::U16 => ("0".into(), "65535".into()),
            IntType::U32 => ("0".into(), "4294967295".into()),
            IntType::U64 => ("0".into(), "18446744073709551615".into()),
            IntType::U128 => ("0".into(), U128_MAX.into()),
            IntType::Usize => {
                #[cfg(target_pointer_width = "64")]
                {
                    ("0".into(), "18446744073709551615".into())
                }
                #[cfg(target_pointer_width = "32")]
                {
                    ("0".into(), "4294967295".into())
                }
            }
            IntType::Isize => {
                #[cfg(target_pointer_width = "64")]
                {
                    ("-9223372036854775808".into(), "9223372036854775807".into())
                }
                #[cfg(target_pointer_width = "32")]
                {
                    ("-2147483648".into(), "2147483647".into())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_detection() {
        assert!(IntType::U8.is_unsigned());
        assert!(IntType::U128.is_unsigned());
        assert!(!IntType::I8.is_unsigned());
        assert!(!IntType::Isize.is_unsigned());
    }

    #[test]
    fn bit_widths() {
        assert_eq!(IntType::I8.bit_width(), 8);
        assert_eq!(IntType::U16.bit_width(), 16);
        assert_eq!(IntType::I32.bit_width(), 32);
        assert_eq!(IntType::U64.bit_width(), 64);
        assert_eq!(IntType::I128.bit_width(), 128);
    }

    #[test]
    fn i128_bounds_exact() {
        assert_eq!(IntType::I8.bounds(), (i8::MIN as i128, i8::MAX as i128));
        assert_eq!(IntType::I64.bounds(), (i64::MIN as i128, i64::MAX as i128));
        assert_eq!(IntType::I128.bounds(), (i128::MIN, i128::MAX));
        assert_eq!(IntType::U8.bounds(), (0, u8::MAX as i128));
    }

    #[test]
    fn numeric_and_string_bounds_agree_for_pointer_types() {
        // The constant path uses `bounds()` and the Z3 path uses `bounds_str()`;
        // they must describe the same range for every type, including the
        // pointer-width ones.
        for ty in [IntType::Usize, IntType::Isize] {
            let (smin, smax) = ty.bounds_str();
            let (bmin, bmax) = ty.bounds();
            assert_eq!(
                smin.parse::<i128>().unwrap(),
                bmin,
                "min mismatch for {:?}",
                ty
            );
            assert_eq!(
                smax.parse::<i128>().unwrap(),
                bmax,
                "max mismatch for {:?}",
                ty
            );
        }
    }

    #[test]
    fn string_bounds_are_exact_for_all_types() {
        // The exact 128-bit extremes must survive the string round-trip.
        let (min, max) = IntType::I128.bounds_str();
        assert_eq!(min.parse::<i128>().unwrap(), i128::MIN);
        assert_eq!(max.parse::<i128>().unwrap(), i128::MAX);

        // u128 max is 2^128 - 1, which does not fit in i128.
        let (umin, umax) = IntType::U128.bounds_str();
        assert_eq!(umin, "0");
        assert_eq!(umax, "340282366920938463463374607431768211455");
        // Sanity: 2^128 - 1 is one less than 2^128.
        assert_eq!(umax.parse::<u128>().unwrap(), u128::MAX);

        // String bounds must round-trip through the i128 bounds when representable.
        for ty in [
            IntType::I8,
            IntType::I16,
            IntType::I32,
            IntType::I64,
            IntType::U8,
            IntType::U16,
            IntType::U32,
            IntType::U64,
        ] {
            let (smin, smax) = ty.bounds_str();
            let (bmin, bmax) = ty.bounds();
            assert_eq!(smin.parse::<i128>().unwrap(), bmin);
            assert_eq!(smax.parse::<i128>().unwrap(), bmax);
        }
    }
}
