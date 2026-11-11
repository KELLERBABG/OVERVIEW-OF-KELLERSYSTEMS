import type { WorkUnit } from '@fungal/core';

export class TimeoutManager {
  private timers = new Map<string, ReturnType<typeof setTimeout>>();
  onTimeout: ((unitId: string) => void) | null = null;

  startTracking(unit: WorkUnit): void {
    const remaining = unit.deadline - Date.now();
    if (remaining <= 0) {
      this.onTimeout?.(unit.id);
      return;
    }
    const timer = setTimeout(() => {
      this.onTimeout?.(unit.id);
      this.timers.delete(unit.id);
    }, remaining);
    this.timers.set(unit.id, timer);
  }

  stopTracking(unitId: string): void {
    const timer = this.timers.get(unitId);
    if (timer) clearTimeout(timer);
    this.timers.delete(unitId);
  }

  hasActive(unitId: string): boolean {
    return this.timers.has(unitId);
  }

  clearAll(): void {
    for (const timer of this.timers.values()) {
      clearTimeout(timer);
    }
    this.timers.clear();
  }

  get activeCount(): number {
    return this.timers.size;
  }
}