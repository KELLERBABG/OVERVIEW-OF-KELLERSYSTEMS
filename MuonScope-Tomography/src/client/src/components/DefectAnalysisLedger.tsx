import React, { useEffect, useMemo, useState } from 'react';
import {
  AlertTriangle,
  Download,
  Search,
  Filter,
  CheckCircle2,
  ExternalLink,
  ChevronRight,
} from 'lucide-react';

/**
 * Defect analysis ledger.
 *
 * This component used to ship a hardcoded database of six named defects across
 * a "5-Story Commercial Building" and a "40m Interstate Highway Bridge" -- with
 * measured densities, volumes, CNR values and structural-impact assessments for
 * structures that were never surveyed -- and KPI cards asserting "2,688 m3
 * monitored", "Mean CNR 28.28 dB" and a "Structural Health Index 91.4%". It was
 * also not mounted anywhere, so those figures were invisible dead data waiting to
 * be wired up by someone who would have had no way to tell they were invented.
 *
 * It now renders whatever the caller passes. Every KPI is computed from the
 * supplied records, and the empty state is explicit.
 *
 * Two field changes worth noting:
 *  * ``astmClassification`` is now ``classification``. ASTM E1441 is a *guide* to
 *    CT imaging and defines no defect classes, so the old values ("ASTM E1441
 *    Class 1 Isolated Subsurface Void") were invented attributions. The field now
 *    carries whatever scheme the caller actually used.
 *  * ``cnrValue`` is documented as dimensionless. CNR is a ratio of a mean
 *    difference to a standard deviation; the old UI displayed it in "dB".
 */

export interface DefectRecord {
  id: string;
  asset: string;
  assetKey: string;
  element: string;
  location: string;
  coordinates: [number, number, number]; // [X, Y, Z] in metres
  defectType: string;
  measuredDensity: string;
  nominalDensity: string;
  densityDeficit: string;
  volumeCm3: number;
  maxDiameterMm: number;
  /** Contrast-to-noise ratio. Dimensionless. */
  cnrValue: number;
  severity: 'CRITICAL' | 'WARNING' | 'BENCHMARK';
  structuralImpact: string;
  classification: string;
}

interface LedgerProps {
  /** Records to display. Empty or omitted renders the empty state. */
  defects?: DefectRecord[];
}

const severityBadge = (severity: DefectRecord['severity']) =>
  severity === 'CRITICAL' ? 'badge-danger' : severity === 'WARNING' ? 'badge-warn' : 'badge-ok';

