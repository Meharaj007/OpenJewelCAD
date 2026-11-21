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
- **Decision:** Expose a version-reporting C ABI, opaque shape handles, fixed-layout error values, and explicit shape destruction. Catch C++ exceptions at every operation boundary.
- **Reason:** C ABI limits coupling and avoids exposing unstable C++ or OCCT types to Rust.
- **Trade-off:** More explicit conversion code and API versioning work.
- **Validation:** Rust checks the ABI version, owns handles with drop-based release, and integration-tests box construction, validity, volume, and structured argument errors.

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

- **Status:** Open
- **Decision:** No source or product license has been selected.
- **Reason:** OCCT obligations, third-party dependencies, and commercial distribution must be considered together.
- **Next step:** Decide before publishing a license or shipping binaries.

## TD-007 — Pin OCCT through the vcpkg manifest

- **Status:** Accepted for the initial Windows kernel spike
- **Decision:** Use the vcpkg manifest and its pinned registry baseline to resolve Open CASCADE Technology 8.0.1 for the x64 Windows MSVC triplet.
- **Reason:** A manifest baseline makes the dependency selection reproducible across developer machines and CI.
- **Trade-off:** The first build compiles a large native dependency set and can take substantial time.
- **License:** OCCT 8.0.1 is distributed under LGPL 2.1 with the Open CASCADE exception; review and include the corresponding notices before distributing the application.
- **Validation:** Installed and built on Windows; the OCCT box-volume/validity smoke test passes.
