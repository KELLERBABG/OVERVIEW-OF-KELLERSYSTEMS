import React, { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

export type PresetType = 'bridge_pier' | 'astm_phantom' | 'nuclear_cargo' | 'subsurface_tunnel';

export const Viewport3D: React.FC = () => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [preset, setPreset] = useState<PresetType>('astm_phantom');
  const [showMuons, setShowMuons] = useState<boolean>(true);
  const [showVoxelCloud, setShowVoxelCloud] = useState<boolean>(true);
  const [showPocaPoints, setShowPocaPoints] = useState<boolean>(true);
  const [showVolumeFog, setShowVolumeFog] = useState<boolean>(true);
  const [cutawayZ, setCutawayZ] = useState<number>(0.0);
  const [muonSpeed, setMuonSpeed] = useState<number>(1);
  const [hoveredInfo, setHoveredInfo] = useState<string>('Hover over the volume to inspect material properties');
  const [liveStreamEvents, setLiveStreamEvents] = useState<number>(0);
  const [wsConnected, setWsConnected] = useState<boolean>(false);

  useEffect(() => {
    if (!containerRef.current) return;

    // 1. Scene & Camera Setup
    const scene = new THREE.Scene();
    scene.background = new THREE.Color('#0a0d14');

    const width = containerRef.current.clientWidth || 800;
    const height = containerRef.current.clientHeight || 600;

    const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 100);
    camera.position.set(3.2, 2.4, 3.6);

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    renderer.setSize(width, height);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    containerRef.current.appendChild(renderer.domElement);

    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.dampingFactor = 0.05;

    // 2. Lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.9);
    scene.add(ambientLight);

    const dirLight1 = new THREE.DirectionalLight(0x38bdf8, 1.6);
    dirLight1.position.set(5, 10, 7);
    scene.add(dirLight1);

    const dirLight2 = new THREE.DirectionalLight(0xffffff, 0.9);
    dirLight2.position.set(-5, -5, -5);
    scene.add(dirLight2);

    const grid = new THREE.GridHelper(5, 20, 0x0284c7, 0x1e293b);
    grid.position.y = -1.2;
    scene.add(grid);

    // 3. Detector Panels
    const panelGroup = new THREE.Group();
    const panelGeo = new THREE.BoxGeometry(1.6, 0.06, 1.6);
    const panelMat = new THREE.MeshStandardMaterial({
      color: 0x0369a1,
      transparent: true,
      opacity: 0.45,
      roughness: 0.2,
      metalness: 0.8,
    });
    const panelWireframeMat = new THREE.LineBasicMaterial({ color: 0x38bdf8 });

    const topPanel = new THREE.Mesh(panelGeo, panelMat);
    topPanel.position.set(0, 1.1, 0);
    topPanel.add(new THREE.LineSegments(new THREE.EdgesGeometry(panelGeo), panelWireframeMat));
    panelGroup.add(topPanel);

    const bottomPanel = new THREE.Mesh(panelGeo, panelMat);
    bottomPanel.position.set(0, -1.1, 0);
    bottomPanel.add(new THREE.LineSegments(new THREE.EdgesGeometry(panelGeo), panelWireframeMat));
    panelGroup.add(bottomPanel);
    scene.add(panelGroup);

    // 4. Target CAD Structure
    const targetGroup = new THREE.Group();

    const concreteGeo = new THREE.BoxGeometry(1.2, 1.2, 1.2);
    const concreteMat = new THREE.MeshStandardMaterial({
      color: 0x475569,
      transparent: true,
      opacity: 0.35,
      roughness: 0.8,
      metalness: 0.1,
    });
    const concreteMesh = new THREE.Mesh(concreteGeo, concreteMat);
    concreteMesh.userData = { material: 'Reinforced Concrete Matrix', density: '2.35 g/cm³', status: 'Intact' };
    targetGroup.add(concreteMesh);

    const rebarMat = new THREE.MeshStandardMaterial({ color: 0xf59e0b, metalness: 0.8, roughness: 0.2 });
    [-0.35, 0.35].forEach((rx) => {
      [-0.35, 0.35].forEach((rz) => {
        const rebar = new THREE.Mesh(new THREE.CylinderGeometry(0.03, 0.03, 1.1, 16), rebarMat);
        rebar.position.set(rx, 0, rz);
        rebar.userData = { material: 'Carbon Steel Rebar (Ø 20mm)', density: '7.87 g/cm³', status: 'Good Condition' };
        targetGroup.add(rebar);
      });
    });

    const voidMat = new THREE.MeshStandardMaterial({ color: 0x06b6d4, roughness: 0.3, transparent: true, opacity: 0.85 });
    const voidMesh = new THREE.Mesh(new THREE.SphereGeometry(0.18, 24, 24), voidMat);
    voidMesh.position.set(-0.25, 0.15, -0.2);
    voidMesh.userData = { material: 'Internal Honeycomb Air Void (Ø 45mm)', density: '0.0012 g/cm³', status: 'CRITICAL STRUCTURAL DEFECT' };
    targetGroup.add(voidMesh);

    const leadMat = new THREE.MeshStandardMaterial({ color: 0xef4444, roughness: 0.3, metalness: 0.5 });
    const leadMesh = new THREE.Mesh(new THREE.CylinderGeometry(0.14, 0.14, 0.5, 24), leadMat);
    leadMesh.position.set(0.25, -0.1, 0.15);
    leadMesh.userData = { material: 'Heavy Shielding Lead Target (Ø 25mm)', density: '11.35 g/cm³', status: 'High Scattering Center' };
    targetGroup.add(leadMesh);

    scene.add(targetGroup);

    // 5. 3D volumetric density cloud.
    //
    // This used to be a hardcoded phantom: a 16^3 grid containing a lead cylinder
    // and a void sphere at fixed positions, drawn regardless of what had been
    // reconstructed. It is now built from the volume the API actually reports,
    // with a transfer function over the real dynamic range. When nothing has been
    // reconstructed the cloud is empty.
    const volumeGroup = new THREE.Group();
    scene.add(volumeGroup);
    let volumeMesh: THREE.InstancedMesh | null = null;

    const renderVolume = (values: number[], dims: number[]) => {
      if (volumeMesh) {
        volumeGroup.remove(volumeMesh);
        volumeMesh.geometry.dispose();
        (volumeMesh.material as THREE.Material).dispose();
        volumeMesh = null;
      }
      const [nx, ny, nz] = dims;
      if (!values.length || !nx || !ny || !nz) return;

      let lo = Infinity;
      let hi = -Infinity;
      for (const v of values) {
        if (v < lo) lo = v;
        if (v > hi) hi = v;
      }
      const span = hi > lo ? hi - lo : 1e-9;

      const geo = new THREE.BoxGeometry(1.2 / nx, 1.2 / ny, 1.2 / nz);
      const mat = new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.2 });
      const mesh = new THREE.InstancedMesh(geo, mat, values.length);
      const dummy = new THREE.Object3D();
      const col = new THREE.Color();

      let idx = 0;
      for (let i = 0; i < nx; i++) {
        for (let j = 0; j < ny; j++) {
          for (let k = 0; k < nz; k++) {
            const v = values[i * ny * nz + j * nz + k];
            const t = Math.min(Math.max((v - lo) / span, 0), 1);
            dummy.position.set(
              (i / Math.max(nx - 1, 1) - 0.5) * 1.2,
              (j / Math.max(ny - 1, 1) - 0.5) * 1.2,
              (k / Math.max(nz - 1, 1) - 0.5) * 1.2,
            );
            dummy.scale.setScalar(0.35 + 0.65 * t);
            dummy.updateMatrix();
            mesh.setMatrixAt(idx, dummy.matrix);
            // Low density reads cool, high density reads hot.
            col.setHSL(0.58 - 0.58 * t, 0.75, 0.25 + 0.35 * t);
            mesh.setColorAt(idx, col);
            idx++;
          }
        }
      }
      mesh.instanceMatrix.needsUpdate = true;
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
      volumeMesh = mesh;
      volumeGroup.add(mesh);
    };

    const loadVolume = async () => {
      try {
        const res = await fetch('http://localhost:8000/api/v1/volume-raw');
        if (!res.ok) return;
        const body = await res.json();
        renderVolume(body.values ?? [], body.grid_size ?? []);
      } catch {
        // Instrument not reachable; leave the cloud empty rather than inventing one.
      }
    };
    void loadVolume();

    // 6. Dynamic PoCA Scattering Point Cloud
    const maxPoca = 1500;
    const pocaGeo = new THREE.BufferGeometry();
    const pocaPosArray = new Float32Array(maxPoca * 3);
    const pocaColArray = new Float32Array(maxPoca * 3);

    pocaGeo.setAttribute('position', new THREE.BufferAttribute(pocaPosArray, 3));
    pocaGeo.setAttribute('color', new THREE.BufferAttribute(pocaColArray, 3));
    const pocaMat = new THREE.PointsMaterial({ size: 0.035, vertexColors: true, transparent: true, opacity: 0.95 });
    const pocaPointsMesh = new THREE.Points(pocaGeo, pocaMat);
    scene.add(pocaPointsMesh);

    let currentPocaCount = 0;

    const addPoCAPoint = (x: number, y: number, z: number, isLead: boolean, isVoid: boolean) => {
      if (currentPocaCount >= maxPoca) currentPocaCount = 0;
      const idx = currentPocaCount;
      pocaPosArray[idx * 3] = (x / 250.0) * 0.6;
      pocaPosArray[idx * 3 + 1] = (z / 200.0) * 0.6;
      pocaPosArray[idx * 3 + 2] = (y / 250.0) * 0.6;

      if (isLead) {
        pocaColArray[idx * 3] = 1.0; pocaColArray[idx * 3 + 1] = 0.2; pocaColArray[idx * 3 + 2] = 0.2;
      } else if (isVoid) {
        pocaColArray[idx * 3] = 0.0; pocaColArray[idx * 3 + 1] = 0.9; pocaColArray[idx * 3 + 2] = 0.9;
      } else {
        pocaColArray[idx * 3] = 0.2; pocaColArray[idx * 3 + 1] = 0.7; pocaColArray[idx * 3 + 2] = 1.0;
      }

      currentPocaCount++;
      pocaGeo.attributes.position.needsUpdate = true;
      pocaGeo.attributes.color.needsUpdate = true;
      pocaGeo.setDrawRange(0, currentPocaCount);
    };

    // Pull whatever points the instrument has actually reconstructed. Nothing is
    // fabricated: this used to pre-populate 400 random points dressed up as lead
    // and void hits, which is indistinguishable on screen from a real scan.
    const loadPocaCloud = async () => {
      try {
        const res = await fetch('http://localhost:8000/api/v1/poca-cloud');
        if (!res.ok) return;
        const body = await res.json();
        for (const pt of body.points ?? []) {
          const heavy = pt.angle_mrad > 20.0;
          addPoCAPoint(pt.x, pt.y, pt.z, heavy, !heavy);
        }
      } catch {
        // Instrument not reachable. Show nothing rather than inventing points.
      }
    };
    void loadPocaCloud();

    // 7. WebSocket Live Muon Particle Stream
    let ws: WebSocket | null = null;
    try {
      ws = new WebSocket('ws://localhost:8000/ws/live-stream');
      ws.onopen = () => setWsConnected(true);
      ws.onclose = () => setWsConnected(false);
      ws.onmessage = (evt) => {
        try {
          const msg = JSON.parse(evt.data);
          if (msg.type === 'MUON_EVENT' && msg.p_in && msg.p_out) {
            setLiveStreamEvents((prev) => prev + 1);
            // The stream carries measured quantities only: no energy, no truth
            // flag. "Heavy" here means the fitted angle is well above the
            // detector's own resolution, which is a real, derivable criterion.
            const heavy = (msg.angle_mrad ?? 0) > 3.0 * (msg.sigma_angle_mrad ?? 1.0);
            addPoCAPoint(msg.p_in[0], msg.p_in[1], msg.p_in[2], heavy, !heavy);
            spawnRay(msg.p_in[0], msg.p_in[2], msg.angle_mrad ?? 0, msg.sigma_angle_mrad ?? 0);
          }
        } catch {
          // ignore
        }
      };
    } catch {
      // ignore
    }

    // 8. Animated muon rays, spawned from the live track stream.
    //
    // These used to be seeded with random positions and random "deflections" and
    // animated regardless of what had been detected. They are now driven by the
    // tracks the instrument reports: a ray appears where a real muon crossed and
    // bends by its measured scattering angle. With no stream, nothing moves.
    const rayGroup = new THREE.Group();
    type AnimatedRay = {
      line: THREE.Line;
      speed: number;
      deflection: number;
      active: boolean;
      bornAt: number;
    };
    const rayPool: AnimatedRay[] = [];

    for (let i = 0; i < 24; i++) {
      const rayGeo = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(0, 0, 0),
        new THREE.Vector3(0, -0.45, 0),
      ]);
      const rayMat = new THREE.LineBasicMaterial({
        color: 0x38bdf8,
        linewidth: 2,
        transparent: true,
        opacity: 0.95,
      });
      const line = new THREE.Line(rayGeo, rayMat);
      line.visible = false;
      rayGroup.add(line);
      rayPool.push({ line, speed: 0.05, deflection: 0, active: false, bornAt: 0 });
    }
    scene.add(rayGroup);

    let rayCursor = 0;

    /**
     * Place a ray for one measured track.
     * `angleMrad` is the fitted scattering angle; `sigmaMrad` its uncertainty.
     * The drawn bend is the measured value, capped so a single hard scatter does
     * not throw the line off screen.
     */
    const spawnRay = (xMm: number, zMm: number, angleMrad: number, sigmaMrad: number) => {
      const slot = rayPool[rayCursor % rayPool.length];
      rayCursor++;
      slot.line.position.set((xMm / 250.0) * 0.6, 2.0, (zMm / 250.0) * 0.6);
      // Bend proportional to how far the measured angle sits above the detector's
      // own resolution -- below that there is nothing to show.
      const excess = Math.max(0, angleMrad - (sigmaMrad || 0));
      slot.deflection = Math.min(excess / 60.0, 1.0) * 0.12;
      slot.speed = 0.04 + Math.min(angleMrad / 400.0, 1.0) * 0.04;
      slot.active = true;
      slot.bornAt = performance.now();
      slot.line.visible = true;
      (slot.line.material as THREE.LineBasicMaterial).color.setHex(
        excess > 2.0 * (sigmaMrad || 1) ? 0xef4444 : 0x38bdf8,
      );
    };

    // 9. Resize Observer
    const resizeHandler = () => {
      if (!containerRef.current) return;
      const w = containerRef.current.clientWidth;
      const h = containerRef.current.clientHeight;
      if (w > 0 && h > 0) {
        camera.aspect = w / h;
        camera.updateProjectionMatrix();
        renderer.setSize(w, h);
      }
    };

    const resizeObserver = new ResizeObserver(() => {
      resizeHandler();
    });
    resizeObserver.observe(containerRef.current);

    // 10. Hover Inspector
    const raycaster = new THREE.Raycaster();
    const mouse = new THREE.Vector2();

    const onMouseMove = (event: MouseEvent) => {
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) return;
      mouse.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
      mouse.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;

      raycaster.setFromCamera(mouse, camera);
      const intersects = raycaster.intersectObjects(targetGroup.children);
      if (intersects.length > 0) {
        const data = intersects[0].object.userData;
        if (data && data.material) {
          setHoveredInfo(`${data.material} | Density: ${data.density} | Assessment: ${data.status}`);
        }
      } else {
        setHoveredInfo('Hover over any internal element to inspect structural density & material properties');
      }
    };

    window.addEventListener('mousemove', onMouseMove);

    // 11. Animation Loop
    let animId: number;
    const animate = () => {
      animId = requestAnimationFrame(animate);
      controls.update();

      if (showMuons) {
        rayPool.forEach((r) => {
          if (!r.active) return;
          r.line.position.y -= r.speed * muonSpeed;
          if (r.line.position.y < 0.2 && r.line.position.y > -0.2) {
            r.line.position.x += r.deflection;
          }
          if (r.line.position.y < -1.8) {
            // Retire the ray. It is respawned by the next real track, not by a
            // random number.
            r.active = false;
            r.line.visible = false;
          }
        });
      }

      rayGroup.visible = showMuons;
      targetGroup.visible = showVoxelCloud;
      pocaPointsMesh.visible = showPocaPoints;
      volumeGroup.visible = showVolumeFog;
      concreteMesh.position.y = cutawayZ * 0.6;

      renderer.render(scene, camera);
    };
    animate();

    return () => {
      cancelAnimationFrame(animId);
      resizeObserver.disconnect();
      window.removeEventListener('mousemove', onMouseMove);
      if (ws) ws.close();
      renderer.dispose();
      if (containerRef.current) {
        containerRef.current.innerHTML = '';
      }
    };
  }, [preset, showMuons, showVoxelCloud, showPocaPoints, showVolumeFog, cutawayZ, muonSpeed]);

  return (
    <div style={{ width: '100%', height: '100%', position: 'relative', display: 'flex', flexDirection: 'column', background: '#0a0d14' }}>
      {/* HUD Controls */}
      <div style={{
        position: 'absolute', top: 16, left: 16, zIndex: 10,
        background: 'rgba(15, 23, 42, 0.9)', backdropFilter: 'blur(8px)',
        border: '1px solid #1f293d', borderRadius: '8px', padding: '16px',
        color: '#fff', width: '330px', display: 'flex', flexDirection: 'column', gap: '12px'
      }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span style={{ fontWeight: 700, color: '#38bdf8', fontSize: '0.95rem' }}>3D Tomography Digital Twin</span>
          <span style={{ fontSize: '0.75rem', background: wsConnected ? '#166534' : '#0284c7', color: '#fff', padding: '2px 8px', borderRadius: '12px', fontWeight: 600 }}>
            {wsConnected ? `⚡ Stream: ${liveStreamEvents} events` : 'WebGL Ready'}
          </span>
        </div>

        <div>
          <label style={{ fontSize: '0.8rem', color: '#94a3b8', display: 'block', marginBottom: '4px' }}>Target Scenario Preset:</label>
          <select
            value={preset}
            onChange={(e) => setPreset(e.target.value as PresetType)}
            style={{ width: '100%', background: '#1e293b', color: '#fff', border: '1px solid #38bdf8', borderRadius: '4px', padding: '6px 8px', fontSize: '0.85rem' }}
          >
            <option value="astm_phantom">ASTM E1441 Reference Cube (Lead/Iron/Void)</option>
            <option value="bridge_pier">Concrete Bridge Pier (Tendon Duct Voids)</option>
            <option value="nuclear_cargo">Cargo Container (Shielded SNM Detection)</option>
            <option value="subsurface_tunnel">Subterranean Tunnel & Cavity Void</option>
          </select>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '0.8rem' }}>
          <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
            <input type="checkbox" checked={showVolumeFog} onChange={(e) => setShowVolumeFog(e.target.checked)} style={{ accentColor: '#38bdf8' }} />
            <span style={{ color: '#38bdf8', fontWeight: 600 }}>🌁 3D Volumetric Density Cloud</span>
          </label>
          <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
            <input type="checkbox" checked={showPocaPoints} onChange={(e) => setShowPocaPoints(e.target.checked)} style={{ accentColor: '#38bdf8' }} />
            <span>3D PoCA Reconstructed Scattering Cloud</span>
          </label>
          <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
            <input type="checkbox" checked={showMuons} onChange={(e) => setShowMuons(e.target.checked)} style={{ accentColor: '#38bdf8' }} />
            <span>Live Cosmic Muon Flux Rays (WebSocket)</span>
          </label>
          <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
            <input type="checkbox" checked={showVoxelCloud} onChange={(e) => setShowVoxelCloud(e.target.checked)} style={{ accentColor: '#38bdf8' }} />
            <span>CAD Ground Truth Geometry (Mesh)</span>
          </label>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginTop: '4px' }}>
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', color: '#94a3b8' }}>
              <span>Simulation Muon Flux Speed:</span>
              <span style={{ color: '#38bdf8' }}>{muonSpeed}x</span>
            </div>
            <input
              type="range" min={0.2} max={3} step={0.2} value={muonSpeed}
              onChange={(e) => setMuonSpeed(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#38bdf8' }}
            />
          </div>

          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', color: '#94a3b8' }}>
              <span>3D Cutaway Plane (Inspect Inside):</span>
              <span style={{ color: '#38bdf8' }}>{(cutawayZ * 100).toFixed(0)}%</span>
            </div>
            <input
              type="range" min={0} max={1} step={0.05} value={cutawayZ}
              onChange={(e) => setCutawayZ(Number(e.target.value))}
              style={{ width: '100%', accentColor: '#38bdf8' }}
            />
          </div>
        </div>
      </div>

      {/* Hover Bar */}
      <div style={{
        position: 'absolute', bottom: 16, left: 16, right: 16, zIndex: 10,
        background: 'rgba(15, 23, 42, 0.9)', backdropFilter: 'blur(8px)',
        border: '1px solid #1f293d', borderRadius: '8px', padding: '12px 20px',
        color: '#fff', display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: '0.85rem'
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <span style={{ color: '#38bdf8', fontWeight: 600 }}>🔍 Live Element Inspector:</span>
          <span style={{ color: '#f8fafc' }}>{hoveredInfo}</span>
        </div>
        <div style={{ color: '#64748b', fontSize: '0.75rem' }}>
          Left-Click Drag: Rotate | Right-Click: Pan | Scroll: Zoom
        </div>
      </div>

      <div ref={containerRef} style={{ width: '100%', height: '100%', minHeight: '400px' }} />
    </div>
  );
};