export const DefectAnalysisLedger: React.FC<LedgerProps> = ({ defects = [] }) => {
  const [selectedId, setSelectedId] = useState<string | null>(defects[0]?.id ?? null);
  const [searchTerm, setSearchTerm] = useState<string>('');
  const [filterSeverity, setFilterSeverity] = useState<string>('ALL');
  const [exportNotice, setExportNotice] = useState<string | null>(null);

  useEffect(() => {
    if (defects.length === 0) {
      setSelectedId(null);
    } else if (!defects.some((d) => d.id === selectedId)) {
      setSelectedId(defects[0].id);
    }
  }, [defects, selectedId]);

  const filtered = useMemo(
    () =>
      defects.filter((d) => {
        const term = searchTerm.toLowerCase();
        const matchesSearch =
          !term ||
          d.id.toLowerCase().includes(term) ||
          d.asset.toLowerCase().includes(term) ||
          d.element.toLowerCase().includes(term) ||
          d.defectType.toLowerCase().includes(term);
        const matchesSev = filterSeverity === 'ALL' || d.severity === filterSeverity;
        return matchesSearch && matchesSev;
      }),
    [defects, searchTerm, filterSeverity],
  );

  const selectedDefect = useMemo(
    () => defects.find((d) => d.id === selectedId) ?? null,
    [defects, selectedId],
  );

  // KPIs computed from the records actually supplied.
  const kpis = useMemo(() => {
    const assets = new Set(defects.map((d) => d.assetKey));
    const critical = defects.filter((d) => d.severity === 'CRITICAL').length;
    const warning = defects.filter((d) => d.severity === 'WARNING').length;
    const cnrs = defects.map((d) => d.cnrValue).filter((v) => Number.isFinite(v));
    const meanCnr = cnrs.length ? cnrs.reduce((a, b) => a + b, 0) / cnrs.length : null;
    const totalVolume = defects.reduce((a, d) => a + (Number.isFinite(d.volumeCm3) ? d.volumeCm3 : 0), 0);
    return { assetCount: assets.size, critical, warning, meanCnr, totalVolume };
  }, [defects]);

  const handleExport = (format: 'PDF' | 'IFC4' | 'CSV') => {
    // No certificate is generated here; the message says what it is.
    setExportNotice(
      selectedDefect
        ? `Export requested for ${selectedDefect.id} as ${format}. No certificate is produced by this view.`
        : `Export requested as ${format}. No records to export.`,
    );
    setTimeout(() => setExportNotice(null), 4000);
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', boxSizing: 'border-box', background: 'var(--bg)', overflowY: 'auto' }}>
      {/* Top Header Bar */}
      <div style={{
        padding: '16px 24px', background: 'var(--bg-elev)', borderBottom: '1px solid var(--border)',
        display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '16px',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <div className="brand-mark" style={{ background: 'rgba(239, 68, 68, 0.15)', color: '#ef4444' }}>
            <AlertTriangle size={20} />
          </div>
          <div>
            <h1 style={{ fontSize: '1.05rem', margin: 0, fontWeight: 700, display: 'flex', alignItems: 'center', gap: 8 }}>
              Porosity &amp; Structural Defect Ledger
            </h1>
            <span style={{ fontSize: '0.78rem', color: 'var(--text-muted)' }}>
              Anomaly extraction and density-deficit reporting. Populated from a
              reconstruction; empty until one is supplied.
            </span>
          </div>
        </div>

        <div style={{ display: 'flex', gap: '8px' }}>
          <button onClick={() => handleExport('PDF')} className="btn btn-primary" style={{ fontSize: '0.82rem', padding: '6px 12px' }}>
            <Download size={14} /> Export Report (PDF)
          </button>
          <button onClick={() => handleExport('IFC4')} className="btn btn-ghost" style={{ fontSize: '0.82rem', padding: '6px 12px' }}>
            <ExternalLink size={14} /> Export BIM (IFC4)
          </button>
        </div>
      </div>

      {exportNotice && (
        <div style={{ padding: '10px 24px', background: 'rgba(56, 189, 248, 0.12)', borderBottom: '1px solid #38bdf8', color: '#7dd3fc', fontSize: '0.82rem', display: 'flex', alignItems: 'center', gap: 8 }}>
          <CheckCircle2 size={16} /> {exportNotice}
        </div>
      )}

      {/* KPI Overview Summary Cards - every value computed from the records */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '16px', padding: '20px 24px 8px 24px' }}>
        <div className="card" style={{ padding: '14px' }}>
          <span style={{ fontSize: '0.74rem', color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 700 }}>Assets Represented</span>
          <div style={{ fontSize: '1.4rem', fontWeight: 800, color: 'var(--accent)', marginTop: 4, fontFamily: 'var(--font-mono)' }}>
            {kpis.assetCount}
          </div>
          <span style={{ fontSize: '0.72rem', color: 'var(--text-faint)' }}>
            {defects.length} record{defects.length === 1 ? '' : 's'}
          </span>
        </div>

        <div className="card" style={{ padding: '14px' }}>
          <span style={{ fontSize: '0.74rem', color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 700 }}>Anomalies</span>
          <div style={{ fontSize: '1.4rem', fontWeight: 800, color: '#f87171', marginTop: 4, fontFamily: 'var(--font-mono)' }}>
            {defects.length}
          </div>
          <span style={{ fontSize: '0.72rem', color: 'var(--text-faint)' }}>
            {kpis.critical} critical · {kpis.warning} warning
          </span>
        </div>

        <div className="card" style={{ padding: '14px' }}>
          <span style={{ fontSize: '0.74rem', color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 700 }}>Mean CNR</span>
          <div style={{ fontSize: '1.4rem', fontWeight: 800, color: '#38bdf8', marginTop: 4, fontFamily: 'var(--font-mono)' }}>
            {kpis.meanCnr === null ? '—' : kpis.meanCnr.toFixed(2)}
          </div>
          <span style={{ fontSize: '0.72rem', color: 'var(--text-faint)' }}>dimensionless ratio</span>
        </div>

        <div className="card" style={{ padding: '14px' }}>
          <span style={{ fontSize: '0.74rem', color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 700 }}>Total Anomaly Volume</span>
          <div style={{ fontSize: '1.4rem', fontWeight: 800, color: '#4ade80', marginTop: 4, fontFamily: 'var(--font-mono)' }}>
            {kpis.totalVolume >= 1000 ? `${(kpis.totalVolume / 1000).toFixed(2)} dm³` : `${kpis.totalVolume} cm³`}
          </div>
          <span style={{ fontSize: '0.72rem', color: 'var(--text-faint)' }}>sum of supplied records</span>
        </div>
      </div>

      {defects.length === 0 ? (
        <div style={{ margin: '16px 24px', padding: '24px', background: 'var(--bg-elev)', border: '1px solid var(--border)', borderRadius: '6px', color: 'var(--text-muted)' }}>
          <div style={{ color: '#f59e0b', fontWeight: 600, marginBottom: 8 }}>No defect records</div>
          <div style={{ fontSize: '0.85rem' }}>
            Run a reconstruction and segment the volume into anomalies. This view
            reports only what it is given; it does not contain sample data.
          </div>
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: '1.1fr 0.9fr', gap: '20px', padding: '16px 24px', flex: 1 }}>
          {/* Left: Defect Table */}
          <div className="panel" style={{ padding: '16px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', alignItems: 'center' }}>
              <div style={{ display: 'flex', alignItems: 'center', background: 'var(--inset)', padding: '6px 12px', borderRadius: '4px', border: '1px solid var(--border)', flex: 1 }}>
                <Search size={15} style={{ color: 'var(--text-muted)', marginRight: 8 }} />
                <input
                  type="text"
                  placeholder="Search by defect ID, element, asset, or flaw type..."
                  value={searchTerm}
                  onChange={(e) => setSearchTerm(e.target.value)}
                  style={{ background: 'transparent', border: 'none', color: 'var(--text)', outline: 'none', width: '100%', fontSize: '0.82rem' }}
                />
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <Filter size={14} style={{ color: 'var(--text-muted)' }} />
                <select
                  value={filterSeverity}
                  onChange={(e) => setFilterSeverity(e.target.value)}
                  style={{ fontSize: '0.8rem', padding: '6px 10px', width: 'auto' }}
                >
                  <option value="ALL">All Severities</option>
                  <option value="CRITICAL">Critical Only</option>
                  <option value="WARNING">Warning Only</option>
                  <option value="BENCHMARK">Benchmark</option>
                </select>
              </div>
            </div>

            <div style={{ overflowX: 'auto', flex: 1 }}>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.8rem', textAlign: 'left' }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid var(--border)', color: 'var(--text-muted)' }}>
                    <th style={{ padding: '8px 10px' }}>Defect ID</th>
                    <th style={{ padding: '8px 10px' }}>Asset &amp; Element</th>
                    <th style={{ padding: '8px 10px' }}>Type</th>
                    <th style={{ padding: '8px 10px' }}>Volume</th>
                    <th style={{ padding: '8px 10px' }}>Severity</th>
                    <th style={{ padding: '8px 10px', textAlign: 'right' }}>Action</th>
                  </tr>
                </thead>
                <tbody>
                  {filtered.map((d) => (
                    <tr
                      key={d.id}
                      onClick={() => setSelectedId(d.id)}
                      style={{
                        borderBottom: '1px solid var(--border)',
                        cursor: 'pointer',
                        background: selectedId === d.id ? 'rgba(56, 189, 248, 0.12)' : 'transparent',
                      }}
                    >
                      <td style={{ padding: '10px', fontWeight: 700, fontFamily: 'var(--font-mono)', color: 'var(--accent)' }}>
                        {d.id}
                      </td>
                      <td style={{ padding: '10px' }}>
                        <div style={{ fontWeight: 600, color: 'var(--text)' }}>{d.asset}</div>
                        <div style={{ fontSize: '0.72rem', color: 'var(--text-muted)' }}>{d.element}</div>
                      </td>
                      <td style={{ padding: '10px', color: 'var(--text-muted)' }}>{d.defectType}</td>
                      <td style={{ padding: '10px', fontFamily: 'var(--font-mono)' }}>
                        {d.volumeCm3 >= 1000 ? `${(d.volumeCm3 / 1000).toFixed(1)} dm³` : `${d.volumeCm3} cm³`}
                      </td>
                      <td style={{ padding: '10px' }}>
                        <span className={`badge ${severityBadge(d.severity)}`}>{d.severity}</span>
                      </td>
                      <td style={{ padding: '10px', textAlign: 'right' }}>
                        <ChevronRight size={16} color="var(--text-muted)" />
                      </td>
                    </tr>
                  ))}
                  {filtered.length === 0 && (
                    <tr>
                      <td colSpan={6} style={{ padding: '16px', color: 'var(--text-muted)' }}>
                        No records match the current search or filter.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>

          {/* Right: Selected defect detail */}
          <div className="panel" style={{ padding: '20px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
            {selectedDefect === null ? (
              <div style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>
                Select a record to inspect it.
              </div>
            ) : (
              <>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', borderBottom: '1px solid var(--border)', paddingBottom: '12px' }}>
                  <div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                      <span className={`badge ${severityBadge(selectedDefect.severity)}`}>
                        {selectedDefect.severity}
                      </span>
                      <span style={{ fontSize: '0.75rem', fontFamily: 'var(--font-mono)', color: 'var(--text-muted)' }}>
                        {selectedDefect.classification}
                      </span>
                    </div>
                    <h2 style={{ fontSize: '1.15rem', margin: '6px 0 2px 0', color: '#fff' }}>{selectedDefect.defectType}</h2>
                    <span style={{ fontSize: '0.82rem', color: 'var(--accent)' }}>
                      {selectedDefect.asset} · {selectedDefect.element}
                    </span>
                  </div>
                  <div style={{ textAlign: 'right' }}>
                    <div style={{ fontSize: '0.74rem', color: 'var(--text-muted)' }}>Centroid Location</div>
                    <div style={{ fontSize: '0.84rem', fontFamily: 'var(--font-mono)', color: '#fff', fontWeight: 700 }}>
                      [{selectedDefect.coordinates.map((c) => c.toFixed(2)).join(', ')}] m
                    </div>
                  </div>
                </div>

                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '10px' }}>
                  <div style={{ background: 'var(--inset)', padding: '10px', borderRadius: '4px', border: '1px solid var(--border)' }}>
                    <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>Measured Density</div>
                    <div style={{ fontSize: '0.95rem', fontWeight: 700, color: '#f87171', fontFamily: 'var(--font-mono)' }}>{selectedDefect.measuredDensity}</div>
                    <div style={{ fontSize: '0.68rem', color: 'var(--text-faint)' }}>Nominal: {selectedDefect.nominalDensity}</div>
                  </div>

                  <div style={{ background: 'var(--inset)', padding: '10px', borderRadius: '4px', border: '1px solid var(--border)' }}>
                    <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>Anomaly Volume</div>
                    <div style={{ fontSize: '0.95rem', fontWeight: 700, color: 'var(--accent)', fontFamily: 'var(--font-mono)' }}>{selectedDefect.volumeCm3} cm³</div>
                    <div style={{ fontSize: '0.68rem', color: 'var(--text-faint)' }}>Ø {selectedDefect.maxDiameterMm}mm max extent</div>
                  </div>

                  <div style={{ background: 'var(--inset)', padding: '10px', borderRadius: '4px', border: '1px solid var(--border)' }}>
                    <div style={{ fontSize: '0.7rem', color: 'var(--text-muted)' }}>Tomographic CNR</div>
                    <div style={{ fontSize: '0.95rem', fontWeight: 700, color: '#38bdf8', fontFamily: 'var(--font-mono)' }}>{selectedDefect.cnrValue.toFixed(2)}</div>
                    <div style={{ fontSize: '0.68rem', color: 'var(--text-faint)' }}>Deficit: {selectedDefect.densityDeficit}</div>
                  </div>
                </div>

                <div style={{ background: 'rgba(239, 68, 68, 0.08)', padding: '12px 14px', borderRadius: '6px', border: '1px solid rgba(239, 68, 68, 0.3)' }}>
                  <span style={{ fontSize: '0.74rem', color: '#f87171', fontWeight: 700, textTransform: 'uppercase', display: 'flex', alignItems: 'center', gap: 5 }}>
                    <AlertTriangle size={14} /> Structural Impact Assessment
                  </span>
                  <p style={{ margin: '6px 0 0 0', fontSize: '0.82rem', color: 'var(--text)', lineHeight: 1.5 }}>
                    {selectedDefect.structuralImpact}
                  </p>
                  <p style={{ margin: '8px 0 0 0', fontSize: '0.74rem', color: 'var(--text-faint)' }}>
                    Assessment text is supplied with the record. This view does not
                    perform a structural analysis.
                  </p>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
