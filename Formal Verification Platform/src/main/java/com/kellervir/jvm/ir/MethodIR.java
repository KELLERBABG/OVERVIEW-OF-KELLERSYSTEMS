package com.kellervir.jvm.ir;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

/**
 * Intermediate representation of a single method in SSA form.
 * Maps to Keller-VIR's SSA Hypergraph structure.
 */
public final class MethodIR {

    private final String className;
    private final String methodName;
    private final String descriptor;
    private final List<SSAInstruction> instructions = new ArrayList<>();
    private final Map<Integer, SSAValue> localVariables = new TreeMap<>();
    private final List<SSAValue> parameters = new ArrayList<>();
    private final String returnType;
    private int nextValueId = 0;

    public MethodIR(String className, String methodName, String descriptor, String returnType) {
        this.className = className;
        this.methodName = methodName;
        this.descriptor = descriptor;
        this.returnType = returnType;
    }

    public int freshValueId() { return nextValueId++; }

    public void addInstruction(SSAInstruction insn) { instructions.add(insn); }
    public void registerLocal(int slot, SSAValue value) { localVariables.put(slot, value); }
    public void addParameter(SSAValue param) { parameters.add(param); }

    public String getClassName() { return className; }
    public String getMethodName() { return methodName; }
    public String getDescriptor() { return descriptor; }
    public List<SSAInstruction> getInstructions() { return instructions; }
    public Map<Integer, SSAValue> getLocalVariables() { return localVariables; }
    public List<SSAValue> getParameters() { return parameters; }
    public String getReturnType() { return returnType; }

    public SSAValue getLocal(int slot) { return localVariables.get(slot); }

    @Override
    public String toString() {
        return className + "." + methodName + descriptor + " [" + instructions.size() + " insns]";
    }
}
