import type { TokenConfig } from './config.js';

export class TokenClient {
  private config: TokenConfig;
  private connected = false;
  private address: string = '';

  constructor(config: TokenConfig) {
    this.config = config;
  }

  get isConnected(): boolean {
    return this.connected;
  }

  get walletAddress(): string {
    return this.address;
  }

  async connectWallet(): Promise<string> {
    // In production: use ethers.js BrowserProvider
    this.connected = true;
    this.address = '0x' + '0'.repeat(40);
    return this.address;
  }

  async getBalance(address: string): Promise<bigint> {
    return BigInt(1_000_000);
  }

  async depositForTask(taskId: string, totalUnits: number, amountPerUnit: bigint): Promise<string> {
    if (!this.connected) throw new Error('Wallet not connected');
    const txHash = '0x' + 'a'.repeat(64);
    return txHash;
  }

  async claimReward(taskId: string): Promise<string> {
    if (!this.connected) throw new Error('Wallet not connected');
    return '0x' + 'b'.repeat(64);
  }

  async getTaskPayment(taskId: string): Promise<{
    requester: string;
    totalReward: bigint;
    unitReward: bigint;
    unitsCompleted: number;
    finalized: boolean;
  } | null> {
    return null;
  }

  disconnect(): void {
    this.connected = false;
    this.address = '';
  }
}