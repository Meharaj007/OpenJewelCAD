# Open Jewel CAD

Open Jewel CAD is an early-stage, Windows-first jewellery CAD project. The long-term goal is a professional design and manufacturing application built around accurate geometry, editable parameters, and dependable workflows.

> **Project stage:** the desktop editor can create box, cylinder, sphere, and torus solids, edit their dimensions, rotate previews, save projects, and regenerate checked OCCT geometry. Boolean editing and jewellery-specific tools are still to come.

## Goals

- Build accurate, unit-aware geometry in millimetres with explicit tolerances and useful validation errors.
- Create a parametric document and feature-history system with safe regeneration and undo/redo.
- Add jewellery workflows for rings, pendants, bangles, earrings, chains, gemstones, and settings.
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
- A C++ OCCT kernel exposed through ABI version 2: create box, cylinder, sphere, and torus solids; query volume; and check B-rep validity.
- A Rust FFI crate that loads the kernel DLL, checks its ABI version, owns shape handles safely, and converts kernel errors.
- A Rust desktop editor with a primitive selector, millimetre controls, draggable previews, project open/save, and OCCT regeneration and validation.
- A command-line interface with versioned `.ojcad` projects and create/edit/inspect commands for each primitive.
- Unit tests for geometry and feature-graph behavior.
- A CMake/vcpkg smoke test that builds all four OCCT primitives and verifies their volumes and validity.

The Rust workspace check and seventeen unit/integration tests pass. The CMake smoke test and command-line create/inspect flows for cylinder, sphere, and torus also pass on the current Windows machine.

## What is next

1. Add boolean union/cut tools and expand geometry regression coverage.
2. Extend the document model with persistence, typed commands, parameters, regeneration, and undo/redo.
3. Extend the desktop editor with an OCCT-meshed viewport, feature history, and undo/redo.
4. Add jewellery and gemstone workflows incrementally with manufacturing-oriented checks.

See the [development roadmap](DEVELOPMENT_ROADMAP.md) for stage gates and the [architecture](ARCHITECTURE.md) for the proposed boundaries.

## Technology direction

- **Rust:** application services, project/document model, commands, persistence, background work, and the current eframe/egui desktop prototype.
- **C++ and OCCT:** CAD kernel operations and jewellery geometry.
- **CMake and Cargo:** native and Rust builds.

The rotatable preview is a lightweight mesh drawn from the primitive parameters. Press **Regenerate solid** to build the OCCT shape and report its validity and volume; the viewport does not yet display OCCT's own tessellation.

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
cargo run -p openjewelcad-cli -- sphere new sphere.ojcad 3
cargo run -p openjewelcad-cli -- torus new ring.ojcad 5 1
cargo run -p openjewelcad-cli -- inspect sample.ojcad
```

The first configure builds or restores OCCT and its dependencies and can take a long time.
The Rust desktop editor uses eframe/egui and requires Rust 1.95 or newer. On this Windows machine the optimized editor executable is 6.1 MB; the OCCT kernel and its runtime DLLs are separate. Build the OCCT C++ kernel before launching the editor.

## Project documents

- [Architecture](ARCHITECTURE.md)
- [Development roadmap](DEVELOPMENT_ROADMAP.md)
- [Technical decisions](TECHNICAL_DECISIONS.md)
- [Contributor and agent instructions](AGENTS.md)

## License

No project license has been selected. Until a license is added, all rights are reserved. The selected OCCT distribution includes LGPL 2.1 with an additional exception; its notices and obligations must be documented before distributing builds.
