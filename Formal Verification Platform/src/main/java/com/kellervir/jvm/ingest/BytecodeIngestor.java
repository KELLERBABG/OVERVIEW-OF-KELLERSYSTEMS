package com.kellervir.jvm.ingest;

import com.kellervir.jvm.ir.*;
import org.objectweb.asm.*;
import org.objectweb.asm.tree.*;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/**
 * Ingests JVM .class files into the Keller-VIR SSA IR.
 * Uses ASM's tree API for structured bytecode access.
 */
public final class BytecodeIngestor {

    private final List<IngestError> errors = new ArrayList<>();

    public static final class IngestError {
        public final String className;
        public final String message;
        public final Throwable cause;

        public IngestError(String className, String message, Throwable cause) {
            this.className = className;
            this.message = message;
            this.cause = cause;
        }
    }

    /**
     * Ingest a single .class file from disk.
     */
    public ClassIR ingestFile(Path classFile) throws IOException {
        byte[] bytes = Files.readAllBytes(classFile);
        return ingestBytes(bytes, classFile.getFileName().toString());
    }

    /**
     * Ingest a class from raw bytecode.
     */
    public ClassIR ingestBytes(byte[] classBytes, String sourceName) {
        ClassNode classNode = new ClassNode();
        ClassReader reader = new ClassReader(classBytes);
        reader.accept(new ClassVisitor(Opcodes.ASM9, classNode) {
            @Override
            public MethodVisitor visitMethod(int access, String name, String descriptor,
                                              String signature, String[] exceptions) {
                return new MethodNode(access, name, descriptor, signature, exceptions);
            }
        }, 0);

        ClassIR ir = new ClassIR(classNode.name, classNode.superName, classNode.access);
        if (classNode.interfaces != null) {
            for (String iface : classNode.interfaces) {
                ir.addInterface(iface);
            }
        }

        if (classNode.methods != null) {
            for (MethodNode methodNode : classNode.methods) {
                MethodIR methodIR = ingestMethod(classNode.name, methodNode);
                if (methodIR != null) {
                    ir.addMethod(methodIR);
                }
            }
        }

        return ir;
    }

    private MethodIR ingestMethod(String className, MethodNode methodNode) {
        String internalName = className.replace('/', '.');
        MethodIR methodIR = new MethodIR(internalName, methodNode.name, methodNode.desc,
                returnTypeFromDescriptor(methodNode.desc));

        int valueId = 0;
        int insnId = 0;

        // Register parameters
        int[] paramSlots = computeParamSlots(methodNode.access, methodNode.desc);
        for (int slot : paramSlots) {
            String paramName = "arg" + slot;
            String typeDesc = paramTypeAt(methodNode.desc, slot);
            SSAValue param = SSAValue.param(valueId++, paramName, typeDesc);
            methodIR.addParameter(param);
            methodIR.registerLocal(slot, param);
        }

        // Process instructions
        for (AbstractInsnNode insnNode : methodNode.instructions) {
            SSAInstruction insn = translateInsn(insnNode, methodIR, insnId++, valueId);
            if (insn != null) {
                methodIR.addInstruction(insn);
            }
        }

        return methodIR;
    }

    private SSAInstruction translateInsn(AbstractInsnNode node, MethodIR methodIR, int insnId, int valueId) {
        String className = methodIR.getClassName();
        String methodName = methodIR.getMethodName();
        int opcode = node.getOpcode();
        SSAInstruction.OpClass opClass = classifyOpcode(opcode);
        String label = describeInsn(node);

        SSAInstruction insn = new SSAInstruction(insnId, opcode, opClass, label,
                node.getStart(), className, methodName);

        switch (opClass) {
            case LOAD:
                handleLoad(insn, node, methodIR, valueId);
                break;
            case STORE:
                handleStore(insn, node, methodIR, valueId);
                break;
            case CONSTANT:
                handleConstant(insn, node, valueId);
                break;
            case ARRAY_LOAD:
                handleArrayLoad(insn, node, methodIR, valueId);
                break;
            case ARRAY_STORE:
                handleArrayStore(insn, node, methodIR, valueId);
                break;
            case CAST:
                handleCast(insn, node, valueId);
                break;
            case CALL:
                handleCall(insn, node, methodIR, valueId);
                break;
            default:
                break;
        }

        return insn;
    }

