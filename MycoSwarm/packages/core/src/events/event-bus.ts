type EventHandler<T = unknown> = (payload: T) => void;

export class EventBus {
  private listeners = new Map<string, Set<EventHandler>>();
  private onceListeners = new Map<string, Set<EventHandler>>();

  on<T>(event: string, handler: EventHandler<T>): void {
    const existing = this.listeners.get(event) ?? new Set();
    existing.add(handler as EventHandler);
    this.listeners.set(event, existing);
  }

  off<T>(event: string, handler: EventHandler<T>): void {
    const existing = this.listeners.get(event);
    if (existing) {
      existing.delete(handler as EventHandler);
      if (existing.size === 0) this.listeners.delete(event);
    }
    const onceExisting = this.onceListeners.get(event);
    if (onceExisting) {
      onceExisting.delete(handler as EventHandler);
      if (onceExisting.size === 0) this.onceListeners.delete(event);
    }
  }

  emit<T>(event: string, payload: T): void {
    const handlers = this.listeners.get(event);
    if (handlers) {
      for (const handler of handlers) {
        try {
          handler(payload);
        } catch (err) {
          console.error(`[EventBus] Error in handler for "${event}":`, err);
        }
      }
    }
    const onceHandlers = this.onceListeners.get(event);
    if (onceHandlers) {
      for (const handler of onceHandlers) {
        try {
          handler(payload);
        } catch (err) {
          console.error(`[EventBus] Error in once-handler for "${event}":`, err);
        }
      }
      this.onceListeners.delete(event);
    }
  }

  once<T>(event: string, handler: EventHandler<T>): void {
    const existing = this.onceListeners.get(event) ?? new Set();
    existing.add(handler as EventHandler);
    this.onceListeners.set(event, existing);
  }

  removeAllListeners(event?: string): void {
    if (event) {
      this.listeners.delete(event);
      this.onceListeners.delete(event);
    } else {
      this.listeners.clear();
      this.onceListeners.clear();
    }
  }

  listenerCount(event: string): number {
    return (this.listeners.get(event)?.size ?? 0) + (this.onceListeners.get(event)?.size ?? 0);
  }
}