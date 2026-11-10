//! Component catalog: ports, parameters, units and defaults. The compiler reads
//! defaults from here and the UI builds its palette and inspector from the same
//! data (exported as JSON), so the two cannot drift apart.

use serde_json::{json, Map, Value};

use crate::error::{Error, Result};
use crate::units::{parse_value, Dim, Schedule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PKind {
    Quantity,
    Fluid,
    Choice,
    Int,
    Bool,
    /// Reference to another component by id (target type in `ParamSpec::target`).
    Ref,
    /// Free text (node names).
    Text,
}

#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: PKind,
    pub dim: Dim,
    /// Default as written in a model file; None means required (or optional when `optional`).
    pub default: Option<&'static str>,
    pub optional: bool,
    pub schedulable: bool,
    pub choices: &'static [&'static str],
    pub help: &'static str,
    /// UI display unit.
    pub unit: &'static str,
    /// For `PKind::Ref`: the component type referenced.
    pub target: &'static str,
}

#[derive(Debug, Clone)]
pub struct CompSpec {
    pub kind: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub ports: &'static [&'static str],
    pub node_like: bool,
    pub help: &'static str,
    pub params: Vec<ParamSpec>,
}

const fn q(
    key: &'static str,
    label: &'static str,
    dim: Dim,
    unit: &'static str,
    default: Option<&'static str>,
    help: &'static str,
) -> ParamSpec {
    ParamSpec {
        key,
        label,
        kind: PKind::Quantity,
        dim,
        default,
        optional: false,
        schedulable: false,
        choices: &[],
        help,
        unit,
        target: "",
    }
}

const fn r(
    key: &'static str,
    label: &'static str,
    target: &'static str,
    help: &'static str,
) -> ParamSpec {
    ParamSpec {
        key,
        label,
        kind: PKind::Ref,
        dim: Dim::None,
        default: None,
        optional: true,
        schedulable: false,
        choices: &[],
        help,
        unit: "",
        target,
    }
}

const fn choice(
    key: &'static str,
    label: &'static str,
    choices: &'static [&'static str],
    default: &'static str,
    help: &'static str,
) -> ParamSpec {
    ParamSpec {
        key,
        label,
        kind: PKind::Choice,
        dim: Dim::None,
        default: Some(default),
        optional: false,
        schedulable: false,
        choices,
        help,
        unit: "",
        target: "",
    }
}

pub const WALL_MATERIALS: &[&str] = &[
    "cucrzr",
    "copper",
    "grcop42",
    "nickel",
    "stainless",
    "inconel718",
    "aluminum",
];

fn drive_params() -> Vec<ParamSpec> {
    vec![
        r(
            "shaft",
            "Shaft",
            "shaft",
            "Shaft this machine is mounted on (torque balance and speed).",
        ),
        q(
            "speed",
            "Fixed speed",
            Dim::Speed,
            "rpm",
            None,
            "Prescribed speed instead of a shaft, constant or [[t, speed], ...].",
        )
        .sched()
        .opt(),
        q(
            "ratio",
            "Gear ratio",
            Dim::None,
            "-",
            Some("1"),
            "Machine speed / shaft speed.",
        ),
    ]
}

impl ParamSpec {
    const fn sched(mut self) -> Self {
        self.schedulable = true;
        self
    }
    const fn opt(mut self) -> Self {
        self.optional = true;
        self
    }
}

fn fluid_param(optional: bool) -> ParamSpec {
    ParamSpec {
        key: "fluid",
        label: "Fluid",
        kind: PKind::Fluid,
        dim: Dim::None,
        default: None,
        optional,
        schedulable: false,
        choices: &[],
        help: "Working fluid (propagates through connected flow elements).",
        unit: "",
        target: "",
    }
}

fn init_params() -> Vec<ParamSpec> {
    vec![
        q(
            "P0",
            "Initial pressure",
            Dim::Pressure,
            "bar",
            None,
            "Initial pressure. Unset: taken from the nearest specified node.",
        )
        .opt(),
        q(
            "T0",
            "Initial temperature",
            Dim::Temperature,
            "K",
            None,
            "Initial temperature (single phase).",
        )
        .opt(),
        q(
            "x0",
            "Initial quality",
            Dim::None,
            "-",
            None,
            "Initial vapour quality (two-phase), instead of T0.",
        )
        .opt(),
    ]
}

