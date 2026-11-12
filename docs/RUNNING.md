# Running everything

One page for running every demo, structure, dataset view, test and tool in this
repository. Commands are PowerShell, from the repository root (`sci_viz`).
Every Explorer command opens a 1280x800 window; close it to end the run.
Flags were taken from the Explorer's argument parser; if a command here stops
working, `cargo run -p svf-explorer -- --bogus` prints the current usage text.

## 0. Before anything

```powershell
cargo test --workspace          # all automated tests (several minutes in debug)
cargo check --workspace
```

Requirements: Rust stable, Windows 10/11, a Direct3D 12 or Vulkan driver.
The workspace has one known warning (`VectorSource3::new` is never used).
Do not run `cargo fmt` as cleanup (standing rule in `HANDOFF.md`).

## 1. Mouse and keys (all Explorer modes)

| Input | Effect |
|---|---|
| Left drag | Orbit |
| Middle drag | Pan |
| Wheel | Zoom |
| Left click (no drag) | Pick: part, or surface point, with its scientific value in the title or console |
| Up / Down arrow (or numpad + / -) | Change the amplitude of the default parametric surface |
| `L` | Light on / off |

Structure mode adds: `I` isolate the picked part, `A` show all and restore layers,
`S` cycle the section (off, across, along), `[` and `]` move the plane, `F` flip
the kept side, `P` peel the outermost layer, `R` restore one, `V` next named view.

## 2. Built-in demos (no files needed)

```powershell
cargo run -p svf-explorer                                # parametric surface, arrows change amplitude
cargo run -p svf-explorer -- --unstructured              # hexahedron + tetrahedron boundary surface
cargo run -p svf-explorer -- --unstructured-scalar       # same, coloured by a scalar field
cargo run -p svf-explorer -- --unstructured-extra        # extra unstructured sample
cargo run -p svf-explorer -- --scalar                    # structured surface with scalar colouring
cargo run -p svf-explorer -- --vectors                   # vector glyphs on an undulating surface
cargo run -p svf-explorer -- --cell-vectors              # cell-centred vector glyphs
cargo run -p svf-explorer -- --isosurface                # tetrahedral isosurface
cargo run -p svf-explorer -- --colored-isosurface        # isosurface coloured by a transferred field
cargo run -p svf-explorer -- --vsepr                     # VSEPR: methane and water
```

Design notes: `docs/unstructured-rendering.md`, `scalar-visualization.md`,
`vector-field-visualization.md`, `tetrahedral-isosurface.md`,
`isosurface-field-transfer.md`, `vsepr-schematic-view.md`.

## 3. Explorable structures (B2)

```powershell
cargo run -p svf-explorer -- --structure fixtures/structures/dicot-stem.ron
cargo run -p svf-explorer -- --structure fixtures/structures/cockroach.ron
cargo run -p svf-explorer -- --structure fixtures/structures/cockroach.ron --view gut
```

Options: `--view <id>`, `--section <x|y|z>=<offset>`, `--section-flip`,
`--peel <layers>`, `--camera-yaw <deg>`, `--camera-pitch <deg>`,
`--camera-zoom <factor>`, `--screenshot <out.png>` (renders one 1600x1200 frame
and exits). An unknown `--view` prints the ids that exist.

Cockroach views: `dorsal`, `side`, `wings-off`, `dorsal-wall-off`, `gut`,
`reproductive`, `tracheae`, `nerve-cord`, `gut-only`, `sagittal`.
Tests for the fixtures: `cargo test -p svf-core --test cockroach_fixture`.
Design notes: `docs/structure-recipes.md`, `sections-and-peeling.md`,
`cockroach-structure.md`, `explorable-structures.md`.

Mesh-based structures (the heart, step 5e) use `Geometry::Mesh` and a `.svfmesh`
file in the same folder as the `.ron`; the Explorer prints the data attribution at
startup. The command will be listed here when the fixture is committed.

## 4. VTK datasets

