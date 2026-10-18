// P&ID-style line symbols. Two-port symbols are drawn left (in) to right (out).
import React from 'react';

const S = { fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, strokeLinejoin: 'round' as const, strokeLinecap: 'round' as const };

export const SYMBOL_SIZE: Record<string, [number, number]> = {
  tank: [44, 64],
  boundary: [36, 48],
  junction: [14, 14],
  pipe: [72, 24],
  orifice: [56, 32],
  venturi: [64, 32],
  valve: [56, 36],
  check_valve: [56, 32],
  regulator: [56, 44],
  loss: [48, 36],
  mass_flow: [48, 36],
  pump: [48, 40],
  turbine: [52, 40],
  shaft: [76, 22],
  combustion_chamber: [104, 60],
};

export function Symbol({ type, size = 1 }: { type: string; size?: number }) {
  const [w, h] = SYMBOL_SIZE[type] ?? [48, 32];
  const cy = h / 2;
  let body: React.ReactNode = null;
  switch (type) {
    case 'tank':
      body = (
        <>
          <rect x={4} y={3} width={w - 8} height={h - 6} rx={(w - 8) / 2.4} {...S} fill="var(--surface)" />
          <line x1={10} y1={h * 0.58} x2={w - 10} y2={h * 0.58} {...S} strokeWidth={1} strokeDasharray="2 2" />
        </>
      );
      break;
    case 'boundary':
      body = (
        <>
          <rect x={4} y={4} width={w - 8} height={h - 8} {...S} fill="var(--surface)" />
          {[0, 1, 2, 3].map((i) => (
            <line key={i} x1={4} y1={10 + i * 9} x2={4 + 9} y2={1 + i * 9} {...S} strokeWidth={1} />
          ))}
          <text x={w / 2 + 3} y={cy + 4} textAnchor="middle" fontSize={11} fontFamily="var(--mono)" fill="currentColor">
            P
          </text>
        </>
      );
      break;
    case 'junction':
      body = <circle cx={w / 2} cy={cy} r={4.5} fill="currentColor" />;
      break;
    case 'pipe':
      body = (
        <>
          <line x1={2} y1={cy - 4} x2={w - 2} y2={cy - 4} {...S} />
          <line x1={2} y1={cy + 4} x2={w - 2} y2={cy + 4} {...S} />
          <line x1={2} y1={cy - 7} x2={2} y2={cy + 7} {...S} />
          <line x1={w - 2} y1={cy - 7} x2={w - 2} y2={cy + 7} {...S} />
        </>
      );
      break;
    case 'orifice':
      body = (
        <>
          <line x1={2} y1={cy} x2={w - 2} y2={cy} {...S} strokeWidth={1} />
          <line x1={w / 2 - 3} y1={3} x2={w / 2 - 3} y2={cy - 4} {...S} strokeWidth={2} />
          <line x1={w / 2 - 3} y1={cy + 4} x2={w / 2 - 3} y2={h - 3} {...S} strokeWidth={2} />
          <line x1={w / 2 + 3} y1={3} x2={w / 2 + 3} y2={cy - 4} {...S} strokeWidth={2} />
          <line x1={w / 2 + 3} y1={cy + 4} x2={w / 2 + 3} y2={h - 3} {...S} strokeWidth={2} />
        </>
      );
      break;
    case 'venturi':
      body = (
        <>
          <path d={`M2 ${cy - 10} L${w * 0.42} ${cy - 3} L${w - 2} ${cy - 10}`} {...S} />
          <path d={`M2 ${cy + 10} L${w * 0.42} ${cy + 3} L${w - 2} ${cy + 10}`} {...S} />
        </>
      );
      break;
    case 'valve':
    case 'regulator': {
      const top = type === 'regulator' ? 10 : 3;
      const by = cy;
      body = (
        <>
          <path d={`M8 ${by - 10} L${w / 2} ${by} L8 ${by + 10} Z M${w - 8} ${by - 10} L${w / 2} ${by} L${w - 8} ${by + 10} Z`} {...S} fill="var(--surface)" />
          <line x1={w / 2} y1={by} x2={w / 2} y2={top} {...S} />
          {type === 'valve' ? (
            <line x1={w / 2 - 8} y1={top} x2={w / 2 + 8} y2={top} {...S} />
          ) : (
            <path d={`M${w / 2 - 11} ${top} A11 8 0 0 1 ${w / 2 + 11} ${top} Z`} {...S} fill="var(--surface)" />
          )}
        </>
      );
      break;
    }
    case 'check_valve':
      body = (
        <>
          <line x1={2} y1={cy} x2={w - 2} y2={cy} {...S} strokeWidth={1} />
          <path d={`M14 ${cy - 10} L${w - 18} ${cy} L14 ${cy + 10} Z`} {...S} fill="var(--surface)" />
          <line x1={w - 16} y1={cy - 11} x2={w - 16} y2={cy + 11} {...S} strokeWidth={2} />
        </>
      );
      break;
    case 'loss':
      body = <path d={`M2 ${cy + 6} L${w / 2 - 6} ${cy + 6} Q${w / 2 + 6} ${cy + 6} ${w / 2 + 6} ${cy - 6} L${w / 2 + 6} 3`} {...S} />;
      break;
    case 'mass_flow':
      body = (
        <>
          <circle cx={w / 2} cy={cy} r={14} {...S} fill="var(--surface)" />
          <path d={`M${w / 2 - 7} ${cy} L${w / 2 + 7} ${cy} M${w / 2 + 2} ${cy - 5} L${w / 2 + 7} ${cy} L${w / 2 + 2} ${cy + 5}`} {...S} />
        </>
      );
      break;
    case 'pump':
      body = (
        <>
          <line x1={2} y1={cy} x2={w / 2 - 15} y2={cy} {...S} strokeWidth={1} />
          <line x1={w / 2 + 15} y1={cy} x2={w - 2} y2={cy} {...S} strokeWidth={1} />
          <circle cx={w / 2} cy={cy} r={15} {...S} fill="var(--surface)" />
          <path d={`M${w / 2 - 8} ${cy - 10} L${w / 2 + 12} ${cy} L${w / 2 - 8} ${cy + 10}`} {...S} />
        </>
      );
      break;
    case 'turbine':
      body = (
        <>
          <line x1={2} y1={cy} x2={10} y2={cy} {...S} strokeWidth={1} />
          <line x1={w - 10} y1={cy} x2={w - 2} y2={cy} {...S} strokeWidth={1} />
          <path d={`M10 ${cy - 8} L${w - 10} ${cy - 17} L${w - 10} ${cy + 17} L10 ${cy + 8} Z`} {...S} fill="var(--surface)" />
          <line x1={w / 2} y1={cy + 12} x2={w / 2} y2={h} {...S} strokeWidth={1} strokeDasharray="2 2" />
        </>
      );
      break;
    case 'shaft':
      body = (
        <>
          <line x1={2} y1={cy} x2={w - 2} y2={cy} {...S} strokeWidth={2.5} />
          <rect x={w / 2 - 9} y={cy - 8} width={18} height={16} rx={3} {...S} fill="var(--surface)" />
          <line x1={10} y1={cy - 6} x2={10} y2={cy + 6} {...S} />
          <line x1={w - 10} y1={cy - 6} x2={w - 10} y2={cy + 6} {...S} />
        </>
      );
      break;
    case 'combustion_chamber': {
      const c0 = 8;
      const c1 = 46;
      const th = 60;
      const r = 17;
      const rt = 6;
      body = (
        <>
          <path
            d={`M${c0} ${cy - r} L${c1} ${cy - r} C${c1 + 8} ${cy - r} ${th - 6} ${cy - rt} ${th} ${cy - rt} C${th + 14} ${cy - rt} ${w - 14} ${cy - 22} ${w - 2} ${cy - 27}
               M${c0} ${cy + r} L${c1} ${cy + r} C${c1 + 8} ${cy + r} ${th - 6} ${cy + rt} ${th} ${cy + rt} C${th + 14} ${cy + rt} ${w - 14} ${cy + 22} ${w - 2} ${cy + 27}
               M${c0} ${cy - r} L${c0} ${cy + r}`}
            {...S}
          />
          <line x1={2} y1={cy - 9} x2={c0} y2={cy - 9} {...S} strokeWidth={1} />
          <line x1={2} y1={cy + 9} x2={c0} y2={cy + 9} {...S} strokeWidth={1} />
          <path d={`M${c0 + 6} ${cy} q4 -5 8 0 t8 0 t8 0`} {...S} strokeWidth={1} />
        </>
      );
      break;
    }
    default:
      body = <rect x={2} y={2} width={w - 4} height={h - 4} {...S} />;
  }
  return (
    <svg width={w * size} height={h * size} viewBox={`0 0 ${w} ${h}`} style={{ display: 'block', overflow: 'visible' }}>
      {body}
    </svg>
  );
}
