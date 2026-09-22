import { useState } from "react";
import { useWidth } from "./ui";

const COLORS = { s1: "#3987e5", s2: "#d95926", s3: "#199e70", s4: "#c98500", grid: "#2e2e2c", text: "#c3c2b7", faint: "#7d7c74", surface: "#1a1a19" };

function niceMax(v: number): number {
  if (v <= 0) return 1;
  const p = Math.pow(10, Math.floor(Math.log10(v)));
  const m = v / p;
  const n = m <= 1 ? 1 : m <= 2 ? 2 : m <= 2.5 ? 2.5 : m <= 5 ? 5 : 10;
  return n * p;
}

function Legend({ items }: { items: { name: string; color: string }[] }) {
  return (
    <div className="mb-2 flex flex-wrap gap-4 text-xs text-dim">
      {items.map((i) => (
        <span key={i.name} className="inline-flex items-center gap-1.5">
          <span className="inline-block h-2.5 w-2.5 rounded-sm" style={{ background: i.color }} />
          {i.name}
        </span>
      ))}
    </div>
  );
}

function Tooltip({ x, y, children, width }: { x: number; y: number; children: React.ReactNode; width: number }) {
  const left = x > width * 0.65 ? undefined : x + 12;
  const right = x > width * 0.65 ? width - x + 12 : undefined;
  return (
    <div className="pointer-events-none absolute z-10 rounded-md border border-line bg-panel2 px-2.5 py-1.5 text-xs shadow-lg" style={{ left, right, top: Math.max(0, y - 10) }}>
      {children}
    </div>
  );
}

export interface BarSeries {
  name: string;
  color?: string;
  values: number[];
}

/** Grouped vertical bars, one group per x label. Hover shows all series. */
export function GroupedBars({ labels, series, height = 200, format }: { labels: string[]; series: BarSeries[]; height?: number; format: (n: number) => string }) {
  const [ref, width] = useWidth<HTMLDivElement>();
  const [hover, setHover] = useState<number | null>(null);
  const pad = { l: 52, r: 8, t: 8, b: 22 };
  const w = width;
  const h = height;
  const iw = w - pad.l - pad.r;
  const ih = h - pad.t - pad.b;
  const dataMax = Math.max(0, ...series.flatMap((s) => s.values));
  // Floor the axis at 100 so an empty chart still shows distinct tick labels.
  const max = niceMax(Math.max(100, dataMax));
  const n = labels.length;
  const gw = iw / Math.max(1, n);
  const labelEvery = gw >= 34 ? 1 : gw >= 18 ? 2 : 3;
  const bw = Math.max(3, Math.min(22, (gw * 0.7) / Math.max(1, series.length)));
  const colors = [COLORS.s1, COLORS.s2, COLORS.s3, COLORS.s4];
  const ticks = [0, 0.25, 0.5, 0.75, 1].map((t) => t * max);
  return (
    <div ref={ref} className="relative">
      <Legend items={series.map((s, i) => ({ name: s.name, color: s.color ?? colors[i % colors.length] }))} />
      <svg width={w} height={h} role="img" aria-label="Bar chart" onMouseLeave={() => setHover(null)}>
        {ticks.map((t) => {
          const y = pad.t + ih - (t / max) * ih;
          return (
            <g key={t}>
              <line x1={pad.l} x2={w - pad.r} y1={y} y2={y} stroke={COLORS.grid} strokeWidth={1} />
              <text x={pad.l - 6} y={y + 3} textAnchor="end" fontSize={10} fill={COLORS.faint}>
                {format(t)}
              </text>
            </g>
          );
        })}
        {labels.map((lab, i) => {
          const gx = pad.l + i * gw;
          const total = series.length * bw + (series.length - 1) * 2;
          const start = gx + (gw - total) / 2;
          return (
            <g key={lab} onMouseEnter={() => setHover(i)}>
              <rect x={gx} y={pad.t} width={gw} height={ih} fill={hover === i ? "#ffffff10" : "transparent"} />
              {series.map((s, si) => {
                const v = s.values[i] ?? 0;
                const bh = (v / max) * ih;
                const x = start + si * (bw + 2);
                const y = pad.t + ih - bh;
                return <rect key={s.name} x={x} y={y} width={bw} height={bh} rx={bh > 4 ? 2 : 0} fill={s.color ?? colors[si % colors.length]} />;
              })}
              {(i % labelEvery === 0 || i === n - 1) && (
                <text x={gx + gw / 2} y={h - 6} textAnchor="middle" fontSize={10} fill={COLORS.text}>
                  {lab}
                </text>
              )}
            </g>
          );
        })}
        {dataMax <= 0 && (
          <text x={pad.l + iw / 2} y={pad.t + ih / 2} textAnchor="middle" fontSize={11} fill={COLORS.faint}>
            No transactions yet
          </text>
        )}
      </svg>
      {hover != null && (
        <Tooltip x={pad.l + hover * gw + gw / 2} y={pad.t + 20} width={w}>
          <div className="mb-0.5 font-semibold text-fg">{labels[hover]}</div>
          {series.map((s, si) => (
            <div key={s.name} className="flex items-center gap-2">
              <span className="inline-block h-2 w-2 rounded-sm" style={{ background: s.color ?? colors[si % colors.length] }} />
              <span className="text-dim">{s.name}</span>
              <span className="num ml-auto pl-3 text-fg">{format(s.values[hover] ?? 0)}</span>
            </div>
          ))}
        </Tooltip>
      )}
    </div>
  );
}

