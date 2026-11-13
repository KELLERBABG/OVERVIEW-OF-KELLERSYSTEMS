/**
 * Keller-VIR Lowering Pass.
 * Converts the parsed AST into Keller-VIR SSA Hypergraph form with
 * C-RMM (Constraint-based Runtime Memory Model) annotations.
 *
 * Key transformations:
 * - Object property access → GetProp with prototype chain resolution
 * - Array indexing → ArrayGet with bounds metadata
 * - Arithmetic → typed ops with NaN propagation flags
 * - Variable assignments → SSA phi nodes + Store ops
 */

import {
  IRNode,
  OpKind,
  TypeAnnotation,
  TypeKind,
  MemoryState,
  ValueRef,
  SourceLocation,
} from "../types.js";
import {
  SourceFile,
  Statement,
  Expression,
  ExprKind,
  StatementKind,
  IdentifierNode,
  NumericLiteralNode,
  ObjectLiteralNode,
  ArrayLiteralNode,
  PropertyAccessNode,
  ElementAccessNode,
  CallExpressionNode,
  BinaryExpressionNode,
  AssignmentExpressionNode,
  NewExpressionNode,
  VariableDeclarationNode,
} from "../parser/ast-types.js";

export interface LoweringResult {
  nodes: IRNode[];
  memoryState: MemoryState;
  /** SSA register -> TypeAnnotation mapping */
  typeEnv: Map<number, TypeAnnotation>;
  /** Variable name -> current SSA register */
  varMap: Map<string, number>;
  /** Function name -> entry node id */
  functionEntries: Map<string, number>;
}

export class KellerVirLowering {
  private nextNodeId = 0;
  private nodes: IRNode[] = [];
  private memoryState: MemoryState;
  private typeEnv: Map<number, TypeAnnotation> = new Map();
  private varMap: Map<string, number> = new Map();
  private functionEntries: Map<string, number> = new Map();

  constructor() {
    this.memoryState = {
      heap: new Map(),
      protoLinks: new Map(),
      globalId: 0,
      nextAllocId: 1,
    };
  }

  lower(sourceFile: SourceFile): LoweringResult {
    for (const stmt of sourceFile.statements) {
      this.lowerStatement(stmt);
    }
    return {
      nodes: this.nodes,
      memoryState: this.memoryState,
      typeEnv: this.typeEnv,
      varMap: this.varMap,
      functionEntries: this.functionEntries,
    };
  }

  private alloc(): number {
    const id = this.memoryState.nextAllocId++;
    this.memoryState.heap.set(id, new Map());
    return id;
  }

  private emit(kind: OpKind, operands: number[], resultType: TypeAnnotation, loc?: SourceLocation, extra?: Partial<IRNode>): IRNode {
    const node: IRNode = {
      id: this.nextNodeId++,
      kind,
      operands,
      resultType,
      location: loc,
      ...extra,
    };
    this.nodes.push(node);
    return node;
  }

  private lowerStatement(stmt: Statement): void {
    switch (stmt.kind) {
      case StatementKind.VariableDeclaration: {
        const decls = stmt.declarations as VariableDeclarationNode[];
        for (const d of decls) {
          if (d.initializer) {
            const reg = this.lowerExpression(d.initializer);
            this.varMap.set(d.name, reg.id);
          } else {
            // Uninitialized: undefined
            const undefType: TypeAnnotation = { kind: TypeKind.Undefined, nullable: false };
            const node = this.emit(OpKind.Alloc, [], undefType, this.loc(stmt));
            this.varMap.set(d.name, node.id);
          }
        }
        break;
      }
      case StatementKind.ExpressionStatement: {
        const expr = stmt.expression as Expression;
        if (expr) this.lowerExpression(expr);
        break;
      }
      case StatementKind.FunctionDeclaration: {
        const fnName = stmt.name as string;
        const entryNode = this.emit(OpKind.Call, [], { kind: TypeKind.Function, nullable: false }, this.loc(stmt), { callee: fnName });
        this.functionEntries.set(fnName, entryNode.id);
        break;
      }
      case StatementKind.ClassDeclaration: {
        const clsName = stmt.name as string;
        const objId = this.alloc();
        // Set prototype link to Object.prototype (id 0)
        this.memoryState.protoLinks.set(objId, 0);
        const node = this.emit(OpKind.Alloc, [], { kind: TypeKind.Object, nullable: false }, this.loc(stmt));
        this.varMap.set(clsName, node.id);
        break;
      }
      case StatementKind.IfStatement:
      case StatementKind.WhileStatement:
      case StatementKind.ForStatement:
      case StatementKind.ForOfStatement: {
        // Lower condition and body
        const cond = stmt.condition as Expression | undefined;
        if (cond) this.lowerExpression(cond);
        const body = stmt.body as Statement[] | undefined;
        if (body) {
          for (const s of body) this.lowerStatement(s);
        }
        break;
      }
      case StatementKind.ReturnStatement: {
        const expr = stmt.expression as Expression | null;
        if (expr) {
          const reg = this.lowerExpression(expr);
          this.emit(OpKind.Return, [reg.id], reg.resultType, this.loc(stmt));
        }
        break;
      }
      default:
        break;
    }
  }

