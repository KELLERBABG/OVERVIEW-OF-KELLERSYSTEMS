import React, { useState, useEffect } from 'react';
import GraphViz from './components/GraphViz';
import { api, GraphAnalysis, SybilResult, ReputationScore } from './services/api';

const App: React.FC = () => {
  const [health, setHealth] = useState<string>('connecting...');
  const [graphData, setGraphData] = useState<GraphAnalysis | null>(null);
  const [detectionData, setDetectionData] = useState<SybilResult | null>(null);
  const [reputation, setReputation] = useState<ReputationScore | null>(null);
  const [nodeId, setNodeId] = useState<string>('');
  const [error, setError] = useState<string>('');

  useEffect(() => {
    api.health().then(h => setHealth(`✅ ${h.service} v${h.version}`)).catch(() => setHealth('❌ API offline'));
  }, []);

  const analyzeGraph = async () => {
    try {
      setError('');
      const data = await api.graphAnalyze();
      setGraphData(data);
    } catch (e: any) { setError(e.message); }
  };

  const runDetection = async () => {
    try {
      setError('');
      const data = await api.detectSybil();
      setDetectionData(data);
    } catch (e: any) { setError(e.message); }
  };

  const createNode = async () => {
    try {
      setError('');
      const res = await api.createNode();
      setNodeId(res.node_id);
    } catch (e: any) { setError(e.message); }
  };

  const loadReputation = async () => {
    if (!nodeId) return;
    try {
      setError('');
      const r = await api.getReputation(nodeId);
      setReputation(r);
    } catch (e: any) { setError(e.message); }
  };

  return (
    <div style={{ padding: '2rem', fontFamily: 'system-ui, -apple-system, sans-serif', maxWidth: 1200, margin: '0 auto' }}>
      <header style={{ marginBottom: '2rem' }}>
        <h1 style={{ margin: 0, fontSize: '1.8rem' }}>Sybil-Resistentes</h1>
        <p style={{ margin: '0.25rem 0 0', color: '#666' }}>Proof-of-Personhood System Monitor</p>
        <div style={{ fontSize: '0.85rem', color: health.startsWith('✅') ? '#2e7d32' : '#c62828', marginTop: 4 }}>{health}</div>
      </header>

      {error && <div style={{ padding: '0.75rem', background: '#ffebee', borderRadius: 6, marginBottom: '1rem', color: '#c62828' }}>⚠ {error}</div>}

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1.5rem' }}>
        <section style={{ border: '1px solid #e0e0e0', borderRadius: 8, padding: '1.5rem' }}>
          <h2>Graph Analysis</h2>
          <button onClick={analyzeGraph} style={btnStyle}>Analyze Graph</button>
          <button onClick={createNode} style={{ ...btnStyle, marginLeft: 8 }}>Create Node</button>
          {nodeId && <p style={{ fontSize: '0.8rem', color: '#666' }}>Node: {nodeId.slice(0, 8)}...</p>}
          {graphData && (
            <div style={{ marginTop: '1rem' }}>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
                <Metric label="Nodes" value={graphData.node_count} />
                <Metric label="Edges" value={graphData.edge_count} />
                <Metric label="Small World" value={graphData.small_world ? '✓ Yes' : '✗ No'} />
                <Metric label="Clustering Coeff" value={graphData.clustering_coefficient.toFixed(3)} />
                <Metric label="Avg Path Length" value={graphData.average_path_length.toFixed(3)} />
                <Metric label="CC Std Dev" value={graphData.cc_statistics.std_dev.toFixed(3)} />
              </div>
              <div style={{ marginTop: '1rem' }}>
                <h3>Graph Visualization</h3>
                <GraphViz data={graphData} />
              </div>
            </div>
          )}
        </section>

        <section style={{ border: '1px solid #e0e0e0', borderRadius: 8, padding: '1.5rem' }}>
          <h2>Sybil Detection</h2>
          <button onClick={runDetection} style={btnStyle}>Run Detection</button>
          {detectionData && (
            <div style={{ marginTop: '1rem' }}>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
                <Metric label="Regions Analyzed" value={detectionData.regions_analyzed} />
                <Metric label="Suspected Sybil" value={detectionData.suspected_sybil} />
              </div>
              {detectionData.details.map((r, i) => (
                <div key={i} style={{
                  marginTop: 8, padding: 8, borderRadius: 6,
                  background: r.suspected ? '#fff3e0' : '#e8f5e9',
                  border: r.suspected ? '1px solid #ff9800' : '1px solid #4caf50'
                }}>
                  <strong>{r.region}</strong> — {r.node_count} nodes
                  <div style={{ fontSize: '0.8rem', color: '#555' }}>
                    entropy={r.entropy.toFixed(3)} anomaly={r.anomaly.toFixed(3)} confidence={r.confidence.toFixed(3)}<br />
                    CC={r.mean_cc.toFixed(3)} int={r.internal_density.toFixed(3)} ext={r.external_density.toFixed(3)}
                  </div>
                </div>
              ))}
            </div>
          )}
          <div style={{ marginTop: '1.5rem' }}>
            <h3>Reputation</h3>
            <button onClick={loadReputation} disabled={!nodeId} style={btnStyle}>Get Score</button>
            {reputation && (
              <div style={{ marginTop: 8, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
                <Metric label="Base Score" value={reputation.base_score.toFixed(3)} />
                <Metric label="Embedding Bonus" value={reputation.embedding_bonus.toFixed(3)} />
                <Metric label="Vouching Power" value={reputation.vouching_power} />
                <Metric label="Decay Factor" value={reputation.decay_factor.toFixed(4)} />
              </div>
            )}
          </div>
        </section>
      </div>
    </div>
  );
};

const Metric: React.FC<{ label: string; value: string | number }> = ({ label, value }) => (
  <div style={{ background: '#f5f5f5', padding: '0.5rem', borderRadius: 6 }}>
    <div style={{ fontSize: '0.75rem', color: '#888' }}>{label}</div>
    <div style={{ fontSize: '1.1rem', fontWeight: 600 }}>{value}</div>
  </div>
);

const btnStyle: React.CSSProperties = {
  padding: '0.5rem 1rem', background: '#1976d2', color: 'white',
  border: 'none', borderRadius: 6, cursor: 'pointer', fontSize: '0.9rem',
};

export default App;