import { nanoid } from 'nanoid';
import { DEFAULTS } from '../constants/defaults.js';

export function generateNodeId(): string {
  return nanoid(DEFAULTS.ID_LENGTH);
}

export function generateTaskId(): string {
  return `task_${nanoid(DEFAULTS.ID_LENGTH)}`;
}

export function generateWorkUnitId(): string {
  return `wu_${nanoid(DEFAULTS.ID_LENGTH)}`;
}

export function generateMessageId(): string {
  return `msg_${nanoid(DEFAULTS.ID_LENGTH)}`;
}

export function generateRoomId(): string {
  return nanoid(8);
}

export function generateProposalId(): string {
  return `prop_${nanoid(DEFAULTS.ID_LENGTH)}`;
}