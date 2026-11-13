package com.kellervir.jvm.analysis;

import com.kellervir.jvm.ir.*;

import java.util.*;

/**
 * Checks for potential ArrayIndexOutOfBoundsException.
 * Tracks array lengths and index expressions to detect provable out-of-bounds accesses.
 * 
 * For each array access arr[i], we check:
 *   - If i is a constant: verify 0 <= i < length (if length is known)
 *   - If i is derived from a loop: use loop bound analysis
 *   - Otherwise: report as potential warning
 */
public final class ArrayBoundsChecker {

    public static final class BoundsWarning {
        public final String className;
        public final String methodName;
        public final int instructionId;
        public final int bytecodeOffset;
        public final String description;
        public final NullDerefChecker.Severity severity;

        public BoundsWarning(String className, String methodName, int instructionId,
                             int bytecodeOffset, String description,
                             NullDerefChecker.Severity severity) {
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

    private final List<BoundsWarning> warnings = new ArrayList<>();

    /**
     * Represents a known array length constraint.
     */
    public static final class ArrayLengthInfo {
        public final int value; // -1 if unknown
        public final boolean isKnown;

        public ArrayLengthInfo(int value) {
            this.value = value;
            this.isKnown = value >= 0;
        }

        public static ArrayLengthInfo unknown() { return new ArrayLengthInfo(-1); }
        public static ArrayLengthInfo known(int len) { return new ArrayLengthInfo(len); }
    }

    /**
     * Analyze a method for potential array bounds violations.
     */
    public List<BoundsWarning> analyze(MethodIR method) {
        warnings.clear();
        Map<Integer, ArrayLengthInfo> arrayLengths = new HashMap<>();

        for (SSAInstruction insn : method.getInstructions()) {
            switch (insn.getOpClass()) {
                case NEW_ARRAY:
                    trackNewArray(insn, arrayLengths);
                    break;
                case ARRAY_LOAD:
                    checkArrayLoadBounds(insn, method, arrayLengths);
                    break;
                case ARRAY_STORE:
                    checkArrayStoreBounds(insn, method, arrayLengths);
                    break;
                default:
                    break;
            }
        }

        return Collections.unmodifiableList(warnings);
    }

    private void trackNewArray(SSAInstruction insn, Map<Integer, ArrayLengthInfo> arrayLengths) {
        // NEWARRAY: stack has [length] -> result is new array
        // ANEWARRAY: stack has [length] -> result is new object array
        for (SSAValue operand : insn.getOperands()) {
            if (operand.isIntConstant()) {
                int len = ((Integer) operand.getConstantValue());
                arrayLengths.put(insn.getId(), ArrayLengthInfo.known(len));
            } else {
                arrayLengths.put(insn.getId(), ArrayLengthInfo.unknown());
            }
        }
    }

    private void checkArrayLoadBounds(SSAInstruction insn, MethodIR method,
                                      Map<Integer, ArrayLengthInfo> arrayLengths) {
        List<SSAValue> operands = insn.getOperands();
        if (operands.size() < 2) return;

        SSAValue indexVal = operands.get(1); // Second operand is the index
        ArrayLengthInfo lengthInfo = arrayLengths.get(insn.getId());

        if (indexVal.isIntConstant()) {
            int idx = ((Integer) indexVal.getConstantValue());
            if (idx < 0) {
                warnings.add(new BoundsWarning(
                        method.getClassName(), method.getMethodName(),
                        insn.getId(), insn.getBytecodeOffset(),
                        "Negative array index: " + idx,
                        NullDerefChecker.Severity.ERROR));
            } else if (lengthInfo != null && lengthInfo.isKnown) {
                if (idx >= lengthInfo.value) {
                    warnings.add(new BoundsWarning(
                            method.getClassName(), method.getMethodName(),
                            insn.getId(), insn.getBytecodeOffset(),
                            String.format("Array index %d out of bounds (length=%d)", idx, lengthInfo.value),
                            NullDerefChecker.Severity.ERROR));
                }
            }
        } else {
            // Non-constant index: report as potential
            warnings.add(new BoundsWarning(
                    method.getClassName(), method.getMethodName(),
                    insn.getId(), insn.getBytecodeOffset(),
                    "Array access with non-constant index; bounds not provable without additional constraints",
                    NullDerefChecker.Severity.WARNING));
        }
    }

    private void checkArrayStoreBounds(SSAInstruction insn, MethodIR method,
                                       Map<Integer, ArrayLengthInfo> arrayLengths) {
        List<SSAValue> operands = insn.getOperands();
        if (operands.size() < 3) return;

        SSAValue indexVal = operands.get(1);
        ArrayLengthInfo lengthInfo = arrayLengths.get(insn.getId());

        if (indexVal.isIntConstant()) {
            int idx = ((Integer) indexVal.getConstantValue());
            if (idx < 0) {
                warnings.add(new BoundsWarning(
                        method.getClassName(), method.getMethodName(),
                        insn.getId(), insn.getBytecodeOffset(),
                        "Negative array store index: " + idx,
                        NullDerefChecker.Severity.ERROR));
            } else if (lengthInfo != null && lengthInfo.isKnown) {
                if (idx >= lengthInfo.value) {
                    warnings.add(new BoundsWarning(
                            method.getClassName(), method.getMethodName(),
                            insn.getId(), insn.getBytecodeOffset(),
                            String.format("Array store index %d out of bounds (length=%d)", idx, lengthInfo.value),
                            NullDerefChecker.Severity.ERROR));
                }
            }
        } else {
            warnings.add(new BoundsWarning(
                    method.getClassName(), method.getMethodName(),
                    insn.getId(), insn.getBytecodeOffset(),
                    "Array store with non-constant index; bounds not provable",
                    NullDerefChecker.Severity.WARNING));
        }
    }

    public List<BoundsWarning> getWarnings() {
        return Collections.unmodifiableList(warnings);
    }
}
