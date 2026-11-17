"""Design summary tables and the Plotly HTML report."""
from __future__ import annotations
import json
from dataclasses import asdict
from pathlib import Path
import numpy as np
import plotly.graph_objects as go
from plotly.subplots import make_subplots

from .cycle import CycleResult
from .architecture import engine_mass
from .fluids import O2

LAYOUT = dict(template="plotly_white", font=dict(family="Inter, Helvetica, Arial, sans-serif", size=13, color="#222"),
              margin=dict(l=60, r=30, t=60, b=50), colorway=["#111111", "#2f6fde", "#d9480f", "#2b8a3e", "#868e96", "#ae3ec9"])


def summary(r: CycleResult) -> dict:
    su = r.setup; cfg = su.cfg; f, o = r.fuel, r.ox; ct = su.ct; m = engine_mass(r)

    def side(s):
        d = {"closed": s.closed, "mdot_pack": s.mdot_pack, "heated_fraction": s.heated_fraction, "T_inj": s.T_inj, "note": s.note}
        if s.pump:
            p = s.pump
            d["pump"] = dict(p_out=p.p_out, rpm=p.rpm, stages=p.stages, eta=p.eta, power=p.power, nq=p.nq, d2=p.d2, u_tip=p.u_tip,
                             Q=p.Q, head=p.head, nss=p.nss, npsh=p.npsh, d_inducer=p.d_inducer, d_shaft=p.d_shaft, dn=p.dn,
                             limit=p.limit, T_out=p.T_out)
        if s.turbine:
            t = s.turbine
            d["turbine"] = dict(p_in=t.p_in, T_in=t.T_in, p_out=t.p_out, T_out=t.T_out, pr=t.p_in / t.p_out, mdot=t.mdot,
                                stages=t.stages, eta=t.eta, power=t.power, dm=t.dm, blade_h=t.blade_h, u=t.u, nu=t.nu,
                                admission=t.admission, dh_s=t.dh_s, bypass=s.bypass)
        if s.zone:
            z = s.zone
            d["zone"] = dict(Q=z.Q, dp=z.dp, T_out=z.T_out, p_in=s.p_zone_in, Twg_max=z.Twg_max, q_max=float(z.q.max()),
                             area=z.area, h_min=float(z.h_ch.min()), h_max=float(z.h_ch.max()), n_ch_min=int(z.n_ch.min()),
                             n_ch_max=int(z.n_ch.max()), v_max=float(z.vel.max()), mach_max=z.mach_max, p_min=float(z.p.min()))
        return d
    from .thermochem import default_provider
    return dict(name=cfg.name, sha=cfg.sha(), thermochem=default_provider().name, thrust_total=cfg.thrust_total, n_chambers=cfg.n_chambers, n_packs=cfg.n_packs,
                thrust_chamber=su.thrust_chamber, pc=cfg.pc, of=cfg.of, isp=su.isp, isp_ideal=su.th.isp_vac, cstar=su.th.cstar,
                Tc=su.th.Tc, eps=su.eps, mdot_total=su.mdot_total, mf_total=su.mf_total, mo_total=su.mo_total,
                d_throat=2 * ct.Rt, d_chamber=2 * ct.Rc, d_exit=2 * ct.Re, L_cyl=ct.L_cyl, L_chamber=ct.L_chamber,
                L_nozzle=ct.L_noz, eps_regen=ct.eps_regen, Q_chamber=r.Q_total_chamber, mass=m, fuel=side(f), ox=side(o))


