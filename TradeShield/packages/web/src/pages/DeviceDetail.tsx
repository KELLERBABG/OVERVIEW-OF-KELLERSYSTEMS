import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { devicesApi } from '../lib/api';
import { format } from 'date-fns';

interface DeviceDetail {
  id: string;
  name: string;
  location: string;
  status: string;
  installed_at: string;
  last_heartbeat: string;
  firmware_version: string;
  battery_level: number;
  recent_events: any[];
  stats: {
    total_cycles: number;
    total_water_l: number;
    total_mg: number;
  };
}

export function DeviceDetail() {
  const { id } = useParams<{ id: string }>();
  const [device, setDevice] = useState<DeviceDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    async function fetchDevice() {
      if (!id) return;
      
      try {
        const res = await devicesApi.get(id);
        setDevice(res.data);
      } catch (err: any) {
        setError(err.message);
      } finally {
        setLoading(false);
      }
    }

    fetchDevice();
  }, [id]);

  if (loading) {
    return <div className="text-center py-12 text-gray-500">Loading device...</div>;
  }

  if (error) {
    return <div className="text-center py-12 text-red-500">Error: {error}</div>;
  }

  if (!device) {
    return <div className="text-center py-12 text-gray-500">Device not found</div>;
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <Link to="/devices" className="text-sm text-gray-500 hover:text-gray-700">
            ← Back to devices
          </Link>
          <h1 className="text-2xl font-bold text-gray-900 mt-2">
            {device.name || device.id}
          </h1>
          <p className="text-gray-500">{device.location}</p>
        </div>
        <span className={`px-3 py-1 text-sm rounded-full ${
          device.status === 'active' 
            ? 'bg-green-100 text-green-700'
            : device.status === 'maintenance'
            ? 'bg-yellow-100 text-yellow-700'
            : 'bg-gray-100 text-gray-700'
        }`}>
          {device.status}
        </span>
      </div>

      {/* Device Info */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="bg-white rounded-xl border border-gray-200 p-6">
          <p className="text-sm text-gray-500">Total Cycles</p>
          <p className="text-3xl font-bold text-gray-900 mt-1">
            {device.stats.total_cycles.toLocaleString()}
          </p>
        </div>
        <div className="bg-white rounded-xl border border-gray-200 p-6">
          <p className="text-sm text-gray-500">Microplastic Captured</p>
          <p className="text-3xl font-bold text-water-dark mt-1">
            {device.stats.total_mg.toFixed(1)} mg
          </p>
        </div>
        <div className="bg-white rounded-xl border border-gray-200 p-6">
          <p className="text-sm text-gray-500">Water Filtered</p>
          <p className="text-3xl font-bold text-gray-900 mt-1">
            {device.stats.total_water_l.toFixed(0)} L
          </p>
        </div>
        <div className="bg-white rounded-xl border border-gray-200 p-6">
          <p className="text-sm text-gray-500">Battery Level</p>
          <p className="text-3xl font-bold text-gray-900 mt-1">
            {device.battery_level || 'N/A'}%
          </p>
        </div>
      </div>

      {/* Device Details */}
      <div className="bg-white rounded-xl border border-gray-200 p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Device Information</h2>
        <dl className="grid grid-cols-2 gap-4">
          <div>
            <dt className="text-sm text-gray-500">Device ID</dt>
            <dd className="font-mono text-sm mt-1">{device.id}</dd>
          </div>
          <div>
            <dt className="text-sm text-gray-500">Firmware Version</dt>
            <dd className="text-sm mt-1">{device.firmware_version || 'Unknown'}</dd>
          </div>
          <div>
            <dt className="text-sm text-gray-500">Installed Date</dt>
            <dd className="text-sm mt-1">
              {device.installed_at ? format(new Date(device.installed_at), 'MMM d, yyyy') : 'Unknown'}
            </dd>
          </div>
          <div>
            <dt className="text-sm text-gray-500">Last Heartbeat</dt>
            <dd className="text-sm mt-1">
              {device.last_heartbeat ? format(new Date(device.last_heartbeat), 'MMM d, yyyy HH:mm') : 'Never'}
            </dd>
          </div>
        </dl>
      </div>

      {/* Recent Events */}
      <div className="bg-white rounded-xl border border-gray-200 p-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Recent Events</h2>
        
        {device.recent_events.length === 0 ? (
          <div className="text-center py-8 text-gray-500">No events recorded yet</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-200">
                  <th className="text-left py-3 px-4 font-medium text-gray-500">Time</th>
                  <th className="text-left py-3 px-4 font-medium text-gray-500">Cycle #</th>
                  <th className="text-left py-3 px-4 font-medium text-gray-500">Water (L)</th>
                  <th className="text-left py-3 px-4 font-medium text-gray-500">Captured (mg)</th>
                </tr>
              </thead>
              <tbody>
                {device.recent_events.map((event) => (
                  <tr key={event.id} className="border-b border-gray-100 hover:bg-gray-50">
                    <td className="py-3 px-4">
                      {format(new Date(event.timestamp), 'MMM d, HH:mm')}
                    </td>
                    <td className="py-3 px-4 font-mono">{event.wash_cycle}</td>
                    <td className="py-3 px-4">{event.water_volume_l.toFixed(1)}</td>
                    <td className="py-3 px-4 text-water-dark">{event.estimated_mg.toFixed(3)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
