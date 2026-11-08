# Scientific Visualization Framework

A Rust-first framework for interactive scientific and mathematical visualization.

The initial target is Windows desktop. The framework is designed as a reusable
Rust library; `apps/explorer` is a small host application used to exercise the
library during development.

## Current status

- **M0–M4 — Foundation, spatial math, scientific geometry, and mathematical
  sampling:** implemented.
- **M5 — Interactive surface demonstrator:** indexed surface rendering,
  parametric surface updates, viewport unprojection, and scientific picking
  implemented and user-validated on Windows.
- **M6.1 — Scientific field semantics:** implemented; Windows workspace test
  run reported 94 passing tests.
- **M6.2 — Dataset containers and attribute attachment:** merged to `main`;
  user-reported Windows validation passed (103 workspace tests).
- **M6 — Scientific datasets and unstructured surfaces:** M6.1–M6.6 are
  implemented and merged to `main`; M6.6 passed Windows validation with
  153 workspace tests (148 core, 1 Explorer, 4 renderer).
- **M7.1 — Unstructured boundary rendering:** merged and user-validated;
  mixed hexahedron/tetrahedron fixture renders 16 provenance-mapped triangles.
- **M7.2 — Reusable scalar visualization:** merged and user-validated;
  structured point and unstructured cell scalar fields render through the
  common core mapping pipeline.
- **M7.3 — Vector field visualization:** merged and user-validated;
  core resolves point/cell vectors and builds provenance-preserving glyphs,
  while Explorer `--vectors` demonstrates point-associated vectors.
- **M7.4 — Structured cell-associated vectors:** merged and user-validated
  on Windows; adds a 64-cell structured-grid demo with explicit-length glyphs.
  The initial fixed-camera occlusion check was inconclusive; M7.5 now
  enables inspection from multiple camera angles.
- **M7.5 — Interactive 3D camera navigation:** implemented, user-validated
  on Windows, and merged through PR #13; orbit, pan, and wheel zoom work
  across all six Explorer modes.

See [Milestones](docs/MILESTONES.md) for scope and validation details, and
[HANDOFF.md](HANDOFF.md) for the current development state.

## Requirements

- Rust stable (minimum 1.87 for the selected wgpu release)
- Windows 10/11, x86_64
- A graphics driver supporting Direct3D 12 or Vulkan
- Visual Studio Build Tools with the MSVC C++ workload and Windows SDK when
  using the `*-pc-windows-msvc` Rust target

## Run

```powershell
cargo run -p svf-explorer
```

Run tests:

```powershell
cargo test --workspace
```

Check formatting and linting:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Workspace

```text
scientific-visualization-framework/
├── Cargo.toml
├── rust-toolchain.toml
├── crates/
│   ├── svf-core/       # Scientific/domain types; no graphics dependency
│   └── svf-render/     # wgpu rendering infrastructure
├── apps/
│   └── explorer/       # Minimal Windows host application
└── docs/
    ├── ARCHITECTURE.md
    └── MILESTONES.md
```

## Design constraints

- Scientific data and algorithms must not depend on GPU types.
- Rendering is a projection of scientific data, not the source of truth.
- The core library must remain usable without the Explorer application.
- GPU-facing code may use `f32`; scientific datasets and calculations must
  preserve `f64` where accuracy requires it.
- The initial renderer is deliberately unlit and minimal. Scientific
  communication, not photorealism, is the objective.

## Dependency policy

Dependencies are added only when a concrete subsystem needs them. In
particular, no ECS, general-purpose scene editor, or broad numerical array
framework is included in the initial workspace.

The initial graphics dependencies are wgpu, winit, and pollster (for blocking
the asynchronous renderer initialization during this small native bootstrap).
