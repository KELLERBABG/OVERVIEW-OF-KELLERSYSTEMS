/**
 * Lightweight AST node types for the TS/JS parser.
 * These mirror a subset of the TypeScript Compiler API nodes
 * that are relevant to the lowering pipeline.
 */

export interface SourceFile {
  kind: "SourceFile";
  fileName: string;
  statements: Statement[];
}

export interface Statement {
  kind: StatementKind;
  loc: SourceLoc;
  [key: string]: unknown;
}

export enum StatementKind {
  VariableDeclaration = "VariableDeclaration",
  FunctionDeclaration = "FunctionDeclaration",
  ClassDeclaration = "ClassDeclaration",
  ImportDeclaration = "ImportDeclaration",
  ExportDeclaration = "ExportDeclaration",
  ExpressionStatement = "ExpressionStatement",
  IfStatement = "IfStatement",
  WhileStatement = "WhileStatement",
  ForStatement = "ForStatement",
  ForOfStatement = "ForOfStatement",
  ReturnStatement = "ReturnStatement",
  TryStatement = "TryStatement",
}

export interface SourceLoc {
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
}

// ─── Expressions ───────────────────────────────────────────────────────────────

export interface Expression {
  kind: ExprKind;
  loc: SourceLoc;
  [key: string]: unknown;
}

export enum ExprKind {
  Identifier = "Identifier",
  NumericLiteral = "NumericLiteral",
  StringLiteral = "StringLiteral",
  BooleanLiteral = "BooleanLiteral",
  NullLiteral = "NullLiteral",
  UndefinedLiteral = "UndefinedLiteral",
  ObjectLiteral = "ObjectLiteral",
  ArrayLiteral = "ArrayLiteral",
  PropertyAccess = "PropertyAccess",
  ElementAccess = "ElementAccess",
  CallExpression = "CallExpression",
  BinaryExpression = "BinaryExpression",
  UnaryExpression = "UnaryExpression",
  ConditionalExpression = "ConditionalExpression",
  AssignmentExpression = "AssignmentExpression",
  NewExpression = "NewExpression",
  ArrowFunction = "ArrowFunction",
  FunctionExpression = "FunctionExpression",
  TemplateLiteral = "TemplateLiteral",
  TypeAssertion = "TypeAssertion",
  AsExpression = "AsExpression",
  SpreadElement = "SpreadElement",
}

// ─── Specific Node Shapes ──────────────────────────────────────────────────────

export interface IdentifierNode extends Expression {
  kind: ExprKind.Identifier;
  name: string;
}

export interface NumericLiteralNode extends Expression {
  kind: ExprKind.NumericLiteral;
  value: number;
}

export interface StringLiteralNode extends Expression {
  kind: ExprKind.StringLiteral;
  value: string;
}

export interface BooleanLiteralNode extends Expression {
  kind: ExprKind.BooleanLiteral;
  value: boolean;
}

export interface NullLiteralNode extends Expression {
  kind: ExprKind.NullLiteral;
}

export interface UndefinedLiteralNode extends Expression {
  kind: ExprKind.UndefinedLiteral;
}

export interface ObjectLiteralNode extends Expression {
  kind: ExprKind.ObjectLiteral;
  properties: PropertyAssignment[];
}

export interface PropertyAssignment {
  name: string;
  value: Expression;
  isShorthand: boolean;
}

export interface ArrayLiteralNode extends Expression {
  kind: ExprKind.ArrayLiteral;
  elements: Expression[];
}

export interface PropertyAccessNode extends Expression {
  kind: ExprKind.PropertyAccess;
  object: Expression;
  propertyName: string;
}

export interface ElementAccessNode extends Expression {
  kind: ExprKind.ElementAccess;
  object: Expression;
  index: Expression;
}

export interface CallExpressionNode extends Expression {
  kind: ExprKind.CallExpression;
  callee: Expression;
  arguments: Expression[];
}

export interface BinaryExpressionNode extends Expression {
  kind: ExprKind.BinaryExpression;
  left: Expression;
  operator: string;
  right: Expression;
}

export interface UnaryExpressionNode extends Expression {
  kind: ExprKind.UnaryExpression;
  operator: string;
  operand: Expression;
}

export interface ConditionalExpressionNode extends Expression {
  kind: ExprKind.ConditionalExpression;
  condition: Expression;
  consequent: Expression;
  alternate: Expression;
}

export interface AssignmentExpressionNode extends Expression {
  kind: ExprKind.AssignmentExpression;
  left: Expression;
  operator: string;
  right: Expression;
}

export interface NewExpressionNode extends Expression {
  kind: ExprKind.NewExpression;
  expression: Expression;
  arguments: Expression[];
}

// ─── Declarations ──────────────────────────────────────────────────────────────

export interface VariableDeclarationNode {
  name: string;
  typeAnnotation?: string;
  initializer?: Expression;
  isConst: boolean;
  isLet: boolean;
}

export interface FunctionDeclarationNode {
  name: string;
  params: ParamDecl[];
  body: Statement[];
  returnTypeAnnotation?: string;
}

export interface ParamDecl {
  name: string;
  typeAnnotation?: string;
  defaultValue?: Expression;
}

export interface ClassDeclarationNode {
  name: string;
  extendsExpr?: Expression;
  implementsExprs?: Expression[];
  members: ClassMember[];
}

export interface ClassMember {
  kind: "method" | "property" | "getter" | "setter";
  name: string;
  params?: ParamDecl[];
  body?: Statement[];
  initializer?: Expression;
}
