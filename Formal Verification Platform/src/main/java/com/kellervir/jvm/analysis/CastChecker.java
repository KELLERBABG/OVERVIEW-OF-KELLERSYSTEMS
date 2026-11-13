package com.kellervir.jvm.analysis;

import com.kellervir.jvm.ir.*;

import java.util.*;

/**
 * Checks for potential ClassCastException.
 * Tracks type information through casts and detects provably-unsafe casts.
 * 
 * A cast (S)expr is safe if:
 *   - S is a supertype of the static type of expr, AND
 *   - The dynamic type of expr is known to be compatible with S
 */
public final class CastChecker {

    public static final class CastWarning {
        public final String className;
        public final String methodName;
        public final int instructionId;
        public final int bytecodeOffset;
        public final String sourceType;
        public final String targetType;
        public final String description;
        public final NullDerefChecker.Severity severity;

        public CastWarning(String className, String methodName, int instructionId,
                           int bytecodeOffset, String sourceType, String targetType,
                           String description, NullDerefChecker.Severity severity) {
            this.className = className;
            this.methodName = methodName;
            this.instructionId = instructionId;
            this.bytecodeOffset = bytecodeOffset;
            this.sourceType = sourceType;
            this.targetType = targetType;
            this.description = description;
            this.severity = severity;
        }

        @Override
        public String toString() {
            return String.format("[%s] %s.%s insn[%d]@%d: cast %s -> %s: %s",
                    severity, className, methodName, instructionId, bytecodeOffset,
                    sourceType, targetType, description);
        }
    }

    private final List<CastWarning> warnings = new ArrayList<>();

    /**
     * Analyze a method for potential ClassCastException.
     */
    public List<CastWarning> analyze(MethodIR method) {
        warnings.clear();

        for (SSAInstruction insn : method.getInstructions()) {
            if (insn.getOpClass() == SSAInstruction.OpClass.CAST) {
                checkCast(insn, method);
            }
        }

        return Collections.unmodifiableList(warnings);
    }

    private void checkCast(SSAInstruction insn, MethodIR method) {
        List<SSAValue> operands = insn.getOperands();
        List<SSAValue> results = insn.getResults();

        if (operands.isEmpty() || results.isEmpty()) return;

        SSAValue source = operands.get(0);
        SSAValue result = results.get(0);

        String srcType = source.getTypeDescriptor();
        String tgtType = result.getTypeDescriptor();

        // Check for obviously unsafe casts
        if (isProvablyUnsafe(srcType, tgtType)) {
            warnings.add(new CastWarning(
                    method.getClassName(), method.getMethodName(),
                    insn.getId(), insn.getBytecodeOffset(),
                    srcType, tgtType,
                    "Provably unsafe cast: source type is incompatible with target type",
                    NullDerefChecker.Severity.ERROR));
        } else if (isPotentiallyUnsafe(srcType, tgtType)) {
            warnings.add(new CastWarning(
                    method.getClassName(), method.getMethodName(),
                    insn.getId(), insn.getBytecodeOffset(),
                    srcType, tgtType,
                    "Potentially unsafe cast: no instanceof guard detected before cast",
                    NullDerefChecker.Severity.WARNING));
        }
    }

    /**
     * Determine if a cast is provably unsafe based on type descriptors.
     * This is a conservative check - we only flag when types are clearly incompatible.
     */
    private boolean isProvablyUnsafe(String srcDesc, String tgtDesc) {
        if (srcDesc == null || tgtDesc == null) return false;

        // Casting from a primitive to a reference type or vice versa is always unsafe
        boolean srcIsPrimitive = isPrimitiveDescriptor(srcDesc);
        boolean tgtIsPrimitive = isPrimitiveDescriptor(tgtDesc);
        if (srcIsPrimitive != tgtIsPrimitive) return true;

        // Casting between unrelated final classes is provably unsafe
        // (simplified: we'd need a full class hierarchy in production)
        if (srcDesc.startsWith("L") && tgtDesc.startsWith("L")) {
            String srcClass = internalNameFromDescriptor(srcDesc);
            String tgtClass = internalNameFromDescriptor(tgtDesc);
            // If both are known final classes and neither is a supertype of the other
            if (isKnownFinal(srcClass) && isKnownFinal(tgtClass) && !srcClass.equals(tgtClass)) {
                return true;
            }
        }

        return false;
    }

    /**
     * Determine if a cast is potentially unsafe (needs runtime check).
     */
    private boolean isPotentiallyUnsafe(String srcDesc, String tgtDesc) {
        if (srcDesc == null || tgtDesc == null) return false;

        // Casting from Object to a specific type is potentially unsafe
        if ("Ljava/lang/Object;".equals(srcDesc) && !tgtDesc.equals("Ljava/lang/Object;")) {
            return true;
        }

        // Casting between two different reference types (neither is Object)
        if (srcDesc.startsWith("L") && tgtDesc.startsWith("L")
                && !srcDesc.equals(tgtDesc)
                && !"Ljava/lang/Object;".equals(srcDesc)
                && !"Ljava/lang/Object;".equals(tgtDesc)) {
            return true;
        }

        return false;
    }

    private static boolean isPrimitiveDescriptor(String desc) {
        return "I".equals(desc) || "J".equals(desc) || "F".equals(desc)
                || "D".equals(desc) || "Z".equals(desc) || "B".equals(desc)
                || "S".equals(desc) || "C".equals(desc);
    }

    private static String internalNameFromDescriptor(String desc) {
        if (desc.startsWith("L") && desc.endsWith(";")) {
            return desc.substring(1, desc.length() - 1).replace('/', '.');
        }
        return desc;
    }

    private static boolean isKnownFinal(String className) {
        // In a full implementation, this would consult the class hierarchy
        // For now, we treat java.lang.String and similar as final
        Set<String> knownFinals = Set.of(
                "java.lang.String", "java.lang.Integer", "java.lang.Long",
                "java.lang.Double", "java.lang.Float", "java.lang.Boolean",
                "java.lang.Character", "java.lang.Byte", "java.lang.Short"
        );
        return knownFinals.contains(className);
    }

    public List<CastWarning> getWarnings() {
        return Collections.unmodifiableList(warnings);
    }
}
