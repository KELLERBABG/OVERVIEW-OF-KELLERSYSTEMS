package com.kellervir.jvm.ir;

import org.objectweb.asm.Opcodes;

import java.util.ArrayList;
import java.util.List;

/**
 * Represents a single JVM bytecode instruction in SSA form.
 * Each instruction has a unique ID and carries operand/result type information.
 */
public final class SSAInstruction {

    public enum OpClass {
        LOAD, STORE, CONSTANT, ARITHMETIC, COMPARE, JUMP, CALL, RETURN,
        ARRAY_LOAD, ARRAY_STORE, NEW_ARRAY, CAST, INSTANCEOF, THROW,
        GOTO, TABLE_SWITCH, LOOKUP_SWITCH, MONITORENTER, MONITOREXIT,
        DUP, SWAP, POP, NOP, FRAME, OTHER
    }

    private final int id;
    private final int opcode;
    private final OpClass opClass;
    private final String label;
    private final List<SSAValue> operands = new ArrayList<>();
    private final List<SSAValue> results = new ArrayList<>();
    private final int bytecodeOffset;
    private final String methodName;
    private final String className;

    public SSAInstruction(int id, int opcode, OpClass opClass, String label,
                          int bytecodeOffset, String className, String methodName) {
        this.id = id;
        this.opcode = opcode;
        this.opClass = opClass;
        this.label = label;
        this.bytecodeOffset = bytecodeOffset;
        this.className = className;
        this.methodName = methodName;
    }

    public int getId() { return id; }
    public int getOpcode() { return opcode; }
    public OpClass getOpClass() { return opClass; }
    public String getLabel() { return label; }
    public List<SSAValue> getOperands() { return operands; }
    public List<SSAValue> getResults() { return results; }
    public int getBytecodeOffset() { return bytecodeOffset; }
    public String getClassName() { return className; }
    public String getMethodName() { return methodName; }

    public void addOperand(SSAValue v) { operands.add(v); }
    public void addResult(SSAValue v) { results.add(v); }

    public boolean isLoad() { return opClass == OpClass.LOAD; }
    public boolean isStore() { return opClass == OpClass.STORE; }
    public boolean isArrayLoad() { return opClass == OpClass.ARRAY_LOAD; }
    public boolean isArrayStore() { return opClass == OpClass.ARRAY_STORE; }
    public boolean isCast() { return opClass == OpClass.CAST; }
    public boolean isCall() { return opClass == OpClass.CALL; }
    public boolean isReturn() { return opClass == OpClass.RETURN; }
    public boolean isJump() { return opClass == OpClass.JUMP; }
    public boolean isConstant() { return opClass == OpClass.CONSTANT; }

    @Override
    public String toString() {
        return String.format("insn[%d] %s (%s) @%d", id, label, opClass, bytecodeOffset);
    }
}