```powershell
cargo run -p svf-explorer -- --vtk apps/explorer/examples/tetra-with-fields.vtk
cargo run -p svf-explorer -- --vtk <file.vtk> --vtk-scalar <field> [--vtk-scalar-exclude <value>]...
cargo run -p svf-explorer -- --vtk <file.vtk> --vtk-vectors <field>
cargo run -p svf-explorer -- --vtk <file.vtk> --vtk-isosurface <field> --vtk-isovalue <v> [--vtk-isosurface-color <point-scalar>]
cargo run -p svf-explorer -- --vtk <file.vtk> --vtk-cell-axial <vector-field> --vtk-cell-tags <field>=<v1,v2,...> [--vtk-axial-bin <mm>] [--vtk-axial-length <mm>]
```

Camera and capture flags also work with `--vtk`: `--camera-yaw`, `--camera-pitch`,
`--camera-zoom`, `--screenshot`.

Analytical heat-diffusion fixtures (committed):

```powershell
cargo run -p svf-explorer -- --vtk fixtures/analytical-heat-diffusion/heat-diffusion-fine.vtk --vtk-isosurface temperature_t0 --vtk-isovalue 0.2 --vtk-isosurface-color probe_linear
cargo run -p svf-explorer -- --vtk fixtures/analytical-heat-diffusion/heat-diffusion-fine.vtk --vtk-vectors heat_flux_t0
python tools/analytical-heat-diffusion/generate.py     # regenerate (deterministic)
python tools/analytical-heat-diffusion/verify.py       # independent check
```

Notes: `docs/explorer-vtk-loading.md`, `explorer-vtk-isosurface.md`,
`vtk-point-vector-visualization.md`, `cell-vector-visualization.md`,
`analytical-heat-diffusion-challenge.md`.

## 5. The heart (Rodero case 01; the data is not committed)

The data: Rodero et al. 2021, Zenodo 4590294 (CC BY 4.0), archive `01.tar.gz`,
file `01.vtk` (about 260 MB). `01-svf-order.vtk` is `01.vtk` reordered by
`reorder_vtk.py` for SVF's reader. In the commands below `$H` is the folder that
holds them (on this machine `~/Downloads/rodero2021`).

```powershell
$H = "$HOME/Downloads/rodero2021"
cargo run -p svf-explorer --release -- --vtk $H/01-svf-order.vtk                      # whole heart boundary
cargo run -p svf-core --release --example vtk_inspect -- $H/01-svf-order.vtk          # what is in the file
cargo run -p svf-core --release --example heart_audit -- $H/01-svf-order.vtk          # 5a: cavities, non-manifold, budget
cargo run -p svf-core --release --example heart_regions -- $H/01-svf-order.vtk        # 5b: closed parts and volumes
cargo run -p svf-core --release --example fibre_diagnostic -- $H/01-svf-order.vtk --source $H/01.vtk --tags 1,2 --lv-tag 1 --bin 4 --length 3 --out svf-case01.json --glyphs glyphs.csv
python -I tools/rodero-fibre-audit/test_audit.py                                       # audit self-tests
python -I tools/rodero-fibre-audit/audit.py $H/01.vtk --tags 1,2 --lv-tag 1 --bin 4 --out tools/rodero-fibre-audit/baseline/case01.json
python -I tools/rodero-fibre-audit/compare.py tools/rodero-fibre-audit/baseline/case01.json svf-case01.json
```

Always use `--release` with the real heart: debug builds are many times slower.
Expected SHA-256 of `01.vtk`: `20bf29b8ec86454de4cfde2a0afc19992715107b6ef8a7dabe61f52e8f78f440`.
Reports from earlier runs: `docs/evidence/`. Notes: `docs/heart-structure.md`,
`heart-fibre-orientation.md`, `biology-heart-anatomy-source-audit.md`.

## 6. Where results are recorded

`HANDOFF.md` (current state and the next action), `docs/MILESTONES.md` (history),
`docs/evidence/` (reports and screenshots by step). Keep this page current when a
new subject, flag or tool is added.
