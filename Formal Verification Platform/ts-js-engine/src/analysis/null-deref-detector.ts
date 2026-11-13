/**
 * Null/Undefined Dereference Detector.
 * Tracks dataflow to detect property access, method calls, or element access
 * on values that may be null or undefined at the point of use.
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

export class NullDerefDetector {
  private diagnostics: Diagnostic[] = [];
  private counter = 0;
  /** Track which SSA registers are known to be null/undefined */
  private nullSet: Set<number> = new Set();
  private undefinedSet: Set<number> = new Set();

  analyze(nodes: IRNode[], typeEnv: Map<number, TypeAnnotation>): Diagnostic[] {
    this.diagnostics = [];
    this.counter = 0;
    this.nullSet = new Set();
    this.undefinedSet = new Set();

    // First pass: identify null/undefined sources
    for (const node of nodes) {
      if (node.resultType.kind === TypeKind.Null) {
        this.nullSet.add(node.id);
      }
      if (node.resultType.kind === TypeKind.Undefined) {
        this.undefinedSet.add(node.id);
      }
      // Track nullable types
      if (node.resultType.nullable && node.resultType.kind !== TypeKind.Any) {
        // Mark as potentially null/undefined
        this.nullSet.add(node.id);
      }
    }

    // Second pass: check dereferences
    for (const node of nodes) {
      switch (node.kind) {
        case OpKind.GetProp:
          this.checkDeref(node, "property access");
          break;
        case OpKind.ArrayGet:
          this.checkDeref(node, "element access");
          break;
        case OpKind.Call:
          if (node.operands.length > 0) {
            this.checkDerefOperand(node, node.operands[0], "function call");
          }
          break;
      }
    }

    return this.diagnostics;
  }

  private checkDeref(node: IRNode, operation: string): void {
    if (node.operands.length === 0) return;
    const objId = node.operands[0];
    this.checkDerefOperand(node, objId, operation);
  }

  private checkDerefOperand(node: IRNode, targetId: number, operation: string): void {
    if (this.nullSet.has(targetId)) {
      this.emit(
        DiagnosticCategory.NullDeref,
        `Possible null dereference: ${operation} on value that may be null`,
        node.location,
        node.id
      );
    }
    if (this.undefinedSet.has(targetId)) {
      this.emit(
        DiagnosticCategory.UndefinedDeref,
        `Possible undefined dereference: ${operation} on value that may be undefined`,
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
      id: `${category === DiagnosticCategory.NullDeref ? "ND" : "UD"}-${this.counter++}`,
      category,
      severity: "error",
      message,
      location: location ?? { file: "", line: 0, column: 0, endLine: 0, endColumn: 0 },
      nodeId,
    });
  }
}
