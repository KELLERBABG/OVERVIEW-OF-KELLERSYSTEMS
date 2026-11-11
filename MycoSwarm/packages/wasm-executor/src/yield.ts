/**
 * Cooperative yielding to the browser event loop.
 *
 * Long-running Wasm computations must periodically yield control
 * back to the browser to:
 * 1. Allow WebRTC heartbeat messages to be processed (prevent false eviction)
 * 2. Prevent browser watchdog timer termination of misbehaving workers
 * 3. Allow UI updates and other browser-maintained connections
 * 4. Avoid GC pauses during critical P2P communication windows
 *
 * This is essential because browsers aggressively throttle background
 * tabs — a single long execution frame can cause the RoomManager's
 * 15-second heartbeat to miss, resulting in false-positive eviction.
 */

const YIELD_INTERVAL_MS = 50; // Yield every 50ms of execution

let lastYieldTime = 0;

/**
 * Check if we should yield to the event loop.
 * Call periodically inside long-running computation loops.
 * Returns a promise that resolves when control returns.
 */
export async function yieldIfNeeded(): Promise<void> {
  const now = performance.now();
  if (now - lastYieldTime < YIELD_INTERVAL_MS) return;
  lastYieldTime = now;

  // Yield to event loop via microtask/setTimeout combination
  return new Promise((resolve) => {
    // setTimeout ensures minimum delay for heartbeat processing
    setTimeout(resolve, 0);
  });
}

/**
 * Reset the yield timer (call before starting a new computation).
 */
export function resetYieldTimer(): void {
  lastYieldTime = performance.now();
}

/**
 * Execute a function with automatic yielding every YIELD_INTERVAL_MS.
 * The function receives a `yield` callback it should call periodically.
 */
export async function withYielding<T>(
  fn: (yieldFn: () => Promise<void>) => Promise<T>,
): Promise<T> {
  resetYieldTimer();
  return fn(yieldIfNeeded);
}

/**
 * Wraps a synchronous loop body to yield periodically.
 * Call this inside a for/while loop instead of the raw operation.
 */
export async function yieldInLoop<T>(
  operation: () => T,
  yieldInterval = YIELD_INTERVAL_MS,
): Promise<T> {
  const now = performance.now();
  if (now - lastYieldTime >= yieldInterval) {
    lastYieldTime = now;
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  return operation();
}