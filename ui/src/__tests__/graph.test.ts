import { describe, expect, it } from 'vitest';
import { parseModel, toYaml } from '../model';
import { alignComps, connKey, extractFragment, fragmentFromText, fragmentToText, freshId, nodeGeometry, pasteFragment, refKeysOf, removeItems, renameComp, spliceInto, spliceNode } from '../graph';

const REFS = refKeysOf(null);
const base = () =>
  parseModel(`
name: t
components:
  - {id: tank, type: boundary, fluid: H2, P: 3 bar, T: 21 K, ui: {x: 0, y: 0}}
  - {id: pump, type: pump, shaft: tp, ui: {x: 100, y: 0}}
  - {id: valve, type: valve, diameter: 8 mm, sense: pump.out, ui: {x: 200, y: 40}}
  - {id: out, type: boundary, fluid: H2, P: 1 bar, T: 21 K, ui: {x: 300, y: 0}}
  - {id: tp, type: shaft, ui: {x: 100, y: 100}}
connections:
  - [tank, pump.in]
  - [pump.out, valve.in]
  - [valve.out, out]
`);

describe('removeItems', () => {
  it('removes a connection only', () => {
    const m = removeItems(base(), [], [connKey('pump.out', 'valve.in')], REFS);
    expect(m.components).toHaveLength(5);
    expect(m.connections).toEqual([['tank', 'pump.in'], ['valve.out', 'out']]);
  });
  it('removes a component with its connections and clears references to it', () => {
    const m = removeItems(base(), ['tp', 'pump'], [], REFS);
    expect(m.components.map((c) => c.id)).toEqual(['tank', 'valve', 'out']);
    expect(m.connections).toEqual([['valve.out', 'out']]);
    expect(m.components.find((c) => c.id === 'valve')!.params.sense).toBeUndefined();
    const m2 = removeItems(base(), ['tp'], [], REFS);
    expect(m2.components.find((c) => c.id === 'pump')!.params.shaft).toBeUndefined();
  });
  it('returns the same model when nothing is selected', () => {
    const m = base();
    expect(removeItems(m, [], [], REFS)).toBe(m);
  });
});

describe('renameComp', () => {
  it('follows the id through connections and references', () => {
    const m = renameComp(renameComp(base(), 'pump', 'fuel_pump', REFS), 'tp', 'turbopump', REFS);
    expect(m.connections).toEqual([['tank', 'fuel_pump.in'], ['fuel_pump.out', 'valve.in'], ['valve.out', 'out']]);
    expect(m.components.find((c) => c.id === 'fuel_pump')!.params.shaft).toBe('turbopump');
    expect(m.components.find((c) => c.id === 'valve')!.params.sense).toBe('fuel_pump.out');
  });
  it('refuses an id that is taken', () => {
    const m = base();
    expect(renameComp(m, 'pump', 'valve', REFS)).toBe(m);
  });
});