def fig_chamber(r: CycleResult) -> go.Figure:
    su = r.setup; ct = su.ct
    fig = make_subplots(rows=4, cols=1, shared_xaxes=True, vertical_spacing=0.04,
                        subplot_titles=("Contour (regeneratively cooled part)", "Hot-gas-side heat flux",
                                        "Wall and coolant temperature", "Coolant pressure and channel height"),
                        specs=[[{}], [{}], [{}], [{"secondary_y": True}]])
    n = ct.i_regen_end + 1
    fig.add_trace(go.Scatter(x=ct.x[:n] * 1e3, y=ct.r[:n] * 1e3, name="wall radius", line=dict(color="#111")), 1, 1)
    for s, col, lab in ((r.fuel, "#2f6fde", "hydrogen"), (r.ox, "#d9480f", "oxygen")):
        z = s.zone
        if z is None:
            continue
        x = ct.x[z.idx] * 1e3
        fig.add_trace(go.Scatter(x=x, y=z.q / 1e6, name=f"heat flux, {lab} zone", line=dict(color=col)), 2, 1)
        fig.add_trace(go.Scatter(x=x, y=z.Twg, name=f"hot wall, {lab} zone", line=dict(color=col)), 3, 1)
        fig.add_trace(go.Scatter(x=x, y=z.Tb, name=f"{lab} bulk", line=dict(color=col, dash="dot")), 3, 1)
        fig.add_trace(go.Scatter(x=x, y=z.p / 1e5, name=f"{lab} pressure", line=dict(color=col)), 4, 1)
        fig.add_trace(go.Scatter(x=x, y=z.h_ch * 1e3, name=f"channel height, {lab}", line=dict(color=col, dash="dash")), 4, 1, secondary_y=True)
    fig.update_yaxes(title_text="mm", row=1, col=1); fig.update_yaxes(title_text="MW/m²", row=2, col=1)
    fig.update_yaxes(title_text="K", row=3, col=1); fig.update_yaxes(title_text="bar", row=4, col=1)
    fig.update_yaxes(title_text="mm", row=4, col=1, secondary_y=True)
    fig.update_xaxes(title_text="axial position from injector face [mm]", row=4, col=1)
    fig.update_layout(height=950, title="Thrust chamber cooling at the design point", **LAYOUT)
    return fig


def fig_arch(rows: list[dict]) -> go.Figure:
    ok = [x for x in rows if "error" not in x]
    fig = make_subplots(rows=1, cols=3, subplot_titles=("Maximum closing chamber pressure [bar]", "Jacket heat per kg propellant [kJ/kg]",
                                                        "Engine length [m]"))
    lab = [x["label"] for x in ok]
    fig.add_trace(go.Bar(x=lab, y=[x["pc_max"] / 1e5 for x in ok], marker_color="#111"), 1, 1)
    fig.add_trace(go.Bar(x=lab, y=[x["q_per_kg"] for x in ok], marker_color="#2f6fde"), 1, 2)
    fig.add_trace(go.Bar(x=lab, y=[x["length"] for x in ok], marker_color="#868e96"), 1, 3)
    fig.update_layout(height=480, showlegend=False, title="Architecture trade at 180 kN total thrust and 3.0 m² total exit area", **LAYOUT)
    return fig


def fig_sens(rows: list[dict]) -> go.Figure:
    base = rows[0]["pc_max"]
    r = [x for x in rows[1:] if x["pc_max"]]
    fig = go.Figure(go.Bar(y=[x["case"] for x in r], x=[(x["pc_max"] - base) / 1e5 for x in r], orientation="h", marker_color="#111"))
    fig.update_layout(height=460, title=f"Sensitivity of the maximum closing chamber pressure (baseline {base / 1e5:.1f} bar)",
                      xaxis_title="change [bar]", **LAYOUT)
    return fig


def fig_throttle(pts: list, pts_floor: list | None = None) -> go.Figure:
    p = [x for x in pts if x is not None and x.converged]
    t = [100 * x.throttle for x in p]
    fig = make_subplots(rows=2, cols=3, subplot_titles=("Shaft speed [% of design]", "Turbine bypass flow fraction", "Turbine inlet temperature [K]",
                                                        "Pump flow coefficient / design", "Injector pressure drop / chamber pressure",
                                                        "Oxygen circuit outlet pressure [bar]"))
    f0, o0 = p[0].n_f, p[0].n_o
    for (r_, c_, yf, yo) in ((1, 1, [100 * x.n_f / f0 for x in p], [100 * x.n_o / o0 for x in p]), (1, 2, [x.byp_f for x in p], [x.byp_o for x in p]),
                             (1, 3, [x.T_ti_f for x in p], [x.T_ti_o for x in p]), (2, 1, [x.phi_f for x in p], [x.phi_o for x in p]),
                             (2, 2, [x.dpinj_f for x in p], [x.dpinj_o for x in p])):
        fig.add_trace(go.Scatter(x=t, y=yf, name="fuel side", line=dict(color="#2f6fde"), showlegend=(r_ == 1 and c_ == 1)), r_, c_)
        fig.add_trace(go.Scatter(x=t, y=yo, name="oxygen side", line=dict(color="#d9480f"), showlegend=(r_ == 1 and c_ == 1)), r_, c_)
    fig.add_trace(go.Scatter(x=t, y=[x.p_zo_o / 1e5 for x in p], name="no back-pressure valve", line=dict(color="#d9480f")), 2, 3)
    if pts_floor:
        q = [x for x in pts_floor if x is not None and x.converged]
        fig.add_trace(go.Scatter(x=[100 * x.throttle for x in q], y=[x.p_zo_o / 1e5 for x in q], name="with back-pressure valve",
                                 line=dict(color="#2b8a3e", dash="dash")), 2, 3)
    fig.add_hline(y=O2().pcrit / 1e5, line=dict(color="#868e96", dash="dot"), row=2, col=3)
    fig.update_xaxes(title_text="pack thrust [% of design chamber pressure]")
    fig.update_layout(height=720, title="Powerpack throttle characteristics (frozen hardware)", **LAYOUT)
    return fig


