# Technical decisions

Record accepted decisions here. Proposed items remain open until reviewed; do not infer approval from their presence.

## TD-001 — Use a Rust desktop shell with a separate OCCT kernel

- **Status:** Accepted for the initial editor
- **Decision:** Rust owns desktop presentation and application workflows through eframe/egui; a C++ library owns OCCT geometry operations behind the existing safe Rust wrapper and versioned C ABI.
- **Reason:** The user prioritizes a quick, lightweight editor, and this avoids a separate Qt toolchain and runtime while keeping the kernel headless-capable.
- **Trade-off:** The first preview is a simple projection from dimensions, not an OCCT-meshed viewport. A later feature-rich renderer may need a separate integration spike.
- **Dependencies:** eframe 0.37.0 (MIT OR Apache-2.0), rfd 0.18.0 (MIT), Rust 1.95 or newer. Exact Rust dependency resolution is recorded in Cargo.lock.
- **Validation:** Built and opened on Windows; dimension editing, box rotation, and OCCT regeneration were exercised. OCCT reported the default box valid with a volume of 200 mm³.

## TD-002 — Use an opaque C ABI for Rust/C++ communication

- **Status:** Accepted for the initial kernel boundary
- **Decision:** Expose a version-reporting C ABI, opaque shape and mesh handles, fixed-layout error values, and explicit destruction. Catch C++ exceptions at every operation boundary. ABI version 21 supports solid builders, curve and surface builders, world-plane shape mirrors, transforms, B-rep booleans, linear/polar arrays, measurements, bounds, validity checks, and controlled tessellation.
- **Reason:** C ABI limits coupling and avoids exposing unstable C++ or OCCT types to Rust.
- **Trade-off:** More explicit conversion code and API versioning work.
- **Validation:** Rust checks ABI version 21, owns handles with drop-based release, and integration-tests curve lengths, surface area and tessellation, primitive validity, expected volumes, bounds, transforms, mirrors, booleans, arrays, gemstone geometry, and structured argument errors.

## TD-003 — Keep internal geometry in millimetres

- **Status:** Proposed
- **Decision:** Convert external units to millimetres at well-defined boundaries and record source units where export fidelity requires it.
- **Reason:** Jewellery dimensions are naturally expressed in millimetres and one internal unit avoids mixed-unit operations.
- **Trade-off:** Imports and UI display must convert carefully; tolerance policy needs explicit units.
- **Validation:** Unit conversion and tolerance tests in Gate 2.

## TD-004 — Separate source features from derived geometry

- **Status:** Proposed
- **Decision:** Persist typed feature definitions and parameters as the source of truth; treat kernel shapes and meshes as derived state/cache when deterministic regeneration is possible.
- **Reason:** Supports editing, dependency regeneration, and schema evolution.
- **Trade-off:** Regeneration can fail across kernel/version changes; migration and cache invalidation need design.
- **Validation:** Save/load, regeneration, and migration fixtures in Gate 3.

## TD-005 — Pin the initial Windows dependency set

- **Status:** Accepted for the initial editor; release packaging remains open
- **Decision:** Use Rust 1.95+, eframe 0.37.0, rfd 0.18.0, OCCT 8.0.1, and the existing Windows MSVC/CMake/vcpkg toolchain.
- **Reason:** This combination builds and runs the first desktop editor on the current Windows machine.
- **Next step:** Verify clean-machine packaging and finish the license inventory before distribution.

## TD-006 — License and distribution model

- **Status:** MIT selected for project-authored source code; binary distribution review remains open
- **Decision:** License project-authored code under MIT. Third-party components retain their own licenses. MIT allows commercial use and redistribution, including selling copies, as long as the required notices are kept.
- **Reason:** The user selected MIT after being informed that it permits paid redistribution.
- **Next step:** Inventory third-party notices and complete the OCCT LGPL 2.1 + Open CASCADE exception distribution review before shipping binaries.

## TD-007 — Pin OCCT through the vcpkg manifest

