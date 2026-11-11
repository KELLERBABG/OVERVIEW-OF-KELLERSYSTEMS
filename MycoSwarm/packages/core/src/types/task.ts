import type { NodeId } from './node.js';

export type TaskId = string;
export type WorkUnitId = string;

export enum TaskPriority {
  LOW = 0,
  MEDIUM = 1,
  HIGH = 2,
  CRITICAL = 3,
}

export enum TaskStatus {
  PENDING = 'pending',
  SPLITTING = 'splitting',
  DISTRIBUTING = 'distributing',
  EXECUTING = 'executing',
  VERIFYING = 'verifying',
  COMPLETED = 'completed',
  FAILED = 'failed',
  TIMEOUT = 'timeout',
}

export interface Task {
  id: TaskId;
  /** Wasm bytecode or JS function reference */
  code: string | Uint8Array;
  /** Input data (structured clone compatible) */
  input: unknown;
  /** Maximum wall-clock time for the entire task (ms) */
  deadline: number;
  priority: TaskPriority;
  /** Expected output schema (for verification) */
  outputSchema?: string;
  /** Whether this task requires zk-SNARK proof */
  requireProof: boolean;
  /** Whether input must be homomorphically encrypted */
  requireEncryption: boolean;
  createdAt: number;
  ownerId: NodeId;
  /** Input type hint for splitter */
  inputType?: 'array' | 'range' | 'recursive' | 'single';
}

export interface WorkUnit {
  id: WorkUnitId;
  taskId: TaskId;
  /** Subset of the original input for this unit */
  input: unknown;
  /** Indices/ranges this unit covers (for reassembly) */
  range: [number, number];
  /** Size estimate in bytes */
  estimatedSize: number;
  /** Individual deadline (derived from task deadline) */
  deadline: number;
  priority: TaskPriority;
  /** Number of redundant copies dispatched */
  redundancyFactor: number;
}

export interface TaskResult {
  workUnitId: WorkUnitId;
  taskId: TaskId;
  nodeId: NodeId;
  output: unknown;
  /** Execution time in ms */
  executionTime: number;
  /** Wasm memory used (bytes) */
  memoryUsed: number;
  /** zk-SNARK proof if requireProof was true */
  proof?: Uint8Array;
  /** Error message if failed */
  error?: string;
  completedAt: number;
}