package com.kellervir.jvm.ir;

/**
 * Represents an SSA value (phi node result, constant, or instruction result).
 */
public final class SSAValue {

    public enum Kind {
        CONSTANT_INT, CONSTANT_LONG, CONSTANT_FLOAT, CONSTANT_DOUBLE,
        CONSTANT_NULL, CONSTANT_STRING, LOCAL_VAR, PARAM, INSN_RESULT, PHI
    }

    private final int id;
    private final Kind kind;
    private final String name;
    private final String typeDescriptor; // JVM type descriptor, e.g. "Ljava/lang/String;"
    private final Object constantValue;
    private final int definingInstructionId; // -1 if not from instruction

    public SSAValue(int id, Kind kind, String name, String typeDescriptor,
                    Object constantValue, int definingInstructionId) {
        this.id = id;
        this.kind = kind;
        this.name = name;
        this.typeDescriptor = typeDescriptor;
        this.constantValue = constantValue;
        this.definingInstructionId = definingInstructionId;
    }

    public static SSAValue constantInt(int id, int value) {
        return new SSAValue(id, Kind.CONSTANT_INT, "const_" + id, "I", value, -1);
    }

    public static SSAValue constantNull(int id) {
        return new SSAValue(id, Kind.CONSTANT_NULL, "null_" + id, "Lnull;", null, -1);
    }

    public static SSAValue localVar(int id, String name, String typeDesc) {
        return new SSAValue(id, Kind.LOCAL_VAR, name, typeDesc, null, -1);
    }

    public static SSAValue param(int id, String name, String typeDesc) {
        return new SSAValue(id, Kind.PARAM, name, typeDesc, null, -1);
    }

    public static SSAValue insnResult(int id, String name, String typeDesc, int defInsnId) {
        return new SSAValue(id, Kind.INSN_RESULT, name, typeDesc, null, defInsnId);
    }

    public static SSAValue phi(int id, String name, String typeDesc) {
        return new SSAValue(id, Kind.PHI, name, typeDesc, null, -1);
    }

    public int getId() { return id; }
    public Kind getKind() { return kind; }
    public String getName() { return name; }
    public String getTypeDescriptor() { return typeDescriptor; }
    public Object getConstantValue() { return constantValue; }
    public int getDefiningInstructionId() { return definingInstructionId; }

    public boolean isNullConstant() { return kind == Kind.CONSTANT_NULL; }
    public boolean isIntConstant() { return kind == Kind.CONSTANT_INT; }
    public boolean isReferenceType() {
        return typeDescriptor != null && (typeDescriptor.startsWith("L") || typeDescriptor.startsWith("["));
    }

    @Override
    public String toString() {
        return name + ":" + typeDescriptor;
    }
}