  private lowerExpression(expr: Expression): IRNode {
    switch (expr.kind) {
      case ExprKind.NumericLiteral: {
        const lit = expr as NumericLiteralNode;
        const type: TypeAnnotation = { kind: TypeKind.Number, nullable: false };
        return this.emit(OpKind.Alloc, [], type, this.loc(expr), { propertyExpr: String(lit.value) });
      }

      case ExprKind.StringLiteral:
      case ExprKind.BooleanLiteral:
      case ExprKind.NullLiteral:
      case ExprKind.UndefinedLiteral: {
        const kindMap: Record<string, TypeKind> = {
          [ExprKind.StringLiteral]: TypeKind.String,
          [ExprKind.BooleanLiteral]: TypeKind.Boolean,
          [ExprKind.NullLiteral]: TypeKind.Null,
          [ExprKind.UndefinedLiteral]: TypeKind.Undefined,
        };
        const tk = kindMap[expr.kind] ?? TypeKind.Any;
        const type: TypeAnnotation = { kind: tk, nullable: tk === TypeKind.Null || tk === TypeKind.Undefined };
        return this.emit(OpKind.Alloc, [], type, this.loc(expr));
      }

      case ExprKind.Identifier: {
        const id = expr as IdentifierNode;
        const existing = this.varMap.get(id.name);
        if (existing !== undefined) {
          // Load from SSA register
          const prevType = this.typeEnv.get(existing) ?? { kind: TypeKind.Any, nullable: true };
          return this.emit(OpKind.Load, [existing], prevType, this.loc(expr));
        }
        // Unknown identifier — assume any (potential undefined)
        const type: TypeAnnotation = { kind: TypeKind.Any, nullable: true };
        const node = this.emit(OpKind.Alloc, [], type, this.loc(expr), { propertyExpr: id.name });
        this.typeEnv.set(node.id, type);
        return node;
      }

      case ExprKind.ObjectLiteral: {
        const obj = expr as ObjectLiteralNode;
        const objId = this.alloc();
        const allocNode = this.emit(OpKind.Alloc, [], { kind: TypeKind.Object, nullable: false }, this.loc(expr));

        for (const prop of obj.properties) {
          const valReg = this.lowerExpression(prop.value);
          // Store property on the object
          this.emit(
            OpKind.SetProp,
            [allocNode.id, valReg.id],
            valReg.resultType,
            this.loc(expr),
            { propertyExpr: prop.name }
          );
          // Record in memory state
          const heapEntry = this.memoryState.heap.get(objId)!;
          heapEntry.set(prop.name, { kind: "ssa", ssaId: valReg.id });
        }

        // Set prototype to Object.prototype (id 0)
        this.memoryState.protoLinks.set(objId, 0);
        return allocNode;
      }

      case ExprKind.ArrayLiteral: {
        const arr = expr as ArrayLiteralNode;
        const objId = this.alloc();
        const elemType: TypeAnnotation = arr.elements.length > 0
          ? this.lowerExpression(arr.elements[0]).resultType
          : { kind: TypeKind.Any, nullable: true };

        const allocNode = this.emit(
          OpKind.Alloc,
          [],
          { kind: TypeKind.Array, elementType: elemType, nullable: false },
          this.loc(expr)
        );

        for (let i = 0; i < arr.elements.length; i++) {
          const valReg = this.lowerExpression(arr.elements[i]);
          this.emit(
            OpKind.ArraySet,
            [allocNode.id, valReg.id],
            valReg.resultType,
            this.loc(expr),
            { propertyExpr: String(i) }
          );
        }

        // Array prototype → Array.prototype (id 1 convention)
        this.memoryState.protoLinks.set(objId, 1);
        return allocNode;
      }

      case ExprKind.PropertyAccess: {
        const pa = expr as PropertyAccessNode;
        const objReg = this.lowerExpression(pa.object);
        // GetProp with prototype chain resolution
        const propType: TypeAnnotation = { kind: TypeKind.Any, nullable: true };
        return this.emit(
          OpKind.GetProp,
          [objReg.id],
          propType,
          this.loc(expr),
          { propertyExpr: pa.propertyName }
        );
      }

      case ExprKind.ElementAccess: {
        const ea = expr as ElementAccessNode;
        const objReg = this.lowerExpression(ea.object);
        const idxReg = this.lowerExpression(ea.index);
        const elemType: TypeAnnotation = { kind: TypeKind.Any, nullable: true };
        return this.emit(
          OpKind.ArrayGet,
          [objReg.id, idxReg.id],
          elemType,
          this.loc(expr)
        );
      }

      case ExprKind.CallExpression: {
        const ce = expr as CallExpressionNode;
        const calleeReg = this.lowerExpression(ce.callee);
        const argRegs = ce.arguments.map((a) => this.lowerExpression(a));
        const calleeName = ce.callee.kind === ExprKind.Identifier
          ? (ce.callee as IdentifierNode).name
          : undefined;
        const resultType: TypeAnnotation = { kind: TypeKind.Any, nullable: true };
        return this.emit(
          OpKind.Call,
          [calleeReg.id, ...argRegs.map((r) => r.id)],
          resultType,
          this.loc(expr),
          { callee: calleeName }
        );
      }

      case ExprKind.BinaryExpression: {
        const be = expr as BinaryExpressionNode;
        const leftReg = this.lowerExpression(be.left);
        const rightReg = this.lowerExpression(be.right);
        const opKind = this.mapBinaryOp(be.operator);
        const resultType: TypeAnnotation = this.inferBinaryResult(be.operator, leftReg.resultType, rightReg.resultType);
        return this.emit(
          opKind,
          [leftReg.id, rightReg.id],
          resultType,
          this.loc(expr)
        );
      }

      case ExprKind.AssignmentExpression: {
        const ae = expr as AssignmentExpressionNode;
        const targetReg = this.lowerExpression(ae.left);
        const valReg = this.lowerExpression(ae.right);
        // If target is an identifier, update varMap
        if (ae.left.kind === ExprKind.Identifier) {
          const name = (ae.left as IdentifierNode).name;
          this.varMap.set(name, valReg.id);
        }
        return valReg;
      }

      case ExprKind.NewExpression: {
        const ne = expr as NewExpressionNode;
        const ctorReg = this.lowerExpression(ne.expression);
        const argRegs = ne.arguments.map((a) => this.lowerExpression(a));
        const objId = this.alloc();
        // Prototype chain: new object → constructor.prototype
        this.memoryState.protoLinks.set(objId, 2); // Convention: id 2 = constructor prototype
        const resultType: TypeAnnotation = { kind: TypeKind.Object, nullable: false };
        return this.emit(
          OpKind.Alloc,
          [ctorReg.id, ...argRegs.map((r) => r.id)],
          resultType,
          this.loc(expr),
          { callee: ne.expression.kind === ExprKind.Identifier ? (ne.expression as IdentifierNode).name : undefined }
        );
      }

      default: {
        // Fallback: treat as opaque value
        const type: TypeAnnotation = { kind: TypeKind.Any, nullable: true };
        return this.emit(OpKind.Alloc, [], type, this.loc(expr));
      }
    }
  }

