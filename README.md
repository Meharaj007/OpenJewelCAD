# Open Jewel CAD

Open Jewel CAD is an early-stage, Windows-first jewellery CAD project. The long-term goal is a professional design and manufacturing application built around accurate geometry, editable parameters, and dependable workflows.

> **Project stage:** Rust geometry/document foundations and first OCCT kernel operation. This is not yet a desktop CAD application.

## Goals

- Build accurate, unit-aware geometry in millimetres with explicit tolerances and useful validation errors.
- Create a parametric document and feature-history system with safe regeneration and undo/redo.
- Add jewellery workflows for rings, pendants, bangles, earrings, chains, gemstones, and settings.
- Support manufacturing estimates and carefully verified import/export formats.
- Keep geometry processing headless-testable, preserve user projects, and keep the core useful offline.
- Explore AI-assisted design only through validated CAD commands; generated images are not manufacturing geometry.

## What is implemented

On `main` today:

- Rust workspace and the `openjewelcad-geometry` crate.
- Finite millimetre values, 3D points and vectors, displacement and distance, vector scaling, dot/cross products, and normalization.
- Validated axis-aligned bounds with dimensions, volume, union, and tolerance-aware containment checks.
- Typed errors for invalid numeric values, zero-length directions, invalid bounds, and invalid tolerances.
- A document crate with stable feature IDs, dependency ordering, missing-reference and cycle checks, and revision tracking.
- A C++ OCCT kernel exposed through an opaque-handle C ABI: create a box, query volume, and check B-rep validity.
- Unit tests for geometry and feature-graph behavior.
- A CMake/vcpkg smoke test that builds a 2×3×4 mm OCCT box and verifies its 24 mm³ volume.

The Rust workspace check and all eleven Rust unit tests pass. The OCCT Debug build and CMake smoke test also pass on the current Windows machine.

## What is next

1. Connect the Rust core to the tested C ABI and define safe handle lifetime and error conversion.
2. Expand kernel primitives, transforms, and geometry regression coverage.
3. Extend the document model with persistence, typed commands, parameters, regeneration, and undo/redo.
4. Build the Qt desktop shell and viewport after the kernel/API boundary is validated.
5. Add jewellery and gemstone workflows incrementally with manufacturing-oriented checks.

See the [development roadmap](DEVELOPMENT_ROADMAP.md) for stage gates and the [architecture](ARCHITECTURE.md) for the proposed boundaries.

## Technology direction

- **Rust:** application services, project/document model, commands, persistence, and background work.
- **C++ and OCCT:** CAD kernel operations and jewellery geometry.
- **Qt 6:** Windows desktop interface and viewport integration.
- **CMake and Cargo:** native and Rust builds.

The Qt desktop is still planned and has not been integrated. The first OCCT-backed kernel operation is implemented and tested.

## Build the Rust foundation (Windows)

Install Rust with the MSVC toolchain and Visual Studio's **Desktop development with C++** workload. From the repository root:

```powershell
cargo check --workspace
cargo test --workspace
```

## Build the C++/OCCT kernel (Windows)

Install Visual Studio with **Desktop development with C++**, CMake, Git, and vcpkg. Set `VCPKG_ROOT` to the vcpkg checkout that matches the baseline pinned in `vcpkg.json`. From a Visual Studio Developer PowerShell at the repository root:

```powershell
$env:VCPKG_ROOT = "$HOME\Documents\vcpkg"
cmake -S . -B build -DCMAKE_TOOLCHAIN_FILE="$env:VCPKG_ROOT\scripts\buildsystems\vcpkg.cmake" -DVCPKG_TARGET_TRIPLET=x64-windows
cmake --build build --config Debug
ctest --test-dir build -C Debug --output-on-failure
```

The first configure builds or restores OCCT and its dependencies and can take a long time.

## Project documents

- [Architecture](ARCHITECTURE.md)
- [Development roadmap](DEVELOPMENT_ROADMAP.md)
- [Technical decisions](TECHNICAL_DECISIONS.md)
- [Contributor and agent instructions](AGENTS.md)

## License

No project license has been selected. Until a license is added, all rights are reserved. The selected OCCT distribution includes LGPL 2.1 with an additional exception; its notices and obligations must be documented before distributing builds.
