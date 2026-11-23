# Open Jewel CAD 0.1.0-alpha.1

This is an early Windows x64 preview of Open Jewel CAD. It is for trying the current modeling workflows and sharing feedback. It is not a finished professional CAD or manufacturing system.

## In this preview

- Dark desktop workspace with a project manager, recent projects, file browser, clipped multi-view viewport, and searchable tool palette.
- Parametric `.ojcad` projects, saved in project format version 21. Existing versions 1–20 remain readable.
- OCCT-based primitives, editable curves, planar faces, directional extrusions, partial and full revolved surfaces, closed-profile extrusions, and full revolved solids.
- Transforms, mirrors, linear and circular arrays, primitive-based boolean operations, measurements, validity checks, and STL/OBJ export.
- A Windows portable build with the OCCT runtime DLLs and third-party notices.

## Known limitations

- This alpha is Windows x64 only and requires a compatible Microsoft Visual C++ runtime.
- The editor is a prototype. It does not yet have a complete feature tree with dependable history, undo/redo, or robust recovery after invalid edits.
- STEP/IGES/3DM import and export, mesh booleans, advanced jewellery-specific tools, rendering, and sculpting are not included.
- STL/OBJ export is available, but manufacturing suitability still needs independent validation.
- The current viewport is a lightweight preview, not a full production visualization or rendering system.
- The native OCCT ABI currently supports one revolved surface or solid profile at a time; broader topology editing and selection remain planned.

## Run

Extract the ZIP and double-click `Start-OpenJewelCAD-Portable.bat`. Project files are saved as `.ojcad`. Keep backups of important work while evaluating this alpha.

## License

Project-authored code is MIT licensed. OCCT and bundled dependencies keep their own licenses; see `THIRD_PARTY_NOTICES` in the package. MIT permits redistribution, including paid redistribution, when its terms and third-party terms are followed.

## Verification

The package script runs the Rust workspace tests, the native OCCT smoke test, and Release builds before creating the ZIP. It is a local preparation script and does not publish a release.