fn area_params(cd_default: &'static str) -> Vec<ParamSpec> {
    vec![
        q(
            "diameter",
            "Diameter",
            Dim::Length,
            "mm",
            None,
            "Bore diameter (or give area).",
        )
        .opt(),
        q(
            "area",
            "Area",
            Dim::Area,
            "mm2",
            None,
            "Geometric flow area (instead of diameter).",
        )
        .opt(),
        q(
            "cd",
            "Discharge coefficient",
            Dim::None,
            "-",
            Some(cd_default),
            "Effective area = cd x area.",
        ),
    ]
}

fn restriction_model() -> ParamSpec {
    ParamSpec {
        key: "model",
        label: "Flow model",
        kind: PKind::Choice,
        dim: Dim::None,
        default: Some("auto"),
        optional: false,
        schedulable: false,
        choices: &["auto", "incompressible"],
        help: "auto: liquid with cavitation/flashing (omega), two-phase HEM (omega), real-gas isentropic nozzle. incompressible: sqrt(2 rho dP).",
        unit: "",
        target: "",
    }
}

pub fn specs() -> Vec<CompSpec> {
    let mut v = Vec::new();

    let mut p = vec![
        fluid_param(false),
        q(
            "P",
            "Pressure",
            Dim::Pressure,
            "bar",
            None,
            "Boundary pressure (constant or [[t, P], ...]).",
        )
        .sched(),
        q(
            "T",
            "Temperature",
            Dim::Temperature,
            "K",
            None,
            "Boundary temperature for inflow into the network.",
        )
        .sched()
        .opt(),
        q(
            "x",
            "Quality",
            Dim::None,
            "-",
            None,
            "Vapour quality for a saturated boundary, instead of T.",
        )
        .sched()
        .opt(),
    ];
    v.push(CompSpec {
        kind: "boundary",
        label: "Pressure boundary",
        category: "Boundaries",
        ports: &["p"],
        node_like: true,
        help: "Fixed (or scheduled) pressure and temperature. Supplies fluid at its own state; absorbs outflow at any state.",
        params: std::mem::take(&mut p),
    });

    let mut tank = vec![
        fluid_param(false),
        q(
            "volume",
            "Volume",
            Dim::Volume,
            "L",
            None,
            "Internal volume.",
        ),
        q(
            "Q",
            "Heat input",
            Dim::Power,
            "W",
            Some("0"),
            "Heat added to the contents.",
        )
        .sched(),
    ];
    tank.extend(init_params());
    tank[3].optional = false;
    tank.push(choice(
        "steady",
        "In a steady-state solve",
        &["hold", "solve"],
        "hold",
        "hold: the tank is a reservoir at its initial (or warm-start) state, as for a propellant or pressurant tank; solve: its state follows from the flow balance like a junction (a plenum between a regulator and a vent).",
    ));
    v.push(CompSpec {
        kind: "tank",
        label: "Tank / volume",
        category: "Volumes",
        ports: &["p"],
        node_like: true,
        help: "Lumped volume with mass and energy balance (homogeneous, equilibrium).",
        params: tank,
    });

    let mut j = vec![
        fluid_param(true),
        q(
            "volume",
            "Volume",
            Dim::Volume,
            "mL",
            Some("1 mL"),
            "Small lumped volume of the junction (adds to attached pipe volumes).",
        ),
    ];
    j.extend(init_params());
    v.push(CompSpec {
        kind: "junction",
        label: "Junction",
        category: "Volumes",
        ports: &["p"],
        node_like: true,
        help: "Connection point for several flow elements (tee, manifold).",
        params: j,
    });

    let mut pipe = vec![
        q("length", "Length", Dim::Length, "m", None, "Pipe length."),
        q(
            "diameter",
            "Inner diameter",
            Dim::Length,
            "mm",
            None,
            "Hydraulic diameter.",
        ),
        q(
            "roughness",
            "Roughness",
            Dim::Length,
            "um",
            Some("1.5 um"),
            "Absolute wall roughness.",
        ),
        ParamSpec {
            key: "segments",
            label: "Segments",
            kind: PKind::Int,
            dim: Dim::None,
            default: Some("4"),
            optional: false,
            schedulable: false,
            choices: &[],
            help: "Number of finite volumes along the pipe (more resolves pressure waves).",
            unit: "",
            target: "",
        },
        q(
            "dz",
            "Elevation change",
            Dim::Length,
            "m",
            Some("0"),
            "Outlet minus inlet height.",
        ),
        q(
            "K",
            "Fittings K",
            Dim::None,
            "-",
            Some("0"),
            "Sum of minor-loss coefficients distributed along the pipe.",
        ),
        q(
            "T_wall",
            "Wall temperature",
            Dim::Temperature,
            "K",
            None,
            "Heated/cooled wall at this temperature (Gnielinski convection).",
        )
        .sched()
        .opt(),
        q(
            "q_wall",
            "Wall heat flux",
            Dim::HeatFlux,
            "kW/m2",
            None,
            "Imposed wall heat flux into the fluid.",
        )
        .sched()
        .opt(),
        fluid_param(true),
    ];
    pipe.extend(init_params());
    pipe.extend([
        ParamSpec {
            key: "parallel",
            label: "Parallel channels",
            kind: PKind::Int,
            dim: Dim::None,
            default: Some("1"),
            optional: false,
            schedulable: false,
            choices: &[],
            help: "Number of identical channels in parallel (cooling jackets). Diameter is per channel.",
            unit: "",
            target: "",
        },
        q("wall_thickness", "Wall thickness", Dim::Length, "mm", None, "Enables a wall with heat capacity between hot side and coolant (hot-side wall / channel lands).").opt(),
        choice("wall_material", "Wall material", WALL_MATERIALS, "cucrzr", "Wall material (density, conductivity, Debye heat capacity)."),
        q("wall_mass_factor", "Wall mass factor", Dim::None, "-", Some("1"), "Multiplier on the wall mass (ribs, closeout, flanges)."),
        q("wall_T0", "Initial wall temperature", Dim::Temperature, "K", None, "Initial wall temperature (default: initial fluid temperature).").opt(),
        r("hot_chamber", "Hot gas: chamber", "combustion_chamber", "Combustion chamber that heats this wall (Bartz correlation on chamber gas properties)."),
        q("hot_area", "Hot-gas area", Dim::Area, "cm2", None, "Hot-gas-side area of the whole line (chamber and nozzle wall it cools).").opt(),
        q("hot_contraction", "Local area ratio A/At", Dim::None, "-", Some("3"), "Mean local flow area over throat area along the cooled length (Bartz), when no hot_contour is given."),
        q("hot_htc_factor", "Hot-side HTC factor", Dim::None, "-", Some("1"), "Multiplier on the Bartz coefficient."),
        q("hot_contour", "Area ratio along the line", Dim::None, "-", None, "Local A/At vs position along the line, [[x/L, A/At], ...] from inlet (0) to outlet (1); per wall element Bartz and hot area in proportion to the local radius. Overrides hot_contraction.").sched().opt(),
        choice("hot_driving", "Hot-gas driving potential", &["enthalpy", "temperature"], "enthalpy", "enthalpy: equilibrium enthalpy of the chamber gas minus that of the gas recombined at the wall (captures recombination heat); temperature: frozen cp (Tc - Tw)."),
        q("hot_T", "Hot gas temperature", Dim::Temperature, "K", None, "Prescribed hot-gas temperature instead of a chamber.").sched().opt(),
        q("hot_htc", "Hot gas HTC", Dim::Htc, "kW/m2/K", None, "Prescribed hot-side heat transfer coefficient (with hot_T).").sched().opt(),
        q("cool_correction", "Coolant property exponent", Dim::None, "-", Some("0"), "Applies (T_bulk/T_wall)^n to the coolant Nusselt number; 0.55 is common for supercritical hydrogen."),
        q("cool_area_factor", "Coolant area factor", Dim::None, "-", Some("1"), "Fin effectiveness times extra wetted area of channel walls."),
    ]);
    v.push(CompSpec {
        kind: "pipe",
        label: "Pipe",
        category: "Lines",
        ports: &["in", "out"],
        node_like: false,
        help: "Discretised line: staggered volumes and momentum (inertia) segments, Churchill friction, gravity.",
        params: pipe,
    });

    let mut orf = area_params("0.7");
    orf.push(q("recovery", "Pressure recovery", Dim::None, "-", Some("0"), "Fraction of the throat pressure drop recovered downstream (0 sharp orifice, ~0.8-0.9 venturi). Sets where cavitation / choking starts."));
    orf.push(restriction_model());
    let mut ven = area_params("0.95");
    ven.push(q(
        "recovery",
        "Pressure recovery",
        Dim::None,
        "-",
        Some("0.85"),
        "Fraction of the throat pressure drop recovered in the diffuser.",
    ));
    ven.push(restriction_model());
    v.push(CompSpec {
        kind: "orifice",
        label: "Orifice / venturi",
        category: "Restrictions",
        ports: &["in", "out"],
        node_like: false,
        help: "Fixed restriction. Handles liquid (with cavitation / flashing), two-phase and choked gas flow.",
        params: orf,
    });

    v.push(CompSpec {
        kind: "venturi",
        label: "Cavitating venturi",
        category: "Restrictions",
        ports: &["in", "out"],
        node_like: false,
        help: "Throat + diffuser. Chokes (cavitates) when the throat reaches vapour pressure; flow then independent of outlet pressure.",
        params: ven,
    });

    let mut valve = area_params("0.8");
    valve.extend([
        q(
            "Cv",
            "Cv",
            Dim::None,
            "-",
            None,
            "Flow coefficient (US), instead of area.",
        )
        .opt(),
        q(
            "Kv",
            "Kv",
            Dim::None,
            "-",
            None,
            "Flow coefficient (metric, m3/h at 1 bar), instead of area.",
        )
        .opt(),
        q(
            "opening",
            "Opening",
            Dim::None,
            "-",
            Some("1"),
            "Stem position 0..1, constant or [[t, u], ...].",
        )
        .sched(),
        ParamSpec {
            key: "characteristic",
            label: "Characteristic",
            kind: PKind::Choice,
            dim: Dim::None,
            default: Some("linear"),
            optional: false,
            schedulable: false,
            choices: &["linear", "equal_percentage", "quick_opening"],
            help: "Effective area vs stem position.",
            unit: "",
            target: "",
        },
        q(
            "leak",
            "Leakage",
            Dim::None,
            "-",
            Some("1e-6"),
            "Closed-position area fraction.",
        ),
        restriction_model(),
        ParamSpec {
            key: "sense",
            label: "Sensed pressure",
            kind: PKind::Text,
            dim: Dim::None,
            default: None,
            optional: true,
            schedulable: false,
            choices: &[],
            help: "Pressure-actuated valve: node whose pressure drives the opening (junction id, component.port, or a combustion chamber id for Pc). Opening then gives the initial position.",
            unit: "",
            target: "",
        },
        ParamSpec {
            key: "sense_ref",
            label: "Reference pressure",
            kind: PKind::Text,
            dim: Dim::None,
            default: None,
            optional: true,
            schedulable: false,
            choices: &[],
            help: "Optional second node: the signal is sense minus sense_ref (e.g. pump outlet minus inlet).",
            unit: "",
            target: "",
        },
        q("sense_lo", "Signal low", Dim::Pressure, "bar", Some("0"), "Sensed signal at which the actuator commands opening_lo."),
        q("sense_hi", "Signal high", Dim::Pressure, "bar", Some("1 bar"), "Sensed signal at which the actuator commands opening_hi (linear in between)."),
        q("opening_lo", "Opening at low signal", Dim::None, "-", Some("0"), "Commanded position at or below sense_lo."),
        q("opening_hi", "Opening at high signal", Dim::None, "-", Some("1"), "Commanded position at or above sense_hi (lower than opening_lo for a valve that closes on rising pressure)."),
        q("actuator_time", "Actuator time constant", Dim::Time, "ms", Some("50 ms"), "First-order lag of the actuator."),
        ParamSpec {
            key: "latch",
            label: "Latching",
            kind: PKind::Bool,
            dim: Dim::None,
            default: Some("false"),
            optional: false,
            schedulable: false,
            choices: &[],
            help: "In transients the actuator moves only towards opening_hi (one-shot sequencing valves).",
            unit: "",
            target: "",
        },
    ]);
    v.push(CompSpec {
        kind: "valve",
        label: "Valve",
        category: "Restrictions",
        ports: &["in", "out"],
        node_like: false,
        help: "Variable restriction with scheduled opening.",
        params: valve,
    });

    let mut cv = area_params("0.7");
    cv.extend([
        q(
            "cracking_dp",
            "Cracking pressure",
            Dim::Pressure,
            "bar",
            Some("0.2 bar"),
            "Differential pressure at which the valve starts to open.",
        ),
        q(
            "full_open_dp",
            "Full-open pressure",
            Dim::Pressure,
            "bar",
            Some("0.7 bar"),
            "Differential pressure at which the valve is fully open.",
        ),
        q(
            "leak",
            "Leakage",
            Dim::None,
            "-",
            Some("1e-6"),
            "Closed-position area fraction.",
        ),
        restriction_model(),
    ]);
    v.push(CompSpec {
        kind: "check_valve",
        label: "Check valve",
        category: "Restrictions",
        ports: &["in", "out"],
        node_like: false,
        help: "Opens with forward differential pressure, closes on reverse.",
        params: cv,
    });

    let mut reg = area_params("0.8");
    reg.extend([
        q(
            "set_pressure",
            "Set pressure",
            Dim::Pressure,
            "bar",
            None,
            "Regulated outlet pressure.",
        )
        .sched(),
        q(
            "band",
            "Proportional band",
            Dim::Pressure,
            "bar",
            Some("2 bar"),
            "Outlet pressure change from fully open to closed (droop).",
        ),
        q(
            "leak",
            "Leakage",
            Dim::None,
            "-",
            Some("1e-6"),
            "Closed-position area fraction.",
        ),
        restriction_model(),
    ]);
    v.push(CompSpec {
        kind: "regulator",
        label: "Pressure regulator",
        category: "Restrictions",
        ports: &["in", "out"],
        node_like: false,
        help: "Proportional pressure-reducing regulator sensing outlet pressure.",
        params: reg,
    });

    v.push(CompSpec {
        kind: "loss",
        label: "Fitting / loss",
        category: "Lines",
        ports: &["in", "out"],
        node_like: false,
        help: "Minor loss dP = K rho u^2 / 2 (elbow, tee, filter, ...).",
        params: vec![
            q(
                "K",
                "Loss coefficient K",
                Dim::None,
                "-",
                Some("0.3"),
                "Elbow 90° ~0.3, tee branch ~1.0, tee run ~0.2, open ball valve ~0.05.",
            ),
            q(
                "diameter",
                "Diameter",
                Dim::Length,
                "mm",
                None,
                "Reference flow diameter.",
            )
            .opt(),
            q(
                "area",
                "Area",
                Dim::Area,
                "mm2",
                None,
                "Reference flow area (instead of diameter).",
            )
            .opt(),
        ],
    });

    v.push(CompSpec {
        kind: "mass_flow",
        label: "Mass flow source",
        category: "Boundaries",
        ports: &["in", "out"],
        node_like: false,
        help: "Imposed mass flow from 'in' to 'out' (ideal pump / flow controller).",
        params: vec![q(
            "mdot",
            "Mass flow",
            Dim::MassFlow,
            "g/s",
            None,
            "Imposed mass flow, constant or [[t, mdot], ...].",
        )
        .sched()],
    });

    v.push(CompSpec {
        kind: "shaft",
        label: "Shaft",
        category: "Machines",
        ports: &[],
        node_like: false,
        help: "Rotor with inertia. Pumps and turbines that name it share its speed; J dw/dt = sum of turbine torques - pump torques - losses.",
        params: vec![
            q("inertia", "Inertia", Dim::Inertia, "kg m2", None, "Polar moment of inertia of the rotating assembly."),
            q("N0", "Initial speed", Dim::Speed, "rpm", Some("0 rpm"), "Speed at t = 0."),
            q("bearing_torque", "Bearing torque", Dim::Torque, "N m", Some("0"), "Speed-independent friction torque (smoothed through zero speed)."),
            q("viscous_loss", "Viscous loss", Dim::None, "N m s", Some("0"), "Loss torque per unit speed."),
            q("speed", "Fixed speed", Dim::Speed, "rpm", None, "Prescribe the speed (motor drive) instead of solving the torque balance.").sched().opt(),
        ],
    });

    let mut pump = drive_params();
    pump.extend([
        q(
            "design_speed",
            "Design speed",
            Dim::Speed,
            "rpm",
            None,
            "Speed of the design point.",
        ),
        q(
            "design_mdot",
            "Design mass flow",
            Dim::MassFlow,
            "kg/s",
            None,
            "Mass flow at the design point.",
        ),
        q(
            "design_dp",
            "Design pressure rise",
            Dim::Pressure,
            "bar",
            None,
            "Pressure rise at the design point.",
        ),
        q(
            "design_eff",
            "Design efficiency",
            Dim::None,
            "-",
            Some("0.65"),
            "Efficiency at the design point (all losses heat the fluid).",
        ),
        q(
            "design_density",
            "Design density",
            Dim::Density,
            "kg/m3",
            None,
            "Inlet density of the design point (default: initial inlet state).",
        )
        .opt(),
        q(
            "shutoff_head",
            "Shutoff head ratio",
            Dim::None,
            "-",
            Some("1.15"),
            "Head at zero flow over design head, same speed.",
        ),
        q(
            "shutoff_power",
            "Shutoff power ratio",
            Dim::None,
            "-",
            Some("0.45"),
            "Shaft power at zero flow over design power, same speed.",
        ),
        q("head_curve", "Head map", Dim::None, "-", None, "Measured head map [[phi, psi], ...]: psi = (H/H_d)/(N/N_d)^2 vs phi = (Q/Q_d)/(N/N_d); replaces the generic curve above 20 % speed.").sched().opt(),
        q("eff_curve", "Efficiency map", Dim::None, "-", None, "Measured efficiency map [[phi, eta/eta_d], ...] vs the same phi; sets the shaft power map (anchored at shutoff_power at zero flow).").sched().opt(),
        q("speed_eff_curve", "Efficiency speed correction", Dim::None, "-", None, "Efficiency multiplier vs speed ratio [[N/N_d, factor], ...] (1 at N_d); divides the shaft torque above 20 % speed, faded in from 5 %.").sched().opt(),
        q(
            "inertance",
            "Fluid inertance L/A",
            Dim::None,
            "1/m",
            Some("2000"),
            "Inertia of the fluid inside the pump.",
        ),
    ]);
    v.push(CompSpec {
        kind: "pump",
        label: "Pump",
        category: "Machines",
        ports: &["in", "out"],
        node_like: false,
        help: "Centrifugal pump with affinity-law head and power maps scaled from one design point; works in all flow and speed ranges.",
        params: pump,
    });

    let mut turb = drive_params();
    turb.extend(area_params("0.95"));
    turb.extend([
        q(
            "mean_diameter",
            "Rotor mean diameter",
            Dim::Length,
            "mm",
            None,
            "Blade mean diameter (sets blade speed u = w D/2).",
        ),
        q(
            "design_eff",
            "Peak efficiency",
            Dim::None,
            "-",
            Some("0.6"),
            "Total-to-static efficiency at the design velocity ratio.",
        ),
        q(
            "design_velocity_ratio",
            "Design velocity ratio u/c0",
            Dim::None,
            "-",
            Some("0.35"),
            "Blade speed over spouting velocity at peak efficiency.",
        ),
        q("eff_curve", "Efficiency map", Dim::None, "-", None, "Measured efficiency map [[u/c0, eta/eta_d], ...] (eta = 0 at u = 0 is added); replaces the parabolic curve.").sched().opt(),
        q("area_map", "Flow area map", Dim::None, "-", None, "Effective nozzle area factor vs velocity ratio and pressure ratio, [[PR, u/c0, factor], ...]; multiplies the nozzle area (bilinear, held at the table edges).").sched().opt(),
    ]);
    if let Some(p) = turb.iter_mut().find(|p| p.key == "diameter") {
        p.label = "Nozzle throat diameter";
        p.help = "Equivalent diameter of the total stator nozzle throat area (or give area).";
    }
    v.push(CompSpec {
        kind: "turbine",
        label: "Turbine",
        category: "Machines",
        ports: &["in", "out"],
        node_like: false,
        help: "Gas turbine: nozzle flow (choked or subsonic, real gas) and impulse-stage torque, linear in blade speed, from stall to runaway.",
        params: turb,
    });

    v.push(CompSpec {
        kind: "combustion_chamber",
        label: "Combustion chamber",
        category: "Engine",
        ports: &["fuel", "ox"],
        node_like: false,
        help: "Thrust chamber: equilibrium combustion tables (Cantera), chamber filling dynamics, choked throat, nozzle thrust and Isp.",
        params: vec![
            q("throat_diameter", "Throat diameter", Dim::Length, "mm", None, "Nozzle throat diameter (or give throat_area).").opt(),
            q("throat_area", "Throat area", Dim::Area, "cm2", None, "Nozzle throat area.").opt(),
            q("volume", "Chamber volume", Dim::Volume, "L", None, "Volume up to the throat (default L* x At).").opt(),
            q("l_star", "Characteristic length L*", Dim::Length, "m", Some("0.9 m"), "Used for the volume when none is given."),
            q("area_ratio", "Nozzle area ratio", Dim::None, "-", Some("40"), "Exit over throat area."),
            q("p_ambient", "Ambient pressure", Dim::Pressure, "bar", Some("0"), "Back pressure on the nozzle (0 for vacuum).").sched(),
            q("cstar_eff", "c* efficiency", Dim::None, "-", Some("0.98"), "Combustion efficiency on c*."),
            q("cf_eff", "Nozzle efficiency", Dim::None, "-", Some("0.98"), "Efficiency on the thrust coefficient (divergence, friction)."),
            q("ignition_time", "Ignition time", Dim::Time, "s", None, "Combustion starts here (default: burning from t = 0). Before, the chamber passes cold propellant gas.").opt(),
            q("ignition_ramp", "Ignition ramp", Dim::Time, "ms", Some("20 ms"), "Duration of the transition to full combustion."),
            q("P0", "Initial pressure", Dim::Pressure, "bar", Some("1 bar"), "Chamber pressure at t = 0."),
        ],
    });

    v
}