describe('copy and paste', () => {
  it('copies internal connections and gives fresh ids', () => {
    const m = base();
    const f = extractFragment(m, ['pump', 'valve', 'tp']);
    expect(f.connections).toEqual([['pump.out', 'valve.in']]);
    const { model, ids } = pasteFragment(m, f, REFS);
    expect(ids).toEqual(['pump_2', 'valve_2', 'tp_2']);
    expect(model.connections.slice(-1)).toEqual([['pump_2.out', 'valve_2.in']]);
    const pump2 = model.components.find((c) => c.id === 'pump_2')!;
    expect(pump2.params.shaft).toBe('tp_2');
    expect(pump2.ui.x).toBe(130);
    expect(model.components.find((c) => c.id === 'valve_2')!.params.sense).toBe('pump_2.out');
    // the original is untouched
    expect(model.components.find((c) => c.id === 'pump')!.params.shaft).toBe('tp');
  });
  it('keeps a reference to a part outside the copy, drops one that does not exist', () => {
    const m = base();
    const { model } = pasteFragment(m, extractFragment(m, ['pump']), REFS);
    expect(model.components.find((c) => c.id === 'pump_2')!.params.shaft).toBe('tp');
    const other = parseModel('components: []');
    const pasted = pasteFragment(other, extractFragment(m, ['pump']), REFS).model;
    expect(pasted.components[0].id).toBe('pump');
    expect(pasted.components[0].params.shaft).toBeUndefined();
  });
  it('round-trips through clipboard text', () => {
    const f = extractFragment(base(), ['pump', 'valve']);
    const g = fragmentFromText(fragmentToText(f))!;
    expect(g.components.map((c) => [c.id, c.type, c.params, c.ui.x, c.ui.y])).toEqual(f.components.map((c) => [c.id, c.type, c.params, c.ui.x, c.ui.y]));
    expect(g.connections).toEqual(f.connections);
    expect(fragmentFromText('just some text')).toBeNull();
  });
  it('counts ids up', () => {
    expect(freshId('valve', new Set(['valve']))).toBe('valve_2');
    expect(freshId('valve_2', new Set(['valve', 'valve_2', 'valve_3']))).toBe('valve_4');
    expect(freshId('free', new Set(['valve']))).toBe('free');
  });
});

describe('splice', () => {
  it('inserts a two-port part in flow order', () => {
    const m = spliceInto(base(), connKey('pump.out', 'valve.in'), 'orifice');
    expect(m.connections).toEqual([['tank', 'pump.in'], ['pump.out', 'orifice.in'], ['orifice.out', 'valve.in'], ['valve.out', 'out']]);
  });
  it('turns a line written downstream-first around', () => {
    const m0 = { ...base(), connections: [['valve.in', 'pump.out']] as [string, string][] };
    expect(spliceInto(m0, connKey('valve.in', 'pump.out'), 'x').connections).toEqual([['pump.out', 'x.in'], ['x.out', 'valve.in']]);
  });
  it('inserts a junction', () => {
    const m = spliceNode(base(), connKey('pump.out', 'valve.in'), 'j');
    expect(m.connections.slice(1, 3)).toEqual([['pump.out', 'j'], ['j', 'valve.in']]);
  });
});

describe('layout', () => {
  it('aligns and distributes', () => {
    const m = alignComps(base(), ['pump', 'valve', 'out'], 'top');
    expect(m.components.filter((c) => ['pump', 'valve', 'out'].includes(c.id)).map((c) => c.ui.y)).toEqual([0, 0, 0]);
    const d = alignComps(base(), ['tank', 'pump', 'out'], 'hdist');
    const cx = (id: string, w: number) => d.components.find((c) => c.id === id)!.ui.x + w / 2;
    expect(cx('pump', 48) - cx('tank', 36)).toBeCloseTo(cx('out', 36) - cx('pump', 48), 0);
  });
  it('rotates and flips ports with the symbol', () => {
    const side = (rot: number, flip: boolean, id: string) => nodeGeometry('valve', rot, flip).handles.find((h) => h.id === id)!.side;
    expect([side(0, false, 'in'), side(0, false, 'out')]).toEqual(['left', 'right']);
    expect([side(0, true, 'in'), side(0, true, 'out')]).toEqual(['right', 'left']);
    expect([side(90, false, 'in'), side(90, false, 'out')]).toEqual(['top', 'bottom']);
    expect([side(180, false, 'in'), side(270, false, 'in')]).toEqual(['right', 'bottom']);
    const g = nodeGeometry('combustion_chamber', 90, false);
    expect([g.W, g.H]).toEqual([60, 104]);
    expect(g.handles.filter((h) => h.kind === 'port').map((h) => h.side)).toEqual(['top', 'top']);
  });
  it('writes rotation to the model file and reads it back', () => {
    const m = base();
    m.components[2].ui.rot = 90;
    expect(parseModel(toYaml(m)).components[2].ui.rot).toBe(90);
    expect(toYaml(base())).not.toContain('rot');
  });
});
