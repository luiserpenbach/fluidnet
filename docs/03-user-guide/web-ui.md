# Web user interface

FNT-USR-HBK-005 · Rev B · Released · 2026-11-18 · Owner: Luis Erpenbach

## Purpose

This handbook describes the fluidnet web user interface: a schematic editor in
the browser in which a model is drawn from components, parameterised in an
inspector, run as a steady state or a transient, and evaluated in charts, line
profiles and a values table. The solver runs locally in the browser as
WebAssembly; no server computes anything. The interface reads and writes the
same YAML model files as the command-line tool and the Python package.

The source is in `ui/src/`: `App.tsx` (layout, schematic, selection, keyboard
and clipboard), `graph.ts` (operations on the model: delete, rename, copy and
paste, insertion into a line, alignment, symbol geometry), `history.ts` (undo
and redo), `Inspector.tsx` (parameter editor), `Results.tsx` (charts, values,
profiles, messages), `Study.tsx` (parameter study), `Menus.tsx` (context menu,
command palette, shortcut list), `files.ts` (saving and CSV export),
`CompNode.tsx` and `symbols.tsx` (component symbols and ports), `model.ts`
(model format and YAML), `engine.ts` and `worker.ts` (engine in a web worker).
`graph.ts` has unit tests in `ui/src/__tests__` (`npm test`).

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
| Top bar | Model name (editable, with "edited" next to it while there are changes since the last save), Examples menu, New, Open, Save, YAML, undo, redo and find, the model status, the Study, Steady state and Run transient buttons (Stop and a progress bar while running), the light/dark theme switch and the shortcut list |
| Left: palette | Component types grouped by category (Boundaries, Volumes, Lines, Restrictions, Machines, Engine), generated from the catalog, with a filter field |
| Centre: canvas | The schematic; lines are coloured by fluid, with a fluid legend in the corner. A row of switches at the top left shows or hides the palette, the inspector and the results, the values on the schematic, grid snapping (10 px) and an overview map |
| Right: inspector | Parameters and results of the selected component, the actions for a multiple selection, or the system and run settings when nothing is selected |
| Bottom: results | Plots, values table, line profile and messages; drag its top edge to resize; a status line with solve statistics and the warning count |

The model is saved automatically in the browser's local storage after every
change and restored when the page is opened again. On the first visit the
closed-expander throttling example is loaded. Whenever a model is loaded at
start-up or from the Examples menu, a transient run starts automatically. The
theme and the canvas switches are remembered in the same way.

Every change to the model can be undone (up to 200 steps, not kept across a
page reload). Loading an example, New, Open and applying YAML are steps too, so
undo brings the previous model back. Typing in the name or description, and
repeated arrow-key moves, each count as one step.

## Steps

### 1. Start a model

- **Examples** loads one of ten example models from `examples/` (RL10A-3-3A
  test point 1 and start, closed expander throttling and start, LOX feed
  startup, LH2 heated channel, regulated tank pressurisation, GN2 bottle
  blowdown, water hammer, cavitating venturi) and runs it.
- **New** starts an empty model.
- **Open** reads a `.yaml`, `.yml` or `.json` model file from disk. A model file
  can also be dropped onto the canvas.

### 2. Add components

Drag a component from the palette onto the canvas, or click it to place it in
the middle of the view. New components get an id from their type (`valve`,
`valve2`, ...; `check` for check valves, `flow` for mass-flow sources) and
starting values for their required parameters (for example a pipe of 1 m length
and 10 mm diameter, a valve of 8 mm diameter). New tanks and boundaries take
the fluid already used in the model, or N2.

Dropping a two-port component (pipe, valve, orifice, pump, ...) onto an
existing line inserts it into that line: `a – b` becomes `a – part.in`,
`part.out – b`, with `in` on the upstream side. The same is offered by
right-clicking a line (**Insert into this line**). A junction or tank can be
inserted only into a line between two flow elements, because two volumes must
not be connected directly. Right-clicking the empty canvas adds a component at
that position (**Add**).