- **Status:** Accepted for the initial Windows kernel spike
- **Decision:** Use the vcpkg manifest and its pinned registry baseline to resolve Open CASCADE Technology 8.0.1 for the x64 Windows MSVC triplet.
- **Reason:** A manifest baseline makes the dependency selection reproducible across developer machines and CI.
- **Trade-off:** The first build compiles a large native dependency set and can take substantial time.
- **License:** OCCT 8.0.1 is distributed under LGPL 2.1 with the Open CASCADE exception; review and include the corresponding notices before distributing the application.
- **Validation:** Installed and built on Windows; the OCCT box-volume/validity smoke test passes.

## TD-008 — Store and apply object transforms

- **Status:** Implemented in the current editor prototype
- **Decision:** `.ojcad` version 16 stores finite millimetre translation, X/Y/Z Euler rotations in degrees, positive uniform scale, optional world-plane mirror flags, optional user-entered density, one editable primitive boolean tool, one editable linear or polar array, polygon, circle, ellipse, arc, line, open polyline, revolved-profile, extrusion-surface, and revolved-surface data. ABI version 21 applies scale, rotations, translation, then enabled world-origin plane mirrors. Versions 1 through 15 remain readable.
- **Reason:** Move, rotate, and scale need to regenerate the same geometry after save and reopen, and the transform order must be deterministic.
- **Validation:** Rust round-trip and version-migration tests, OCCT transform/mass-properties/bounds tests, and CLI create/transform/inspect checks.

## TD-009 — Keep boolean inputs editable

- **Status:** Implemented for one primitive tool per project
- **Decision:** Version 3 introduced persistence of the base primitive and boolean tool parameters; version 4 added array inputs, version 5 added extrusion profile points, version 6 added polygon curve and planar-surface stages, version 7 added analytic circle stages, and version 8 adds ellipse stages. Regenerate union, difference, or intersection through OCCT. Store source parameters rather than serializing opaque kernel results; display resulting OCCT geometry after regeneration.
- **Reason:** Boolean results should remain adjustable and auditable while preserving exact B-rep geometry.
- **Trade-off:** Multi-feature boolean chains and topology references require the planned document graph and are not represented by this first schema.
- **Validation:** FFI tests cover overlapping and disjoint boxes, expected union/difference/intersection volumes, validity, and empty-result errors; project tests cover boolean round-trip and legacy versions.

## TD-010 — Build round-brilliant geometry from facets

- **Status:** Implemented as an editable geometry generator
- **Decision:** Construct a closed faceted solid from five regular 32-vertex rings parameterized by girdle diameter, total height, and table diameter. The result is a kernel solid, so transforms, B-rep booleans, measurements, tessellation, and mesh export use the same validated geometry path.
- **Reason:** A stone needs explicit, editable geometry for CAD operations and manufacturing exchange; an illustrative viewport mesh would not be sufficient.
- **Trade-off:** The proportions are a starter layout, not a cut-grade specification or a substitute for a curated gemstone library.
- **Validation:** The native smoke test checks B-rep validity and its analytically expected faceted volume. Rust tests check parameter validation, bounds, tessellation, and project round-trip.

## TD-011 — Keep linear and polar arrays as editable compounds

- **Status:** Implemented in ABI version 9 and project version 4
- **Decision:** Create an OCCT compound containing 2–64 transformed copies of the input shape. Linear arrays use an XYZ step in millimetres; polar arrays rotate around world Z by a degree increment. Persist the parameters and regenerate the compound instead of storing a flattened mesh.
- **Reason:** Arrays should remain adjustable and should participate in precise measurement and exchange workflows.
- **Trade-off:** The current polar axis is fixed to world Z, and arrays are separate solid instances; broad boolean unions are an explicit follow-up operation.
- **Validation:** FFI and native smoke checks cover copy count bounds, validity, summed volume, and expected bounds; project tests cover linear/polar serialization and prior-version migration.

## TD-012 — Persist straight-sided profile extrusion inputs

