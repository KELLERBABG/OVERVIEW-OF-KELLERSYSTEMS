import React, { useEffect, useState } from 'react';

/**
 * Station telemetry.
 *
 * This panel used to render four invented stations ("Alpha-North" ... ) with
 * fabricated battery, temperature, barometric-flux and RTK readings, plus a log
 * asserting values that were never measured -- including a chi-squared figure
 * this geometry cannot compute, and a 128-channel bias trim for a 512-channel
 * detector. On screen it was indistinguishable from a real survey.
 *
 * It now displays only what the instrument reports, and shows an explicit empty
 * state when nothing has run.
 */

interface Station {
  id: number;
  name: string;
  status: string;
  channel_count: number;
  tdc_bin_ps: number;
  channels_calibrated: number;
  deskew_residual_ps: number;
  calibration_source: string;
  track_efficiency: number;
  sigma_theta_mrad: number;
  tof_median_ns: number | null;
  lever_arm_mm: number;
  packets_decoded: number;
  packets_rejected: number;
  tracks: number;
}

interface Telemetry {
  connected: boolean;
  stations: Station[];
  log: string[];
  note?: string;
}

const ENDPOINT = 'http://localhost:8000/api/v1/telemetry';

export const StationTelemetry: React.FC = () => {
  const [telemetry, setTelemetry] = useState<Telemetry | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const poll = async () => {
      try {
        const res = await fetch(ENDPOINT);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const body: Telemetry = await res.json();
        if (!cancelled) {
          setTelemetry(body);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setTelemetry(null);
          setError(err instanceof Error ? err.message : 'unreachable');
        }
      }
    };

    void poll();
    const timer = setInterval(poll, 2000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, []);

  const stations = telemetry?.stations ?? [];

  const readout = (label: string, value: React.ReactNode, colour = '#fff') => (
    <div>
      {label}: <strong style={{ color: colour }}>{value}</strong>
    </div>
  );

  return (
    <div style={{ padding: '24px', color: '#fff', display: 'flex', flexDirection: 'column', gap: '20px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h2 style={{ margin: 0, fontSize: '1.25rem', color: '#38bdf8' }}>Station Telemetry</h2>
          <div style={{ fontSize: '0.85rem', color: '#94a3b8', marginTop: '4px' }}>
            Measured state of the acquisition chain. Values are reported by the
            instrument; nothing here is a placeholder.
          </div>
        </div>
        <div style={{
          fontSize: '0.8rem',
          color: telemetry?.connected ? '#4ade80' : '#f59e0b',
          fontWeight: 600,
        }}>
          {telemetry?.connected ? '🟢 Connected' : '🟠 Not connected'}
          {error ? ` — ${error}` : ''}
        </div>
      </div>

      {stations.length === 0 ? (
        <div style={{
          background: '#111827', borderRadius: '8px', border: '1px solid #1f293d',
          padding: '24px', color: '#94a3b8',
        }}>
          <div style={{ color: '#f59e0b', fontWeight: 600, marginBottom: '8px' }}>
            No telemetry available
          </div>
          <div style={{ fontSize: '0.9rem' }}>
            {telemetry?.note
              ?? 'The acquisition service is not reachable. Run a reconstruction to populate this panel.'}
          </div>
          <div style={{ fontSize: '0.85rem', marginTop: '10px', color: '#64748b' }}>
            A single station is modelled. No multi-station mesh exists, so no
            cross-station synchronisation is reported.
          </div>
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: `repeat(${Math.min(stations.length, 4)}, 1fr)`, gap: '16px' }}>
          {stations.map((st) => (
            <div key={st.id} style={{ background: '#111827', borderRadius: '8px', border: '1px solid #1f293d', padding: '16px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '12px' }}>
                <span style={{ fontWeight: 600, color: '#f8fafc' }}>{st.name}</span>
                <span style={{
                  background: st.status === 'ACTIVE' ? '#166534' : '#374151',
                  color: st.status === 'ACTIVE' ? '#86efac' : '#cbd5e1',
                  fontSize: '0.75rem', padding: '2px 8px', borderRadius: '12px', fontWeight: 600,
                }}>
                  {st.status}
                </span>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '0.85rem', color: '#94a3b8' }}>
                {readout('Tracks fitted', st.tracks)}
                {readout('Packets decoded', st.packets_decoded)}
                {readout('Packets rejected', st.packets_rejected,
                  st.packets_rejected > 0 ? '#f59e0b' : '#4ade80')}
                {readout('TDC fine bin', `${st.tdc_bin_ps.toFixed(2)} ps`)}
                {readout('Channels deskewed', `${st.channels_calibrated} / ${st.channel_count}`)}
                {readout('Deskew residual', `${st.deskew_residual_ps.toFixed(1)} ps`)}
                {readout('Angular resolution', `${st.sigma_theta_mrad.toFixed(2)} mrad`)}
                {readout('Lever arm', `${st.lever_arm_mm.toFixed(1)} mm`)}
                {readout('Median ToF',
                  st.tof_median_ns === null ? 'not measured' : `${st.tof_median_ns.toFixed(3)} ns`)}
                {readout('Track efficiency', st.track_efficiency.toFixed(3))}
              </div>
            </div>
          ))}
        </div>
      )}

      <div style={{ background: '#111827', borderRadius: '8px', border: '1px solid #1f293d', padding: '20px', flex: 1 }}>
        <h3 style={{ margin: '0 0 12px 0', fontSize: '1rem', color: '#38bdf8' }}>System Log</h3>
        <div style={{
          background: '#030712', borderRadius: '6px', border: '1px solid #1f293d',
          padding: '12px', fontFamily: 'monospace', fontSize: '0.8rem',
          color: '#94a3b8', height: '180px', overflowY: 'auto',
        }}>
          {(telemetry?.log ?? []).length === 0
            ? <div style={{ color: '#64748b' }}>No log entries. Run a reconstruction.</div>
            : telemetry!.log.map((line, i) => <div key={i}>{line}</div>)}
        </div>
      </div>
    </div>
  );
};
