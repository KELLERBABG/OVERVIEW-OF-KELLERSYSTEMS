import { generateTaskId, TaskPriority } from '@fungal/core';
import type { Task, TaskDistributor } from '@fungal/core';
import type { MeshLike } from '@fungal/task-distribution';

export async function runPrimeSearchDemo(
  distributor: TaskDistributor & { mesh: MeshLike },
  limit: number,
) {
  const task: Task = {
    id: generateTaskId(),
    code: `
      function sieve(limit) {
        const sieve = new Uint8Array(limit + 1);
        const primes = [];
        for (let i = 2; i <= limit; i++) {
          if (!sieve[i]) {
            primes.push(i);
            for (let j = i * i; j <= limit; j += i) sieve[j] = 1;
          }
        }
        return primes;
      }
      return sieve(input.limit);
    `,
    input: { limit },
    deadline: Date.now() + 300_000,
    priority: TaskPriority.MEDIUM,
    requireProof: false,
    requireEncryption: false,
    createdAt: Date.now(),
    ownerId: distributor.mesh.peerId,
    inputType: 'single',
  };

  const startTime = performance.now();
  const result = await distributor.submitTask(task);
  const elapsed = performance.now() - startTime;
  const primes = (result as number[]) ?? [];

  return {
    taskId: task.id,
    primes,
    primeCount: primes.length,
    executionTime: elapsed,
  };
}