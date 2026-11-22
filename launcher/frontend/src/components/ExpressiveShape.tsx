import { useMemo } from 'react';

export type ShapeName = 'sunny' | 'cookie4' | 'cookie9' | 'softBurst' | 'gem' | 'clamShell' | 'flower';

const SHAPES: Record<ShapeName, { lobes: number; amplitude: number }> = {
  sunny: { lobes: 8, amplitude: 0.075 },
  cookie4: { lobes: 4, amplitude: 0.12 },
  cookie9: { lobes: 9, amplitude: 0.07 },
  softBurst: { lobes: 10, amplitude: 0.11 },
  gem: { lobes: 6, amplitude: 0.05 },
  clamShell: { lobes: 2, amplitude: 0.08 },
  flower: { lobes: 5, amplitude: 0.16 },
};

export function polarShapePath(size: number, lobes: number, amplitude: number, rotation = 0, steps = 180): string {
  const center = size / 2;
  const radius = size / 2 / (1 + amplitude);
  const points: string[] = [];
  for (let index = 0; index < steps; index += 1) {
    const angle = (index / steps) * Math.PI * 2;
    const r = radius * (1 + amplitude * Math.cos(lobes * (angle + rotation)));
    points.push(`${(center + r * Math.cos(angle)).toFixed(2)},${(center + r * Math.sin(angle)).toFixed(2)}`);
  }
  return `M${points.join('L')}Z`;
}

export function ExpressiveShape({ name, className = '' }: { name: ShapeName; className?: string }) {
  const d = useMemo(() => {
    const shape = SHAPES[name] ?? SHAPES.sunny;
    return polarShapePath(100, shape.lobes, shape.amplitude, -Math.PI / 2 / shape.lobes);
  }, [name]);

  return (
    <svg viewBox="0 0 100 100" className={`shape${className ? ` ${className}` : ''}`} aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

export function HeroOrnament({ name = 'flower' }: { name?: ShapeName }) {
  return (
    <div className="hero-ornament" aria-hidden="true">
      <ExpressiveShape name={name} />
    </div>
  );
}

export function Icon({
  name,
  filled = false,
  className = '',
  style,
}: {
  name: string;
  filled?: boolean;
  className?: string;
  style?: React.CSSProperties;
}) {
  return (
    <span
      className={`icon${filled ? ' filled' : ''}${className ? ` ${className}` : ''}`}
      style={style}
      aria-hidden="true"
    >
      {name}
    </span>
  );
}

function polar(cx: number, cy: number, radius: number, degrees: number): [number, number] {
  const radians = (degrees * Math.PI) / 180;
  return [cx + radius * Math.cos(radians), cy + radius * Math.sin(radians)];
}

export function arcPath(cx: number, cy: number, radius: number, start: number, sweep: number): string {
  if (sweep >= 360) {
    const [x0, y0] = polar(cx, cy, radius, start);
    const [x1, y1] = polar(cx, cy, radius, start + 180);
    return `M${x0} ${y0}A${radius} ${radius} 0 1 1 ${x1} ${y1}A${radius} ${radius} 0 1 1 ${x0} ${y0}`;
  }
  const [x0, y0] = polar(cx, cy, radius, start);
  const [x1, y1] = polar(cx, cy, radius, start + sweep);
  return `M${x0} ${y0}A${radius} ${radius} 0 ${sweep > 180 ? 1 : 0} 1 ${x1} ${y1}`;
}

export function ArcRing({
  size,
  stroke,
  start = -90,
  sweep = 360,
  color,
  fraction,
  className = '',
  title,
  children,
}: {
  size: number;
  stroke: number;
  start?: number;
  sweep?: number;
  color: string;
  fraction: number;
  className?: string;
  title?: string;
  children?: React.ReactNode;
}) {
  const clamped = Number.isFinite(fraction) ? Math.min(1, Math.max(0, fraction)) : 0;
  const radius = (size - stroke) / 2;
  const length = (radius * Math.PI * sweep) / 180;
  const path = useMemo(() => arcPath(size / 2, size / 2, radius, start, sweep), [size, radius, start, sweep]);

  return (
    <div className={`ring ${className}`.trim()} title={title}>
      <svg className="ring-svg" viewBox={`0 0 ${size} ${size}`}>
        <path className="ring-track" d={path} strokeWidth={stroke} />
        <path
          className="ring-value"
          d={path}
          strokeWidth={stroke}
          stroke={color}
          strokeDasharray={`${(length * clamped).toFixed(2)} ${(length + 1).toFixed(2)}`}
          style={{ transition: 'stroke-dasharray 550ms cubic-bezier(0.2, 0, 0, 1)' }}
        />
      </svg>
      {children}
    </div>
  );
}
