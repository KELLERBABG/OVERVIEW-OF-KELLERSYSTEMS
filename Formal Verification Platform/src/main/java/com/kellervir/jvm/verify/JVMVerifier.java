package com.kellervir.jvm.verify;

import com.kellervir.jvm.analysis.*;
import com.kellervir.jvm.ir.ClassIR;
import com.kellervir.jvm.ir.MethodIR;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Main verification engine for JVM bytecode.
 * Orchestrates all checkers and produces a unified VerificationResult.
 * 
 * Operates in audit-only mode by default: no source mutation, no healing.
 */
public final class JVMVerifier {

    public static final class VerificationResult {
        public final String className;
        public final List<NullDerefChecker.NullDerefWarning> nullWarnings;
        public final List<ArrayBoundsChecker.BoundsWarning> boundsWarnings;
        public final List<CastChecker.CastWarning> castWarnings;
        public final long elapsedNanos;
        public final boolean passed;

        public VerificationResult(String className,
                                  List<NullDerefChecker.NullDerefWarning> nullWarnings,
                                  List<ArrayBoundsChecker.BoundsWarning> boundsWarnings,
                                  List<CastChecker.CastWarning> castWarnings,
                                  long elapsedNanos) {
            this.className = className;
            this.nullWarnings = Collections.unmodifiableList(nullWarnings);
            this.boundsWarnings = Collections.unmodifiableList(boundsWarnings);
            this.castWarnings = Collections.unmodifiableList(castWarnings);
            this.elapsedNanos = elapsedNanos;
            this.passed = nullWarnings.stream().noneMatch(w -> w.severity == NullDerefChecker.Severity.ERROR)
                    && boundsWarnings.stream().noneMatch(w -> w.severity == NullDerefChecker.Severity.ERROR)
                    && castWarnings.stream().noneMatch(w -> w.severity == NullDerefChecker.Severity.ERROR);
        }

        public int totalErrorCount() {
            return (int) (nullWarnings.stream().filter(w -> w.severity == NullDerefChecker.Severity.ERROR).count()
                    + boundsWarnings.stream().filter(w -> w.severity == NullDerefChecker.Severity.ERROR).count()
                    + castWarnings.stream().filter(w -> w.severity == NullDerefChecker.Severity.ERROR).count());
        }

        public int totalWarningCount() {
            return (int) (nullWarnings.stream().filter(w -> w.severity == NullDerefChecker.Severity.WARNING).count()
                    + boundsWarnings.stream().filter(w -> w.severity == NullDerefChecker.Severity.WARNING).count()
                    + castWarnings.stream().filter(w -> w.severity == NullDerefChecker.Severity.WARNING).count());
        }

        @Override
        public String toString() {
            return String.format("VerificationResult[%s: %s, errors=%d, warnings=%d, time=%.2fms]",
                    className, passed ? "PASS" : "FAIL",
                    totalErrorCount(), totalWarningCount(), elapsedNanos / 1_000_000.0);
        }
    }

    private final NullDerefChecker nullChecker = new NullDerefChecker();
    private final ArrayBoundsChecker boundsChecker = new ArrayBoundsChecker();
    private final CastChecker castChecker = new CastChecker();

    /**
     * Verify a single class in audit-only mode.
     */
    public VerificationResult verify(ClassIR classIR) {
        long start = System.nanoTime();
        List<NullDerefChecker.NullDerefWarning> allNullWarnings = new ArrayList<>();
        List<ArrayBoundsChecker.BoundsWarning> allBoundsWarnings = new ArrayList<>();
        List<CastChecker.CastWarning> allCastWarnings = new ArrayList<>();

        for (MethodIR method : classIR.getMethods()) {
            allNullWarnings.addAll(nullChecker.analyze(method));
            allBoundsWarnings.addAll(boundsChecker.analyze(method));
            allCastWarnings.addAll(castChecker.analyze(method));
        }

        long elapsed = System.nanoTime() - start;
        return new VerificationResult(classIR.getInternalName(),
                allNullWarnings, allBoundsWarnings, allCastWarnings, elapsed);
    }

    /**
     * Verify a class and produce a human-readable report.
     */
    public String verifyAndReport(ClassIR classIR) {
        VerificationResult result = verify(classIR);
        StringBuilder sb = new StringBuilder();
        sb.append("=== JVM Verification Report ===\n");
        sb.append(String.format("Class: %s%n", result.className));
        sb.append(String.format("Status: [%s]%n", result.passed ? "PASS" : "FAIL"));
        sb.append(String.format("Time: %.2f ms%n", result.elapsedNanos / 1_000_000.0));
        sb.append(String.format("Errors: %d, Warnings: %d%n%n",
                result.totalErrorCount(), result.totalWarningCount()));

        if (!result.nullWarnings.isEmpty()) {
            sb.append("--- NullPointerException Analysis ---%n");
            for (var w : result.nullWarnings) {
                sb.append(String.format("  %s%n", w));
            }
            sb.append("%n");
        }

        if (!result.boundsWarnings.isEmpty()) {
            sb.append("--- ArrayIndexOutOfBoundsException Analysis ---%n");
            for (var w : result.boundsWarnings) {
                sb.append(String.format("  %s%n", w));
            }
            sb.append("%n");
        }

        if (!result.castWarnings.isEmpty()) {
            sb.append("--- ClassCastException Analysis ---%n");
            for (var w : result.castWarnings) {
                sb.append(String.format("  %s%n", w));
            }
            sb.append("%n");
        }

        return sb.toString();
    }
}
