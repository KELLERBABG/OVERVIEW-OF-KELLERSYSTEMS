import { describe, it, expect } from 'vitest';
import { NodeStatus, NodeRole } from '../src/types/node.js';
import { TaskPriority, TaskStatus } from '../src/types/task.js';
import { MessageType } from '../src/types/message.js';
import { ProposalType } from '../src/types/consensus.js';

describe('NodeStatus enum', () => {
  it('should have expected values', () => {
    expect(NodeStatus.ONLINE).toBe('online');
    expect(NodeStatus.OFFLINE).toBe('offline');
    expect(NodeStatus.BUSY).toBe('busy');
    expect(NodeStatus.DEGRADED).toBe('degraded');
  });
});

describe('NodeRole enum', () => {
  it('should have expected values', () => {
    expect(NodeRole.LEAF).toBe('leaf');
    expect(NodeRole.RELAY).toBe('relay');
    expect(NodeRole.BOOTSTRAP).toBe('bootstrap');
  });
});

describe('TaskPriority enum', () => {
  it('should have numeric values ordered by priority', () => {
    expect(TaskPriority.LOW).toBe(0);
    expect(TaskPriority.MEDIUM).toBe(1);
    expect(TaskPriority.HIGH).toBe(2);
    expect(TaskPriority.CRITICAL).toBe(3);
  });
});

describe('TaskStatus enum', () => {
  it('should have expected values', () => {
    expect(TaskStatus.PENDING).toBe('pending');
    expect(TaskStatus.COMPLETED).toBe('completed');
    expect(TaskStatus.FAILED).toBe('failed');
  });
});

describe('MessageType enum', () => {
  it('should have unique hex values', () => {
    const values = Object.values(MessageType).filter((v) => typeof v === 'number');
    const unique = new Set(values);
    expect(unique.size).toBe(values.length);
  });
});

describe('ProposalType enum', () => {
  it('should have expected values', () => {
    expect(ProposalType.PRIORITY_CHANGE).toBe('priority_change');
    expect(ProposalType.SATURATION_REPORT).toBe('saturation_report');
  });
});