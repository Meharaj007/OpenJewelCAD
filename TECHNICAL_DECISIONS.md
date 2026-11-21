# Technical decisions

Record accepted decisions here. Proposed items remain open until reviewed; do not infer approval from their presence.

## TD-001 — Separate application orchestration from geometry

- **Status:** Proposed
- **Decision:** Rust coordinates application services and document workflows; a C++ library owns OCCT geometry operations; Qt handles desktop presentation.
- **Reason:** This follows the project’s language goals while keeping geometry testable without the GUI.
- **Trade-off:** Adds FFI, packaging, and debugging complexity across two toolchains.
- **Validation:** Gate 1 must build a Windows integration spike and exercise ownership, errors, cancellation, and shutdown.

## TD-002 — Use an opaque C ABI for Rust/C++ communication

- **Status:** Proposed
- **Decision:** Expose versioned C functions, opaque handles, fixed-layout data, explicit allocation/free routines, and structured error codes. Catch all C++ exceptions at the ABI boundary.
- **Reason:** C ABI limits coupling and avoids exposing unstable C++ or OCCT types to Rust.
- **Trade-off:** More explicit conversion code and API versioning work.
- **Validation:** Add ABI compatibility and lifetime tests in Gate 1.

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

## TD-005 — Defer exact dependency versions and packaging selection

- **Status:** Open
- **Decision:** Select exact Qt, OCCT, Rust, compiler, and package-manager versions only after a Windows build spike and license review.
- **Reason:** The brief names technologies but does not establish compatible pinned versions or a distribution model.
- **Next step:** Record versions, platform matrix, and license inventory at Gate 1.

## TD-006 — License and distribution model

- **Status:** Open
- **Decision:** No source or product license has been selected.
- **Reason:** Qt edition, OCCT obligations, third-party dependencies, and commercial distribution must be considered together.
- **Next step:** Decide before publishing a license or shipping binaries.

## TD-007 — Pin OCCT through the vcpkg manifest

- **Status:** Accepted for the initial Windows kernel spike
- **Decision:** Use the vcpkg manifest and its pinned registry baseline to resolve Open CASCADE Technology 8.0.1 for the x64 Windows MSVC triplet.
- **Reason:** A manifest baseline makes the dependency selection reproducible across developer machines and CI.
- **Trade-off:** The first build compiles a large native dependency set and can take substantial time.
- **License:** OCCT 8.0.1 is distributed under LGPL 2.1 with the Open CASCADE exception; review and include the corresponding notices before distributing the application.
- **Validation:** Installed and built on Windows; the OCCT box-volume/validity smoke test passes.