Select a component by clicking it; Shift+click (or Ctrl/Cmd+click) adds to the
selection, Shift+drag on the canvas selects with a box, and Ctrl+A selects
everything. The selection can be moved together by dragging or with the arrow
keys. For the selection:

| Action | Where | Keyboard |
|---|---|---|
| Delete, with the connections of the deleted components | Inspector, right-click menu | Delete or Backspace |
| Duplicate, with the connections between the duplicated components | Inspector, right-click menu | Ctrl+D |
| Copy, cut, paste (at the pointer) | Right-click menu | Ctrl+C, Ctrl+X, Ctrl+V |
| Rotate by a quarter turn | Inspector, right-click menu | R |
| Flip (mirror, so that `in` is on the other side) | Inspector, right-click menu | F |
| Align edges or centres, space evenly (two or more, three or more) | Inspector, right-click menu **Align** | – |

Copies get new ids (`valve` → `valve_2` → `valve_3`). References inside the
copied group (`shaft`, `hot_chamber`, `sense`, `sense_ref`) point to the copies;
references to components outside it are kept. The clipboard holds the copied
components as a YAML fragment, so they can be pasted into another browser tab
or a text editor, and such a fragment can be pasted from one. Deleting a
component also clears the references other components have to it.

### 3. Connect ports

Drag from one port to another. Two-port components have `in` on the left and
`out` on the right (after a flip, the reverse); tanks and junctions have a port
on each side and boundaries on the left and right, all of which are the same
fluid port `p`. A combustion chamber has two ports on its left side, `fuel`
(upper) and `ox` (lower). Each connection is stored as a pair such as
`[feed_line.out, main_valve.in]`; duplicates and direct connections between two
volumes are refused with a note. Click a line to select it (it is highlighted)
and press Delete to remove it, or right-click it and choose **Delete
connection**. Lines are included in box selections and in Ctrl+A. After
rotating a symbol its ports turn with it.

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
| Yes/no | Check box. |
| Schedulable quantity | The **f(t)** button converts the value into a time table, shown as a text area with one `t, value` pair per line (units allowed, for example `100 ms, 50 %`); the **const** button converts back to the last value. A small curve under a two-column table shows its shape. |
| Table parameter (curves, `area_map`) | Text area with one row per line, values separated by commas; any number of columns. |

The id of the selected component is edited in the field under its type name;
characters other than letters, digits, `_` and `-` are replaced by `_`, and the
connections and the references of other components (`shaft`, `hot_chamber`,
`sense`, `sense_ref`) are renamed with it. An id that is already in use is
refused.

When results exist, the inspector lists under the parameters the values of the
selected component at the fixed time: pressure and temperature at its ports and
all of its channels. Clicking a row adds the channel to the plots or removes
it.

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
also shown in the inspector's model check and in the inspector of the component
named in it, which is marked on the canvas. Clicking the status selects that
component and moves the view to it. The run buttons are disabled while the
model has an error.

### 6. Run

| Action | Button | Keyboard |
|---|---|---|
| Transient (the `simulation` settings) | Run transient | Ctrl+Enter (Cmd+Enter on macOS) |
| Steady state at `steady_time` | Steady state | Shift+Enter (not inside a text area) |
| Parameter study (steady states over a range of one parameter) | Study | – |
| Stop a running solve | Stop | – |

While a transient runs, the progress bar shows simulated time over `t_end`.
**Stop** terminates the web worker and starts a new one; the run is discarded.
If a transient stops early on its own (solver failure), the data up to that
time is shown with the banner "Stopped early: ..." and the message. Other
errors (for example a steady state that does not converge) appear as a banner
above the results.

**Study** asks for a component, one of its quantity parameters, a range
(`From`, `To`, with a unit, for example `10 bar` to `30 bar`) and a number of
points (2 to 200), and solves the steady state at each value. The model itself
is not changed. The result is shown like a transient with the parameter in
place of time: the plots have the parameter on the x axis, and the slider and
the fixed point step the schematic and the values table through the points.
Points without a solution are left out of the curves and listed under
Messages.

### 7. Read the results