export interface LineSeries {
  name: string;
  color?: string;
  values: number[];
  dashed?: boolean;
}

/** Multi-series line chart sharing one y-axis. Hover shows a crosshair. */
export function LineChart({ labels, series, height = 220, format, xEvery }: { labels: string[]; series: LineSeries[]; height?: number; format: (n: number) => string; xEvery?: number }) {
  const [ref, width] = useWidth<HTMLDivElement>();
  const [hover, setHover] = useState<number | null>(null);
  const pad = { l: 56, r: 12, t: 10, b: 22 };
  const w = width;
  const h = height;
  const iw = w - pad.l - pad.r;
  const ih = h - pad.t - pad.b;
  const n = labels.length;
  const max = niceMax(Math.max(100, ...series.flatMap((s) => s.values)));
  const colors = [COLORS.s1, COLORS.s2, COLORS.s3, COLORS.s4];
  const xAt = (i: number) => pad.l + (n <= 1 ? iw / 2 : (i / (n - 1)) * iw);
  const yAt = (v: number) => pad.t + ih - (Math.max(0, v) / max) * ih;
  const ticks = [0, 0.25, 0.5, 0.75, 1].map((t) => t * max);
  const every = xEvery ?? Math.max(1, Math.ceil(n / Math.max(1, Math.floor(iw / 60))));
  return (
    <div ref={ref} className="relative">
      {series.length > 1 && <Legend items={series.map((s, i) => ({ name: s.name, color: s.color ?? colors[i % colors.length] }))} />}
      <svg
        width={w}
        height={h}
        role="img"
        aria-label="Line chart"
        onMouseLeave={() => setHover(null)}
        onMouseMove={(e) => {
          const rect = e.currentTarget.getBoundingClientRect();
          const x = e.clientX - rect.left - pad.l;
          const i = Math.round((x / Math.max(1, iw)) * (n - 1));
          setHover(Math.min(n - 1, Math.max(0, i)));
        }}
      >
        {ticks.map((t) => (
          <g key={t}>
            <line x1={pad.l} x2={w - pad.r} y1={yAt(t)} y2={yAt(t)} stroke={COLORS.grid} strokeWidth={1} />
            <text x={pad.l - 6} y={yAt(t) + 3} textAnchor="end" fontSize={10} fill={COLORS.faint}>
              {format(t)}
            </text>
          </g>
        ))}
        {labels.map((lab, i) =>
          i % every === 0 || i === n - 1 ? (
            <text key={i} x={xAt(i)} y={h - 6} textAnchor="middle" fontSize={10} fill={COLORS.text}>
              {lab}
            </text>
          ) : null,
        )}
        {series.map((s, si) => {
          const d = s.values.map((v, i) => `${i === 0 ? "M" : "L"}${xAt(i).toFixed(1)},${yAt(v).toFixed(1)}`).join(" ");
          return <path key={s.name} d={d} fill="none" stroke={s.color ?? colors[si % colors.length]} strokeWidth={2} strokeDasharray={s.dashed ? "5 4" : undefined} strokeLinejoin="round" />;
        })}
        {hover != null && (
          <g>
            <line x1={xAt(hover)} x2={xAt(hover)} y1={pad.t} y2={pad.t + ih} stroke={COLORS.faint} strokeDasharray="3 3" />
            {series.map((s, si) => (
              <circle key={s.name} cx={xAt(hover)} cy={yAt(s.values[hover] ?? 0)} r={4} fill={s.color ?? colors[si % colors.length]} stroke={COLORS.surface} strokeWidth={2} />
            ))}
          </g>
        )}
      </svg>
      {hover != null && (
        <Tooltip x={xAt(hover)} y={pad.t + 20} width={w}>
          <div className="mb-0.5 font-semibold text-fg">{labels[hover]}</div>
          {series.map((s, si) => (
            <div key={s.name} className="flex items-center gap-2">
              <span className="inline-block h-2 w-2 rounded-sm" style={{ background: s.color ?? colors[si % colors.length] }} />
              <span className="text-dim">{s.name}</span>
              <span className="num ml-auto pl-3 text-fg">{format(s.values[hover] ?? 0)}</span>
            </div>
          ))}
        </Tooltip>
      )}
    </div>
  );
}
