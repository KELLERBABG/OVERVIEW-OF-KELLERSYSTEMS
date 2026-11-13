/**
 * Core type definitions for the Keller-VIR TS/JS Frontend Engine.
 * Defines the intermediate representation (IR) nodes, analysis results,
 * and configuration types used throughout the pipeline.
 */

// ─── Source Location ───────────────────────────────────────────────────────────

export interface SourceLocation {
  file: string;
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
}

// ─── Keller-VIR SSA Hypergraph Nodes ──────────────────────────────────────────

export enum OpKind {
  // Arithmetic
  Add = "add",
  Sub = "sub",
  Mul = "mul",
  Div = "div",
  Mod = "mod",
  BitAnd = "bit_and",
  BitOr = "bit_or",
  BitXor = "bit_xor",
  Shl = "shl",
  Shr = "shr",
  UShr = "ushr",

  // Comparison
  Eq = "eq",
  Neq = "neq",
  Lt = "lt",
  Gt = "gt",
  Le = "le",
  Ge = "ge",
  StrictEq = "strict_eq",
  StrictNeq = "strict_neq",

  // Object / Property
  GetProp = "get_prop",
  SetProp = "set_prop",
  DeleteProp = "delete_prop",
  InProp = "in_prop",
  GetProto = "get_proto",
  HasProto = "has_proto",

  // Array
  ArrayGet = "array_get",
  ArraySet = "array_set",
  ArrayLength = "array_length",
  ArrayPush = "array_push",
  ArrayPop = "array_pop",

  // Control Flow
  Branch = "branch",
  Phi = "phi",
  Return = "return",
  Call = "call",

  // Memory (C-RMM)
  Alloc = "alloc",
  Load = "load",
  Store = "store",
  Free = "free",

  // Type Guards
  TypeOf = "typeof",
  InstanceOf = "instanceof",
  NullCheck = "null_check",
  UndefinedCheck = "undefined_check",
}

export interface IRNode {
  id: number;
  kind: OpKind;
  operands: number[];
  resultType: TypeAnnotation;
  location?: SourceLocation;
  /** For GetProp/ArrayGet: the property name or index expression */
  propertyExpr?: string | number;
  /** For Call: callee identifier */
  callee?: string;
  /** SSA version counter for memory locations */
  version?: number;
}

export interface TypeAnnotation {
  kind: TypeKind;
  /** For object types: property map */
  properties?: Map<string, TypeAnnotation>;
  /** For array types: element type */
  elementType?: TypeAnnotation;
  /** Whether the type includes null/undefined */
  nullable: boolean;
}

export enum TypeKind {
  Number = "number",
  String = "string",
  Boolean = "boolean",
  Object = "object",
  Array = "array",
  Null = "null",
  Undefined = "undefined",
  Any = "any",
  Never = "never",
  Function = "function",
}

// ─── C-RMM (Constraint-based Runtime Memory Model) ────────────────────────────

export interface MemoryState {
  /** Heap: object id -> property map */
  heap: Map<number, Map<string, ValueRef>>;
  /** Prototype chains: object id -> prototype object id */
  protoLinks: Map<number, number>;
  /** Global object id */
  globalId: number;
  /** Next allocation counter */
  nextAllocId: number;
}

export interface ValueRef {
  kind: "const" | "ssa" | "heap";
  /** For const: literal value */
  value?: unknown;
  /** For ssa: SSA register id */
  ssaId?: number;
  /** For heap: (objectId, property) */
  objectId?: number;
  property?: string;
}

// ─── Analysis Results ──────────────────────────────────────────────────────────

export type Severity = "error" | "warning" | "info";

export interface Diagnostic {
  id: string;
  category: DiagnosticCategory;
  severity: Severity;
  message: string;
  location: SourceLocation;
  /** The IR node that triggered the diagnostic */
  nodeId?: number;
  /** Suggested fix (not applied in audit mode) */
  suggestion?: string;
}

export enum DiagnosticCategory {
  TypeError = "TYPE_ERROR",
  NullDeref = "NULL_DEREF",
  UndefinedDeref = "UNDEFINED_DEREF",
  NaNPropagation = "NAN_PROPAGATION",
  ArrayOutOfBounds = "ARRAY_OOB",
  PrototypeChainViolation = "PROTO_CHAIN_VIOLATION",
}

export interface AnalysisResult {
  diagnostics: Diagnostic[];
  /** Summary counts by category */
  summary: Map<DiagnosticCategory, number>;
  /** Total nodes analyzed */
  nodeCount: number;
  /** Execution time in ms */
  elapsedMs: number;
}

// ─── Test Synthesis ────────────────────────────────────────────────────────────

export interface SynthesizedTest {
  name: string;
  framework: "jest" | "vitest";
  code: string;
  /** The diagnostic this test is designed to catch */
  targetDiagnostic?: Diagnostic;
}

// ─── Audit Report ──────────────────────────────────────────────────────────────

export interface AuditReport {
  project: string;
  timestamp: string;
  mode: "audit";
  filesAnalyzed: number;
  totalDiagnostics: number;
  resultsByFile: Map<string, AnalysisResult>;
  synthesizedTests: SynthesizedTest[];
  /** Keller-VIR IR statistics */
  irStats: {
    totalNodes: number;
    ssaRegisters: number;
    memoryAllocations: number;
    prototypeLinks: number;
  };
}

// ─── Engine Configuration ──────────────────────────────────────────────────────

export interface EngineConfig {
  mode: "audit" | "heal";
  targetDir: string;
  outputDir?: string;
  testFramework: "jest" | "vitest";
  /** Maximum file size to analyze (bytes) */
  maxFileSize: number;
  /** File extensions to include */
  extensions: string[];
  /** Enable prototype chain analysis */
  enableProtoAnalysis: boolean;
  /** Enable NaN propagation tracking */
  enableNaNTracking: boolean;
  /** Enable array bounds checking */
  enableArrayBounds: boolean;
}

export const DEFAULT_CONFIG: EngineConfig = {
  mode: "audit",
  targetDir: ".",
  outputDir: "./verification-report",
  testFramework: "vitest",
  maxFileSize: 1_000_000,
  extensions: [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"],
  enableProtoAnalysis: true,
  enableNaNTracking: true,
  enableArrayBounds: true,
};
