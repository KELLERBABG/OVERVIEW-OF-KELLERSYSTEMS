export class SybilResistance {
  private ipClusterMap = new Map<string, number>();

  async evaluateNode(nodeId: string, ip: string, stake: bigint): Promise<number> {
    const ipPrefix = ip.split('.').slice(0, 3).join('.');
    const clusterCount = this.ipClusterMap.get(ipPrefix) ?? 0;

    const ipDiversity = Math.max(0, 1 - clusterCount / 5);
    const stakeScore = stake > 0n ? Math.min(Number(stake) / 1000, 1) : 0;

    return ipDiversity * 0.6 + stakeScore * 0.4;
  }

  registerNode(nodeId: string, ip: string): void {
    const ipPrefix = ip.split('.').slice(0, 3).join('.');
    this.ipClusterMap.set(ipPrefix, (this.ipClusterMap.get(ipPrefix) ?? 0) + 1);
  }

  removeNode(nodeId: string, ip: string): void {
    const ipPrefix = ip.split('.').slice(0, 3).join('.');
    const count = this.ipClusterMap.get(ipPrefix) ?? 0;
    if (count <= 1) {
      this.ipClusterMap.delete(ipPrefix);
    } else {
      this.ipClusterMap.set(ipPrefix, count - 1);
    }
  }

  clear(): void {
    this.ipClusterMap.clear();
  }
}