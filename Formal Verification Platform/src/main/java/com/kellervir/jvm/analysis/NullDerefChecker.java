package com.kellervir.jvm.analysis;

import com.kellervir.jvm.ir.*;

import java.util.*;

/**
 * Checks for potential NullPointerException dereferences.
 * Uses abstract interpretation over a null/non-null lattice.
 * 
 * Lattice: BOTTOM ⊥ < NULL < NON_NULL < TOP ⊤
 * - BOTTOM: unreachable state
 * - NULL: value is definitely null
 * - NON_NULL: value is definitely non-null
 * - TOP: value may or may not be null (unknown)
 */
public final class NullDerefChecker {

    public enum Nullness {
        BOTTOM, NULL, NON_NULL, TOP
    }

    public static final class NullDerefWarning {
        public final String className;
        public final String methodName;
        public final int instructionId;
        public final int bytecodeOffset;
        public final String description;
        public final Severity severity;

        public NullDerefWarning(String className, String methodName, int instructionId,
                                int bytecodeOffset, String description, Severity severity) {
            this.className = className;
            this.methodName = methodName;
            this.instructionId = instructionId;
            this.bytecodeOffset = bytecodeOffset;
            this.description = description;
            this.severity = severity;
        }

        @Override
        public String toString() {
            return String.format("[%s] %s.%s insn[%d]@%d: %s",
                    severity, className, methodName, instructionId, bytecodeOffset, description);
        }
    }

    public enum Severity { ERROR, WARNING, INFO }

    private final List<NullDerefWarning> warnings = new ArrayList<>();

    /**
     * Analyze a method for potential null dereferences.
     */
    public List<NullDerefWarning> analyze(MethodIR method) {
        warnings.clear();
        Map<Integer, Nullness> localNullness = new HashMap<>();

        // Initialize: parameters are TOP (unknown), locals are BOTTOM until assigned
        for (SSAValue param : method.getParameters()) {
            // We track by instruction, not by value directly in this simplified model
        }

        for (SSAInstruction insn : method.getInstructions()) {
            switch (insn.getOpClass()) {
                case CONSTANT:
                    handleConstant(insn, localNullness);
                    break;
                case LOAD:
                    // Loading a null constant and then dereferencing is the key pattern
                    break;
                case CALL:
                    checkCallDeref(insn, method, localNullness);
                    break;
                case ARRAY_LOAD:
                    checkArrayLoadNull(insn, method);
                    break;
                default:
                    break;
            }
        }

        return Collections.unmodifiableList(warnings);
    }

    private void handleConstant(SSAInstruction insn, Map<Integer, Nullness> localNullness) {
        for (SSAValue result : insn.getResults()) {
            if (result.isNullConstant()) {
                // Mark this value as NULL in our tracking
                localNullness.put(result.getId(), Nullness.NULL);
            } else if (result.isIntConstant()) {
                localNullness.put(result.getId(), Nullness.NON_NULL);
            } else {
                localNullness.put(result.getId(), Nullness.TOP);
            }
        }
    }

    private void checkCallDeref(SSAInstruction insn, MethodIR method, Map<Integer, Nullness> localNullness) {
        // If we call a method on a reference that is known NULL, that's an NPE
        for (SSAValue operand : insn.getOperands()) {
            Nullness nullness = localNullness.get(operand.getId());
            if (nullness == Nullness.NULL) {
                warnings.add(new NullDerefWarning(
                        method.getClassName(), method.getMethodName(),
                        insn.getId(), insn.getBytecodeOffset(),
                        "Method call on known-null reference: " + insn.getLabel(),
                        Severity.ERROR));
            }
        }
    }

    private void checkArrayLoadNull(SSAInstruction insn, MethodIR method) {
        // Array load on null array reference
        for (SSAValue operand : insn.getOperands()) {
            if (operand.isReferenceType() && operand.getTypeDescriptor().startsWith("[")) {
                // This is the array reference - check if it could be null
                Nullness nullness = inferNullness(operand);
                if (nullness == Nullness.NULL) {
                    warnings.add(new NullDerefWarning(
                            method.getClassName(), method.getMethodName(),
                            insn.getId(), insn.getBytecodeOffset(),
                            "Array access on known-null array reference",
                            Severity.ERROR));
                } else if (nullness == Nullness.TOP) {
                    warnings.add(new NullDerefWarning(
                            method.getClassName(), method.getMethodName(),
                            insn.getId(), insn.getBytecodeOffset(),
                            "Array access on possibly-null array reference",
                            Severity.WARNING));
                }
            }
        }
    }

    private Nullness inferNullness(SSAValue value) {
        if (value.isNullConstant()) return Nullness.NULL;
        if (value.isIntConstant()) return Nullness.NON_NULL;
        // For instruction results, check the defining instruction
        int defId = value.getDefiningInstructionId();
        if (defId >= 0) {
            // Would need to trace back through the defining instruction
            // In this simplified model, we return TOP for unknown origins
        }
        return Nullness.TOP;
    }

    /**
     * Compute the join (meet in lattice terms) of two nullness values.
     */
    public static Nullness join(Nullness a, Nullness b) {
        if (a == Nullness.BOTTOM) return b;
        if (b == Nullness.BOTTOM) return a;
        if (a == b) return a;
        return Nullness.TOP;
    }

    /**
     * Compute the meet of two nullness values.
     */
    public static Nullness meet(Nullness a, Nullness b) {
        if (a == Nullness.BOTTOM || b == Nullness.BOTTOM) return Nullness.BOTTOM;
        if (a == Nullness.TOP || b == Nullness.TOP) return Nullness.TOP;
        if (a == b) return a;
        // NULL and NON_NULL -> BOTTOM (contradiction)
        return Nullness.BOTTOM;
    }

    public List<NullDerefWarning> getWarnings() {
        return Collections.unmodifiableList(warnings);
    }
}