- **Status:** First implemented in ABI version 10 and project version 5; retained in current ABI 24/project 19
- **Decision:** Store 3–64 XY vertices as the source of a closed straight-sided profile, reject invalid or self-intersecting outlines, and extrude the exact wire and face along +Z. The editor previews the profile while it is being edited and persists every point.
- **Reason:** A simple editable sketch-to-solid workflow enables custom plate, pendant, and signet shapes without adding placeholder geometry tools.
- **Trade-off:** Curved sketch segments, constraints, arbitrary extrusion directions, and profile booleans are not represented yet.
- **Validation:** Rust and native kernel tests cover rectangle and concave profiles, expected volumes, bounds, self-intersection rejection, and versioned project round-trip.

## TD-013 — Build a curve-to-surface-to-solid workflow

- **Status:** Implemented as the first precision-modeling sequence
- **Decision:** Store one closed XY polyline profile as source data. Build a real OCCT wire for the curve stage, a planar OCCT face for the surface stage, and an extruded OCCT solid for the solid stage. Users can edit points in each stage and carry the same outline forward; new project version 6 preserves the active stage and profile.
- **Reason:** The user asked for Rhino-style modeling to begin with curves, then surfaces, then solids. This gives the application a real geometric progression before adding broader curve and surface commands.
- **Trade-off:** This is a closed, straight-segment profile only; it is not yet a general curve editor, constrained sketch system, NURBS surface modeller, or multi-object feature graph.
- **Validation:** C ABI and Rust tests verify wire length, face area/centroid/tessellation, extrusion volume, project round-trip, and self-intersection rejection; CLI conversion was exercised across all three stages.

## TD-015 — Add an analytic ellipse profile workflow

- **Status:** Implemented in ABI version 13 and project version 8; retained in current ABI 24/project 19
- **Decision:** Store editable X/Y radii and build an exact OCCT ellipse wire, planar face, or +Z prism. Use analytic OCCT geometry for regenerated results and a lightweight polygonal approximation only for the live editor preview.
- **Reason:** Ellipses are a common precision sketch primitive and can drive planar jewellery components without converting the source profile to a mesh.
- **Trade-off:** The ellipse is centered in XY and extrudes along +Z; rotated-plane sketching, trim curves, and arbitrary directions remain future work.
- **Validation:** Native and Rust tests cover perimeter, bounds, exact area and volume, tessellation, equal-radius degeneration to a circle, project persistence, and conversion through all three stages.

## TD-014 — Add an analytic circle profile workflow

- **Status:** Implemented in ABI version 12 and project version 7
- **Decision:** Represent a circle as an exact OCCT wire, planar disk face, or extruded cylinder from a shared saved radius. Keep the circle radius editable in the properties panel and carry it through stage changes; expose each stage through the command line as well.
- **Reason:** Jewellery sketches frequently start with circular outlines, and a circle gives a second real curve-to-surface-to-solid path alongside polygon profiles.
- **Trade-off:** The profile is a centered XY circle with straight +Z extrusion. Arbitrary planes and curved extrusion paths remain future work.
- **Validation:** C ABI smoke and Rust integration tests verify analytic circumference, disk area and tessellation, cylinder volume, project persistence, and conversion through all three stages.

## TD-016 — Add an editable open circular arc

- **Status:** Implemented in ABI version 14 and project version 9; retained in current ABI 24/project 19
- **Decision:** Store radius, start angle, and positive counter-clockwise sweep as editable project parameters. Construct a trimmed OCCT circular edge and display a sampled preview without closing its endpoints.
- **Reason:** Arcs are a basic curve command in the requested Rhino-style curve-first workflow, and their open topology must remain explicit.
- **Trade-off:** The first arc is centered in XY and exists as a curve only; arc-to-surface, sweep, and arbitrary-plane operations need their own geometric definitions.
- **Validation:** Kernel smoke and Rust integration tests check validity, arc length, expected bounds, and invalid full-circle sweep rejection; CLI tests check versioned project round-trip and OCCT regeneration.

## TD-017 — Add an editable open line curve

