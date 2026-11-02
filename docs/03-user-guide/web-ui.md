# Web user interface

FNT-USR-HBK-005 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Purpose

This handbook describes the fluidnet web user interface: a schematic editor in
the browser in which a model is drawn from components, parameterised in an
inspector, run as a steady state or a transient, and evaluated in charts, line
profiles and a values table. The solver runs locally in the browser as
WebAssembly; no server computes anything. The interface reads and writes the
same YAML model files as the command-line tool and the Python package.

The source is in `ui/src/`: `App.tsx` (layout, schematic, file handling),
`Inspector.tsx` (parameter editor), `Results.tsx` (charts, values, profiles),
`CompNode.tsx` and `symbols.tsx` (component symbols and ports), `model.ts`
(model format and YAML), `engine.ts` and `worker.ts` (engine in a web worker).

## Prerequisites

- A built site (`ui/dist`) served over HTTP, or the development server, as in
  [installation, step 5](getting-started.md#5-build-the-web-user-interface).
- A current desktop browser with WebAssembly, Web Workers and
  `DecompressionStream` (used to unpack the fluid tables).

## Where it runs

| Form | How it is built | How it is used |
|---|---|---|
| Development server | `cd ui && npm run dev` | http://localhost:5173, reloads on source changes |
| Static site | `npm run build` → `ui/dist` (`index.html`, `app.js`, `app.css`, `fluidnet.wasm`, `fluids/*.txt`) | Copy the directory to any static web server; all URLs are relative |
| Single-page artifact | `npm run artifact [OUT_DIR]` after `npm run build` → `ui/artifact` | One HTML file without a document wrapper (`fluidnet.html`, with `app.js` and `app.css` inlined) plus `fluidnet.wasm` and `fluids/*.txt` next to it, for hosts that serve a page fragment with supporting files |

The artifact script is `ui/scripts/make-artifact.mjs`; it reads `ui/dist` and
`ui/public`, so run `npm run wasm` and `npm run build` first. Both the site and
the artifact load the IBM Plex fonts from Google Fonts; without network access
the browser falls back to its default fonts and everything else works.

On start the page fetches `fluidnet.wasm`, starts it in a web worker and reads
the component catalog from it. Fluid tables are fetched only when a model
uses the fluid (`fluids/<FLUID>.txt`, gzip-compressed base64 text, unpacked in
the browser).

## Layout

| Area | Content |
|---|---|
| Top bar | Model name (editable), Examples menu, New, Open, YAML, the model status, and the Steady state and Run transient buttons (Stop and a progress bar while running) |
| Left: palette | Component types grouped by category (Boundaries, Volumes, Lines, Restrictions, Machines, Engine), generated from the catalog |
| Centre: canvas | The schematic; lines are coloured by fluid, with a fluid legend in the corner |
| Right: inspector | Parameters of the selected component, or the system and run settings when nothing is selected |
| Bottom: results | Plots, values table and line profile; drag its top edge to resize; a status line with solve statistics and the warning count |

The model is saved automatically in the browser's local storage after every
change and restored when the page is opened again. On the first visit the
closed-expander throttling example is loaded. Whenever a model is loaded at
start-up or from the Examples menu, a transient run starts automatically.

## Steps

### 1. Start a model

- **Examples** loads one of ten example models from `examples/` (RL10A-3-3A
  test point 1 and start, closed expander throttling and start, LOX feed
  startup, LH2 heated channel, regulated tank pressurisation, GN2 bottle
  blowdown, water hammer, cavitating venturi) and runs it.
- **New** starts an empty model.
- **Open** reads a `.yaml`, `.yml` or `.json` model file from disk.

### 2. Add components

Drag a component from the palette onto the canvas, or click it to place it in
the middle of the view. New components get an id from their type (`valve`,
`valve2`, ...; `check` for check valves, `flow` for mass-flow sources) and
starting values for their required parameters (for example a pipe of 1 m length
and 10 mm diameter, a valve of 8 mm diameter). New tanks and boundaries take
the fluid already used in the model, or N2.

Select a component by clicking it. **Delete** or **Backspace** removes the
selected component with its connections; the inspector also has a Delete
button. **Flip** in the inspector mirrors a two-port symbol so that `in` is on
the right.

### 3. Connect ports

Drag from one port to another. Two-port components have `in` on the left and
`out` on the right (after a flip, the reverse); tanks and junctions have a port
on each side and boundaries on the left and right, all of which are the same
fluid port `p`. A combustion chamber has two ports on its left side, `fuel`
(upper) and `ox` (lower). Each connection is stored as a pair such as
`[feed_line.out, main_valve.in]`; duplicates are ignored. Select a connection
and press Delete to remove it.

Mechanical and thermal couplings are not drawn by hand. A pump or turbine is
put on a shaft by choosing the shaft in its `shaft` parameter, and a cooled
pipe is heated by a chamber through its `hot_chamber` parameter; the schematic
then shows a dashed link. Shafts cannot be connected to fluid ports.

The topology rules of the [model file standard](model-files.md#25-ports-and-connections)
apply: two tanks, junctions or boundaries must not be connected directly, and
every port of a flow element must be connected.

### 4. Set parameters in the inspector

The inspector is generated from the component catalog, so it always shows
every parameter of the selected type with its label, its display unit, a help
line, and a red asterisk for required parameters. Empty optional fields show
the default as a placeholder.

| Parameter kind | Input |
|---|---|
| Quantity | Text field. Type a number (in SI units) or a number with a unit: `300 bar`, `20 degC`, `6 mm`, `50 L` ([units](model-files.md#27-quantities-and-units)). The value is applied on Enter or when the field loses focus; Escape restores it. An empty field removes the value (default applies). |
| Fluid | Selection of the eight fluids; on junctions and pipes "From connected parts" leaves it unset. |
| Choice | Selection of the allowed values (valve characteristic, flow model, wall material, ...). |
| Reference | Selection of the components of the required type (shafts for `shaft`, chambers for `hot_chamber`). |
| Schedulable quantity | The **f(t)** button converts the value into a time table, shown as a text area with one `t, value` pair per line (units allowed, for example `100 ms, 50 %`); the **const** button converts back to the last value. |
| Table parameter (curves, `area_map`) | Text area with one row per line, values separated by commas; any number of columns. |

The id of the selected component is edited in the field under its type name;
characters other than letters, digits, `_` and `-` are replaced by `_`, and the
connections are renamed with it.

With nothing selected, the inspector shows the system settings: the
description; the run settings end time (`t_end`), output interval
(`output_dt`, placeholder `t_end`/500), relative tolerance (`rtol`, 1e-2 to
1e-6), maximum step (`max_step`), "Start from the steady state at t = 0"
(`start_from_steady`) and "Steady state evaluated at" (`steady_time`); and the
model check (number of states, volumes and boundaries, flow elements, compile
warnings or the compile error). `atol_scale` and `max_steps` are not shown but
are kept when present in a loaded file.

### 5. Check the model

The model is compiled in the background 250 ms after every change (moving a
symbol does not trigger it). The status in the top bar shows the number of
states, or "Model has an error" with the message as a tooltip; the message is
also shown in the inspector's model check, and the component named in it is
marked on the canvas. The run buttons are disabled while the model has an error.

### 6. Run

| Action | Button | Keyboard |
|---|---|---|
| Transient (the `simulation` settings) | Run transient | Ctrl+Enter (Cmd+Enter on macOS) |
| Steady state at `steady_time` | Steady state | Shift+Enter (not inside a text area) |
| Stop a running solve | Stop | – |

While a transient runs, the progress bar shows simulated time over `t_end`.
**Stop** terminates the web worker and starts a new one; the run is discarded.
If a transient stops early on its own (solver failure), the data up to that
time is shown with the banner "Stopped early: ..." and the message. Other
errors (for example a steady state that does not converge) appear as a banner
above the results.

### 7. Read the results

The results area has up to three tabs.

**Plots** (transient only). On the left, the channel browser lists all channels
grouped by element, in component order; type in the filter field to search,
click a channel to add or remove it from the plot. The internal volumes and
segments of pipes are hidden until "Show line internals" is pressed. After a
run, a default selection is plotted (chamber pressure, thrust and O/F; shaft
speeds; tank and junction pressures; flows; jacket outlet and wall
temperatures), and a selection made earlier is kept if the channels still
exist. Channels with the same display unit share one chart; charts are stacked.

- The cursors of all charts are synchronised: moving over one chart shows the
  values of all charts at that time in their legends.
- Drag horizontally to zoom into a time range; double-click to zoom out.
- Click in a chart to fix a time (dashed line). The schematic overlays and the
  values table then show the values at that time; **end** next to the time
  display returns to the last output time.

Display units: pressure in bar, enthalpy in kJ/kg, power in kW, rotational
speed in rpm, force in kN; all other channels in their SI units.

**Values** (for a steady state: **Steady state**). One row per component with
inlet and outlet pressure and temperature, inlet density, quality where the
fluid is two-phase, and mass flow, plus labels for the main results of the
type: choked, valve opening, tank mass, heat load and wall temperature, pump
pressure rise, efficiency, power and NPSH, turbine power, efficiency, pressure
ratio and velocity ratio, shaft speed, and chamber pressure, O/F, thrust,
specific impulse, c* and temperature.

**Line profile** (models with discretised pipes). Pressure and temperature
along a selected pipe, from the `in` node through the internal volumes to the
`out` node, at the fixed time.

The schematic itself shows the main values next to each symbol at the fixed
time (or the last time): pressure, temperature and quality on volumes, mass
flow and pressure drop on flow elements, pump pressure rise and efficiency,
turbine power and pressure ratio, shaft speed in rpm, chamber pressure, O/F,
thrust and specific impulse, and "choked" where it applies.

The status line under the results gives the simulated time, solve time,
number of states and steps (or the steady solution method), and the number of
compile warnings.

### 8. Open, save and exchange model files

- **YAML** opens a dialog with the model as YAML text, written in the compact
  style of the example files (one line per component). The text can be edited
  and applied with **Apply**, copied with **Copy**, or written to a file with
  **Save as…**. Save as… appears only in browsers that offer the File System
  Access API (Chromium-based browsers) and when the page is not embedded in a
  frame; elsewhere use Copy and paste into a file.
- **Open** reads a model file.

The file written by the interface stores the symbol position of each component
as `ui: {x, y}` (and `flip: true` if flipped); the solver ignores the `ui` key.
Models written by hand without `ui` are laid out on a grid when opened. Comments
in a YAML file are not kept when it is applied or saved from the interface.

## Troubleshooting

| Symptom | Cause and remedy |
|---|---|
| "Engine failed to load" / "The simulation engine could not start: cannot load fluidnet.wasm (404)" | The engine was not copied (`npm run wasm`) or the site is opened from the file system instead of a web server. |
| "cannot load fluid table X (404)" | `ui/public/fluids` (or `dist/fluids`) is missing; run `npm run wasm` and rebuild. |
| A fluid is not offered | The interface offers the eight shipped fluids; a newly generated table also needs an entry in `FLUIDS` in `ui/src/engine.ts`. |
| A schedule shows `[object Object]` | The parameter uses the mapping form `{table: ..., interp: step}`, which the inspector cannot edit. Edit it in the YAML dialog. |
| Run buttons disabled | The model has a compile error; read the model check in the inspector. |
| A run takes long and the page shows "Solving…" | Large start transients take seconds natively and about twice as long in the browser ([performance](modelling-practice.md#7-performance)); use Stop to abort. |
| The model from the last session is gone | Local storage was cleared or the page is opened from a different address (storage is per origin). Keep models as files. |
| Keyboard shortcuts do nothing | Shift+Enter is ignored inside text areas; click on the canvas first. |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
