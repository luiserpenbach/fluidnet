import React, { memo } from 'react';
import { Handle, Position } from '@xyflow/react';
import { Symbol, SYMBOL_SIZE } from './symbols';

export interface CompNodeData {
  id: string;
  type: string;
  flip: boolean;
  nodeLike: boolean;
  overlay: string[];
  color: string;
  error: boolean;
  [k: string]: unknown;
}

const hs: React.CSSProperties = { width: 7, height: 7 };

function CompNodeImpl({ data, selected }: { data: any; selected?: boolean }) {
  const { id, type, flip, nodeLike, overlay, color, error } = data as CompNodeData;
  const [w, h] = SYMBOL_SIZE[type] ?? [48, 32];
  const L = flip ? Position.Right : Position.Left;
  const R = flip ? Position.Left : Position.Right;
  const handles: React.ReactNode[] = [];
  const anchor = (id: string, pos: Position, style: React.CSSProperties = {}) => (
    <Handle key={id} id={id} type="source" position={pos} isConnectable={false} className="anchor" style={{ ...hs, ...style }} />
  );
  if (type === 'shaft') {
    handles.push(anchor('mech', Position.Top));
  } else if (type === 'combustion_chamber') {
    handles.push(<Handle key="fuel" id="fuel" type="source" position={Position.Left} style={{ ...hs, top: h / 2 - 9 }} className="port" title="fuel" />);
    handles.push(<Handle key="ox" id="ox" type="source" position={Position.Left} style={{ ...hs, top: h / 2 + 9 }} className="port" title="oxidizer" />);
    handles.push(anchor('heat', Position.Top, { left: 30 }));
  } else if (nodeLike) {
    const sides: [string, Position][] =
      type === 'boundary'
        ? [['p-l', Position.Left], ['p-r', Position.Right]]
        : [['p-l', Position.Left], ['p-r', Position.Right], ['p-t', Position.Top], ['p-b', Position.Bottom]];
    for (const [hid, pos] of sides) {
      handles.push(<Handle key={hid} id={hid} type="source" position={pos} style={hs} className="port" />);
    }
  } else {
    handles.push(<Handle key="in" id="in" type="source" position={L} style={hs} className="port port-in" />);
    handles.push(<Handle key="out" id="out" type="source" position={R} style={hs} className="port port-out" />);
    if (type === 'pump' || type === 'turbine') handles.push(anchor('mech', Position.Bottom));
    if (type === 'pipe') handles.push(anchor('heat', Position.Top));
  }
  return (
    <div className={`cnode${selected ? ' sel' : ''}${error ? ' err' : ''}`} style={{ color }}>
      <div className="cnode-sym" style={{ width: w, height: h, transform: flip ? 'scaleX(-1)' : undefined }}>
        <Symbol type={type} />
      </div>
      {handles}
      <div className="cnode-label">{id}</div>
      {overlay.length > 0 && (
        <div className="cnode-overlay">
          {overlay.map((l, i) => (
            <div key={i}>{l}</div>
          ))}
        </div>
      )}
    </div>
  );
}

export const CompNode = memo(CompNodeImpl);
