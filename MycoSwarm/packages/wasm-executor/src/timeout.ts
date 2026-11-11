export interface TimeoutSignal {
  aborted: boolean;
  abort: () => void;
}

export function createTimeoutSignal(ms: number): TimeoutSignal {
  const signal: TimeoutSignal = { aborted: false, abort: () => {} };
  const timer = setTimeout(() => {
    signal.aborted = true;
  }, ms);
  signal.abort = () => {
    clearTimeout(timer);
    signal.aborted = true;
  };
  return signal;
}

export function withExecutionTimeout<T>(
  promise: Promise<T>,
  ms: number,
  taskName?: string,
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) =>
      setTimeout(
        () => reject(new Error(`Execution timed out after ${ms}ms${taskName ? ` (${taskName})` : ''}`)),
        ms,
      ),
    ),
  ]);
}