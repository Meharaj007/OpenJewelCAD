# Open Jewel CAD development roadmap

This roadmap is a sequence of reviewable gates. Dates and staffing estimates are intentionally omitted until the architecture and toolchain are validated.

The complete command catalog, dependency mapping, test expectations, and user-requested nine-phase feature order are in [TOOL_INVENTORY.md](TOOL_INVENTORY.md). The phases describe feature delivery; the gates below remain prerequisites and release checks.

## Gate 0 — Architecture review

- Review the Rust/egui and C++/OCCT boundaries, document graph, geometry ownership, and persistence approach.
- Resolve the release license, desktop dependency inventory, OCCT version and distribution plan.
- Define measurable acceptance criteria for the first supported Windows build.

**Exit:** reviewed architecture and recorded decisions; no broad feature implementation yet.

## Gate 1 — Reproducible toolchain spike

- Build a minimal Rust/egui desktop shell and a headless OCCT geometry library on supported Windows CI.
- Build a Rust crate and call a single C ABI operation in both directions needed by the application.
- Verify ownership, error conversion, cancellation behavior, and clean shutdown.
- Record exact compiler, Rust, OCCT, CMake, and dependency-manager versions and licenses.

**Exit:** clean checkout builds reproducibly; one geometry operation runs headlessly and through the ABI; dependency and license inventory is approved.

**Progress:** The Windows MSVC/CMake/vcpkg toolchain is verified and OCCT 8.0.1 is pinned. ABI version 25 creates open lines and polylines, cubic Bezier curves, arcs, polygon/circle/ellipse curves, planar faces, open-curve extrusion surfaces along any non-zero 3D direction, revolved open-profile surfaces including cubic Bezier curves, closed-profile extrusions along editable 3D directions, and 360° revolved solids through curve → surface → solid workflows; it also supports primitive generators, transforms, world-plane mirrors, booleans, arrays, measurements, validity checks, and tessellation for STL/OBJ export. The Rust/egui desktop app organizes tools by modeling stage, provides a Ctrl+K searchable picker, edits curve control points, direction, closure, and extrusion direction, mirrors models and Boolean tools, and displays exact OCCT results after regeneration. Project version 20 stores revolved Bezier profiles, control points, and solid-extrusion directions and retains earlier extrusion and mirror settings; versions 1 through 19 remain readable. Multi-feature history, general curve/surface tools, topology picking, broader jewelry generators, ZBrush-inspired sculpting, and distribution license review remain open.

## Gate 2 — Geometry foundation

- Unit-aware points, vectors, transforms, primitives, topology inspection, and validity diagnostics.
- Typed errors, explicit tolerances, meshing and selection IDs.
- Headless tests for expected dimensions, validity, and tolerance behavior.

**Exit:** API behavior and regression fixtures are stable enough for feature modules.

## Gate 3 — Document and command foundation

- Versioned document schema, stable IDs, typed commands, transactions, save/load, recovery, and migrations.
- Feature graph, cycle detection, incremental invalidation, and undo/redo coordination.

**Exit:** edits survive save/load and recovery; dependent features regenerate deterministically in tests.

## Gate 4 — Usable desktop workflow

- Main window, viewport, object selection, properties, command feedback, and background job progress/cancellation.
- Confirm the UI remains responsive during a representative long operation.

**Exit:** a reviewer can create, edit, save, reopen, and inspect a basic model.

## Gate 5 — Jewellery and gemstone workflows

- Start with one constrained ring workflow and one gemstone shape/placement path.
- Add settings and other jewellery generators only with parameter definitions, failure behavior, and manufacturing-oriented tests.
- Expand to bangle, pendant, earring, and chain modules based on validated user workflows.

**Exit:** each generator has documented parameters and repeatable geometry regression coverage.

## Gate 6 — Manufacturing and exchange

- Material density and weight estimates with stated assumptions.
- Export/import adapters for selected formats, with unit and round-trip verification.
- Manufacturing checks clearly distinguish geometric validity from process feasibility.

**Exit:** documented accuracy limits and regression fixtures for each released calculation/format.

## Gate 7 — Library, plugins, and optional AI

- Searchable local catalogue and thumbnail pipeline.
- Versioned plugin API with explicit permissions and compatibility policy.
- Optional AI providers that produce validated typed commands; generated imagery stays separate from manufacturing geometry.

**Exit:** offline core remains functional; plugin/AI failures cannot corrupt saved documents.

## Gate 8 — Production readiness

- Performance profiling on representative assemblies, crash recovery, packaging, installer, accessibility, and release documentation.
- CI builds/tests across supported Windows configurations; security and dependency review.

**Exit:** release checklist, supported-platform matrix, support policy, and reproducible release artifacts are approved.

## Risks that can change sequencing

- eframe/OCCT/MSVC packaging compatibility or license constraints may require changing dependency versions or the UI/core boundary.
- Unstable topology references may require a semantic naming strategy before advanced parametric features.
- Real-world jewellery workflows may change which generator should be first; validate with target designers before expanding feature scope.
