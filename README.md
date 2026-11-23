# Open Jewel CAD

Open Jewel CAD is an early-stage, Windows-first jewellery CAD project. The long-term goal is a professional design and manufacturing application built around accurate geometry, editable parameters, and dependable workflows.

> **Project stage:** the desktop editor now follows curve → surface → solid paths: edit closed profiles, create planar faces, extrude closed profiles along editable 3D directions, extrude open line/polyline/arc curves into surface shells, revolve an open profile into a surface, or revolve a positive-radius section into a solid. It also creates analytic circles and ellipses; eight solid primitives; a parametric ring band; and a faceted round-brilliant stone. It supports transforms and arrays, B-rep union/difference/intersection with a second primitive, OCCT measurements, and binary STL or OBJ export. Broader jewellery builders and brush-based sculpting remain planned.

## Goals

- Build accurate, unit-aware geometry in millimetres with explicit tolerances and useful validation errors.
- Create a parametric document and feature-history system with safe regeneration and undo/redo.
- Add jewellery workflows for rings, pendants, bangles, earrings, chains, gemstones, and settings.
- Combine precise Rhino-style B-rep modeling with a separate ZBrush-inspired sculpt workflow for organic forms, using shared units, projects, materials, and export.
- Support manufacturing estimates and carefully verified import/export formats.
- Keep geometry processing headless-testable, preserve user projects, and keep the core useful offline.
- Explore AI-assisted design only through validated CAD commands; generated images are not manufacturing geometry.

## What is implemented

Current implementation:

- Rust workspace and the `openjewelcad-geometry` crate.
- Finite millimetre values, 3D points and vectors, displacement and distance, vector scaling, dot/cross products, and normalization.
- Validated axis-aligned bounds with dimensions, volume, union, and tolerance-aware containment checks.
- Typed errors for invalid numeric values, zero-length directions, invalid bounds, and invalid tolerances.
- A document crate with stable feature IDs, dependency ordering, missing-reference and cycle checks, and revision tracking.
- A C++ OCCT kernel exposed through ABI version 23: create solid primitives, open lines and polylines, arcs, closed polygons, circles, and ellipses; planar faces, exact open line/polyline/arc and revolved-profile surface shells, closed-profile extrusions along editable 3D directions, and revolved solids; transform, mirror, boolean-combine, and array shapes; and report length, mass properties, bounds, and validity.
- A Rust FFI crate that loads the kernel DLL, checks its ABI version, owns shape handles safely, and converts kernel errors.
- A Rust desktop app that opens to a project manager with create/open actions, a persisted recent-project list, and a folder browser for `.ojcad` files.
- A modern dark editor workspace with a left primitive toolbox, fitted and clipped central viewport, isometric/top/front/right views, shaded/edge display modes, right-side dimensions, move/rotate/scale controls, save controls, and OCCT regeneration and validation.
- World-plane mirror controls for the model and Boolean tool, with live preview, exact OCCT regeneration, and saved mirror settings.
- A searchable modeling-tool palette, opened with **Ctrl+K**, so curve, surface, solid, and jewelry generators can be found by name.
- Binary STL and Wavefront OBJ export from OCCT tessellation, available in the editor and command line; mesh output coordinates are millimetres.
- Exact solid union, difference, and intersection using a second primitive as the tool, with persistent editable inputs and exact OCCT mesh preview after regeneration.
- Editable linear and circular arrays with up to 64 copies, non-degenerate spacing/angle validation, exact OCCT compound geometry, and a responsive fitted preview.
- A ring band generator with editable inner diameter, wall thickness, and band width, built and measured as an exact OCCT hollow solid.
- A round-brilliant gemstone generator with editable girdle diameter, total height, and table diameter. OCCT builds a closed faceted solid that can be transformed, measured, boolean-combined, and exported.
- An editable polygon profile with 3–64 vertices, live coordinate editing, self-intersection validation, exact OCCT extrusion along an editable 3D direction, and 360° revolve around the Y axis when all profile points stay at positive radius. The curve editor builds dimensioned rectangles, regular polygons, and ring cross sections; generated points stay editable.
- Rhino-style workflows: create a closed polyline curve, convert it into a planar surface, then extrude or revolve that same profile into a solid. Open line, polyline, and arc curves can be extruded along any non-zero 3D direction; open polyline profiles can also be revolved around Y to form valid surface shells. Project version 15 introduced revolved surfaces, version 16 stores world-plane mirror settings, version 17 stores open-curve extrusion directions, and version 18 adds directions for closed-profile solids; versions 1 through 17 remain readable.
- A circle workflow: create a true OCCT circle curve, convert it to a planar disk, and extrude the same saved radius into a cylinder along a chosen 3D direction. Project version 7 introduced these stages; current saves use version 18.
- An ellipse workflow: create an exact OCCT ellipse, convert it to a planar face, then extrude the saved X/Y radii into a solid along a chosen 3D direction. Project version 8 introduced these stages; current saves use version 18.
- An open arc curve with editable radius, start angle, and counter-clockwise sweep. It is saved as an exact OCCT circular edge; introduced in project version 9.
- An open line curve with editable XY endpoints, exact OCCT geometry, length validation, and a two-endpoint preview. Project version 10 introduced endpoint persistence.
- An open polyline with 2–64 ordered editable XY points, exact OCCT wire geometry, measured length, endpoint markers, and project persistence introduced in version 11.
- Curve editing that reverses line, open-polyline, and closed-polyline direction, and closes an open polyline into a validated closed curve. Both operations are available in the editor and through `curve-reverse` / `curve-close` project commands.
- Open line, polyline, and arc surface tools create valid OCCT shells with exact area and tessellation. They retain and edit their source curve geometry, extrusion distance, and 3D direction in the UI and project file. A revolved surface retains its positive-radius open profile and produces the full 360° shell around Y.
- An analysis panel that reports solid validity, volume, surface area, center of mass, and axis-aligned bounds from OCCT, with an optional custom-density metal weight estimate.
- A command-line interface with versioned `.ojcad` projects, curve and profile editing, curve → surface conversion, transforms, mirrors, booleans, arrays, measurement, STL/OBJ export, and `surface-direction` for open-curve extrusion vectors.
- Unit tests for geometry and feature-graph behavior.
- A CMake/vcpkg smoke test that builds all eight OCCT solids and verifies their volumes and validity.

