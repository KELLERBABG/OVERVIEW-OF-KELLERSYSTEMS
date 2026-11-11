import { generateTaskId, TaskPriority } from '@fungal/core';
import type { Task } from '@fungal/core';
import type { TaskDistributor } from '@fungal/task-distribution';
import type { MeshLike } from '@fungal/task-distribution';

export async function runMandelbrotDemo(
  distributor: TaskDistributor & { mesh: MeshLike },
  width: number,
  height: number,
) {
  const task: Task = {
    id: generateTaskId(),
    code: `
      function mandelbrotPixel(cx, cy) {
        let x = 0, y = 0, iteration = 0, maxIteration = 100;
        while (x*x + y*y <= 4 && iteration < maxIteration) {
          const xtemp = x*x - y*y + cx;
          y = 2*x*y + cy;
          x = xtemp;
          iteration++;
        }
        return iteration;
      }
      const result = [];
      for (let px = input.startX; px < input.endX; px++) {
        for (let py = 0; py < input.height; py++) {
          const cx = (px / input.width) * 3.5 - 2.5;
          const cy = (py / input.height) * 2.0 - 1.0;
          result.push(mandelbrotPixel(cx, cy));
        }
      }
      return result;
    `,
    input: { width, height, startX: 0, endX: width },
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

  return {
    taskId: task.id,
    pixels: (result as number[]) ?? [],
    dimensions: { width, height },
    executionTime: elapsed,
  };
}