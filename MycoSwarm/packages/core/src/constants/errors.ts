export enum ErrorCode {
  // General
  UNKNOWN = 'UNKNOWN',
  NOT_IMPLEMENTED = 'NOT_IMPLEMENTED',
  INVALID_PARAMETER = 'INVALID_PARAMETER',
  TIMEOUT = 'TIMEOUT',

  // Connection
  CONNECTION_FAILED = 'CONNECTION_FAILED',
  CONNECTION_TIMEOUT = 'CONNECTION_TIMEOUT',
  PEER_NOT_FOUND = 'PEER_NOT_FOUND',
  ROOM_FULL = 'ROOM_FULL',
  ROOM_NOT_FOUND = 'ROOM_NOT_FOUND',

  // Execution
  EXECUTION_TIMEOUT = 'EXECUTION_TIMEOUT',
  EXECUTION_OOM = 'EXECUTION_OOM',
  EXECUTION_PANIC = 'EXECUTION_PANIC',
  INVALID_CODE = 'INVALID_CODE',
  SANDBOX_CRASH = 'SANDBOX_CRASH',

  // Crypto
  PROOF_VERIFICATION_FAILED = 'PROOF_VERIFICATION_FAILED',
  KEY_NOT_FOUND = 'KEY_NOT_FOUND',
  DECRYPTION_FAILED = 'DECRYPTION_FAILED',

  // Consensus
  PROPOSAL_EXPIRED = 'PROPOSAL_EXPIRED',
  INSUFFICIENT_QUORUM = 'INSUFFICIENT_QUORUM',
  VOTE_REJECTED = 'VOTE_REJECTED',

  // Token
  INSUFFICIENT_BALANCE = 'INSUFFICIENT_BALANCE',
  CHANNEL_CLOSED = 'CHANNEL_CLOSED',
}

export class FungalError extends Error {
  public code: ErrorCode;
  public details?: Record<string, unknown>;

  constructor(code: ErrorCode, message: string, details?: Record<string, unknown>) {
    super(message);
    this.name = 'FungalError';
    this.code = code;
    this.details = details;
  }

  static timeout(msg = 'Operation timed out'): FungalError {
    return new FungalError(ErrorCode.TIMEOUT, msg);
  }

  static connectionFailed(msg = 'Connection failed'): FungalError {
    return new FungalError(ErrorCode.CONNECTION_FAILED, msg);
  }

  static invalidParameter(param: string, value: unknown): FungalError {
    return new FungalError(
      ErrorCode.INVALID_PARAMETER,
      `Invalid parameter ${param}: ${String(value)}`,
    );
  }
}