The current verified phase passes all Rust unit/integration tests and the CMake kernel smoke test. The smoke test checks OCCT line, open and closed polylines, arc, circle, and ellipse curves; planar, open-polyline, and arc extrusion surfaces; solid primitives; round-brilliant, polygon, and revolved geometry; transforms; booleans; arrays; measurements; and tessellation. CLI profile editing, curve creation, stage conversion, save/reopen, inspection, and STL export were also exercised.

## What is next

Feature work follows the full command inventory and nine-phase roadmap in [TOOL_INVENTORY.md](TOOL_INVENTORY.md). The architecture review gate in [DEVELOPMENT_ROADMAP.md](DEVELOPMENT_ROADMAP.md) must be completed before broad product implementation.

See the [development roadmap](DEVELOPMENT_ROADMAP.md) for stage gates and the [architecture](ARCHITECTURE.md) for the proposed boundaries.

## Technology direction

- **Rust:** application services, project/document model, commands, persistence, background work, and the current eframe/egui desktop prototype.
- **C++ and OCCT:** CAD kernel operations and jewellery geometry.
- **CMake and Cargo:** native and Rust builds.

The viewport uses a fast curve/profile preview while editing and switches to exact OCCT tessellation after regeneration. Orbit, wheel zoom, middle-button pan, and Fit keep the model easy to frame inside the clipped view. Mesh export uses OCCT tessellation at 0.05 mm linear deflection and 0.35 rad angular deflection. New `.ojcad` saves use version 18 for editable closed-profile extrusion directions; versions 1 through 17 still open.

## Build and test (Windows)

Install Visual Studio with **Desktop development with C++**, CMake, Git, and vcpkg. Set `VCPKG_ROOT` to the vcpkg checkout that matches the baseline pinned in `vcpkg.json`. From a Visual Studio Developer PowerShell at the repository root:

