/**
 * Background Web Worker for Wasm execution.
 * Listens for work units, executes them, and posts results back.
 */

self.onmessage = async (e: MessageEvent) => {
  const { type, payload } = e.data;

  switch (type) {
    case 'execute': {
      const { workUnit, provingKey } = payload;
      try {
        const startTime = performance.now();

        // Execute JS code (Wasm would use WebAssembly.compile/instantiate)
        const fn = new Function('input', workUnit.code);
        const output = await fn(workUnit.input);

        const executionTime = performance.now() - startTime;

        self.postMessage({
          type: 'result',
          payload: {
            workUnitId: workUnit.id,
            taskId: workUnit.taskId,
            output,
            executionTime,
            memoryUsed: 0,
            proof: provingKey ? { mock: true } : undefined,
          },
        });
      } catch (error) {
        self.postMessage({
          type: 'error',
          payload: {
            workUnitId: workUnit.id,
            taskId: workUnit.taskId,
            error: (error as Error).message,
          },
        });
      }
      break;
    }

    case 'terminate':
      self.close();
      break;
  }
};