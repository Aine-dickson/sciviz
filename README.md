# Scientific Visualization Framework

A Rust-first framework for interactive scientific and mathematical visualization.

The initial target is Windows desktop. The framework is designed as a reusable
Rust library; `apps/explorer` is a small host application used to exercise the
library during development.

## Current status

- **M0 — Foundation:** workspace and architecture established.
- **M1 — Graphics foundation:** winit window, wgpu device/surface initialization,
  clear rendering, resize handling, and basic surface recovery.
- **M2 — Spatial foundation:** camera mathematics is locally validated; reference-scene renderer implementation is committed and awaiting local validation.

M1 is an initial implementation scaffold. It has not yet been compiled or run
in this environment because the Rust toolchain is unavailable here.

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
