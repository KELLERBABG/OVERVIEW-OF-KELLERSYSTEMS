interface TrendIndicatorProps {
  current: number;
  previous: number;
  invert?: boolean; // For metrics where lower is better (e.g., cost)
}

export function TrendIndicator({ current, previous, invert = false }: TrendIndicatorProps) {
  if (previous === 0) return null;

  const change = ((current - previous) / previous) * 100;
  const isPositive = invert ? change < 0 : change > 0;
  const isZero = Math.abs(change) < 0.1;

  if (isZero) {
    return (
      <span className="text-xs text-gray-500 flex items-center gap-1">
        <span>→</span>
        <span>0%</span>
      </span>
    );
  }

  return (
    <span
      className={`text-xs font-medium flex items-center gap-1 ${
        isPositive ? 'text-green-600' : 'text-red-500'
      }`}
    >
      <span>{isPositive ? '↑' : '↓'}</span>
      <span>{Math.abs(change).toFixed(1)}%</span>
    </span>
  );
}
