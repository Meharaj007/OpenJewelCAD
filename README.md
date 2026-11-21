# Open Jewel CAD

Open Jewel CAD is an early-stage, Windows-first jewellery CAD project. The long-term goal is a professional design and manufacturing application built around accurate geometry, editable parameters, and dependable workflows.

> **Project stage:** foundational Rust geometry and document model. This is not yet a desktop CAD application; OCCT kernel integration is in progress.

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
- Unit tests for geometry and feature-graph behavior.

The Rust workspace check and all eleven unit tests passed on the current Windows development machine.

## What is next

1. Complete the Windows C++/OCCT dependency setup and compile the first real headless kernel operation.
2. Establish and test the Rust/C++ C ABI with owned shape handles and structured errors.
3. Extend the document model with persistence, typed commands, parameters, regeneration, and undo/redo.
4. Build the Qt desktop shell and viewport after the kernel/API boundary is validated.
5. Add jewellery and gemstone workflows incrementally with geometry regression tests.

See the [development roadmap](DEVELOPMENT_ROADMAP.md) for stage gates and the [architecture](ARCHITECTURE.md) for the proposed boundaries.

## Technology direction

- **Rust:** application services, project/document model, commands, persistence, and background work.
- **C++ and OCCT:** CAD kernel operations and jewellery geometry.
- **Qt 6:** Windows desktop interface and viewport integration.
- **CMake and Cargo:** native and Rust builds.

These are planned boundaries. Qt integration and OCCT kernel code are not yet part of the built application.

## Build the Rust foundation (Windows)

Install Rust with the MSVC toolchain and Visual Studio's **Desktop development with C++** workload. From the repository root:

```powershell
cargo check --workspace
cargo test --workspace
```

The C++/OCCT build instructions will be added after the kernel dependency setup is reproducible.

## Project documents

- [Architecture](ARCHITECTURE.md)
- [Development roadmap](DEVELOPMENT_ROADMAP.md)
- [Technical decisions](TECHNICAL_DECISIONS.md)
- [Contributor and agent instructions](AGENTS.md)

## License

No project license has been selected. Until a license is added, all rights are reserved. The selected OCCT distribution includes LGPL 2.1 with an additional exception; its notices and obligations must be documented before distributing builds.