pub fn spec(kind: &str) -> Option<CompSpec> {
    specs().into_iter().find(|s| s.kind == kind)
}

/// JSON catalog for the UI.
pub fn catalog_json() -> Value {
    let comps: Vec<Value> = specs()
        .iter()
        .map(|c| {
            json!({
                "type": c.kind, "label": c.label, "category": c.category, "ports": c.ports,
                "node_like": c.node_like, "help": c.help,
                "params": c.params.iter().map(|p| json!({
                    "key": p.key, "label": p.label,
                    "kind": match p.kind { PKind::Quantity => "quantity", PKind::Fluid => "fluid", PKind::Choice => "choice", PKind::Int => "int", PKind::Bool => "bool", PKind::Ref => "ref", PKind::Text => "text" },
                    "target": p.target,
                    "dim": format!("{:?}", p.dim), "si": p.dim.si_unit(), "unit": p.unit,
                    "default": p.default, "optional": p.optional || p.default.is_some(),
                    "required": !p.optional && p.default.is_none(),
                    "schedulable": p.schedulable, "choices": p.choices, "help": p.help,
                })).collect::<Vec<_>>()
            })
        })
        .collect();
    json!({ "components": comps })
}

/// Typed parameter access for one component instance, with defaults from the catalog.
pub struct Params<'a> {
    pub id: &'a str,
    pub spec: CompSpec,
    pub map: &'a Map<String, Value>,
}

