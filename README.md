# Open Jewel CAD

Open Jewel CAD is an early-stage, Windows-first jewellery CAD project. The long-term goal is a professional design and manufacturing application built around accurate geometry, editable parameters, and dependable workflows.

> **Project stage:** the first desktop editor can create and resize a box, rotate its preview, save projects, and regenerate a checked OCCT solid. Jewellery-specific tools and a kernel-meshed viewport are still to come.

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
- A Rust desktop editor with millimetre dimension controls, a draggable shaded box preview, project open/save, and OCCT regeneration and validation.
- A command-line interface with a versioned `.ojcad` project file and create/edit/inspect commands.
- Unit tests for geometry and feature-graph behavior.
- A CMake/vcpkg smoke test that builds a 2×3×4 mm OCCT box and verifies its 24 mm³ volume.

The Rust workspace check and fifteen unit/integration tests pass. The command-line editor has been exercised end to end, and the desktop editor has been launched and used to regenerate a valid OCCT box. The OCCT Debug build and CMake smoke test also pass on the current Windows machine.

## What is next

1. Expand kernel primitives, transforms, and geometry regression coverage.
2. Extend the document model with persistence, typed commands, parameters, regeneration, and undo/redo.
3. Extend the desktop editor with an OCCT-meshed viewport, feature history, and undo/redo.
4. Add jewellery and gemstone workflows incrementally with manufacturing-oriented checks.

See the [development roadmap](DEVELOPMENT_ROADMAP.md) for stage gates and the [architecture](ARCHITECTURE.md) for the proposed boundaries.

## Technology direction

- **Rust:** application services, project/document model, commands, persistence, background work, and the current eframe/egui desktop prototype.
- **C++ and OCCT:** CAD kernel operations and jewellery geometry.
- **CMake and Cargo:** native and Rust builds.

The desktop preview is a rotatable shaded box drawn from its parameters. Press **Regenerate solid** to create the corresponding OCCT shape and report its validity and volume; the viewport does not yet display OCCT's tessellated geometry.

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
cargo run -p openjewelcad-cli -- inspect sample.ojcad
```

The first configure builds or restores OCCT and its dependencies and can take a long time.
The Rust desktop editor uses eframe/egui and requires Rust 1.95 or newer. On this Windows machine the optimized editor executable is 6.0 MB; the OCCT kernel and its runtime DLLs are separate. Build the OCCT C++ kernel before launching the editor.

## Project documents

- [Architecture](ARCHITECTURE.md)
- [Development roadmap](DEVELOPMENT_ROADMAP.md)
- [Technical decisions](TECHNICAL_DECISIONS.md)
- [Contributor and agent instructions](AGENTS.md)

## License

No project license has been selected. Until a license is added, all rights are reserved. The selected OCCT distribution includes LGPL 2.1 with an additional exception; its notices and obligations must be documented before distributing builds.