- **Status:** Implemented in ABI version 15 and project version 10; retained in current ABI 24/project 19
- **Decision:** Store two XY endpoints in millimetres, reject coincident or out-of-range points, and construct an exact OCCT edge in an open wire. The editor exposes both endpoints and previews their two distinct vertices.
- **Reason:** A line is the simplest open sketch primitive and a foundation for future open polylines, constraints, and curve-based surface tools.
- **Trade-off:** The first implementation is a single XY segment. Its +Z extrusion surface is supported; snapping, constraints, arbitrary planes, and curved-path operations remain future work.
- **Validation:** Native smoke, Rust FFI, and project tests cover validity, length, bounds, invalid equal endpoints, and save/load; CLI creation and inspection are exercised.

## TD-018 — Add an editable open polyline curve

- **Status:** Implemented in ABI version 16 and project version 11; retained in current ABI 24/project 19
- **Decision:** Store 2–64 ordered XY points as project data, reject non-finite, out-of-range, or duplicate neighboring points, and construct a non-closed OCCT wire. The editor permits point insertion, removal, and coordinate editing.
- **Reason:** Open polylines are a basic sketch path for later sweep and surface tools and must remain distinct from closed face boundaries.
- **Trade-off:** Self-crossing paths are allowed as wire geometry; constraints, snapping, and downstream sweeps remain future work.
- **Validation:** Native smoke, FFI, model persistence, and CLI tests cover open length, bounds, point edits, and invalid consecutive points.

## TD-019 — Build solids by revolving an editable section

- **Status:** Implemented in ABI version 17 and project version 12; retained in current ABI 24/project 19
- **Decision:** Revolve a simple closed XY polygon around the Y axis through 360°. X coordinates represent radius and must remain positive; Y is the axial height. Store and edit the profile points, and generate the exact B-rep solid through OCCT.
- **Reason:** Profile revolution is a core curve-first solid workflow for rings and turned jewellery forms.
- **Trade-off:** Partial revolutions, alternate axes, curved profile segments, and revolved surface stages remain future work.
- **Validation:** CMake smoke, Rust FFI, project persistence, and CLI checks cover analytic volume, bounds, validity, and rejection of a profile touching the axis.

## TD-020 — Extrude open curves into surface shells

- **Status:** Implemented in ABI version 18 and project version 13; retained in current ABI 24/project 19
- **Decision:** Extrude a validated open XY line or polyline along +Z using OCCT to create an open surface shell. Preserve the source endpoints or ordered polyline points and extrusion height in the project file.
- **Reason:** Rhino-style curve → surface workflows need a surface operation for open paths as well as planar faces built from closed boundaries.
- **Trade-off:** This creates an open shell, not a closed solid. It supports straight polyline segments; arbitrary vectors were added later, while curved edges and solid thickening remain future work.
- **Validation:** Native smoke and Rust/CLI checks cover shell validity, expected area, tessellation, invalid repeated points, persistence, and curve-to-surface conversion.

## TD-021 — Extrude an analytic arc into a cylindrical surface

- **Status:** Implemented in ABI version 19 and project version 14; retained in current ABI 24/project 19
- **Decision:** Preserve the arc radius, start angle, and sweep as editable parameters and extrude its exact OCCT circular edge along +Z to create a cylindrical surface shell.
- **Reason:** The arc command should continue through the same curve → surface workflow as line and polyline curves while retaining exact circular geometry in the kernel.
- **Trade-off:** This creates an open shell, not a solid. The live preview samples the analytic arc; regenerated geometry and measurements come from OCCT. Arbitrary extrusion vectors were added later; thickening remains future work.
- **Validation:** CMake smoke, Rust FFI, project persistence, and CLI tests cover exact area, bounds, validity, tessellation, invalid full-circle sweep, and conversion from an arc curve.

## TD-022 — Revolve an open profile into a surface shell

- **Status:** Implemented in ABI version 20 and project version 15; retained in current ABI 24/project 19
- **Decision:** Revolve a validated open XY profile around +Y through 360° using OCCT. Every X coordinate is a radius and must stay positive; Y is the axial coordinate. Preserve and edit the ordered profile points in the project.
- **Reason:** A Rhino-style surface toolbox should let designers create surfaces of revolution from curves, in addition to extruding them and revolving closed profiles into solids.
- **Trade-off:** This creates an open surface shell, not a solid. It supports straight profile segments and a full 360° turn; partial turns and arbitrary axes remain future work.
- **Validation:** Native smoke, Rust FFI, project persistence, and CLI conversion tests cover analytic area, bounds, validity, tessellation, axis rejection, and curve-to-surface conversion.

