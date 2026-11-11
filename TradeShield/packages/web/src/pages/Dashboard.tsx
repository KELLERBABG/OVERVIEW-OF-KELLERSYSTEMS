import { useEffect, useState } from 'react';
import { dashboardApi } from '../lib/api';
import { CaptureAreaChart } from '../components/charts/CaptureAreaChart';
import { DeviceBarChart } from '../components/charts/DeviceBarChart';
import { TrendIndicator } from '../components/charts/TrendIndicator';
import { SparkLine } from '../components/charts/SparkLine';

interface DashboardSummary {
  total_devices: number;
  active_devices: number;
  inactive_devices: number;
  maintenance_devices: number;
  total_events_30d: number;
  total_mg_30d: number;
  total_water_l_30d: number;
  avg_mg_per_cycle: number;
  avg_water_per_cycle: number;
  total_credits_mg: number;
  total_credits_eur: number;
  total_batches: number;
  pending_batches: number;
  issued_batches: number;
  comparison: {
    total_events_prev: number;
    total_mg_prev: number;
    events_change_pct: number;
    mg_change_pct: number;
  };
}

interface TimelinePoint {
  date: string;
  total_mg: number;
  total_water_l: number;
  event_count: number;
}

interface DeviceRanking {
  device_id: string;
  device_name: string | null;
  total_mg: number;
  total_cycles: number;
  rank: number;
}

interface AlertSummary {
  total: number;
  critical: number;
  warning: number;
  info: number;
}

