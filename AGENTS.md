# Open Jewel CAD project instructions

## Project status and scope

- Open Jewel CAD is a planned Windows desktop jewellery CAD application. Treat roadmap items as proposals until implemented and reviewed.
- Keep changes small, coherent, and traceable. Do not claim a feature works unless it has been implemented and appropriately verified.
- Do not begin broad product implementation while the architecture review is pending.

## Architecture boundaries

- Rust owns the eframe/egui desktop presentation, application orchestration, project/document services, command coordination, persistence policy, and background task scheduling.
- C++ owns CAD-kernel-facing geometry and jewellery algorithms. Keep the geometry library independent of desktop UI code.
- Keep presentation code free of CAD algorithms and direct OCCT object management. Use the safe Rust kernel wrapper and its versioned C ABI.
- Cross-language calls go through a narrow, versioned C ABI. Do not expose C++ classes, STL containers, exceptions, or OCCT handles across that boundary.
- Keep all geometry in millimetres internally. Convert at import/export and presentation boundaries, with explicit units.
- Geometry errors, tolerances, cancellation, and validity must be explicit; never silently accept invalid solids.

## Safety and reliability

- Never execute arbitrary AI-generated code or shell commands from the application. AI features must map validated requests to typed commands.
- Validate imported files and bound resource consumption where practical.
- Do not commit credentials, private keys, generated build trees, or machine-specific paths.
- Document new dependencies, versions, licenses, and platform constraints before adopting them.
- Preserve user data and existing behavior. Do not remove tests or suppress warnings to make a build pass.

## Verification and documentation

- For code changes, run the narrowest relevant build and tests, then report exactly what ran and any environment limits.
- Geometry tests should cover shape validity and tolerances as well as expected dimensions or volume where applicable.
- Update architecture and roadmap documents when implementation changes a decision or milestone.
- Do not claim Windows, OCCT, or Rust/C++ integration support until it has been verified in CI or a documented local build.

## Git workflow

- Keep `main` releasable. Use focused branches and reviewable commits for implementation work.
- Do not push or publish without an explicit request. Do not commit broken builds as stable releases.
