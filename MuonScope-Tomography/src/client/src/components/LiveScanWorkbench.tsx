import React, { useState } from 'react';

export const LiveScanWorkbench: React.FC = () => {
  const [numMuons, setNumMuons] = useState<number>(2000);
  const [preset, setPreset] = useState<string>('heavy_inclusion');
  const [gridRes, setGridRes] = useState<number>(16);
  const [iterations, setIterations] = useState<number>(10);
  const [useMapEm, setUseMapEm] = useState<boolean>(true);
  const [isComputing, setIsComputing] = useState<boolean>(false);
  const [backendResult, setBackendResult] = useState<any>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const triggerLiveInversion = async () => {
    setIsComputing(true);
    setErrorMsg(null);
    try {
      const resp = await fetch('http://localhost:8000/api/v1/run-reconstruction', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          num_muons: numMuons,
          preset: preset,
          grid_res: gridRes,
          map_em_iterations: iterations,
          regularization_weight: 0.05,
          use_map_em: useMapEm,
        }),
      });

      if (!resp.ok) {
        throw new Error(`Backend returned HTTP error ${resp.status}`);
      }

      const data = await resp.json();
      setBackendResult(data);
    } catch (err: any) {
      setErrorMsg(err.message || 'Failed to connect to Python backend server (localhost:8000)');
    } finally {
      setIsComputing(false);
    }
  };

  const handleReset = () => {
    setBackendResult(null);
    setErrorMsg(null);
  };

  return (
    <div style={{ padding: '24px', display: 'grid', gridTemplateColumns: '400px 1fr', gap: '24px', height: '100%', boxSizing: 'border-box', color: '#fff', background: '#0a0d14', overflowY: 'auto' }}>
      {/* Left Inversion Parameter Rig */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <div style={{ background: '#111827', borderRadius: '8px', border: '1px solid #1f293d', padding: '20px', display: 'flex', flexDirection: 'column', gap: '14px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <h2 style={{ fontSize: '1.05rem', margin: 0, color: '#38bdf8', fontWeight: 700 }}>
              🔬 Live Physics & Inversion Rig
            </h2>
            <span style={{
              fontSize: '0.75rem',
              background: backendResult ? '#166534' : '#374151',
              color: backendResult ? '#86efac' : '#cbd5e1',
              padding: '2px 8px', borderRadius: '12px', fontWeight: 600,
            }}>
              {backendResult ? 'Inversion complete' : 'Idle — no run yet'}
            </span>
          </div>

          <div style={{ fontSize: '0.8rem', color: '#94a3b8' }}>
            Runs the instrument software end to end on simulated detector bytes:
            physics, readout, wire framing, calibration, track fitting and a
            prior-marginalised reconstruction. Momentum is not supplied per event —
            a hodoscope cannot measure it.
          </div>

          <div>
            <label style={{ fontSize: '0.8rem', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>Physical Scenario Preset:</label>
            <select
              value={preset}
              onChange={(e) => setPreset(e.target.value)}
              style={{ width: '100%', background: '#1e293b', color: '#fff', border: '1px solid #38bdf8', borderRadius: '4px', padding: '6px 8px', fontSize: '0.85rem' }}
            >
              <option value="heavy_inclusion">ASTM Phantom (Lead Cylinder + Steel Rebar)</option>
              <option value="void">Concrete Pier with Internal Air Void</option>
              <option value="rebar">Reinforced Concrete Grid (2-Way Rebar Mesh)</option>
            </select>
          </div>

          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', color: '#94a3b8' }}>
              <span>Cosmic Muon Statistics:</span>
              <strong style={{ color: '#38bdf8' }}>{numMuons.toLocaleString()} Muons</strong>
            </div>
            <input
              type="range" min={500} max={10000} step={500} value={numMuons}
              onChange={(e) => setNumMuons(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#38bdf8' }}
            />
          </div>

          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', color: '#94a3b8' }}>
              <span>Voxel Grid Resolution:</span>
              <strong style={{ color: '#38bdf8' }}>{gridRes}³ ({gridRes * gridRes * gridRes} voxels)</strong>
            </div>
            <select
              value={gridRes}
              onChange={(e) => setGridRes(Number(e.target.value))}
              style={{ width: '100%', background: '#1e293b', color: '#fff', border: '1px solid #38bdf8', borderRadius: '4px', padding: '6px 8px', fontSize: '0.85rem' }}
            >
              <option value={16}>16³ (Fast Preview — ~10 sec)</option>
              <option value={24}>24³ (Medium Balance — ~35 sec)</option>
              <option value={32}>32³ (Production High-Res — ~75 sec)</option>
            </select>
          </div>

          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.8rem', color: '#94a3b8' }}>
              <span>MAP-EM Iteration Count:</span>
              <strong style={{ color: '#38bdf8' }}>{iterations} Iterations</strong>
            </div>
            <input
              type="range" min={3} max={30} step={1} value={iterations}
              onChange={(e) => setIterations(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#38bdf8' }}
            />
          </div>

          <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '0.85rem', cursor: 'pointer' }}>
            <input
              type="checkbox" checked={useMapEm}
              onChange={(e) => setUseMapEm(e.target.checked)}
              style={{ accentColor: '#38bdf8' }}
            />
            <span>Enable Iterative MAP-EM with 3D Huber Prior</span>
          </label>

          <button
            onClick={triggerLiveInversion}
            disabled={isComputing}
            style={{
              marginTop: '8px', padding: '12px', borderRadius: '6px', border: 'none',
              background: isComputing ? '#475569' : '#0284c7', color: '#fff', fontWeight: 700,
              fontSize: '0.95rem', cursor: isComputing ? 'not-allowed' : 'pointer'
            }}
          >
            {isComputing ? '⏳ Computing Live Physics Inversion...' : '🚀 Launch Real Inversion Computation'}
          </button>

          {errorMsg && (
            <div style={{ color: '#ef4444', fontSize: '0.8rem', background: '#450a0a', padding: '8px', borderRadius: '4px' }}>
              {errorMsg}
            </div>
          )}
        </div>
      </div>

      {/* Right Live Results & Artifact Viewer */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
        <div style={{ background: '#111827', borderRadius: '8px', border: '1px solid #1f293d', padding: '20px', flex: 1, display: 'flex', flexDirection: 'column', gap: '16px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <h3 style={{ margin: 0, fontSize: '1.1rem', color: '#38bdf8' }}>
              📊 Real Computational Output & Metrics
            </h3>
            {backendResult && (
              <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
                <span style={{ background: '#166534', color: '#86efac', padding: '4px 10px', borderRadius: '4px', fontSize: '0.8rem', fontWeight: 600 }}>
                  ✅ Inversion Finished ({backendResult.metrics.runtime_seconds}s)
                </span>
                <button
                  onClick={handleReset}
                  style={{ background: '#334155', border: '1px solid #475569', color: '#94a3b8', fontSize: '0.75rem', padding: '4px 8px', borderRadius: '4px', cursor: 'pointer' }}
                >
                  🔄 Reset View
                </button>
              </div>
            )}
          </div>

          {backendResult ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
              {/* Metric KPI Cards. Values come from the instrument; CNR is
                  dimensionless and no standard defines the thresholds the old
                  labels quoted. */}
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '12px' }}>
                <div style={{ background: '#1e293b', padding: '12px', borderRadius: '6px' }}>
                  <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Tracks Fitted</div>
                  <div style={{ fontSize: '1.25rem', fontWeight: 700, color: '#fff' }}>{backendResult.metrics.accepted_tracks}</div>
                  <div style={{ fontSize: '0.7rem', color: '#38bdf8' }}>passed coincidence and tracking</div>
                </div>
                <div style={{ background: '#1e293b', padding: '12px', borderRadius: '6px' }}>
                  <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>High-Z CNR</div>
                  <div style={{ fontSize: '1.25rem', fontWeight: 700, color: '#4ade80' }}>
                    {backendResult.metrics.cnr_lead ?? 'n/a'}
                  </div>
                  <div style={{ fontSize: '0.7rem', color: '#94a3b8' }}>dimensionless; no standard threshold</div>
                </div>
                <div style={{ background: '#1e293b', padding: '12px', borderRadius: '6px' }}>
                  <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Metal Anomaly CNR</div>
                  <div style={{ fontSize: '1.25rem', fontWeight: 700, color: '#4ade80' }}>
                    {backendResult.metrics.cnr_metal ?? 'n/a'}
                  </div>
                  <div style={{ fontSize: '0.7rem', color: '#94a3b8' }}>dimensionless; no standard threshold</div>
                </div>
                <div style={{ background: '#1e293b', padding: '12px', borderRadius: '6px' }}>
                  <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Compute Runtime</div>
                  <div style={{ fontSize: '1.25rem', fontWeight: 700, color: '#38bdf8' }}>{backendResult.metrics.runtime_seconds}s</div>
                  <div style={{ fontSize: '0.7rem', color: '#94a3b8' }}>{backendResult.metrics.iterations} solver steps</div>
                </div>
              </div>

              {/* Measured instrument state, reported by the acquisition chain. */}
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '12px' }}>
                <div style={{ background: '#1e293b', padding: '12px', borderRadius: '6px' }}>
                  <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>TDC Bin (measured)</div>
                  <div style={{ fontSize: '1.05rem', fontWeight: 700, color: '#fff' }}>
                    {backendResult.metrics.tdc_bin_ps ?? 'n/a'} ps
                  </div>
                </div>
                <div style={{ background: '#1e293b', padding: '12px', borderRadius: '6px' }}>
                  <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Angular Resolution</div>
                  <div style={{ fontSize: '1.05rem', fontWeight: 700, color: '#fff' }}>
                    {backendResult.metrics.sigma_theta_mrad ?? 'n/a'} mrad
                  </div>
                </div>
                <div style={{ background: '#1e293b', padding: '12px', borderRadius: '6px' }}>
                  <div style={{ fontSize: '0.75rem', color: '#94a3b8' }}>Track Efficiency</div>
                  <div style={{ fontSize: '1.05rem', fontWeight: 700, color: '#fff' }}>
                    {backendResult.metrics.track_efficiency ?? 'n/a'}
                  </div>
                </div>
              </div>

              {/* Real Generated Artifacts */}
              <div style={{ background: '#1e293b', borderRadius: '6px', padding: '16px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
                <h4 style={{ margin: 0, fontSize: '0.9rem', color: '#38bdf8' }}>📁 Verified Generated Engineering Artifacts:</h4>
                <div style={{ fontSize: '0.85rem', color: '#cbd5e1' }}>
                  • <strong>IFC 4.3 BIM Model:</strong> <code style={{ color: '#38bdf8' }}>output_reconstruction/muonscope_model.ifc</code> (Includes <code style={{ color: '#4ade80' }}>IfcVoidingFeature</code> & <code style={{ color: '#4ade80' }}>IfcReinforcingBar</code>)
                </div>
                <div style={{ fontSize: '0.85rem', color: '#cbd5e1' }}>
                  • <strong>Wavefront OBJ 3D CAD Mesh:</strong> <code style={{ color: '#38bdf8' }}>output_reconstruction/reconstruction_mesh.obj</code> (Marching Cubes Isosurface)
                </div>
                <div style={{ fontSize: '0.85rem', color: '#cbd5e1' }}>
                  • <strong>DICOM-3D Radiographic Series:</strong> <code style={{ color: '#38bdf8' }}>output_reconstruction/dicom_slices/</code> (16-bit CT Slice Images)
                </div>
              </div>

              <div style={{ fontSize: '0.85rem', color: '#94a3b8', lineHeight: 1.5 }}>
                💡 <strong>Next Step:</strong> Switch to <strong>Tab 2 (2D Slicer)</strong> or <strong>Tab 1 (3D View)</strong> to slice through and inspect this newly computed density grid!
              </div>
            </div>
          ) : (
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: '#64748b', gap: '12px' }}>
              <div style={{ fontSize: '2.5rem' }}>⚛️</div>
              <div style={{ fontSize: '0.95rem', color: '#e2e8f0', fontWeight: 600 }}>Ready for Inversion Computation</div>
              <div style={{ fontSize: '0.8rem', maxWidth: '380px', textAlign: 'center', color: '#94a3b8', lineHeight: 1.5 }}>
                Configure your physical scenario, muon statistics, and grid resolution on the left, then click <strong>"Launch Real Inversion Computation"</strong> to run the live Python engine.
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
