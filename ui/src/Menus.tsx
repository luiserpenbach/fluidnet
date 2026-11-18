// Floating UI: context menu, command palette, shortcut reference, toasts.
import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';

export interface MenuItem {
  label?: string;
  hint?: string;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  onClick?: () => void;
  items?: MenuItem[];
}

export function ContextMenu({ x, y, items, onClose }: { x: number; y: number; items: MenuItem[]; onClose: () => void }) {
  const el = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });
  const [sub, setSub] = useState<number | null>(null);
  useLayoutEffect(() => {
    const r = el.current?.getBoundingClientRect();
    if (!r) return;
    setPos({ x: Math.max(4, Math.min(x, window.innerWidth - r.width - 4)), y: Math.max(4, Math.min(y, window.innerHeight - r.height - 4)) });
  }, [x, y]);
  useEffect(() => {
    const down = (e: PointerEvent) => {
      if (!el.current?.contains(e.target as Node)) onClose();
    };
    const key = (e: KeyboardEvent) => e.key === 'Escape' && onClose();
    window.addEventListener('pointerdown', down, true);
    window.addEventListener('keydown', key);
    window.addEventListener('blur', onClose);
    return () => {
      window.removeEventListener('pointerdown', down, true);
      window.removeEventListener('keydown', key);
      window.removeEventListener('blur', onClose);
    };
  }, [onClose]);
  const flipSub = pos.x > window.innerWidth - 420;
  return (
    <div ref={el} className="ctx" role="menu" style={{ left: pos.x, top: pos.y }} onContextMenu={(e) => e.preventDefault()}>
      {items.map((it, i) =>
        it.separator ? (
          <div key={i} className="ctx-sep" />
        ) : (
          <div key={i} className="ctx-wrap" onMouseEnter={() => setSub(it.items ? i : null)}>
            <button
              role="menuitem"
              className={`ctx-item${it.danger ? ' danger' : ''}`}
              disabled={it.disabled}
              onClick={() => {
                if (it.items) return setSub(sub === i ? null : i);
                onClose();
                it.onClick?.();
              }}
            >
              <span>{it.label}</span>
              <span className="ctx-hint">{it.items ? '›' : it.hint}</span>
            </button>
            {it.items && sub === i && (
              <div className={`ctx ctx-sub${flipSub ? ' left' : ''}`} role="menu">
                {it.items.map((s, j) => (
                  <button
                    key={j}
                    role="menuitem"
                    className="ctx-item"
                    disabled={s.disabled}
                    onClick={() => {
                      onClose();
                      s.onClick?.();
                    }}
                  >
                    <span>{s.label}</span>
                    <span className="ctx-hint">{s.hint}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        )
      )}
    </div>
  );
}

export interface Command {
  group: string;
  label: string;
  hint?: string;
  keywords?: string;
  run: () => void;
}

export function CommandPalette({ commands, onClose }: { commands: Command[]; onClose: () => void }) {
  const [q, setQ] = useState('');
  const [idx, setIdx] = useState(0);
  const list = useRef<HTMLDivElement>(null);
  const hits = useMemo(() => {
    const terms = q.toLowerCase().split(/\s+/).filter(Boolean);
    if (!terms.length) return commands.slice(0, 80);
    return commands
      .map((c) => {
        const hay = `${c.label} ${c.group} ${c.keywords ?? ''}`.toLowerCase();
        if (!terms.every((t) => hay.includes(t))) return null;
        const lab = c.label.toLowerCase();
        return { c, score: lab.startsWith(terms[0]) ? 0 : lab.includes(terms[0]) ? 1 : 2 };
      })
      .filter(Boolean)
      .sort((a, b) => a!.score - b!.score)
      .slice(0, 80)
      .map((h) => h!.c);
  }, [q, commands]);
  useEffect(() => setIdx(0), [q]);
  useEffect(() => {
    list.current?.querySelector('.cmd-item.on')?.scrollIntoView({ block: 'nearest' });
  }, [idx]);
  const go = (c?: Command) => {
    if (!c) return;
    onClose();
    c.run();
  };
  return (
    <div className="modal-back top" onPointerDown={onClose}>
      <div className="cmd" onPointerDown={(e) => e.stopPropagation()}>
        <input
          id="cmd-input"
          className="cmd-input"
          autoFocus
          placeholder="Find a component, add a part, or run a command"
          value={q}
          spellCheck={false}
          onChange={(e) => setQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown') {
              e.preventDefault();
              setIdx((i) => Math.min(hits.length - 1, i + 1));
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              setIdx((i) => Math.max(0, i - 1));
            } else if (e.key === 'Enter') {
              e.preventDefault();
              go(hits[idx]);
            } else if (e.key === 'Escape') {
              onClose();
            }
          }}
        />
        <div className="cmd-list" ref={list}>
          {hits.length === 0 && <div className="cmd-empty">Nothing matches “{q}”.</div>}
          {hits.map((c, i) => (
            <button key={`${c.group}:${c.label}`} className={`cmd-item${i === idx ? ' on' : ''}`} onMouseMove={() => setIdx(i)} onClick={() => go(c)}>
              <span className="cmd-group">{c.group}</span>
              <span className="cmd-label">{c.label}</span>
              <span className="ctx-hint">{c.hint}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

const MAC = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);
export const MOD = MAC ? '⌘' : 'Ctrl';

const SHORTCUTS: [string, [string, string][]][] = [
  [
    'Model',
    [
      [`${MOD}+Z`, 'Undo'],
      [`${MOD}+Shift+Z`, 'Redo'],
      [`${MOD}+S`, 'Save the model file'],
      [`${MOD}+O`, 'Open a model file'],
      [`${MOD}+K`, 'Find a component or command'],
    ],
  ],
  [
    'Schematic',
    [
      ['Click a line', 'Select a connection'],
      ['Delete', 'Delete the selection'],
      ['Shift+drag', 'Select with a box'],
      ['Shift+click', 'Add to the selection'],
      [`${MOD}+A`, 'Select everything'],
      [`${MOD}+C / X / V`, 'Copy, cut, paste'],
      [`${MOD}+D`, 'Duplicate'],
      ['R', 'Rotate a quarter turn'],
      ['F', 'Flip'],
      ['Arrow keys', 'Nudge the selection'],
      ['Right-click', 'Context menu'],
      ['Drop a part on a line', 'Insert it into the line'],
      ['1', 'Fit the schematic to the view'],
    ],
  ],
  [
    'Simulation',
    [
      [`${MOD}+Enter`, 'Run the transient'],
      ['Shift+Enter', 'Solve the steady state'],
      ['P', 'Play or pause the result'],
      ['Space+drag', 'Pan the schematic'],
      ['?', 'This list'],
    ],
  ],
];

export function ShortcutsDialog({ onClose }: { onClose: () => void }) {
  useEffect(() => {
    const key = (e: KeyboardEvent) => e.key === 'Escape' && onClose();
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [onClose]);
  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal narrow" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <b>Keyboard and mouse</b>
        </div>
        <div className="keys">
          {SHORTCUTS.map(([group, rows]) => (
            <div key={group} className="keys-group">
              <div className="section-label">{group}</div>
              {rows.map(([k, what]) => (
                <div key={k} className="keys-row">
                  <span>{what}</span>
                  <kbd>{k}</kbd>
                </div>
              ))}
            </div>
          ))}
        </div>
        <div className="modal-actions">
          <div className="spacer" />
          <button className="btn-secondary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

export interface Toast {
  id: number;
  text: string;
  kind: 'info' | 'bad';
}

export function Toasts({ toasts }: { toasts: Toast[] }) {
  return (
    <div className="toasts" role="status" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.kind}`}>
          {t.text}
        </div>
      ))}
    </div>
  );
}
