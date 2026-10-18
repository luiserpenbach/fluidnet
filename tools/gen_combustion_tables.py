"""Generate equilibrium combustion tables for fluidnet with Cantera.

For each propellant pair and grid point (O/F, Pc):
  * chamber: adiabatic, isobaric equilibrium (HP) from reactants at their
    reference injection states -> Tc, molar mass, frozen gamma;
  * throat: isentropic equilibrium expansion (shifting) to the point where flow
    velocity equals the equilibrium sound speed -> c* = Pc / (rho_t u_t),
    isentropic exponent at the throat gamma_s = rho a^2 / P;
  * sensitivities to the reactant mixture enthalpy, dc*/dh and dTc/dh, so the
    solver can correct for injection temperatures that differ from the
    reference (e.g. hydrogen leaving a cooling jacket warmer than assumed);
  * the hot-gas heat transfer driving potential: chamber enthalpy minus the
    enthalpy of the same gas in equilibrium at a wall temperature of T_WALL
    (dh_wall, includes the recombination of dissociated species at the wall),
    and the equilibrium specific heat near the wall (cp_wall) to shift it to
    other wall temperatures.

Reactant enthalpies: NASA ideal-gas formation enthalpy at 298.15 K plus the
real-fluid enthalpy change from 298.15 K / 1 bar to the reference state from
CoolProp. The CoolProp-convention reference enthalpies are stored so the solver
can compute the deviation of the actual injected enthalpy in its own units.

Usage: python gen_combustion_tables.py OUT.json
       python gen_combustion_tables.py --augment TABLES.json   (add dh_wall/cp_wall only)
"""
import json
import sys
import time

import numpy as np
import cantera as ct
import CoolProp.CoolProp as CP
from scipy.optimize import brentq

SPECIES = ['H2', 'O2', 'H2O', 'OH', 'H', 'O', 'HO2', 'H2O2', 'CH4', 'CO', 'CO2', 'HCO',
           'CH3', 'C2H2,acetylene', 'C2H4', 'C2H5OH', 'CH3OH', 'N2', 'NO', 'N', 'NO2', 'N2O', 'NH3']

# fluid key in fluidnet -> (Cantera species, CoolProp name, reference T [K], reference P [Pa], description)
PROPELLANTS = {
    'H2': ('H2', 'ParaHydrogen', 250.0, 50e5, 'gaseous hydrogen, 250 K'),
    'CH4': ('CH4', 'Methane', 200.0, 60e5, 'methane, 200 K (supercritical)'),
    'C2H5OH': ('C2H5OH', 'Ethanol', 293.15, 20e5, 'liquid ethanol, 293 K'),
    'O2': ('O2', 'Oxygen', 90.0, 20e5, 'liquid oxygen, 90 K'),
}

PAIRS = [
    ('H2', 'O2', np.r_[np.linspace(0.5, 3.0, 6), np.linspace(3.5, 10.0, 27), 11.0, 12.0]),
    ('CH4', 'O2', np.r_[np.linspace(1.0, 2.2, 5), np.linspace(2.4, 5.0, 27), 5.5, 6.0]),
    ('C2H5OH', 'O2', np.r_[np.linspace(0.4, 1.0, 4), np.linspace(1.1, 2.8, 27), 3.0, 3.5]),
]
PC = np.array([1, 2, 5, 10, 20, 35, 50, 75, 100, 150, 200]) * 1e5
T_WALL = 600.0


def gas():
    sp = ct.Species.list_from_file('nasa_gas.yaml')
    return ct.Solution(thermo='ideal-gas', species=[s for s in sp if s.name in SPECIES])


G = gas()


def reactant_h(key):
    """(h in Cantera convention J/kg, h in CoolProp convention J/kg) at the reference state."""
    sp, cpname, T, P, _ = PROPELLANTS[key]
    G.TPX = 298.15, 101325.0, {sp: 1.0}
    h298 = G.enthalpy_mass
    dh = CP.PropsSI('H', 'T', T, 'P', P, cpname) - CP.PropsSI('H', 'T', 298.15, 'P', 1e5, cpname)
    return h298 + dh, CP.PropsSI('H', 'T', T, 'P', P, cpname)


def sound_speed_eq(g):
    s, p = g.s, g.P
    rho0 = g.density
    g.SP = s, p * 1.001
    g.equilibrate('SP')
    rho1 = g.density
    g.SP = s, p
    g.equilibrate('SP')
    return np.sqrt(0.001 * p / (rho1 - rho0))