```powershell
$env:VCPKG_ROOT = "$HOME\Documents\vcpkg"
cmake -S . -B build -DCMAKE_TOOLCHAIN_FILE="$env:VCPKG_ROOT\scripts\buildsystems\vcpkg.cmake" -DVCPKG_TARGET_TRIPLET=x64-windows
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
$kernelDir = (Resolve-Path .\build\Debug).Path
$env:OPENJEWELCAD_KERNEL_DLL = "$kernelDir\openjewelcad_kernel.dll"
$env:PATH = "$kernelDir;$env:PATH"
cargo check --workspace
cargo test --workspace
cargo run -p openjewelcad-cli --
cargo run -p openjewelcad-cli -- gui
cargo run -p openjewelcad-cli -- box new sample.ojcad 2 3 4
cargo run -p openjewelcad-cli -- box edit sample.ojcad 2 3 5
cargo run -p openjewelcad-cli -- cylinder new cylinder.ojcad 2 5
cargo run -p openjewelcad-cli -- cone new cone.ojcad 2 5
cargo run -p openjewelcad-cli -- truncated-cone new frustum.ojcad 3 1 4
cargo run -p openjewelcad-cli -- ellipsoid new oval.ojcad 4 3 2
cargo run -p openjewelcad-cli -- pipe new tube.ojcad 3 1.5 4
cargo run -p openjewelcad-cli -- sphere new sphere.ojcad 3
cargo run -p openjewelcad-cli -- torus new ring.ojcad 5 1
cargo run -p openjewelcad-cli -- ring-band new band.ojcad 18.2 1.5 4
cargo run -p openjewelcad-cli -- round-brilliant new stone.ojcad 6 3.6 3.3
cargo run -p openjewelcad-cli -- polyline-curve new profile.ojcad
cargo run -p openjewelcad-cli -- open-polyline-curve new open_path.ojcad
cargo run -p openjewelcad-cli -- open-curve-point open_path.ojcad 1 0 5
cargo run -p openjewelcad-cli -- line-curve new line.ojcad -1 2 2 6
cargo run -p openjewelcad-cli -- arc-curve new arc.ojcad 5 0 90
cargo run -p openjewelcad-cli -- profile-point profile.ojcad 1 6 -5
cargo run -p openjewelcad-cli -- planar-surface from-curve profile.ojcad
cargo run -p openjewelcad-cli -- extruded-polygon from-profile profile.ojcad 2
cargo run -p openjewelcad-cli -- boolean difference sample.ojcad sphere 2 0 0 5 4 1
cargo run -p openjewelcad-cli -- linear-array sample.ojcad 4 0 8 0
cargo run -p openjewelcad-cli -- polar-array sample.ojcad 8 45
cargo run -p openjewelcad-cli -- transform ring.ojcad 1 2 3 0 0 45 1.25
cargo run -p openjewelcad-cli -- inspect sample.ojcad
cargo run -p openjewelcad-cli -- export-stl sample.ojcad sample.stl
cargo run -p openjewelcad-cli -- export-obj sample.ojcad sample.obj
```

The first configure builds or restores OCCT and its dependencies and can take a long time.
The Rust desktop editor uses eframe/egui and requires Rust 1.95 or newer. On this Windows machine the optimized editor executable is 6.1 MB; the OCCT kernel and its runtime DLLs are separate. Local development builds search `build/Debug` or `build/Release` automatically for the kernel. Set `OPENJEWELCAD_KERNEL_DLL` to a custom kernel location when needed.

## Project documents

- [Architecture](ARCHITECTURE.md)
- [Development roadmap](DEVELOPMENT_ROADMAP.md)
- [Complete tool inventory and phased implementation plan](TOOL_INVENTORY.md)
- [Technical decisions](TECHNICAL_DECISIONS.md)
- [Contributor and agent instructions](AGENTS.md)

## License

Project-authored code is licensed under the MIT License; see [LICENSE](LICENSE). MIT permits commercial use and resale of copies when the required notices are retained. Third-party components keep their own licenses. OCCT 8.0.1 is under LGPL 2.1 with the Open CASCADE exception; review and include its notices and comply with its requirements before distributing builds.
