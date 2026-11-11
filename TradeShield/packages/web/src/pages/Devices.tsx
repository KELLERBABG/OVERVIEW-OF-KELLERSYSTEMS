import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { dashboardApi } from '../lib/api';

interface Device {
  id: string;
  name: string;
  location: string;
  status: string;
  last_heartbeat: string;
  total_cycles: number;
  total_mg: number;
  total_water_l: number;
}

export function Devices() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    async function fetchDevices() {
      try {
        const res = await dashboardApi.getDevices();
        setDevices(res.data || []);
      } catch (err: any) {
        setError(err.message);
      } finally {
        setLoading(false);
      }
    }

    fetchDevices();
  }, []);

  if (loading) {
    return <div className="text-center py-12 text-gray-500">Loading devices...</div>;
  }

  if (error) {
    return <div className="text-center py-12 text-red-500">Error: {error}</div>;
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Devices</h1>
        <p className="text-gray-500 mt-1">Manage your ESP32 filter devices</p>
      </div>

      {devices.length === 0 ? (
        <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
          <div className="text-4xl mb-4">🔌</div>
          <h3 className="text-lg font-medium text-gray-900">No devices yet</h3>
          <p className="text-gray-500 mt-2">Install your first WashTrap filter to get started</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {devices.map((device) => (
            <Link
              key={device.id}
              to={`/devices/${device.id}`}
              className="bg-white rounded-xl border border-gray-200 p-6 hover:shadow-md transition-shadow"
            >
              <div className="flex items-start justify-between">
                <div>
                  <h3 className="font-semibold text-gray-900">{device.name || device.id}</h3>
                  <p className="text-sm text-gray-500 mt-1">{device.location}</p>
                </div>
                <span className={`px-2 py-1 text-xs rounded-full ${
                  device.status === 'active' 
                    ? 'bg-green-100 text-green-700'
                    : device.status === 'maintenance'
                    ? 'bg-yellow-100 text-yellow-700'
                    : 'bg-gray-100 text-gray-700'
                }`}>
                  {device.status}
                </span>
              </div>

              <div className="mt-6 grid grid-cols-3 gap-4 text-center">
                <div>
                  <p className="text-2xl font-bold text-gray-900">{device.total_cycles.toLocaleString()}</p>
                  <p className="text-xs text-gray-500">Cycles</p>
                </div>
                <div>
                  <p className="text-2xl font-bold text-water-dark">{device.total_mg.toFixed(1)}</p>
                  <p className="text-xs text-gray-500">mg captured</p>
                </div>
                <div>
                  <p className="text-2xl font-bold text-gray-900">{device.total_water_l.toFixed(0)}</p>
                  <p className="text-xs text-gray-500">Liters</p>
                </div>
              </div>

              <div className="mt-4 pt-4 border-t border-gray-100">
                <p className="text-xs text-gray-500">
                  Last seen: {device.last_heartbeat ? new Date(device.last_heartbeat).toLocaleString() : 'Never'}
                </p>
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
