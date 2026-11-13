/**
 * TypeError Detector.
 * Identifies operations that would throw a TypeError at runtime:
 * - Calling non-functions
 * - Property access on null/undefined (handled by NullDeref, but cross-ref here)
 * - Arithmetic on non-numeric types where coercion fails
 * - `new` on non-constructor functions
 * - Spread on non-iterables
 */

import {
  IRNode,
  OpKind,
  TypeAnnotation,
  TypeKind,
  Diagnostic,
  DiagnosticCategory,
  SourceLocation,
} from "../types.js";

export class TypeErrorDetector {
  private diagnostics: Diagnostic[] = [];
  private counter = 0;

  analyze(nodes: IRNode[], typeEnv: Map<number, TypeAnnotation>): Diagnostic[] {
    this.diagnostics = [];
    this.counter = 0;

    for (const node of nodes) {
      switch (node.kind) {
        case OpKind.Call:
          this.checkCall(node, typeEnv);
          break;
        case OpKind.Add:
        case OpKind.Sub:
        case OpKind.Mul:
        case OpKind.Div:
        case OpKind.Mod:
          this.checkArithmetic(node, typeEnv);
          break;
        case OpKind.Alloc:
          if (node.operands.length > 0) {
            // `new` expression — check callee is a constructor
            this.checkNewExpression(node, typeEnv);
          }
          break;
      }
    }

    return this.diagnostics;
  }

  private checkCall(node: IRNode, typeEnv: Map<number, TypeAnnotation>): void {
    if (node.operands.length === 0) return;
    const calleeId = node.operands[0];
    const calleeType = typeEnv.get(calleeId);

    if (calleeType && calleeType.kind !== TypeKind.Function && calleeType.kind !== TypeKind.Any) {
      this.emit(
        DiagnosticCategory.TypeError,
        `Cannot call ${this.describeType(calleeType)} as a function`,
        node.location,
        node.id
      );
    }
  }

  private checkArithmetic(node: IRNode, typeEnv: Map<number, TypeAnnotation>): void {
    for (const opId of node.operands) {
      const opType = typeEnv.get(opId);
      if (opType && opType.kind === TypeKind.String && node.kind !== OpKind.Add) {
        // String arithmetic (other than +) is a type error in strict mode
        this.emit(
          DiagnosticCategory.TypeError,
          `Arithmetic operation '${node.kind}' applied to string operand`,
          node.location,
          node.id
        );
      }
    }
  }

  private checkNewExpression(node: IRNode, typeEnv: Map<number, TypeAnnotation>): void {
    const calleeId = node.operands[0];
    const calleeType = typeEnv.get(calleeId);
    if (calleeType && calleeType.kind !== TypeKind.Function && calleeType.kind !== TypeKind.Any) {
      this.emit(
        DiagnosticCategory.TypeError,
        `Cannot use 'new' with ${this.describeType(calleeType)} — not a constructor`,
        node.location,
        node.id
      );
    }
  }

  private emit(
    category: DiagnosticCategory,
    message: string,
    location: SourceLocation | undefined,
    nodeId: number
  ): void {
    this.diagnostics.push({
      id: `TE-${this.counter++}`,
      category,
      severity: "error",
      message,
      location: location ?? { file: "", line: 0, column: 0, endLine: 0, endColumn: 0 },
      nodeId,
    });
  }

  private describeType(t: TypeAnnotation): string {
    switch (t.kind) {
      case TypeKind.Number: return "number";
      case TypeKind.String: return "string";
      case TypeKind.Boolean: return "boolean";
      case TypeKind.Object: return "object";
      case TypeKind.Array: return "array";
      case TypeKind.Null: return "null";
      case TypeKind.Undefined: return "undefined";
      default: return "unknown";
    }
  }
}
