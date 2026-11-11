export class NetworkInspector {
  private messages: InspectedMessage[] = [];
  private maxEntries = 1000;

  capture(message: InspectedMessage): void {
    this.messages.push(message);
    if (this.messages.length > this.maxEntries) {
      this.messages = this.messages.slice(-this.maxEntries);
    }
  }

  getMessages(filter?: { type?: number; peerId?: string }): InspectedMessage[] {
    let result = this.messages;
    if (filter?.type !== undefined) {
      result = result.filter((m) => m.type === filter.type);
    }
    if (filter?.peerId) {
      result = result.filter((m) => m.peerId === filter.peerId);
    }
    return result;
  }

  getStats(): { totalMessages: number; avgLatency: number; messageTypes: Record<number, number> } {
    const messageTypes: Record<number, number> = {};
    for (const msg of this.messages) {
      messageTypes[msg.type] = (messageTypes[msg.type] ?? 0) + 1;
    }
    const latencies = this.messages.map((m) => m.size).filter(Boolean);
    const avgLatency = latencies.length > 0
      ? latencies.reduce((a, b) => a + b, 0) / latencies.length
      : 0;
    return {
      totalMessages: this.messages.length,
      avgLatency,
      messageTypes,
    };
  }

  clear(): void {
    this.messages = [];
  }
}

export interface InspectedMessage {
  type: number;
  peerId: string;
  size: number;
  timestamp: number;
  direction: 'inbound' | 'outbound';
}