export interface TopologyNode {
  id: string;
  x: number;
  y: number;
  status: string;
  score: number;
}

export interface TopologyLink {
  source: string;
  target: string;
  latency: number;
}

export class TopologyViewer {
  private nodes: TopologyNode[] = [];
  private links: TopologyLink[] = [];

  update(nodes: TopologyNode[], links: TopologyLink[]): void {
    this.nodes = nodes;
    this.links = links;
  }

  getNodes(): TopologyNode[] {
    return [...this.nodes];
  }

  getLinks(): TopologyLink[] {
    return [...this.links];
  }

  findNode(id: string): TopologyNode | undefined {
    return this.nodes.find((n) => n.id === id);
  }

  getConnectedNodes(nodeId: string): TopologyNode[] {
    const connectedIds = new Set<string>();
    for (const link of this.links) {
      if (link.source === nodeId) connectedIds.add(link.target);
      if (link.target === nodeId) connectedIds.add(link.source);
    }
    return this.nodes.filter((n) => connectedIds.has(n.id));
  }
}