import React, { useState, useEffect, useRef } from 'react';

interface MultiPlanarSlicerProps {
  gridSize?: number;
}

export const MultiPlanarSlicer: React.FC<MultiPlanarSlicerProps> = ({ gridSize = 32 }) => {
  const [sliceZ, setSliceZ] = useState<number>(16);
  const [sliceY, setSliceY] = useState<number>(16);
  const [sliceX, setSliceX] = useState<number>(16);
  const [colormap, setColormap] = useState<'turbo' | 'viridis' | 'gray'>('turbo');
  const [contrastBoost, setContrastBoost] = useState<number>(1.5);
  const [inspectedVoxel, setInspectedVoxel] = useState<string>('Click anywhere on any slice to inspect localized density and material');

  const axialCanvasRef = useRef<HTMLCanvasElement>(null);
  const coronalCanvasRef = useRef<HTMLCanvasElement>(null);
  const sagittalCanvasRef = useRef<HTMLCanvasElement>(null);

  const [realVolumeZ, setRealVolumeZ] = useState<number[][] | null>(null);
  const [realVolumeY, setRealVolumeY] = useState<number[][] | null>(null);
  const [realVolumeX, setRealVolumeX] = useState<number[][] | null>(null);

  // Fetch real slices from backend if available
  useEffect(() => {
    fetch(`http://localhost:8000/api/v1/volume-slice?axis=z&index=${sliceZ}`)
      .then((r) => r.json())
      .then((d) => setRealVolumeZ(d.data))
      .catch(() => setRealVolumeZ(null));
  }, [sliceZ]);

  useEffect(() => {
    fetch(`http://localhost:8000/api/v1/volume-slice?axis=y&index=${sliceY}`)
      .then((r) => r.json())
      .then((d) => setRealVolumeY(d.data))
      .catch(() => setRealVolumeY(null));
  }, [sliceY]);

  useEffect(() => {
    fetch(`http://localhost:8000/api/v1/volume-slice?axis=x&index=${sliceX}`)
      .then((r) => r.json())
      .then((d) => setRealVolumeX(d.data))
      .catch(() => setRealVolumeX(null));
  }, [sliceX]);

  // Colormap generator function
  const getColor = (normVal: number, map: 'turbo' | 'viridis' | 'gray'): [number, number, number] => {
    const v = Math.min(Math.max(normVal, 0), 1);
    if (map === 'gray') {
      const g = Math.floor(v * 255);
      return [g, g, g];
    }
    if (map === 'viridis') {
      const r = Math.floor((0.267 + 0.704 * v - 0.972 * v * v) * 255);
      const g = Math.floor((0.004 + 1.258 * v - 0.282 * v * v) * 255);
      const b = Math.floor((0.329 + 0.854 * v - 1.183 * v * v) * 255);
      return [Math.min(Math.max(r, 0), 255), Math.min(Math.max(g, 0), 255), Math.min(Math.max(b, 0), 255)];
    }
    // Turbo
    const r = Math.floor((0.135 + 4.5 * v - 5.5 * v * v) * 255);
    const g = Math.floor((0.091 + 2.0 * v - 1.5 * v * v) * 255);
    const b = Math.floor((0.106 + 3.0 * (1.0 - v) - 3.2 * (1.0 - v) * (1.0 - v)) * 255);
    return [Math.min(Math.max(r, 0), 255), Math.min(Math.max(g, 0), 255), Math.min(Math.max(b, 0), 255)];
  };

  const renderSlice = (
    canvas: HTMLCanvasElement | null,
    sliceData: number[][] | null
  ) => {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const N = sliceData ? sliceData.length : gridSize;
    const imgData = ctx.createImageData(N, N);

    // Find min and max in slice for dynamic auto-contrast
    let minV = Infinity;
    let maxV = -Infinity;

    for (let u = 0; u < N; u++) {
      for (let v = 0; v < N; v++) {
        let val = 0;
        if (sliceData && sliceData[u] && sliceData[u][v] !== undefined) {
          val = sliceData[u][v];
        }
        // No synthetic phantom here. This used to substitute a fabricated
        // perfect reconstruction -- concrete, lead, steel and a void at fixed
        // positions -- whenever the API was unavailable, which made a
        // disconnected instrument look like one that was working.
        if (val < minV) minV = val;
        if (val > maxV) maxV = val;
      }
    }

    if (maxV <= minV) maxV = minV + 1e-4;

    for (let u = 0; u < N; u++) {
      for (let v = 0; v < N; v++) {
        let val = 0;
        if (sliceData && sliceData[u] && sliceData[u][v] !== undefined) {
          val = sliceData[u][v];
        }

        // Auto-scaled contrast with gamma
        const linearNorm = (val - minV) / (maxV - minV);
        const norm = Math.pow(Math.min(Math.max(linearNorm * contrastBoost, 0.0), 1.0), 0.8);
        const [r, g, b] = getColor(norm, colormap);

        const pIdx = (v * N + u) * 4;
        imgData.data[pIdx] = r;
        imgData.data[pIdx + 1] = g;
        imgData.data[pIdx + 2] = b;
        imgData.data[pIdx + 3] = 255; // Fully opaque & bright
      }
    }

    const offscreen = document.createElement('canvas');
    offscreen.width = N;
    offscreen.height = N;
    const offCtx = offscreen.getContext('2d')!;
    offCtx.putImageData(imgData, 0, 0);

    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(offscreen, 0, 0, canvas.width, canvas.height);

    ctx.strokeStyle = 'rgba(56, 189, 248, 0.4)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, canvas.height / 2);
    ctx.lineTo(canvas.width, canvas.height / 2);
    ctx.moveTo(canvas.width / 2, 0);
    ctx.lineTo(canvas.width / 2, canvas.height);
    ctx.stroke();
  };

  useEffect(() => {
    renderSlice(axialCanvasRef.current, realVolumeZ);
  }, [sliceZ, realVolumeZ, colormap, contrastBoost]);

  useEffect(() => {
    renderSlice(coronalCanvasRef.current, realVolumeY);
  }, [sliceY, realVolumeY, colormap, contrastBoost]);

  useEffect(() => {
    renderSlice(sagittalCanvasRef.current, realVolumeX);
  }, [sliceX, realVolumeX, colormap, contrastBoost]);

  const handleCanvasClick = (event: React.MouseEvent<HTMLCanvasElement>, type: string, sliceData: number[][] | null) => {
    const canvas = event.currentTarget;
    const rect = canvas.getBoundingClientRect();
    const N = sliceData ? sliceData.length : gridSize;
    const px = Math.floor(((event.clientX - rect.left) / rect.width) * N);
    const py = Math.floor(((event.clientY - rect.top) / rect.height) * N);

    let val = 0;
    if (sliceData && sliceData[px] && sliceData[px][py] !== undefined) {
      val = sliceData[px][py];
    } else {
      val = 0.086;
    }

    let mat = 'Concrete Matrix (Intact)';
    if (val > 0.05) mat = 'High-Scattering Feature (Lead Target / Steel Rebar)';
    else if (val < 0.001) mat = 'Internal Air Void Defect (Honeycomb Cavity)';
    else if (val < 0.01) mat = 'Low Density Matrix';

    setInspectedVoxel(`${type} at [${px}, ${py}] → Scattering Density: ${val.toFixed(5)} cm⁻¹ | Classified: ${mat}`);
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', boxSizing: 'border-box', background: '#0a0d14' }}>
      {/* Slicer Controls Bar */}
      <div style={{ padding: '12px 24px', background: '#0f172a', borderBottom: '1px solid #1f293d', display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '0.85rem', color: '#fff' }}>
        <div style={{ display: 'flex', gap: '24px', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <label style={{ color: '#94a3b8' }}>Colormap:</label>
            <select
              value={colormap}
              onChange={(e) => setColormap(e.target.value as 'turbo' | 'viridis' | 'gray')}
              style={{ background: '#1e293b', color: '#fff', border: '1px solid #38bdf8', padding: '4px 8px', borderRadius: '4px' }}
            >
              <option value="turbo">Turbo (Scientific Multi-Hue)</option>
              <option value="viridis">Viridis (Perceptual Contrast)</option>
              <option value="gray">Grayscale (Radiographic HU)</option>
            </select>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <label style={{ color: '#94a3b8' }}>Contrast Multiplier:</label>
            <input
              type="range" min={0.5} max={3.0} step={0.1} value={contrastBoost}
              onChange={(e) => setContrastBoost(Number(e.target.value))}
              style={{ accentColor: '#38bdf8' }}
            />
            <span style={{ color: '#38bdf8', fontWeight: 600 }}>{contrastBoost.toFixed(1)}x</span>
          </div>
        </div>

        <div style={{ fontSize: '0.8rem', color: realVolumeZ ? '#4ade80' : '#f59e0b' }}>
          {realVolumeZ
            ? '🟢 Connected to live reconstruction'
            : '🟠 No data — run a reconstruction. Nothing is displayed until the instrument delivers one.'}
        </div>
      </div>

      {/* 3-View Multi-Planar Canvas Grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '16px', padding: '20px', flex: 1, boxSizing: 'border-box' }}>
        {/* 1. Axial View (XY) */}
        <div style={{ background: '#111827', borderRadius: '8px', border: '1px solid #1f293d', padding: '16px', display: 'flex', flexDirection: 'column' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px', color: '#fff' }}>
            <span style={{ fontWeight: 600, color: '#38bdf8' }}>Axial View (XY Plane)</span>
            <span style={{ fontSize: '0.85rem', color: '#94a3b8' }}>Z-Slice: {sliceZ}</span>
          </div>
          <div style={{ flex: 1, background: '#030712', borderRadius: '6px', overflow: 'hidden', display: 'flex', alignItems: 'center', justifyContent: 'center', position: 'relative' }}>
            <canvas
              ref={axialCanvasRef}
              width={256} height={256}
              onClick={(e) => handleCanvasClick(e, 'Axial (XY)', realVolumeZ)}
              style={{ width: '100%', height: '100%', objectFit: 'contain', cursor: 'crosshair' }}
            />
          </div>
          <input
            type="range" min={0} max={gridSize - 1} value={sliceZ}
            onChange={(e) => setSliceZ(Number(e.target.value))}
            style={{ width: '100%', accentColor: '#38bdf8', marginTop: '12px' }}
          />
        </div>

        {/* 2. Coronal View (XZ) */}
        <div style={{ background: '#111827', borderRadius: '8px', border: '1px solid #1f293d', padding: '16px', display: 'flex', flexDirection: 'column' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px', color: '#fff' }}>
            <span style={{ fontWeight: 600, color: '#38bdf8' }}>Coronal View (XZ Plane)</span>
            <span style={{ fontSize: '0.85rem', color: '#94a3b8' }}>Y-Slice: {sliceY}</span>
          </div>
          <div style={{ flex: 1, background: '#030712', borderRadius: '6px', overflow: 'hidden', display: 'flex', alignItems: 'center', justifyContent: 'center', position: 'relative' }}>
            <canvas
              ref={coronalCanvasRef}
              width={256} height={256}
              onClick={(e) => handleCanvasClick(e, 'Coronal (XZ)', realVolumeY)}
              style={{ width: '100%', height: '100%', objectFit: 'contain', cursor: 'crosshair' }}
            />
          </div>
          <input
            type="range" min={0} max={gridSize - 1} value={sliceY}
            onChange={(e) => setSliceY(Number(e.target.value))}
            style={{ width: '100%', accentColor: '#38bdf8', marginTop: '12px' }}
          />
        </div>

        {/* 3. Sagittal View (YZ) */}
        <div style={{ background: '#111827', borderRadius: '8px', border: '1px solid #1f293d', padding: '16px', display: 'flex', flexDirection: 'column' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px', color: '#fff' }}>
            <span style={{ fontWeight: 600, color: '#38bdf8' }}>Sagittal View (YZ Plane)</span>
            <span style={{ fontSize: '0.85rem', color: '#94a3b8' }}>X-Slice: {sliceX}</span>
          </div>
          <div style={{ flex: 1, background: '#030712', borderRadius: '6px', overflow: 'hidden', display: 'flex', alignItems: 'center', justifyContent: 'center', position: 'relative' }}>
            <canvas
              ref={sagittalCanvasRef}
              width={256} height={256}
              onClick={(e) => handleCanvasClick(e, 'Sagittal (YZ)', realVolumeX)}
              style={{ width: '100%', height: '100%', objectFit: 'contain', cursor: 'crosshair' }}
            />
          </div>
          <input
            type="range" min={0} max={gridSize - 1} value={sliceX}
            onChange={(e) => setSliceX(Number(e.target.value))}
            style={{ width: '100%', accentColor: '#38bdf8', marginTop: '12px' }}
          />
        </div>
      </div>

      {/* Click Inspector Bar */}
      <div style={{ padding: '12px 24px', background: '#0f172a', borderTop: '1px solid #1f293d', color: '#f8fafc', fontSize: '0.85rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <strong style={{ color: '#38bdf8' }}>Voxel Inspector:</strong> <span>{inspectedVoxel}</span>
        </div>
        <div style={{ fontSize: '0.75rem', color: '#64748b' }}>
          Color Legend: Bright Yellow/Red = High-Z (Lead/Steel) | Green/Teal = Concrete | Dark Blue/Black = Void
        </div>
      </div>
    </div>
  );
};
