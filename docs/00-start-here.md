# Start Here

FNT-DOC-STD-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

This page explains how the fluidnet documentation is organised, how documents
are identified and what kind of content goes where. Read it before adding or
changing a page.

## What fluidnet is

fluidnet is a lumped-parameter simulator for fluid systems and liquid rocket
engine cycles. It solves steady states and transients of feed lines, tanks,
valves, regulators, turbopumps, regeneratively cooled walls and thrust
chambers, with real-fluid properties from CoolProp tables and equilibrium
combustion from Cantera tables. The core is written in Rust; models are YAML
files; the solver is used from a command-line tool, from Python and from a
browser schematic editor.

## How the documentation is organised

Pages are grouped into numbered sections. Every page belongs to exactly one
section.

| Section | What lives there |
|---|---|
| [01 Project](01-project/index.md) | Platform development plan, release history, decision log, open items |
| [02 Engineering](02-engineering/index.md) | Design descriptions of the software and its physical models: architecture, network formulation, solvers, fluid properties, flow elements, turbomachinery, heat transfer, combustion |
| [03 User Guide](03-user-guide/index.md) | How to install and use fluidnet: model file standard, component reference, output channels, CLI, Python API, web UI, modelling practice, examples |
| [04 Verification and Validation](04-verification/index.md) | Verification against analytic solutions, validation against RL10A-3-3A test data, map digitization |
| [05 Development](05-development/index.md) | Building, testing and releasing; code and documentation conventions; how to add a component |
| [06 References](06-references/index.md) | Literature, data sources, nomenclature |

## Document IDs

Every design document, analysis, standard, handbook, plan and register carries
an ID in its first line, never in the page title. The format follows the
Hopper propulsion convention:

`<SCOPE>-<SUBSYSTEM>-<TYPE>-<NNN>`

| Field | Values |
|---|---|
| SCOPE | **FNT** fluidnet |
| SUBSYSTEM | **ARC** software architecture · **SLV** network formulation and solvers · **FLD** fluid properties · **CMP** components and flow elements · **TRB** turbomachinery · **HTX** heat transfer and walls · **CMB** combustion · **USR** user-facing tools (CLI, Python, UI) · **VAL** verification and validation · **DEV** development and release · **PLT** platform and project management · **DOC** documentation itself · **REF** references |
| TYPE | **DOC** design description · **ANA** analysis · **HBK** handbook · **STD** standard · **REG** register · **PLN** plan · **TST** test report |
| NNN | three digits |

Header line format, first line under the title:

`FNT-SLV-DOC-001 · Rev A · Released · 2026-11-02 · Owner: Name`

Status values: **Draft** (being written, content may be wrong), **In review**,
**Released** (checked against the code and the results files of the stated
version), **Superseded** (kept so links resolve; says where the content went).

A revision letter changes when the content changes in substance (a new model,
a changed equation, new results). Typo fixes do not change the revision. Each
page ends with a revision table.

## What goes where

Every technical page is one of the following kinds. Keeping them apart means a
design page stays readable, a validation can be rerun without rewriting the
design, and theory is written once.

- **Design description (DOC).** What a part of fluidnet is, how it works and
  why it is built this way: equations, assumptions, limits, and where it lives
  in the code. It quotes key numbers but does not derive them; it links to the
  analysis that does.
- **Analysis (ANA).** One question answered for one software version:
  inputs, method, results, what they mean, and a run record (commit, command,
  results file). Validation reports are analyses. When the code changes the
  results, the analysis gets a new revision.
- **Handbook (HBK).** How to do something: install, build a model, run a
  study, add a component.
- **Standard (STD).** Rules that files and code must follow: the model file
  format, naming, this page.
- **Register (REG).** A living list: releases, decisions, open items, example
  models, references.
- **Plan (PLN).** What will be done, in which order, with exit criteria.

## Page skeletons

**Section index:** purpose paragraph · status callout · pages grouped by kind
(design descriptions, analyses, handbooks, standards, registers) · open items.

**Design description:** ID header · scope · model and equations · numerical
treatment · parameters and outputs · limits and assumptions · verification
(links) · code location · open items · revision table.

**Analysis:** ID header · question · model and inputs · method · results ·
discussion · conclusions · run record · revision table.

**Handbook:** ID header · purpose · prerequisites · steps · troubleshooting ·
revision table.

## Writing rules

- Plain technical English. No metaphors, no coined names for features. Use
  the standard term (pseudo-transient continuation, not "relaxation trick").
- State units for every number. SI unless the source data is in US units
  (RL10 data stays in psia, lb/s and rpm with SI in brackets where useful).
- Equations go in code blocks in plain Unicode (`ρ V dh/dt − V dP/dt = …`),
  so they render the same in a browser, an editor and Notion.
- Name the code location for every model (`crates/fluidnet-core/src/flow.rs`,
  `restriction_flux`).
- A number in a released page must be traceable to a test, a results file or a
  cited source. If it is an assumption, say so.
- Do not copy requirements, results tables or parameter lists between pages;
  link to the page that owns them. The component reference is generated from
  the code (see [FNT-DEV-HBK-001](05-development/build-and-release.md)).

## Reading the documentation

The pages are plain Markdown and readable in any editor or on a Git host. For a
local website with search and navigation:

```sh
pip install mkdocs-material
mkdocs serve            # in the repository root, then open http://127.0.0.1:8000
```

To move the documentation into Notion later, `python tools/docs_to_notion.py`
writes a zip in Notion's import layout (one page per file, sections as parent
pages); import it with Notion's Markdown import. The Markdown files in the
repository stay the source; regenerate the zip when they change.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