    private void handleLoad(SSAInstruction insn, AbstractInsnNode node, MethodIR methodIR, int valueId) {
        if (node instanceof VarInsnNode varInsn) {
            int slot = varInsn.var;
            SSAValue local = methodIR.getLocal(slot);
            if (local != null) {
                insn.addOperand(local);
            }
        }
    }

    private void handleStore(SSAInstruction insn, AbstractInsnNode node, MethodIR methodIR, int valueId) {
        // Store: the value being stored is on the stack; we track it via local variable mapping
        if (node instanceof VarInsnNode varInsn) {
            int slot = varInsn.var;
            SSAValue local = methodIR.getLocal(slot);
            if (local != null) {
                insn.addOperand(local);
            }
        }
    }

    private void handleConstant(SSAInstruction insn, AbstractInsnNode node, int valueId) {
        if (node instanceof LdcInsnNode ldc) {
            Object cst = ldc.cst;
            if (cst instanceof Integer i) {
                SSAValue v = SSAValue.constantInt(valueId++, i);
                insn.addResult(v);
            } else if (cst == null) {
                SSAValue v = SSAValue.constantNull(valueId++);
                insn.addResult(v);
            } else if (cst instanceof String s) {
                SSAValue v = new SSAValue(valueId++, SSAValue.Kind.CONSTANT_STRING, "str_" + valueId,
                        "Ljava/lang/String;", s, -1);
                insn.addResult(v);
            }
        } else if (node instanceof IntInsnNode intInsn) {
            SSAValue v = SSAValue.constantInt(valueId++, intInsn.operand);
            insn.addResult(v);
        } else if (node instanceof FieldInsnNode fieldInsn && fieldInsn.getOpcode() == Opcodes.GETSTATIC) {
            // Static field load - treat as unknown reference
            SSAValue v = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT, "static_" + valueId,
                    fieldInsn.desc, null, -1);
            insn.addResult(v);
        }
    }

    private void handleArrayLoad(SSAInstruction insn, AbstractInsnNode node, MethodIR methodIR, int valueId) {
        // IALOAD/AALOAD/etc: stack has [array, index] -> result
        SSAValue arrayVal = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT, "arr_ref", "[I", null, -1);
        SSAValue indexVal = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT, "arr_idx", "I", null, -1);
        SSAValue resultVal = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT, "arr_load_result", "I", null, insn.getId());
        insn.addOperand(arrayVal);
        insn.addOperand(indexVal);
        insn.addResult(resultVal);
    }

    private void handleArrayStore(SSAInstruction insn, AbstractInsnNode node, MethodIR methodIR, int valueId) {
        // IASTORE/AASTORE/etc: stack has [array, index, value] -> void
        SSAValue arrayVal = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT, "arr_ref", "[I", null, -1);
        SSAValue indexVal = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT, "arr_idx", "I", null, -1);
        SSAValue valueVal = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT, "arr_val", "I", null, -1);
        insn.addOperand(arrayVal);
        insn.addOperand(indexVal);
        insn.addOperand(valueVal);
    }

    private void handleCast(SSAInstruction insn, AbstractInsnNode node, MethodIR methodIR, int valueId) {
        if (node instanceof TypeInsnNode typeInsn) {
            String targetDesc = "L" + typeInsn.type + ";";
            SSAValue sourceVal = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT, "cast_src", "Ljava/lang/Object;", null, -1);
            SSAValue resultVal = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT, "cast_result", targetDesc, null, insn.getId());
            insn.addOperand(sourceVal);
            insn.addResult(resultVal);
        }
    }

    private void handleCall(SSAInstruction insn, AbstractInsnNode node, MethodIR methodIR, int valueId) {
        if (node instanceof MethodInsnNode methodInsn) {
            String retDesc = returnTypeFromDescriptor(methodInsn.desc);
            SSAValue resultVal = new SSAValue(valueId++, SSAValue.Kind.INSN_RESULT,
                    "call_result", retDesc, null, insn.getId());
            insn.addResult(resultVal);
        }
    }

    private static SSAInstruction.OpClass classifyOpcode(int opcode) {
        switch (opcode) {
            case Opcodes.ILOAD: case Opcodes.LLOAD: case Opcodes.FLOAD: case Opcodes.DLOAD:
            case Opcodes.ALOAD: return SSAInstruction.OpClass.LOAD;
            case Opcodes.ISTORE: case Opcodes.LSTORE: case Opcodes.FSTORE: case Opcodes.DSTORE:
            case Opcodes.ASTORE: return SSAInstruction.OpClass.STORE;
            case Opcodes.ICONST_M1: case Opcodes.ICONST_0: case Opcodes.ICONST_1:
            case Opcodes.ICONST_2: case Opcodes.ICONST_3: case Opcodes.ICONST_4: case Opcodes.ICONST_5:
            case Opcodes.LCONST_0: case Opcodes.LCONST_1:
            case Opcodes.FCONST_0: case Opcodes.FCONST_1: case Opcodes.FCONST_2:
            case Opcodes.DCONST_0: case Opcodes.DCONST_1:
            case Opcodes.BIPUSH: case Opcodes.SIPUSH: case Opcodes.LDC:
            case Opcodes.LDC_W: case Opcodes.LDC2_W: return SSAInstruction.OpClass.CONSTANT;
            case Opcodes.IADD: case Opcodes.LADD: case Opcodes.FADD: case Opcodes.DADD:
            case Opcodes.ISUB: case Opcodes.LSUB: case Opcodes.FSUB: case Opcodes.DSUB:
            case Opcodes.IMUL: case Opcodes.LMUL: case Opcodes.FMUL: case Opcodes.DMUL:
            case Opcodes.IDIV: case Opcodes.LDIV: case Opcodes.FDIV: case Opcodes.DDIV:
            case Opcodes.IREM: case Opcodes.LREM: case Opcodes.FREM: case Opcodes.DREM:
            case Opcodes.INEG: case Opcodes.LNEG: case Opcodes.FNEG: case Opcodes.DNEG:
            case Opcodes.ISHL: case Opcodes.LSHL: case Opcodes.ISHR: case Opcodes.LSHR:
            case Opcodes.IAND: case Opcodes.LAND: case Opcodes.IOR: case Opcodes.LOR:
            case Opcodes.IXOR: case Opcodes.LXOR: return SSAInstruction.OpClass.ARITHMETIC;
            case Opcodes.IFCMPEQ: case Opcodes.IFCMPNE: case Opcodes.IFCMPLT:
            case Opcodes.IFCMPGT: case Opcodes.IFACMPEQ: case Opcodes.IFACMPNE:
            case Opcodes.IFACMPLT: case Opcodes.IFACMPGT: return SSAInstruction.OpClass.COMPARE;
            case Opcodes.IF_EQ: case Opcodes.IF_NE: case Opcodes.IF_LT:
            case Opcodes.IF_GE: case Opcodes.IF_GT: case Opcodes.IF_LE:
            case Opcodes.GOTO: return SSAInstruction.OpClass.JUMP;
            case Opcodes.INVOKESPECIAL: case Opcodes.INVOKEVIRTUAL:
            case Opcodes.INVOKESTATIC: case Opcodes.INVOKEINTERFACE:
            case Opcodes.INVOKEDYNAMIC: return SSAInstruction.OpClass.CALL;
            case Opcodes.IRETURN: case Opcodes.LRETURN: case Opcodes.FRETURN:
            case Opcodes.DRETURN: case Opcodes.ARETURN: case Opcodes.RETURN:
                return SSAInstruction.OpClass.RETURN;
            case Opcodes.IALOAD: case Opcodes.LALOAD: case Opcodes.FALOAD:
            case Opcodes.DALOAD: case Opcodes.AALOAD: case Opcodes.BALOAD:
            case Opcodes.CALOAD: case Opcodes.SALOAD: return SSAInstruction.OpClass.ARRAY_LOAD;
            case Opcodes.IASTORE: case Opcodes.LASTORE: case Opcodes.FASTORE:
            case Opcodes.DASTORE: case Opcodes.AASTORE: case Opcodes.BASTORE:
            case Opcodes.CASTORE: case Opcodes.SASTORE: return SSAInstruction.OpClass.ARRAY_STORE;
            case Opcodes.NEWARRAY: case Opcodes.ANEWARRAY: return SSAInstruction.OpClass.NEW_ARRAY;
            case Opcodes.CHECKCAST: return SSAInstruction.OpClass.CAST;
            case Opcodes.INSTANCEOF: return SSAInstruction.OpClass.INSTANCEOF;
            case Opcodes.ATHROW: return SSAInstruction.OpClass.THROW;
            case Opcodes.DUP: case Opcodes.DUP_X1: case Opcodes.DUP_X2:
            case Opcodes.DUP2: case Opcodes.DUP2_X1: case Opcodes.DUP2_X2:
                return SSAInstruction.OpClass.DUP;
            case Opcodes.SWAP: return SSAInstruction.OpClass.SWAP;
            case Opcodes.POP: case Opcodes.POP2: return SSAInstruction.OpClass.POP;
            case Opcodes.NOP: return SSAInstruction.OpClass.NOP;
            case Opcodes.MONITORENTER: return SSAInstruction.OpClass.MONITORENTER;
            case Opcodes.MONITOREXIT: return SSAInstruction.OpClass.MONITOREXIT;
            case Opcodes.TABLESWITCH: return SSAInstruction.OpClass.TABLE_SWITCH;
            case Opcodes.LOOKUPSWITCH: return SSAInstruction.OpClass.LOOKUP_SWITCH;
            default: return SSAInstruction.OpClass.OTHER;
        }
    }

    private static String describeInsn(AbstractInsnNode node) {
        if (node instanceof LabelNode) return "label";
        if (node instanceof FrameNode) return "frame";
        if (node instanceof LineNumberNode) return "line:" + ((LineNumberNode) node).getLine();
        if (node instanceof VarInsnNode) return "var:" + ((VarInsnNode) node).var;
        if (node instanceof MethodInsnNode) {
            MethodInsnNode m = (MethodInsnNode) node;
            return m.owner.replace('/', '.') + "." + m.name + m.desc;
        }
        if (node instanceof FieldInsnNode) {
            FieldInsnNode f = (FieldInsnNode) node;
            return f.owner.replace('/', '.') + "." + f.name;
        }
        if (node instanceof TypeInsnNode) return "type:" + ((TypeInsnNode) node).type;
        if (node instanceof LdcInsnNode) return "ldc:" + ((LdcInsnNode) node).cst;
        if (node instanceof JumpInsnNode) return "jump";
        if (node instanceof TableSwitchInsnNode) return "tableswitch";
        if (node instanceof LookupSwitchInsnNode) return "lookupswitch";
        return "op:" + node.getOpcode();
    }

    private static String returnTypeFromDescriptor(String desc) {
        int parenClose = desc.lastIndexOf(')');
        if (parenClose < 0) return "V";
        return desc.substring(parenClose + 1);
    }

    private static String paramTypeAt(String desc, int index) {
        // Simplified: parse descriptor to get parameter types
        int parenOpen = desc.indexOf('(');
        if (parenOpen < 0) return "I";
        String params = desc.substring(parenOpen + 1, desc.lastIndexOf(')'));
        // Very simplified parsing - in production this would use a proper parser
        return "I";
    }

    private static int[] computeParamSlots(int accessFlags, String descriptor) {
        boolean isStatic = (accessFlags & Opcodes.ACC_STATIC) != 0;
        int count = countParams(descriptor);
        int[] slots = new int[count];
        for (int i = 0; i < count; i++) {
            slots[i] = isStatic ? i : i + 1;
        }
        return slots;
    }

    private static int countParams(String desc) {
        int parenOpen = desc.indexOf('(');
        if (parenOpen < 0) return 0;
        String params = desc.substring(parenOpen + 1, desc.lastIndexOf(')'));
        int count = 0;
        for (int i = 0; i < params.length(); i++) {
            char c = params.charAt(i);
            if (c == '[') continue;
            if (c == 'L') { i = params.indexOf(';', i); continue; }
            count++;
        }
        return count;
    }

    public List<IngestError> getErrors() { return errors; }
}
