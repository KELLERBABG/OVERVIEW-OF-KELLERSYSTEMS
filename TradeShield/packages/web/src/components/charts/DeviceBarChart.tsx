import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';

interface DeviceData {
  device_id: string;
  device_name: string | null;
  total_mg: number;
  total_cycles: number;
}

interface DeviceBarChartProps {
  data: DeviceData[];
  height?: number;
  maxBars?: number;
}

const COLORS = ['#0ea5e9', '#06b6d4', '#14b8a6', '#10b981', '#22c55e'];

function CustomTooltip({ active, payload }: any) {
  if (!active || !payload?.length) return null;

  const data = payload[0]?.payload;
  if (!data) return null;

  return (
    <div className="bg-white rounded-lg shadow-lg border border-gray-200 px-4 py-3 min-w-[180px]">
      <p className="text-sm font-medium text-gray-900 mb-1">
        {data.device_name || data.device_id}
      </p>
      <div className="space-y-1 text-sm">
        <div className="flex justify-between gap-4">
          <span className="text-gray-600">Captured</span>
          <span className="font-medium text-water-dark">{data.total_mg.toFixed(1)} mg</span>
        </div>
        <div className="flex justify-between gap-4">
          <span className="text-gray-600">Cycles</span>
          <span className="font-medium text-gray-900">{data.total_cycles.toLocaleString()}</span>
        </div>
      </div>
    </div>
  );
}

export function DeviceBarChart({
  data,
  height = 250,
  maxBars = 8,
}: DeviceBarChartProps) {
  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center text-gray-400" style={{ height }}>
        No device data
      </div>
    );
  }

  // Sort by total_mg descending and limit
  const chartData = [...data]
    .sort((a, b) => b.total_mg - a.total_mg)
    .slice(0, maxBars)
    .map((d) => ({
      ...d,
      name: d.device_name || d.device_id,
    }));

  return (
    <ResponsiveContainer width="100%" height={height}>
      <BarChart
        data={chartData}
        layout="vertical"
        margin={{ top: 5, right: 30, left: 0, bottom: 5 }}
      >
        <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" horizontal={false} />

        <XAxis
          type="number"
          tick={{ fontSize: 12, fill: '#9ca3af' }}
          tickLine={false}
          axisLine={{ stroke: '#e5e7eb' }}
          tickFormatter={(v) => `${v} mg`}
        />

        <YAxis
          dataKey="name"
          type="category"
          width={140}
          tick={{ fontSize: 12, fill: '#374151' }}
          tickLine={false}
          axisLine={false}
        />

        <Tooltip content={<CustomTooltip />} cursor={{ fill: 'rgba(14, 165, 233, 0.05)' }} />

        <Bar
          dataKey="total_mg"
          name="Microplastic"
          radius={[0, 6, 6, 0]}
          barSize={24}
        >
          {chartData.map((_, index) => (
            <Cell key={index} fill={COLORS[index % COLORS.length]} />
          ))}
        </Bar>
      </BarChart>
    </ResponsiveContainer>
  );
}