def fig_start(res: dict, r: CycleResult) -> go.Figure:
    fig = make_subplots(rows=2, cols=2, subplot_titles=("Shaft speed [% of design]", "Chamber pressure [bar]", "Mixture ratio", "Liner temperature [K]"))
    t = res["t"]
    fig.add_trace(go.Scatter(x=t, y=100 * res["n_f"] / r.fuel.pump.rpm, name="fuel turbopump", line=dict(color="#2f6fde")), 1, 1)
    fig.add_trace(go.Scatter(x=t, y=100 * res["n_o"] / r.ox.pump.rpm, name="oxygen turbopump", line=dict(color="#d9480f")), 1, 1)
    fig.add_trace(go.Scatter(x=t, y=res["pc"] / 1e5, name="chamber pressure", line=dict(color="#111")), 1, 2)
    fig.add_trace(go.Scatter(x=t, y=np.clip(res["of"], 0, 12), name="mixture ratio", line=dict(color="#111")), 2, 1)
    fig.add_trace(go.Scatter(x=t, y=res["Twf"], name="hydrogen zone liner", line=dict(color="#2f6fde", dash="dot")), 2, 2)
    fig.add_trace(go.Scatter(x=t, y=res["Two"], name="oxygen zone liner", line=dict(color="#d9480f", dash="dot")), 2, 2)
    fig.update_xaxes(title_text="time [s]")
    fig.update_layout(height=640, title="Start transient, lumped model (fidelity L0)", **LAYOUT)
    return fig


def write_html(figs: list[go.Figure], path: Path, title: str, intro: str):
    parts = [f"<html><head><meta charset='utf-8'><title>{title}</title><style>body{{font-family:Inter,Helvetica,Arial,sans-serif;"
             f"max-width:1180px;margin:32px auto;color:#222;background:#fff}}h1{{font-weight:600}}p{{color:#555;line-height:1.5}}"
             f".fig{{border:1px solid #e5e5e5;border-radius:8px;margin:18px 0;padding:6px}}</style></head><body><h1>{title}</h1><p>{intro}</p>"]
    for i, f in enumerate(figs):
        parts.append("<div class='fig'>" + f.to_html(full_html=False, include_plotlyjs=("cdn" if i == 0 else False)) + "</div>")
    parts.append("</body></html>")
    Path(path).write_text("\n".join(parts))


def fig_start_fluidnet(res, r: CycleResult) -> go.Figure:
    import math
    fig = make_subplots(rows=2, cols=2, subplot_titles=("Shaft speed [% of design]", "Chamber pressure [bar]", "Mixture ratio",
                                                        "Turbine bypass valve area / design area"))
    t = res.time
    fig.add_trace(go.Scatter(x=t, y=100 * res["fuel_shaft.speed"] * 30 / math.pi / r.fuel.pump.rpm, name="fuel turbopump", line=dict(color="#2f6fde")), 1, 1)
    fig.add_trace(go.Scatter(x=t, y=100 * res["lox_shaft.speed"] * 30 / math.pi / r.ox.pump.rpm, name="oxygen turbopump", line=dict(color="#d9480f")), 1, 1)
    fig.add_trace(go.Scatter(x=t, y=res["tca.Pc"] / 1e5, name="chamber pressure", line=dict(color="#111")), 1, 2)
    fig.add_trace(go.Scatter(x=t, y=np.clip(res["tca.OF"], 0, 10), name="mixture ratio", line=dict(color="#111")), 2, 1)
    fig.add_trace(go.Scatter(x=t, y=16 * res["ftbv.opening"], name="fuel bypass (scheduled)", line=dict(color="#2f6fde", dash="dot")), 2, 2)
    fig.add_trace(go.Scatter(x=t, y=16 * res["otbv.opening"], name="oxygen bypass (closed loop)", line=dict(color="#d9480f", dash="dot")), 2, 2)
    fig.update_xaxes(title_text="time [s]")
    fig.update_layout(height=640, title="Tank-head start of one powerpack, fluidnet network transient", **LAYOUT)
    return fig