When the model is changed after a run, the results stay and are marked "out of
date" until the next run. The results area has up to four tabs.

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
- Click in a chart to fix a time (dashed line), or drag the slider above the
  charts. The schematic overlays, the inspector and the values table then show
  the values at that time; **end** next to the time display returns to the last
  output time. **play** (key P) runs the fixed time through the result.
- **keep as reference** keeps the current run; the next runs are then plotted
  with the reference as dashed curves of the same colour, to compare a change
  against it. **clear reference** drops it.
- **CSV** downloads all channels of the result in display units.

Display units: pressure in bar, enthalpy in kJ/kg, power in kW, rotational
speed in rpm, force in kN; all other channels in their SI units.

**Values** (for a steady state: **Steady state**). One row per component with
inlet and outlet pressure and temperature, inlet density, quality where the
fluid is two-phase, and mass flow, plus labels for the main results of the
type: choked, valve opening, tank mass, heat load and wall temperature, pump
pressure rise, efficiency, power and NPSH, turbine power, efficiency, pressure
ratio and velocity ratio, shaft speed, and chamber pressure, O/F, thrust,
specific impulse, c* and temperature. The field in the first column header
filters the rows; clicking a row selects the component and moves the schematic
to it.

**Line profile** (models with discretised pipes). Pressure and temperature
along a selected pipe, from the `in` node through the internal volumes to the
`out` node, at the fixed time.

**Messages**. The warnings of the run and of the model check, the error of a
failed run, and the solver statistics. The warning count in the status line
opens it.

The schematic itself shows the main values next to each symbol at the fixed
time (or the last time): pressure, temperature and quality on volumes, mass
flow and pressure drop on flow elements, pump pressure rise and efficiency,
turbine power and pressure ratio, shaft speed in rpm, chamber pressure, O/F,
thrust and specific impulse, and "choked" where it applies. Arrowheads on the
lines show the direction of flow at that time. The **Values** switch on the
canvas hides both.

The status line under the results gives the simulated time, solve time,
number of states and steps (or the steady solution method), and the number of
compile warnings.

### 8. Open, save and exchange model files

- **Save** (Ctrl+S) writes the model as a YAML file. In browsers with the File
  System Access API (Chromium-based browsers, page not embedded in a frame) it
  asks for a file once and then writes to the same file, also to a file opened
  with Open; Ctrl+Shift+S asks for a new file. Elsewhere it downloads
  `<model name>.yaml`.
- **Open** (Ctrl+O) reads a model file; so does dropping a file on the canvas.
- **YAML** opens a dialog with the model as YAML text, written in the compact
  style of the example files (one line per component). The text can be edited
  and applied with **Apply**, copied with **Copy**, or downloaded with
  **Download**.

The file written by the interface stores the symbol position of each component
as `ui: {x, y}` (with `flip: true` if flipped and `rot: 90`, `180` or `270` if
rotated, clockwise); the solver ignores the `ui` key.
Models written by hand without `ui` are laid out on a grid when opened. Comments
in a YAML file are not kept when it is applied or saved from the interface.

### 9. Find things and commands

Ctrl+K (or the magnifier in the top bar) opens a search field over everything:
type part of a component id to select it and move the view to it, a component
type to add one, the name of an example to load it, or a command (run, save,
fit to view, theme, export). The **?** key or the question mark in the top bar
lists all keyboard and mouse actions. Key 1 fits the schematic to the view.

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
| Keyboard shortcuts do nothing | Single-key shortcuts, Delete, undo and copy/paste act on the schematic only while no text field has the focus (there they edit the text); click on the canvas first. Shift+Enter is ignored inside text areas. |
| A junction is not offered under "Insert into this line" | One end of the line is a tank, junction or boundary; a volume next to it would be connected to it directly. Insert a flow element first. |
| Save downloads a new file every time | The page is embedded in a frame or the browser has no File System Access API, so it cannot write to the same file again. |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
| B | 2026-11-18 | Undo and redo, selectable connections, multiple selection, copy and paste, rotate, insertion into a line, context menus, Save, find, parameter study, reference run, playback, CSV export, messages, dark theme |
