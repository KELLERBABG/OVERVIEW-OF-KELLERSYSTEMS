import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  ReferenceLine,
} from 'recharts';
import { format, parseISO } from 'date-fns';

interface DataPoint {
  date: string;
  total_mg: number;
  total_water_l: number;
  event_count: number;
}

interface CaptureAreaChartProps {
  data: DataPoint[];
  height?: number;
  showWater?: boolean;
  showEvents?: boolean;
  average?: number;
}

function CustomTooltip({ active, payload, label }: any) {
  if (!active || !payload?.length) return null;

  const dateStr = label as string;
  let formattedDate: string;
  try {
    formattedDate = format(parseISO(dateStr), 'MMM d, yyyy');
  } catch {
    formattedDate = dateStr;
  }

  return (
    <div className="bg-white rounded-lg shadow-lg border border-gray-200 px-4 py-3 min-w-[180px]">
      <p className="text-sm font-medium text-gray-900 mb-2">{formattedDate}</p>
      {payload.map((entry: any, i: number) => (
        <div key={i} className="flex items-center justify-between text-sm gap-4">
          <div className="flex items-center gap-2">
            <span
              className="w-2.5 h-2.5 rounded-full"
              style={{ backgroundColor: entry.color }}
            />
            <span className="text-gray-600">{entry.name}</span>
          </div>
          <span className="font-medium text-gray-900">
            {entry.name === 'Microplastic'
              ? `${Number(entry.value).toFixed(2)} mg`
              : entry.name === 'Water'
              ? `${Number(entry.value).toFixed(0)} L`
              : entry.value}
          </span>
        </div>
      ))}
    </div>
  );
}

export function CaptureAreaChart({
  data,
  height = 350,
  showWater = true,
  showEvents = false,
  average,
}: CaptureAreaChartProps) {
  if (data.length === 0) {
    return (
      <div className="flex items-center justify-center text-gray-400" style={{ height }}>
        No data available
      </div>
    );
  }

  // Format dates for display
  const chartData = data.map((d) => ({
    ...d,
    displayDate: (() => {
      try {
        return format(parseISO(d.date), 'MMM d');
      } catch {
        return d.date;
      }
    })(),
  }));

  // Calculate average line
  const avgLine = average ?? data.reduce((sum, d) => sum + d.total_mg, 0) / data.length;

  return (
    <ResponsiveContainer width="100%" height={height}>
      <AreaChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
        <defs>
          <linearGradient id="gradientMg" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor="#0ea5e9" stopOpacity={0.3} />
            <stop offset="95%" stopColor="#0ea5e9" stopOpacity={0.05} />
          </linearGradient>
          {showWater && (
            <linearGradient id="gradientWater" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#8b5cf6" stopOpacity={0.2} />
              <stop offset="95%" stopColor="#8b5cf6" stopOpacity={0.02} />
            </linearGradient>
          )}
        </defs>

        <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />

        <XAxis
          dataKey="displayDate"
          tick={{ fontSize: 12, fill: '#9ca3af' }}
          tickLine={false}
          axisLine={{ stroke: '#e5e7eb' }}
          interval="preserveStartEnd"
          minTickGap={40}
        />

        <YAxis
          yAxisId="mg"
          tick={{ fontSize: 12, fill: '#9ca3af' }}
          tickLine={false}
          axisLine={false}
          label={{
            value: 'mg',
            position: 'insideTopLeft',
            offset: 10,
            style: { fontSize: 12, fill: '#9ca3af' },
          }}
        />

        {showWater && (
          <YAxis
            yAxisId="water"
            orientation="right"
            tick={{ fontSize: 12, fill: '#9ca3af' }}
            tickLine={false}
            axisLine={false}
            label={{
              value: 'liters',
              position: 'insideTopRight',
              offset: 10,
              style: { fontSize: 12, fill: '#9ca3af' },
            }}
          />
        )}

        {showEvents && (
          <YAxis
            yAxisId="events"
            orientation="right"
            tick={{ fontSize: 12, fill: '#9ca3af' }}
            tickLine={false}
            axisLine={false}
          />
        )}

        <Tooltip content={<CustomTooltip />} />

        <Legend
          verticalAlign="top"
          height={36}
          iconType="circle"
          iconSize={8}
          wrapperStyle={{ fontSize: 13, paddingBottom: 8 }}
        />

        {average && (
          <ReferenceLine
            yAxisId="mg"
            y={avgLine}
            stroke="#f59e0b"
            strokeDasharray="6 4"
            strokeWidth={1.5}
            label={{
              value: `Avg: ${avgLine.toFixed(2)} mg`,
              position: 'insideTopRight',
              style: { fontSize: 11, fill: '#f59e0b' },
            }}
          />
        )}

        <Area
          yAxisId="mg"
          type="monotone"
          dataKey="total_mg"
          name="Microplastic"
          stroke="#0ea5e9"
          strokeWidth={2}
          fill="url(#gradientMg)"
          dot={false}
          activeDot={{ r: 5, strokeWidth: 2, stroke: '#fff' }}
        />

        {showWater && (
          <Area
            yAxisId="water"
            type="monotone"
            dataKey="total_water_l"
            name="Water"
            stroke="#8b5cf6"
            strokeWidth={1.5}
            fill="url(#gradientWater)"
            dot={false}
            activeDot={{ r: 4, strokeWidth: 2, stroke: '#fff' }}
          />
        )}

        {showEvents && (
          <Area
            yAxisId="events"
            type="monotone"
            dataKey="event_count"
            name="Wash Cycles"
            stroke="#10b981"
            strokeWidth={1.5}
            fill="transparent"
            dot={false}
            activeDot={{ r: 4, strokeWidth: 2, stroke: '#fff' }}
          />
        )}
      </AreaChart>
    </ResponsiveContainer>
  );
}
