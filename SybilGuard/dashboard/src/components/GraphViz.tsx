import React, { useEffect, useRef } from 'react';
import * as d3 from 'd3';
import { api, GraphAnalysis } from '../services/api';

interface Props {
  data: GraphAnalysis | null;
  width?: number;
  height?: number;
}

const GraphViz: React.FC<Props> = ({ data, width = 700, height = 500 }) => {
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (!svgRef.current || !data) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll('*').remove();

    // Simple force-directed graph visualization
    const g = svg.append('g');

    // Generate synthetic nodes from analysis data
    const nodeCount = data.node_count || 50;
    const linkCount = data.edge_count || Math.floor(nodeCount * 3);

    const nodes = Array.from({ length: Math.min(nodeCount, 100) }, (_, i) => ({
      id: i,
      label: `N${i}`,
      centrality: data.top_centrality?.[i]?.centrality ?? 0.5,
    }));

    const links = Array.from({ length: Math.min(linkCount, 200) }, (_, i) => ({
      source: i % nodes.length,
      target: (i * 7) % nodes.length,
    }));

    const simulation = d3.forceSimulation(nodes)
      .force('link', d3.forceLink(links).distance(50))
      .force('charge', d3.forceManyBody().strength(-100))
      .force('center', d3.forceCenter(width / 2, height / 2));

    const link = g.append('g')
      .selectAll('line')
      .data(links)
      .join('line')
      .attr('stroke', '#999')
      .attr('stroke-opacity', 0.4)
      .attr('stroke-width', 1);

    const node = g.append('g')
      .selectAll('circle')
      .data(nodes)
      .join('circle')
      .attr('r', d => 3 + d.centrality * 8)
      .attr('fill', d => {
        if (data.small_world) return d.centrality > 0.5 ? '#4CAF50' : '#2196F3';
        return d.centrality > 0.5 ? '#f44336' : '#FF9800';
      })
      .attr('stroke', '#fff')
      .attr('stroke-width', 1.5);

    node.append('title')
      .text(d => `${d.label}\ncentrality: ${d.centrality.toFixed(3)}`);

    simulation.on('tick', () => {
      link
        .attr('x1', (d: any) => d.source.x)
        .attr('y1', (d: any) => d.source.y)
        .attr('x2', (d: any) => d.target.x)
        .attr('y2', (d: any) => d.target.y);
      node
        .attr('cx', (d: any) => d.x)
        .attr('cy', (d: any) => d.y);
    });

    return () => { simulation.stop(); };
  }, [data, width, height]);

  return (
    <svg
      ref={svgRef}
      width={width}
      height={height}
      style={{ border: '1px solid #e0e0e0', borderRadius: 8, background: '#fafafa' }}
    />
  );
};

export default GraphViz;