## TD-023 — Mirror geometry across world-origin planes

- **Status:** Implemented in ABI version 21 and project version 16
- **Decision:** Mirror B-rep geometry across the world-origin YZ, XZ, and XY planes through an explicit OCCT transform. Persist the three independent mirror flags for both the primary shape and an optional Boolean tool. Apply them after scale, rotation, and translation.
- **Reason:** Mirror is a basic Rhino-style modeling operation and must reproduce as exact geometry after project save and reopen.
- **Trade-off:** The first implementation mirrors only across world-origin planes; arbitrary mirror planes and multi-object results remain future work.
- **Validation:** Native smoke, Rust FFI, and CLI/project tests cover plane selection, B-rep validity, preserved volume, reflected bounds, saved Boolean-tool settings, and older project compatibility. The editor mirrors the same plane selection in its live preview.

## TD-024 — Extrude open curves along an editable 3D direction

- **Status:** Implemented in ABI version 22 and project version 17; retained in current ABI 24/project 19
- **Decision:** Add vector-based OCCT extrusion for open lines, polylines, and circular arcs. Store a finite, non-zero XYZ direction with the surface feature; the saved height is the travel distance, so the kernel normalizes the direction before building the surface shell. Keep the original +Z ABI calls as compatibility wrappers, expose direction components in the editor, and provide a CLI command to update them.
- **Reason:** Rhino-style curve → surface workflows need a general extrusion vector rather than limiting designers to the world Z axis.
- **Trade-off:** This creates an open shell, not a closed solid. Curved polyline segments, surface thickening, and unrestricted sweep directions on other profile tools remain future work.
- **Validation:** Native smoke, Rust FFI, and CLI project tests cover non-axis-aligned displacement, exact surface area and bounds, zero-direction rejection, project version 17 round trips, and version 16 compatibility.

## TD-025 — Extrude closed profiles along an editable 3D direction

- **Status:** Implemented in ABI version 23 and project version 18; retained in current ABI 24/project 19
- **Decision:** Extend vector extrusion to closed polygon, circle, and ellipse profiles. Normalize the saved XYZ vector so the height parameter remains the travel distance. Keep existing +Z entry points as compatibility wrappers and serialize direction values only in project version 18 or newer.
- **Reason:** The curve → surface → solid workflow should let a designer tilt the solid extrusion without changing the source XY profile, using the same direction controls as open-curve surface extrusions.
- **Trade-off:** These profiles remain centered/oriented in XY; an extrusion with no Z component cannot produce a volumetric solid and is rejected by OCCT. Arbitrary sketch planes and profile-as-boolean-tool remain future work.
- **Validation:** Native smoke, Rust FFI, CLI persistence, and compatibility tests cover polygon/circle/ellipse volume and bounds, non-axis-aligned displacement, zero-direction rejection, version 18 round trips, default-direction migration from version 17, and rejection of unsupported old-version direction fields.

## TD-026 — Add editable cubic Bezier curves and extrusion surfaces

- **Status:** Implemented in ABI version 24 and project version 19
- **Decision:** Store exactly four finite XY control points, create an exact OCCT cubic Bezier edge behind the C ABI, and extrude that curve into a surface shell along a normalized 3D direction. Expose the four editable points in the Rust editor and CLI, use sampled points only for the live preview, and persist the new feature in project version 19 while retaining compatibility with versions 1 through 18.
- **Reason:** Designers need a smooth freeform curve stage that continues into the same curve → surface modeling flow as line, polyline, and arc tools.
- **Trade-off:** This first spline tool is cubic and has four control points; arbitrary degree, interpolation curves, NURBS editing, and constraints remain future work.
- **Validation:** Native smoke, Rust FFI, GUI preview, project round-trip, CLI conversion, and compatibility tests check curve length, surface area, bounds, validity, four-point validation, and version 18 rejection for the new feature.
