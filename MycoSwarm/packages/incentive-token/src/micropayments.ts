import type { TokenClient } from './token-client.js';
import type { Receipt } from './types.js';

export class MicropaymentChannel {
  private tokenClient: TokenClient;
  private receipts: Receipt[] = [];

  constructor(tokenClient: TokenClient) {
    this.tokenClient = tokenClient;
  }

  async openChannel(amount: bigint): Promise<string> {
    return this.tokenClient.depositForTask(
      'channel_' + Date.now(),
      1000,
      amount / 1000n,
    );
  }

  async signReceipt(unitId: string, amount: bigint, _workerAddress: string): Promise<string> {
    // In production: sign with ethers
    const signature = `sig_${unitId}_${amount}`;
    this.receipts.push({ unitId, amount, signature });
    return signature;
  }

  async submitBatch(): Promise<string> {
    if (this.receipts.length === 0) throw new Error('No receipts to submit');
    const total = this.receipts.reduce((s, r) => s + r.amount, 0n);
    this.receipts = [];
    return `batch_${total}`;
  }

  get pendingCount(): number {
    return this.receipts.length;
  }

  get pendingTotal(): bigint {
    return this.receipts.reduce((s, r) => s + r.amount, 0n);
  }
}