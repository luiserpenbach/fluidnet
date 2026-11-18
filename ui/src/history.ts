import { useCallback, useRef, useState } from 'react';

interface Hist<T> {
  past: T[];
  present: T;
  future: T[];
}

const LIMIT = 200;

/**
 * Undo/redo history over an immutable value. `commit(f, key)` with the same
 * key within a second replaces the present without adding a step, so typing in
 * a text field or nudging with the arrow keys undoes as one edit.
 */
export function useHistory<T>(init: () => T) {
  const [st, setSt] = useState<Hist<T>>(() => ({ past: [], present: init(), future: [] }));
  const last = useRef({ key: '', t: 0 });

  const commit = useCallback((f: (v: T) => T, coalesce?: string) => {
    const now = Date.now();
    const merge = !!coalesce && last.current.key === coalesce && now - last.current.t < 1000;
    last.current = { key: coalesce ?? '', t: now };
    setSt((s) => {
      const next = f(s.present);
      if (next === s.present) return s;
      return { past: merge && s.past.length ? s.past : [...s.past.slice(-(LIMIT - 1)), s.present], present: next, future: [] };
    });
  }, []);

  const undo = useCallback(() => {
    last.current = { key: '', t: 0 };
    setSt((s) => (s.past.length ? { past: s.past.slice(0, -1), present: s.past[s.past.length - 1], future: [s.present, ...s.future] } : s));
  }, []);

  const redo = useCallback(() => {
    last.current = { key: '', t: 0 };
    setSt((s) => (s.future.length ? { past: [...s.past, s.present], present: s.future[0], future: s.future.slice(1) } : s));
  }, []);

  return { value: st.present, commit, undo, redo, canUndo: st.past.length > 0, canRedo: st.future.length > 0 };
}
