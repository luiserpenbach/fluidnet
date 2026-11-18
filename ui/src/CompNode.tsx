import React, { memo } from 'react';
import { Handle, Position } from '@xyflow/react';
import { Symbol } from './symbols';
import { nodeGeometry } from './graph';

export interface CompNodeData {
  id: string;
  type: string;
  flip: boolean;
  rot: number;
  overlay: string[];
  color: string;
  error: boolean;
  [k: string]: unknown;
}

function CompNodeImpl({ data, selected }: { data: any; selected?: boolean }) {
  const { id, type, flip, rot, overlay, color, error } = data as CompNodeData;
  const g = nodeGeometry(type, rot, flip);
  // a line leaves a turned element (or boundary) through the bottom, where the label would be
  const sideLabel = g.handles.some((x) => x.kind === 'port' && x.side === 'bottom' && (type === 'boundary' || !x.id.startsWith('p-')));
  return (
    <div className={`cnode${selected ? ' sel' : ''}${error ? ' err' : ''}${sideLabel ? ' side-label' : ''}`} style={{ color, width: g.W, height: g.H }}>
      <div
        className="cnode-sym"
        style={{ left: (g.W - g.w) / 2, top: (g.H - g.h) / 2, width: g.w, height: g.h, transform: rot || flip ? `rotate(${rot}deg)${flip ? ' scaleX(-1)' : ''}` : undefined }}
      >
        <Symbol type={type} />
      </div>
      {g.handles.map((hd) => (
        <Handle
          key={hd.id}
          id={hd.id}
          type="source"
          position={hd.side as Position}
          isConnectable={hd.kind === 'port'}
          className={hd.kind === 'anchor' ? 'anchor' : `port${hd.cls ? ` ${hd.cls}` : ''}`}
          title={hd.title}
          style={hd.side === 'left' || hd.side === 'right' ? { top: hd.along } : { left: hd.along }}
        />
      ))}
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