export function Dashboard() {
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [timeline, setTimeline] = useState<TimelinePoint[]>([]);
  const [ranking, setRanking] = useState<DeviceRanking[]>([]);
  const [alerts, setAlerts] = useState<AlertSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    async function fetchData() {
      try {
        const [summaryRes, timelineRes, rankingRes, alertsRes] = await Promise.all([
          dashboardApi.getSummary(),
          dashboardApi.getTimeline({ period: 'day' }),
          fetch('/api/v1/dashboard/ranking?days=30', { credentials: 'include' }).then((r) => r.json()),
          fetch('/api/v1/dashboard/alerts', { credentials: 'include' }).then((r) => r.json()),
        ]);
        setSummary(summaryRes);
        setTimeline(timelineRes.data || []);
        setRanking(rankingRes.data || []);
        setAlerts(alertsRes.summary || null);
      } catch (err: any) {
        setError(err.message);
      } finally {
        setLoading(false);
      }
    }

    fetchData();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="flex items-center gap-3 text-gray-500">
          <svg className="animate-spin h-5 w-5" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
          Loading dashboard...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="text-center py-24">
        <div className="text-4xl mb-4">⚠️</div>
        <h3 className="text-lg font-medium text-gray-900">Failed to load dashboard</h3>
        <p className="text-red-500 mt-2">{error}</p>
      </div>
    );
  }

  if (!summary) {
    return (
      <div className="text-center py-24 text-gray-500">
        No data available
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Dashboard</h1>
          <p className="text-gray-500 mt-1">Overview of your microplastic capture</p>
        </div>
        {alerts && alerts.total > 0 && (
          <div className="flex items-center gap-3 text-sm">
            {alerts.critical > 0 && (
              <span className="flex items-center gap-1.5 px-3 py-1.5 bg-red-50 text-red-700 rounded-full border border-red-200">
                <span className="w-2 h-2 bg-red-500 rounded-full animate-pulse" />
                {alerts.critical} critical
              </span>
            )}
            {alerts.warning > 0 && (
              <span className="flex items-center gap-1.5 px-3 py-1.5 bg-yellow-50 text-yellow-700 rounded-full border border-yellow-200">
                <span className="w-2 h-2 bg-yellow-500 rounded-full" />
                {alerts.warning} warning
              </span>
            )}
          </div>
        )}
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {/* Active Devices */}
        <div className="bg-white rounded-xl border border-gray-200 p-6 hover:shadow-md transition-shadow">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <p className="text-sm font-medium text-gray-500">Active Devices</p>
              <div className="flex items-baseline gap-2 mt-1">
                <p className="text-3xl font-bold text-gray-900">{summary.active_devices}</p>
                <span className="text-sm text-gray-500">/ {summary.total_devices}</span>
              </div>
            </div>
            <div className="text-3xl">🔌</div>
          </div>
          <div className="mt-4 flex items-center justify-between">
            <div className="flex gap-2 text-xs">
              {summary.inactive_devices > 0 && (
                <span className="px-2 py-0.5 bg-gray-100 text-gray-600 rounded-full">
                  {summary.inactive_devices} inactive
                </span>
              )}
              {summary.maintenance_devices > 0 && (
                <span className="px-2 py-0.5 bg-yellow-100 text-yellow-700 rounded-full">
                  {summary.maintenance_devices} maintenance
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Microplastic Captured */}
        <div className="bg-white rounded-xl border border-gray-200 p-6 hover:shadow-md transition-shadow">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <p className="text-sm font-medium text-gray-500">Captured (30d)</p>
              <div className="flex items-baseline gap-2 mt-1">
                <p className="text-3xl font-bold text-water-dark">
                  {summary.total_mg_30d.toFixed(1)}
                </p>
                <span className="text-sm font-medium text-water">mg</span>
              </div>
              <div className="flex items-center gap-2 mt-1">
                <TrendIndicator
                  current={summary.total_mg_30d}
                  previous={summary.comparison.total_mg_prev}
                />
                <span className="text-xs text-gray-400">vs prev 30d</span>
              </div>
            </div>
            <SparkLine
              data={timeline.slice(-14).map((t) => t.total_mg)}
              color="#0ea5e9"
            />
          </div>
          <div className="mt-3 pt-3 border-t border-gray-100 text-xs text-gray-500">
            {summary.total_water_l_30d.toFixed(0)}L water · {summary.avg_mg_per_cycle} mg/cycle avg
          </div>
        </div>

        {/* Wash Cycles */}
        <div className="bg-white rounded-xl border border-gray-200 p-6 hover:shadow-md transition-shadow">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <p className="text-sm font-medium text-gray-500">Wash Cycles (30d)</p>
              <div className="flex items-baseline gap-2 mt-1">
                <p className="text-3xl font-bold text-gray-900">
                  {summary.total_events_30d.toLocaleString()}
                </p>
              </div>
              <div className="flex items-center gap-2 mt-1">
                <TrendIndicator
                  current={summary.total_events_30d}
                  previous={summary.comparison.total_events_prev}
                />
                <span className="text-xs text-gray-400">vs prev 30d</span>
              </div>
            </div>
            <SparkLine
              data={timeline.slice(-14).map((t) => t.event_count)}
              color="#6366f1"
            />
          </div>
          <div className="mt-3 pt-3 border-t border-gray-100 text-xs text-gray-500">
            {summary.avg_water_per_cycle}L avg per cycle
          </div>
        </div>

        {/* Credits Value */}
        <div className="bg-white rounded-xl border border-gray-200 p-6 hover:shadow-md transition-shadow">
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <p className="text-sm font-medium text-gray-500">Credits Value</p>
              <div className="flex items-baseline gap-2 mt-1">
                <p className="text-3xl font-bold text-green-600">
                  €{summary.total_credits_eur.toFixed(2)}
                </p>
              </div>
              <div className="mt-1">
                <span className="text-xs text-gray-500">
                  {summary.total_credits_mg.toFixed(1)} mg offset
                </span>
              </div>
            </div>
            <div className="text-3xl">💰</div>
          </div>
          <div className="mt-3 pt-3 border-t border-gray-100 flex gap-2 text-xs">
            <span className="px-2 py-0.5 bg-yellow-100 text-yellow-700 rounded-full">
              {summary.pending_batches} pending
            </span>
            <span className="px-2 py-0.5 bg-green-100 text-green-700 rounded-full">
              {summary.issued_batches} issued
            </span>
          </div>
        </div>
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Main Timeline Chart */}
        <div className="lg:col-span-2 bg-white rounded-xl border border-gray-200 p-6">
          <div className="flex items-center justify-between mb-6">
            <div>
              <h2 className="text-lg font-semibold text-gray-900">Capture Timeline</h2>
              <p className="text-sm text-gray-500 mt-0.5">Daily microplastic capture over 30 days</p>
            </div>
            <div className="flex items-center gap-4 text-xs">
              <div className="flex items-center gap-1.5">
                <span className="w-2.5 h-2.5 rounded-full bg-water" />
                <span className="text-gray-600">Microplastic (mg)</span>
              </div>
              <div className="flex items-center gap-1.5">
                <span className="w-2.5 h-2.5 rounded-full bg-purple-500" />
                <span className="text-gray-600">Water (L)</span>
              </div>
            </div>
          </div>
          <CaptureAreaChart
            data={timeline}
            height={300}
            showWater={true}
            showEvents={false}
            average={summary.total_mg_30d / Math.max(timeline.length, 1)}
          />
        </div>

        {/* Device Ranking */}
        <div className="bg-white rounded-xl border border-gray-200 p-6">
          <div className="mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Device Ranking</h2>
            <p className="text-sm text-gray-500 mt-0.5">By capture volume (30d)</p>
          </div>
          <DeviceBarChart data={ranking} height={280} maxBars={5} />
        </div>
      </div>

      {/* Bottom Row */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Environmental Impact */}
        <div className="bg-white rounded-xl border border-gray-200 p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Environmental Impact</h3>
          <div className="space-y-4">
            <div className="flex items-center justify-between p-3 bg-blue-50 rounded-lg">
              <div className="flex items-center gap-3">
                <span className="text-2xl">🧴</span>
                <div>
                  <p className="text-sm text-gray-600">Microplastic prevented</p>
                  <p className="text-xl font-bold text-water-dark">
                    {summary.total_credits_mg.toFixed(1)} mg
                  </p>
                </div>
              </div>
            </div>
            <div className="flex items-center justify-between p-3 bg-green-50 rounded-lg">
              <div className="flex items-center gap-3">
                <span className="text-2xl">♻️</span>
                <div>
                  <p className="text-sm text-gray-600">Equivalent plastic bottles</p>
                  <p className="text-xl font-bold text-green-700">
                    ~{Math.floor(summary.total_credits_mg / 500)}
                  </p>
                </div>
              </div>
            </div>
            <div className="flex items-center justify-between p-3 bg-purple-50 rounded-lg">
              <div className="flex items-center gap-3">
                <span className="text-2xl">💧</span>
                <div>
                  <p className="text-sm text-gray-600">Water filtered</p>
                  <p className="text-xl font-bold text-purple-700">
                    {summary.total_water_l_30d.toLocaleString()} L
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Compliance Status */}
        <div className="bg-white rounded-xl border border-gray-200 p-6">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Compliance Status</h3>
          <div className="space-y-4">
            <div className="flex items-center justify-between p-3 bg-green-50 rounded-lg">
              <div className="flex items-center gap-3">
                <span className="w-3 h-3 bg-green-500 rounded-full" />
                <span className="text-gray-700">EU ESPR filter mandate</span>
              </div>
              <span className="text-sm font-medium text-green-700">Compliant</span>
            </div>
            <div className="flex items-center justify-between p-3 bg-green-50 rounded-lg">
              <div className="flex items-center gap-3">
                <span className="w-3 h-3 bg-green-500 rounded-full" />
                <span className="text-gray-700">Device monitoring active</span>
              </div>
              <span className="text-sm font-medium text-green-700">
                {summary.active_devices}/{summary.total_devices}
              </span>
            </div>
            <div className="flex items-center justify-between p-3 bg-green-50 rounded-lg">
              <div className="flex items-center gap-3">
                <span className="w-3 h-3 bg-green-500 rounded-full" />
                <span className="text-gray-700">Data integrity verified</span>
              </div>
              <span className="text-sm font-medium text-green-700">✓</span>
            </div>
            {alerts && alerts.total > 0 && (
              <div className="flex items-center justify-between p-3 bg-yellow-50 rounded-lg">
                <div className="flex items-center gap-3">
                  <span className="w-3 h-3 bg-yellow-500 rounded-full" />
                  <span className="text-gray-700">Active alerts</span>
                </div>
                <span className="text-sm font-medium text-yellow-700">{alerts.total}</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
