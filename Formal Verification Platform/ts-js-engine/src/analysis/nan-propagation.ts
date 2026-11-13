/**
 * NaN Propagation Analyzer.
 * Tracks arithmetic operations that can produce NaN and propagates
 * the NaN possibility through subsequent computations.
 *
 * Sources of NaN:
 * - 0 / 0, Infinity - Infinity, 0 * Infinity
 * - Arithmetic on NaN (any op with NaN operand → NaN)
 * - Number("") → 0, Number("abc") → NaN
 * - Math operations on non-finite values
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

export class NaNPropagationAnalyzer {
  private diagnostics: Diagnostic[] = [];
  private counter = 0;
  /** SSA registers that may hold NaN */
  private nanSet: Set<number> = new Set();

  analyze(nodes: IRNode[], typeEnv: Map<number, TypeAnnotation>): Diagnostic[] {
    this.diagnostics = [];
    this.counter = 0;
    this.nanSet = new Set();

    for (const node of nodes) {
      switch (node.kind) {
        case OpKind.Div:
          this.checkDivision(node);
          break;
        case OpKind.Mul:
          this.checkMultiplication(node);
          break;
        case OpKind.Add:
        case OpKind.Sub:
          this.checkArithmetic(node);
          break;
      }
    }

    return this.diagnostics;
  }

  private checkDivision(node: IRNode): void {
    // Division can produce NaN if both operands are 0, or if either is NaN/Infinity
    const [aId, bId] = node.operands;
    if (aId === undefined || bId === undefined) return;

    // If either operand is in nanSet, result is definitely NaN
    if (this.nanSet.has(aId) || this.nanSet.has(bId)) {
      this.nanSet.add(node.id);
      this.emit(
        `NaN propagation: division with NaN operand produces NaN`,
        node.location,
        node.id
      );
      return;
    }

    // 0/0 is a potential NaN source (warning level)
    this.nanSet.add(node.id);
    this.emit(
      `Division may produce NaN (0/0 or Infinity/Infinity)`,
      node.location,
      node.id,
      "warning"
    );
  }

  private checkMultiplication(node: IRNode): void {
    const [aId, bId] = node.operands;
    if (aId === undefined || bId === undefined) return;

    if (this.nanSet.has(aId) || this.nanSet.has(bId)) {
      this.nanSet.add(node.id);
      this.emit(
        `NaN propagation: multiplication with NaN operand produces NaN`,
        node.location,
        node.id
      );
    }
  }

  private checkArithmetic(node: IRNode): void {
    for (const opId of node.operands) {
      if (this.nanSet.has(opId)) {
        this.nanSet.add(node.id);
        this.emit(
          `NaN propagation: '${node.kind}' with NaN operand produces NaN`,
          node.location,
          node.id
        );
        return;
      }
    }
  }

  private emit(
    message: string,
    location: SourceLocation | undefined,
    nodeId: number,
    severity: "error" | "warning" = "warning"
  ): void {
    this.diagnostics.push({
      id: `NAN-${this.counter++}`,
      category: DiagnosticCategory.NaNPropagation,
      severity,
      message,
      location: location ?? { file: "", line: 0, column: 0, endLine: 0, endColumn: 0 },
      nodeId,
    });
  }
}
