# Open Jewel CAD

Open Jewel CAD is an early-stage, Windows-first jewellery CAD project. The long-term goal is a professional design and manufacturing application built around accurate geometry, editable parameters, and dependable workflows.

> **Project stage:** a first command-line CAD editor can create, resize, save, and regenerate a box solid. A graphical desktop interface is still planned.

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
- A Rust FFI crate that loads the kernel DLL, checks its ABI version, owns shape handles safely, and converts kernel errors.
- A command-line box editor with a versioned `.ojcad` project file, create/edit/inspect commands, and OCCT regeneration.
- Unit tests for geometry and feature-graph behavior.
- A CMake/vcpkg smoke test that builds a 2×3×4 mm OCCT box and verifies its 24 mm³ volume.

The Rust workspace check and fifteen unit/integration tests pass. The box editor has also been exercised end to end: create a 2 × 3 × 4 mm project, resize it to 2 × 3 × 5 mm, save it, and confirm the regenerated OCCT shape is valid with a 30 mm³ volume. The OCCT Debug build and CMake smoke test also pass on the current Windows machine.

## What is next

1. Expand kernel primitives, transforms, and geometry regression coverage.
2. Extend the document model with persistence, typed commands, parameters, regeneration, and undo/redo.
3. Build the Qt desktop shell and viewport after the kernel/API boundary is validated.
4. Add jewellery and gemstone workflows incrementally with manufacturing-oriented checks.

See the [development roadmap](DEVELOPMENT_ROADMAP.md) for stage gates and the [architecture](ARCHITECTURE.md) for the proposed boundaries.

## Technology direction

- **Rust:** application services, project/document model, commands, persistence, and background work.
- **C++ and OCCT:** CAD kernel operations and jewellery geometry.
- **Qt 6:** Windows desktop interface and viewport integration.
- **CMake and Cargo:** native and Rust builds.

The command-line editor, Rust-to-C++ bridge, and first OCCT-backed kernel operation are implemented and tested. The Qt desktop is still planned and has not been integrated.

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
cargo run -p openjewelcad-cli -- box new sample.ojcad 2 3 4
cargo run -p openjewelcad-cli -- box edit sample.ojcad 2 3 5
cargo run -p openjewelcad-cli -- inspect sample.ojcad
```

The first configure builds or restores OCCT and its dependencies and can take a long time.

## Project documents

- [Architecture](ARCHITECTURE.md)
- [Development roadmap](DEVELOPMENT_ROADMAP.md)
- [Technical decisions](TECHNICAL_DECISIONS.md)
- [Contributor and agent instructions](AGENTS.md)

## License

No project license has been selected. Until a license is added, all rights are reserved. The selected OCCT distribution includes LGPL 2.1 with an additional exception; its notices and obligations must be documented before distributing builds.