const IGNORED_KEYS: &[&str] = &["ui", "label", "notes", "x", "y", "position"];

impl<'a> Params<'a> {
    pub fn new(id: &'a str, kind: &str, map: &'a Map<String, Value>) -> Result<Self> {
        let spec = spec(kind)
            .ok_or_else(|| Error::Model(format!("{id}: unknown component type '{kind}'")))?;
        Ok(Params { id, spec, map })
    }

    pub fn unknown_keys(&self) -> Vec<String> {
        self.map
            .keys()
            .filter(|k| {
                !IGNORED_KEYS.contains(&k.as_str())
                    && !self.spec.params.iter().any(|p| p.key == k.as_str())
            })
            .cloned()
            .collect()
    }

    /// The parameter name closest to an unknown key (edit distance), if close.
    pub fn suggest(&self, key: &str) -> Option<&'static str> {
        let k = key.to_ascii_lowercase();
        self.spec
            .params
            .iter()
            .map(|p| (edit_distance(&k, &p.key.to_ascii_lowercase()), p.key))
            .filter(|(d, name)| *d <= 2.max(name.len() / 3))
            .min_by_key(|(d, _)| *d)
            .map(|(_, name)| name)
    }

    fn pspec(&self, key: &str) -> &ParamSpec {
        self.spec
            .params
            .iter()
            .find(|p| p.key == key)
            .unwrap_or_else(|| {
                panic!(
                    "internal: parameter {key} not in catalog for {}",
                    self.spec.kind
                )
            })
    }

    pub fn raw(&self, key: &str) -> Option<Value> {
        match self.map.get(key) {
            Some(Value::Null) | None => self
                .pspec(key)
                .default
                .map(|d| Value::String(d.to_string())),
            Some(Value::String(s)) if s.trim().is_empty() => self
                .pspec(key)
                .default
                .map(|d| Value::String(d.to_string())),
            Some(v) => Some(v.clone()),
        }
    }

    pub fn has(&self, key: &str) -> bool {
        matches!(self.map.get(key), Some(v) if !v.is_null() && v.as_str().map(|s| !s.trim().is_empty()).unwrap_or(true))
    }

    pub fn q_opt(&self, key: &str) -> Result<Option<f64>> {
        let ps = self.pspec(key);
        match self.raw(key) {
            None => Ok(None),
            Some(v) => parse_value(&v, ps.dim)
                .map(Some)
                .map_err(|e| Error::Model(format!("{}.{key}: {e}", self.id))),
        }
    }

    pub fn q(&self, key: &str) -> Result<f64> {
        self.q_opt(key)?
            .ok_or_else(|| Error::Model(format!("{}: missing required parameter '{key}'", self.id)))
    }

    pub fn sched_opt(&self, key: &str) -> Result<Option<Schedule>> {
        let ps = self.pspec(key);
        match self.raw(key) {
            None => Ok(None),
            Some(v) => Schedule::parse(&v, ps.dim)
                .map(Some)
                .map_err(|e| Error::Model(format!("{}.{key}: {e}", self.id))),
        }
    }

    pub fn sched(&self, key: &str) -> Result<Schedule> {
        self.sched_opt(key)?
            .ok_or_else(|| Error::Model(format!("{}: missing required parameter '{key}'", self.id)))
    }

    pub fn str_opt(&self, key: &str) -> Option<String> {
        self.raw(key)
            .and_then(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
    }

    pub fn int(&self, key: &str) -> Result<i64> {
        match self.raw(key) {
            Some(Value::Number(n)) => n
                .as_i64()
                .or_else(|| n.as_f64().map(|f| f.round() as i64))
                .ok_or_else(|| Error::Model(format!("{}.{key}: not an integer", self.id))),
            Some(Value::String(s)) => s
                .trim()
                .parse::<f64>()
                .map(|f| f.round() as i64)
                .map_err(|_| Error::Model(format!("{}.{key}: not an integer", self.id))),
            _ => Err(Error::Model(format!("{}: missing '{key}'", self.id))),
        }
    }

    /// Effective area cd*A from diameter/area (and Cv/Kv where offered).
    pub fn cda(&self) -> Result<f64> {
        let has = |k: &str| self.spec.params.iter().any(|p| p.key == k) && self.has(k);
        if has("Cv") {
            return Ok(self.q("Cv")? * 1.699e-5);
        }
        if has("Kv") {
            return Ok(self.q("Kv")? * 1.9642e-5);
        }
        let a = self.area()?;
        Ok(a * self.q("cd")?)
    }

    pub fn area(&self) -> Result<f64> {
        if self.has("area") {
            return self.q("area");
        }
        if self.has("diameter") {
            let d = self.q("diameter")?;
            return Ok(std::f64::consts::PI * d * d / 4.0);
        }
        Err(Error::Model(format!(
            "{}: give 'diameter' or 'area'",
            self.id
        )))
    }
}

/// Levenshtein distance (for "did you mean" hints).
pub fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != *cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}
