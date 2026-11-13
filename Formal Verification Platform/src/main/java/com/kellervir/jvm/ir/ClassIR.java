package com.kellervir.jvm.ir;

import java.util.ArrayList;
import java.util.List;

/**
 * Intermediate representation of a JVM class file.
 */
public final class ClassIR {

    private final String internalName; // e.g. "com/example/Foo"
    private final String superClassName;
    private final List<String> interfaceNames = new ArrayList<>();
    private final List<MethodIR> methods = new ArrayList<>();
    private final int accessFlags;

    public ClassIR(String internalName, String superClassName, int accessFlags) {
        this.internalName = internalName;
        this.superClassName = superClassName;
        this.accessFlags = accessFlags;
    }

    public void addInterface(String name) { interfaceNames.add(name); }
    public void addMethod(MethodIR method) { methods.add(method); }

    public String getInternalName() { return internalName; }
    public String getSuperClassName() { return superClassName; }
    public List<String> getInterfaceNames() { return interfaceNames; }
    public List<MethodIR> getMethods() { return methods; }
    public int getAccessFlags() { return accessFlags; }

    public String getSimpleName() {
        int lastSlash = internalName.lastIndexOf('/');
        return lastSlash >= 0 ? internalName.substring(lastSlash + 1) : internalName;
    }

    @Override
    public String toString() {
        return "ClassIR[" + internalName + ", methods=" + methods.size() + "]";
    }
}
