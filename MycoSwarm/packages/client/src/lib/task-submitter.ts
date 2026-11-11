import { TaskPriority, generateTaskId } from '@fungal/core';
import type { Task } from '@fungal/core';
import type { TaskDistributor } from '@fungal/task-distribution';

export interface TaskFormData {
  code: string;
  input: string;
  timeout: number;
  priority: string;
  requireProof: boolean;
  requireEncryption: boolean;
  inputType?: string;
}

export class TaskSubmitter {
  private distributor: TaskDistributor;
  private mesh: { peerId: string };

  constructor(distributor: TaskDistributor, mesh: { peerId: string }) {
    this.distributor = distributor;
    this.mesh = mesh;
  }

  async submitTaskFromUI(formData: TaskFormData): Promise<unknown> {
    const task: Task = {
      id: generateTaskId(),
      code: formData.code,
      input: JSON.parse(formData.input),
      deadline: Date.now() + formData.timeout * 1000,
      priority: TaskPriority[formData.priority as keyof typeof TaskPriority] ?? TaskPriority.MEDIUM,
      requireProof: formData.requireProof,
      requireEncryption: formData.requireEncryption,
      createdAt: Date.now(),
      ownerId: this.mesh.peerId,
      inputType: formData.inputType as any,
    };

    if (!task.code) throw new Error('Code is required');
    if (!task.input) throw new Error('Input is required');

    return this.distributor.submitTask(task);
  }
}