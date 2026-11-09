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

- **M8.1 — Tetrahedral isosurface extraction:** implemented, user-validated on Windows, and merged to `main` through PRs #16 and #17. Adds deterministic, provenance-preserving marching tetrahedra and the `--isosurface` Explorer mode.
- **M8.2 — Provenance-based scalar transfer:** implemented, Windows-validated, and merged through PRs #20 and #21. Adds provenance-safe point-scalar transfer, mapped-isosurface rendering, and the `--colored-isosurface` Explorer mode.

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


- **M9.1 — VTK legacy ASCII unstructured-grid import:** implemented and merged through PR #23. Adds a graphics-independent `svf-core` parser for supported mixed-cell unstructured grids and point/cell scalar/vector fields. User-validated on Windows: 12 focused tests, 240 workspace tests, and `cargo check --workspace` passed. Explorer file loading is deferred to M9.2.


- **M9.2 — Explorer VTK dataset loading:** implemented, Windows-validated, and merged through PR #25. Adds `--vtk <path>` and optional `--vtk-scalar <field-name>` for VTK legacy ASCII unstructured-grid files. Renders tetrahedral/hexahedral volume boundaries, supports point/cell scalar coloring, and fits the initial camera to imported geometry. User validation: 249 workspace tests passed, doc-tests passed, and `cargo check --workspace` passed.

- **M9.3 — VTK isosurface extraction:** implemented, Windows-validated, and merged through PR #27. Adds `--vtk-isosurface`, finite isovalue selection, optional independent point-scalar coloring via `--vtk-isosurface-color`, and a depth-tested triangle-edge overlay. User validation: 258 workspace tests passed (216 core, 35 Explorer, 7 renderer), doc-tests and `cargo check --workspace` passed; both triangles and independent pressure coloring were visually confirmed.


- **M10.1 — Imported scientific-surface inspection:** implemented, Windows-validated, and merged through PR #29. Adds provenance-aware ray inspection for VTK boundary surfaces and tetrahedral isosurfaces, including barycentric point-field values and exact source-cell values. User validation: 262 workspace tests passed, doc-tests and `cargo check --workspace` passed; runtime inspection and camera interactions were confirmed.


- **M10.2 — Imported VTK point-vector visualization:** implemented and merged through PR #31. Adds point-associated vector glyphs to imported VTK boundary scenes, optionally alongside scalar surface coloring. User-reported Windows tests and workspace check passed; vector rendering was visually confirmed.

**Development direction:** after M10.2, work is driven by the user's own scientific and engineering use cases. No further general-purpose milestone is selected automatically.


- **E1 — Chemistry composition core:** implemented, Windows-validated, and merged through PR #33. Adds graphics-independent element, atom, bond, and molecule types with structural validation. User validation: 16 chemistry tests, 236 core tests, 283 workspace tests, and `cargo check --workspace` passed.


- **E2 — Chemistry molecular views:** implemented, Windows-validated, and merged through PR #35. Adds graphics-independent ball-and-stick representations preserving atom/bond identity and semantic bond order, with deterministic multiple-bond axes. User validation: 10 focused tests, 246 core tests, 293 workspace tests (246 core, 40 Explorer, 7 renderer), and `cargo check --workspace` passed. See [design contract](docs/chemistry-molecular-views.md).


- **E3 — Upper-secondary chemistry VSEPR learning case:** design merged through PR #37 and bounded electron-domain semantics implemented through PR #39. Adds stable domain identities, bonding/lone-pair classification, and validated methane/water VSEPR centres in `svf-core`. User validation: 259 core tests, 306 workspace tests (259 core, 40 Explorer, 7 renderer), and `cargo check --workspace` passed. See [learning case](docs/upper-secondary-chemistry-vsepr-learning-case.md) and [VSEPR model design](docs/vsepr-electron-domain-design.md).


- **E3.1/E3.2 — VSEPR electron-domain model:** design merged through PR #38; bounded semantic core implemented and merged through PR #39. Supports only the Senior Five CH₄/H₂O four-domain cases, with strict source provenance and no geometry inference from molecular coordinates. User validation: 259 core tests, 306 workspace tests, and `cargo check --workspace` passed. No schematic adapter, renderer, or general VSEPR engine has been added. See [design contract](docs/vsepr-electron-domain-design.md).


- **E3.3 — VSEPR schematic core:** implemented and Windows-validated. Adds a deterministic dimensionless tetrahedral electron-domain schematic preserving domain identity and bonding provenance, with molecular shape kept separate. User validation: 5 focused tests, 264 core tests, 311 workspace tests (264 core, 40 Explorer, 7 renderer), and `cargo check --workspace` passed. A standalone Explorer sample for methane and water is the next bounded evaluation step. See [VSEPR schematic design](docs/vsepr-schematic-view.md) and [Milestones](docs/MILESTONES.md).


- **E3.4 — Standalone VSEPR Explorer sample:** implemented, Windows-validated, visually inspected, and merged through [PR #42](https://github.com/Aine-dickson/sciviz/pull/42). Run `cargo run -p svf-explorer -- --vsepr` to display methane (left) and water (right). User validation: 3 focused Explorer tests; 315 workspace tests (265 core, 43 Explorer, 7 renderer); `cargo check --workspace` passed. Five camera angles showed both groups clearly separated without obvious clipping or overlap. Bonding-pair and lone-pair domains use distinct marker geometry; molecular shape is labelled separately. The static schematic does not show hydrogen atoms, molecular bonds, or an H–O–H angle, so molecular shape is not independently apparent from geometry alone. See [handoff](HANDOFF.md) and [milestones](docs/MILESTONES.md).


- **Analytical heat-diffusion challenge (design proposal):** defines a manufactured exact temperature and Fourier heat-flux dataset on coarse/fine tetrahedral meshes, with a linear probe field. Intended to stress existing VTK import, boundary/scalar visualization, isosurfaces, provenance transfer, vector glyphs, and inspection without adding a heat solver. No fixture or implementation is included pending design review. See [challenge specification](docs/analytical-heat-diffusion-challenge.md).
