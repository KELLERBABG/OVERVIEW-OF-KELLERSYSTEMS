import React, { useState } from 'react';
import ReactDOM from 'react-dom/client';
import { Viewport3D } from './components/Viewport3D';
import { MultiPlanarSlicer } from './components/MultiPlanarSlicer';
import { LiveScanWorkbench } from './components/LiveScanWorkbench';
import { StationTelemetry } from './components/StationTelemetry';

function App() {
  const [activeTab, setActiveTab] = useState<'3d' | 'mpr' | 'lab' | 'telemetry'>('lab');
  const [showGuide, setShowGuide] = useState<boolean>(false);

  return (
    <div style={{ width: '100vw', height: '100vh', display: 'flex', flexDirection: 'column', background: '#0a0d14', color: '#fff', fontFamily: 'system-ui, -apple-system, sans-serif' }}>
      {/* Navigation Header */}
      <header style={{ padding: '12px 24px', borderBottom: '1px solid #1f293d', display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: '#0f172a' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '24px' }}>
          <h1 style={{ margin: 0, fontSize: '1.2rem', fontWeight: 700, color: '#38bdf8' }}>
            MuonScope™ Digital Twin Suite
          </h1>
          <nav style={{ display: 'flex', gap: '8px' }}>
            <button
              onClick={() => setActiveTab('3d')}
              style={{
                padding: '6px 14px', borderRadius: '6px', border: '1px solid #38bdf8',
                background: activeTab === '3d' ? '#0284c7' : 'transparent', color: '#fff', fontSize: '0.85rem', cursor: 'pointer', fontWeight: 600
              }}
            >
              1. 3D Raymarching & Tracks
            </button>
            <button
              onClick={() => setActiveTab('mpr')}
              style={{
                padding: '6px 14px', borderRadius: '6px', border: '1px solid #38bdf8',
                background: activeTab === 'mpr' ? '#0284c7' : 'transparent', color: '#fff', fontSize: '0.85rem', cursor: 'pointer', fontWeight: 600
              }}
            >
              2. 2D Slicer (Axial/Coronal/Sagittal)
            </button>
            <button
              onClick={() => setActiveTab('lab')}
              style={{
                padding: '6px 14px', borderRadius: '6px', border: '1px solid #38bdf8',
                background: activeTab === 'lab' ? '#0284c7' : 'transparent', color: '#fff', fontSize: '0.85rem', cursor: 'pointer', fontWeight: 600
              }}
            >
              3. Scan Time Calculator & Rig
            </button>
            <button
              onClick={() => setActiveTab('telemetry')}
              style={{
                padding: '6px 14px', borderRadius: '6px', border: '1px solid #38bdf8',
                background: activeTab === 'telemetry' ? '#0284c7' : 'transparent', color: '#fff', fontSize: '0.85rem', cursor: 'pointer', fontWeight: 600
              }}
            >
              4. Field Station Mesh
            </button>
          </nav>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
          <button
            onClick={() => setShowGuide(true)}
            style={{ background: '#334155', color: '#38bdf8', border: '1px solid #475569', padding: '6px 14px', borderRadius: '6px', fontSize: '0.85rem', fontWeight: 600, cursor: 'pointer' }}
          >
            ❓ Quick Start Guide
          </button>
          <div style={{ fontSize: '0.8rem', color: '#94a3b8' }}>
            Flux: <strong style={{ color: '#fff' }}>9,850/m²·min</strong> | <strong style={{ color: '#4ade80' }}>4 Stations Sync'd</strong>
          </div>
        </div>
      </header>

      {/* Guide Modal */}
      {showGuide && (
        <div style={{
          position: 'fixed', inset: 0, zIndex: 100, background: 'rgba(0,0,0,0.75)', backdropFilter: 'blur(4px)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '20px'
        }}>
          <div style={{
            background: '#0f172a', border: '1px solid #38bdf8', borderRadius: '12px', padding: '28px', maxWidth: '680px', width: '100%',
            color: '#fff', display: 'flex', flexDirection: 'column', gap: '16px', boxShadow: '0 20px 25px -5px rgba(0,0,0,0.5)'
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <h2 style={{ margin: 0, fontSize: '1.25rem', color: '#38bdf8' }}>🎓 How MuonScope™ Works — Quick Guide</h2>
              <button onClick={() => setShowGuide(false)} style={{ background: 'transparent', border: 'none', color: '#94a3b8', fontSize: '1.4rem', cursor: 'pointer' }}>×</button>
            </div>

            <div style={{ fontSize: '0.9rem', lineHeight: 1.6, color: '#cbd5e1', display: 'flex', flexDirection: 'column', gap: '12px' }}>
              <div>
                <strong style={{ color: '#38bdf8' }}>1. What are you looking at?</strong>
                <div>Natural cosmic-ray muons continuously penetrate deep concrete, rock, and steel. When passing through heavy atoms (like lead or steel rebar), they scatter sharply; when passing through hollow air voids or cracks, they pass straight through without scattering.</div>
              </div>

              <div>
                <strong style={{ color: '#38bdf8' }}>2. How to use this interactive dashboard:</strong>
                <ul style={{ margin: '4px 0 0 20px', padding: 0 }}>
                  <li><strong>Tab 1 (3D View):</strong> Orbit, zoom, and adjust the <em>3D Cutaway Slider</em> to slice open a concrete bridge pier or ASTM calibration cube. Hover your mouse over any internal rebar, void, or lead cylinder to inspect its density.</li>
                  <li><strong>Tab 2 (2D Slicer):</strong> Drag the Z, Y, and X sliders to inspect the exact internal cross-sections in Axial, Coronal, and Sagittal views. Click anywhere on a slice to classify the material.</li>
                  <li><strong>Tab 3 (Scan Rig & Calculator):</strong> Adjust the wall thickness and resolution sliders to calculate real-world measurement times (15 minutes to 2.5 hours).</li>
                  <li><strong>Tab 4 (Telemetry):</strong> Monitor 4 uncoupled field stations synchronized via sub-nanosecond GNSS timestamps.</li>
                </ul>
              </div>
            </div>

            <button
              onClick={() => setShowGuide(false)}
              style={{ alignSelf: 'flex-end', background: '#0284c7', color: '#fff', border: 'none', padding: '8px 20px', borderRadius: '6px', fontWeight: 600, cursor: 'pointer', marginTop: '8px' }}
            >
              Got it, let's explore!
            </button>
          </div>
        </div>
      )}

      {/* Main Viewport Content - Persistent Views without Unmounting */}
      <main style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
        <div style={{ display: activeTab === '3d' ? 'block' : 'none', width: '100%', height: '100%' }}>
          <Viewport3D />
        </div>
        <div style={{ display: activeTab === 'mpr' ? 'block' : 'none', width: '100%', height: '100%' }}>
          <MultiPlanarSlicer />
        </div>
        <div style={{ display: activeTab === 'lab' ? 'block' : 'none', width: '100%', height: '100%' }}>
          <LiveScanWorkbench />
        </div>
        <div style={{ display: activeTab === 'telemetry' ? 'block' : 'none', width: '100%', height: '100%' }}>
          <StationTelemetry />
        </div>
      </main>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
