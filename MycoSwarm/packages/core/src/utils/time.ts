export function now(): number {
  return Date.now();
}

export function remaining(deadline: number): number {
  return Math.max(0, deadline - now());
}

export function isExpired(timestamp: number, ttl: number): boolean {
  return now() - timestamp > ttl;
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function withTimeout<T>(promise: Promise<T>, ms: number, msg?: string): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) =>
      setTimeout(() => reject(new Error(msg ?? `Timed out after ${ms}ms`)), ms),
    ),
  ]);
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms.toFixed(0)}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60_000).toFixed(1)}min`;
}