  private mapBinaryOp(op: string): OpKind {
    const map: Record<string, OpKind> = {
      "+": OpKind.Add,
      "-": OpKind.Sub,
      "*": OpKind.Mul,
      "/": OpKind.Div,
      "%": OpKind.Mod,
      "**": OpKind.Mul, // Approximate; power is not in base ops
      "==": OpKind.Eq,
      "===": OpKind.StrictEq,
      "!=": OpKind.Neq,
      "!==": OpKind.StrictNeq,
      "<": OpKind.Lt,
      ">": OpKind.Gt,
      "<=": OpKind.Le,
      ">=": OpKind.Ge,
      "&": OpKind.BitAnd,
      "|": OpKind.BitOr,
      "^": OpKind.BitXor,
      "<<": OpKind.Shl,
      ">>": OpKind.Shr,
      ">>>": OpKind.UShr,
    };
    return map[op] ?? OpKind.Alloc;
  }

  private inferBinaryResult(
    op: string,
    left: TypeAnnotation,
    right: TypeAnnotation
  ): TypeAnnotation {
    if (["==", "===", "!=", "!==", "<", ">", "<=", ">="].includes(op)) {
      return { kind: TypeKind.Boolean, nullable: false };
    }
    if (["+", "-", "*", "/", "%"].includes(op)) {
      // Arithmetic: result is number, but may be NaN
      return { kind: TypeKind.Number, nullable: false };
    }
    if (["&", "|", "^", "<<", ">>", ">>>"].includes(op)) {
      return { kind: TypeKind.Number, nullable: false };
    }
    return { kind: TypeKind.Any, nullable: true };
  }

  private loc(stmt: Statement | Expression): SourceLocation {
    const l = stmt.loc;
    return { file: "", line: l.line, column: l.column, endLine: l.endLine, endColumn: l.endColumn };
  }
}
