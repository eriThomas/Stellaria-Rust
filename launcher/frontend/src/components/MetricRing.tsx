interface MetricRingProps {
  fraction: number;
  size?: number;
  stroke?: number;
  color: string;
  label: string;
  valueText?: string;
}

export function MetricRing({
  fraction,
  size = 64,
  stroke = 6,
  color,
  label,
  valueText,
}: MetricRingProps) {
  const clamped = Number.isFinite(fraction) ? Math.min(1, Math.max(0, fraction)) : 0;
  const radius = (size - stroke) / 2;
  const circumference = 2 * Math.PI * radius;
  const dash = `${(circumference * clamped).toFixed(2)} ${(circumference + 1).toFixed(2)}`;

  return (
    <div className="ring-wrap" title={label}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-label={label}>
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="var(--m3-surface-container-high)"
          strokeWidth={stroke}
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke={color}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={dash}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
        <text
          x="50%"
          y="53%"
          textAnchor="middle"
          dominantBaseline="middle"
          fill="var(--m3-on-surface)"
          fontSize={size > 90 ? 18 : 12}
          fontWeight={600}
        >
          {valueText ?? `${Math.round(clamped * 100)}%`}
        </text>
      </svg>
      <span>{label}</span>
    </div>
  );
}