def point(fuel, ox, of, pc, hmix):
    yf, yo = 1.0 / (1.0 + of), of / (1.0 + of)
    # equilibrate at a guess temperature first: the frozen reactant mixture
    # cannot represent cryogenic enthalpies within the NASA polynomial range
    G.TPY = 2500.0, pc, {PROPELLANTS[fuel][0]: yf, PROPELLANTS[ox][0]: yo}
    G.equilibrate('TP')
    G.HP = hmix, pc
    G.equilibrate('HP')
    Tc, Mc, h0, s0 = G.T, G.mean_molecular_weight, G.enthalpy_mass, G.entropy_mass
    gam_f = G.cp_mass / G.cv_mass
    state0 = G.state

    def mach_resid(pt):
        G.SP = s0, pt
        G.equilibrate('SP')
        u = np.sqrt(max(2.0 * (h0 - G.enthalpy_mass), 1e-9))
        return u - sound_speed_eq(G)

    pt = brentq(mach_resid, 0.35 * pc, 0.85 * pc, xtol=1e-6 * pc)
    G.SP = s0, pt
    G.equilibrate('SP')
    u = np.sqrt(2.0 * (h0 - G.enthalpy_mass))
    a = sound_speed_eq(G)
    cstar = pc / (G.density * u)
    gam_s = G.density * a * a / pt
    G.state = state0
    return dict(Tc=Tc, M=Mc, gamma_f=gam_f, gamma_s=gam_s, cstar=cstar, pt_pc=pt / pc)


def wall_potential(fuel, ox, of, pc, hmix):
    """(h_chamber - h_eq(T_WALL), equilibrium cp at T_WALL) for the chamber gas."""
    yf, yo = 1.0 / (1.0 + of), of / (1.0 + of)
    G.TPY = 2500.0, pc, {PROPELLANTS[fuel][0]: yf, PROPELLANTS[ox][0]: yo}
    G.equilibrate('TP')
    G.HP = hmix, pc
    G.equilibrate('HP')
    h0 = G.enthalpy_mass
    G.TP = T_WALL, pc
    G.equilibrate('TP')
    h1 = G.enthalpy_mass
    G.TP = T_WALL + 100.0, pc
    G.equilibrate('TP')
    h2 = G.enthalpy_mass
    return h0 - h1, (h2 - h1) / 100.0


def augment(path):
    with open(path) as f:
        data = json.load(f)
    for t in data['tables']:
        fuel, ox = t['fuel'], t['oxidizer']
        hf, _ = reactant_h(fuel)
        ho, _ = reactant_h(ox)
        dhw, cpw = [], []
        for pc in t['pc']:
            r1, r2 = [], []
            for of in t['of']:
                yf, yo = 1 / (1 + of), of / (1 + of)
                a, b = wall_potential(fuel, ox, of, pc, yf * hf + yo * ho)
                r1.append(float(a))
                r2.append(float(b))
            dhw.append(r1)
            cpw.append(r2)
        t['dh_wall'], t['cp_wall'] = dhw, cpw
        print(f'{fuel}/{ox}: wall potential added')
    data['meta']['t_wall'] = T_WALL
    with open(path, 'w') as f:
        json.dump(data, f, separators=(',', ':'))


def main(out):
    tables = []
    for fuel, ox, ofs in PAIRS:
        t0 = time.time()
        hf, hf_cp = reactant_h(fuel)
        ho, ho_cp = reactant_h(ox)
        nO, nP = len(ofs), len(PC)
        fields = {k: np.zeros((nP, nO)) for k in ['cstar', 'Tc', 'M', 'gamma_s', 'gamma_f', 'dcstar_dh', 'dTc_dh', 'dh_wall', 'cp_wall']}
        for i, pc in enumerate(PC):
            for j, of in enumerate(ofs):
                yf, yo = 1 / (1 + of), of / (1 + of)
                hmix = yf * hf + yo * ho
                r = point(fuel, ox, of, pc, hmix)
                dh = 50e3
                r2 = point(fuel, ox, of, pc, hmix + dh)
                for k in ['cstar', 'Tc', 'M', 'gamma_s', 'gamma_f']:
                    fields[k][i, j] = r[k]
                fields['dcstar_dh'][i, j] = (r2['cstar'] - r['cstar']) / dh
                fields['dTc_dh'][i, j] = (r2['Tc'] - r['Tc']) / dh
                fields['dh_wall'][i, j], fields['cp_wall'][i, j] = wall_potential(fuel, ox, of, pc, hmix)
        best = int(np.argmax(fields['cstar'][PC.searchsorted(50e5)]))
        print(f'{fuel}/{ox}: {nP}x{nO} points in {time.time() - t0:.0f} s; at 50 bar max c* = '
              f'{fields["cstar"][PC.searchsorted(50e5), best]:.0f} m/s at O/F {ofs[best]:.2f}', flush=True)
        tables.append(dict(
            fuel=fuel, oxidizer=ox,
            fuel_ref=PROPELLANTS[fuel][4], ox_ref=PROPELLANTS[ox][4],
            h_ref_fuel=hf_cp, h_ref_ox=ho_cp,
            of=[float(x) for x in ofs], pc=[float(x) for x in PC],
            **{k: [[float(x) for x in row] for row in v] for k, v in fields.items()},
        ))
    meta = dict(generator='tools/gen_combustion_tables.py', cantera=ct.__version__, coolprop=CP.get_global_param_string('version'), t_wall=T_WALL,
                note='shifting-equilibrium throat; reactant enthalpy references in CoolProp convention')
    with open(out, 'w') as f:
        json.dump(dict(meta=meta, tables=tables), f, separators=(',', ':'))


if __name__ == '__main__':
    if sys.argv[1] == '--augment':
        augment(sys.argv[2])
    else:
        main(sys.argv[1])
