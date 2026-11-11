import { describe, it, expect } from 'vitest';
import { FungalError, ErrorCode } from '../src/constants/errors.js';

describe('FungalError', () => {
  it('should create error with code and message', () => {
    const err = new FungalError(ErrorCode.TIMEOUT, 'Operation timed out');
    expect(err.code).toBe(ErrorCode.TIMEOUT);
    expect(err.message).toBe('Operation timed out');
    expect(err.name).toBe('FungalError');
  });

  it('should create timeout error via static method', () => {
    const err = FungalError.timeout();
    expect(err.code).toBe(ErrorCode.TIMEOUT);
  });

  it('should create connection failed error via static method', () => {
    const err = FungalError.connectionFailed('Failed to connect to peer');
    expect(err.code).toBe(ErrorCode.CONNECTION_FAILED);
    expect(err.message).toBe('Failed to connect to peer');
  });

  it('should create invalid parameter error via static method', () => {
    const err = FungalError.invalidParameter('port', -1);
    expect(err.code).toBe(ErrorCode.INVALID_PARAMETER);
    expect(err.message).toContain('port');
    expect(err.message).toContain('-1');
  });

  it('should support optional details', () => {
    const err = new FungalError(ErrorCode.ROOM_FULL, 'Room is full', { maxPeers: 50 });
    expect(err.details).toEqual({ maxPeers: 50 });
  });
});