# Analytical heat-diffusion challenge dataset and specification

## Status

Challenge design proposal. This document defines the analytical reference,
dataset contract, and evaluation criteria. It does not add fixture files,
production code, or new SVF capabilities. Those are authorized only after
this specification is reviewed and accepted.

## Objective

Evaluate the current standalone SVF capabilities against a single,
mathematically controlled 3D scientific dataset that combines:

- tetrahedral unstructured-grid topology;
- multiple point-associated scalar fields;
- multiple point-associated vector fields;
- imported volume-boundary visualization;
- scalar mapping and isosurface extraction;
- provenance-based transfer onto derived isosurfaces;
- vector source resolution and glyph construction; and
- source-aware inspection.

The challenge is a manufactured heat-diffusion solution. Its analytical
fields are known at every mesh point, so expected values and invariants can
be independently checked. It tests SVF as a scientific-data visualization
framework, not as a heat-equation solver.

## Scientific model

### Domain and material constants

Use the unit cube:

\[
\Omega = [0,1]^3
\]

Coordinates are in metres. Use thermal diffusivity
\(\alpha = 1\;\mathrm{m^2/s}\) and thermal conductivity
\(k = 1\;\mathrm{W/(m\,K)}\). These are deliberately convenient
manufactured constants, not a claim to model a particular material.

The temperature field satisfies the heat equation:

\[
\frac{\partial T}{\partial t} = \alpha \nabla^2 T
\]

with zero-temperature Dirichlet boundary conditions on all six cube faces.

### Exact temperature solution

Define

\[
T(x,y,z,t) =
e^{-3\pi^2\alpha t}
\sin(\pi x)\sin(\pi y)\sin(\pi z)
\quad [\mathrm K].
\]

This is an exact solution because

\[
\nabla^2 T = -3\pi^2 T,
\qquad
\frac{\partial T}{\partial t} = -3\pi^2\alpha T.
\]

Use two snapshots:

- \(t_0 = 0\;\mathrm s\)
- \(t_1 = \ln(2)/(3\pi^2\alpha)\;\mathrm s\)

The second snapshot is exactly half the first at every spatial point:

\[
T(x,y,z,t_1) = \tfrac12 T(x,y,z,t_0).
\]

### Exact heat-flux vector

Use Fourier's law:

\[
\mathbf q = -k\nabla T.
\]

For either snapshot, let \(A(t)=e^{-3\pi^2\alpha t}\). Then

\[
q_x = -k\pi A(t)\cos(\pi x)\sin(\pi y)\sin(\pi z),
\]
\[
q_y = -k\pi A(t)\sin(\pi x)\cos(\pi y)\sin(\pi z),
\]
\[
q_z = -k\pi A(t)\sin(\pi x)\sin(\pi y)\cos(\pi z).
\]

Units are \(\mathrm{W/m^2}\). The vector fields also halve exactly
between \(t_0\) and \(t_1\). SVF must not infer these vectors from the
scalar field: they are supplied as independently evaluated analytical
reference data.

### Independent linear probe field

Add one point scalar field:

\[
P(x,y,z)=x/(1\;\mathrm m)+2y/(1\;\mathrm m)+3z/(1\;\mathrm m).
\]

It is dimensionless because coordinates are normalized by one metre. It is
intentionally independent of temperature and exactly linear in space. It provides
a strong check of interpolation and provenance transfer onto extracted
isosurfaces: linear interpolation within a tetrahedron should reproduce
this field up to floating-point serialization error.

## Mesh definition

Generate two conforming tetrahedral meshes over the same unit cube.

| Fixture | Points per axis | Point count | Cube count | Tetrahedra |
|---|---:|---:|---:|---:|
| coarse | 5 | 125 | 64 | 384 |
| fine | 9 | 729 | 512 | 3,072 |

For each axis, coordinates are uniformly spaced and include both endpoints.
Point order is deterministic with X varying fastest, then Y, then Z.

Split each hexahedral grid cell into six tetrahedra using the Freudenthal
decomposition: for each of the six permutations of the X/Y/Z axes, form the
simplex from the cell's lower corner, the first cumulative axis step, the
second cumulative axis step, and the upper corner. Apply a deterministic
orientation correction so each tetrahedron has positive signed volume.
Adjacent cubes must share identical face triangulations; do not create
cracks, duplicate points, or nonconforming subdivisions.

Expected mesh counts are exact. Every tetrahedron must reference four
distinct in-range points and have positive nonzero volume. The outer boundary
must be the six cube faces, with no internal boundary faces.

## Dataset fields and file format

Produce one VTK legacy ASCII UNSTRUCTURED_GRID file per resolution.

Geometry and topology:

- POINTS: double precision coordinates in metres.
- CELLS: tetrahedra only.
- CELL_TYPES: VTK type 10 (TETRA) for every cell.
- Preserve deterministic point and cell order.

POINT_DATA contains these arrays, in this exact order:

1. temperature_t0 — SCALARS double, kelvin.
2. temperature_t1 — SCALARS double, kelvin.
3. probe_linear — SCALARS double, dimensionless.
4. heat_flux_t0 — VECTORS double, W/m².
5. heat_flux_t1 — VECTORS double, W/m².

The VTK legacy subset currently imported by SVF does not carry portable
units metadata for these arrays. Therefore the fixture README/manifest must
state the units, formulas, constants, time values, coordinate convention,
and field association. Do not encode units by guessing from field names.

All arrays are point-associated and must have exactly one tuple per point.
Use enough decimal digits to round-trip f64 values (17 significant decimal
digits is the target). Do not write NaN or infinity into this challenge
dataset; non-finite handling is not the focus of this test.

## Analytical invariants and reference values

The fixture generator and independent verifier must check:

1. **Boundary condition:** temperature is zero on all six cube faces,
   within the declared serialization tolerance.
2. **Interior positivity:** temperature is nonnegative everywhere and
   strictly positive at all strictly interior mesh points.
3. **Maximum:** at the centre (0.5, 0.5, 0.5), \(T(t_0)=1\;\mathrm K\)
   and \(T(t_1)=0.5\;\mathrm K\).
4. **Snapshot relation:** every node satisfies
   \(T(t_1)=0.5T(t_0)\) and
   \(\mathbf q(t_1)=0.5\mathbf q(t_0)\).
5. **Symmetry:** temperature is invariant under permutation of X/Y/Z
   and reflection of any coordinate about 0.5. The corresponding heat-flux
   components transform with the expected component permutation/reflection.
6. **Centre flux:** \(\mathbf q(0.5,0.5,0.5,t)=\mathbf 0\).
7. **Flux direction:** on each face, the normal component points outward
   for positive interior temperature; tangential components are retained
   according to the exact formula.
8. **Linear probe:** \(P\) equals \(x+2y+3z\) at every node.
9. **Heat-equation identity:** the closed-form expressions satisfy
   \(\partial_t T-\alpha\nabla^2T=0\). This is checked by the
   independent verifier/formula tests, not claimed as an SVF computation.

Use absolute tolerance \(10^{-12}\) for values calculated directly in
f64 by the fixture generator/verifier, and a separately documented
round-trip tolerance for values parsed from the decimal VTK text.

## SVF challenge workflow

Run the same sequence against both resolutions where applicable.

### A. Import and structural integrity

- Load each fixture through the existing --vtk path.
- Verify exact point and cell counts, tetrahedron-only topology, and all
  five field names.
- Verify all five arrays are point-associated and retain exact tuple order.
- Verify scalar values and vector components match the independent
  analytical verifier within serialization tolerance.
- Verify no existing parser/import semantics are changed to accommodate
  the fixture.

### B. Boundary surface

- Render the imported volume boundary.
- Confirm the result is one closed cube boundary, not internal tetrahedron
  faces.
- Confirm no missing faces, cracks, duplicated exterior patches, or
  inverted-face artifacts.
- Confirm camera fit, orbit, pan, zoom, resize, minimize, and restore remain
  functional.

### C. Temperature scalar visualization

- Map temperature_t0 on the boundary.
- Verify the boundary scalar range is exactly the constant interval [0, 0]
  within serialization tolerance. There is no low/high gradient on the
  boundary because the Dirichlet field is zero on all six faces.
- Verify the renderer applies its existing constant-field mapping policy;
  do not fabricate an interior temperature gradient on the boundary.

### D. Interior isosurface and independent scalar transfer

Extract the temperature_t0 isosurface at value 0.2 K:

    cargo run -p svf-explorer -- --vtk <fine-fixture.vtk> --vtk-isosurface temperature_t0 --vtk-isovalue 0.2 --vtk-isosurface-color probe_linear

Required checks:

- The isosurface is nonempty and entirely inside the cube.
- It is closed: every undirected output triangle edge has exactly two
  incident triangles. No boundary edges or non-manifold edges are expected.
- Every extracted vertex, evaluated through the extraction field's retained
  source provenance, reproduces 0.2 K within a tight interpolation tolerance.
- The transferred probe_linear value at every output vertex agrees with
  the exact analytical value \(x+2y+3z\) at that vertex within
  \(10^{-10}\) (plus serialization tolerance).
- Every output triangle retains a valid source tetrahedron ID.
- Compare the maximum continuous analytical temperature residual
  \(|T(x,y,z,t_0)-0.2|\) at extracted vertices between coarse and fine
  fixtures. The fine result must be lower; the fine maximum should be no
  greater than 0.04 K for this mesh and level. This measures geometric
  approximation error, not an error in the extracted piecewise-linear
  isovalue.

The boundary scalar being constant and the interior isosurface being
nontrivial are both intentional: they distinguish boundary representation
from volumetric field interpretation.

### E. Vector field visualization

Run the imported boundary vector path:

    cargo run -p svf-explorer -- --vtk <fine-fixture.vtk> --vtk-vectors heat_flux_t0

Required checks:

- Exactly one source vector is retained per source point, in original order.
- All source anchors equal the corresponding VTK point coordinates.
- The centre vector is zero and therefore has no arrow geometry.
- All nonzero finite source vectors retain their analytical direction.
- The current uniform-length glyph policy is reported and understood:
  arrow length represents display policy, not heat-flux magnitude.
- Boundary arrows directed into the volume may be naturally occluded; do
  not disable depth testing to make every arrow visible.
- Compare heat_flux_t1 against heat_flux_t0 at the source-data level:
  every component is half. Do not expect the current Explorer mode to display
  two vector fields simultaneously.

### F. Combined scalar and vector rendering

Run --vtk-scalar temperature_t0 --vtk-vectors heat_flux_t0 together.
Confirm both representations are prepared from the same unchanged Dataset.
Because the boundary temperature is constant, the scalar surface should
remain uniformly mapped while the heat-flux directions remain visible where
not naturally occluded.

### G. Inspection and provenance

Use the existing surface inspection workflow on the boundary and on the
derived isosurface.

- Boundary inspection resolves the correct source tetrahedron and boundary
  face.
- Point-associated temperature and probe values agree with independent
  barycentric interpolation at the hit location.
- Isosurface inspection resolves a valid source tetrahedron.
- The extraction field evaluates to 0.2 K within interpolation tolerance.
- probe_linear agrees with its exact linear value at the hit location.
- No source IDs are fabricated or lost through extraction, transfer, or
  inspection.

## Scope boundaries

This challenge evaluates existing SVF capabilities only. It does not
authorize new algorithms merely because the dataset suggests them.

Explicitly out of scope:

- A heat-equation solver, time integrator, or transient playback.
- Gradient, divergence, curl, Laplacian, or heat-flux computation inside SVF.
- Streamlines, pathlines, particle tracing, or vector integration.
- Volume rendering or interior slice planes.
- Hexahedral, structured-grid, binary VTK, or other new import support.
- Cell-associated or dataset-associated field visualization.
- Automatic unit inference, field-expression parsing, or a general metadata
  schema.
- New generic UI, legends, field pickers, or scene-editor infrastructure.
- Mava/Prismal integration.

The two time snapshots are separate named arrays in a static VTK dataset.
They do not imply time-series semantics in SVF.

## Acceptance and failure classification

The challenge passes only if:

1. The generator and independent verifier validate the analytical fields,
   mesh counts, topology, and all stated invariants.
2. Both VTK fixtures import without data loss or field-association changes.
3. Existing boundary, scalar, isosurface, transfer, vector, and inspection
   paths satisfy the checks above.
4. Coarse-to-fine isosurface approximation improves as specified.
5. Workspace tests and check pass on the user's Windows environment.
6. Runtime observations confirm the expected representations and interactions.

Classify failures rather than immediately adding features:

- **Data/generator failure:** the fixture violates the mathematical or mesh
  contract.
- **Importer failure:** correct VTK data is rejected or changed.
- **Core numerical/topology failure:** SVF produces incorrect derived values,
  topology, or provenance.
- **Representation/renderer limitation:** correct data exists but is hidden,
  misleading, or incorrectly framed.
- **Current-scope limitation:** the requested analysis (for example,
  computing a gradient from temperature) is not an existing SVF capability.
  Record it as a limitation; do not silently expand scope.

The fixture generator must be deterministic, and the committed VTK files
must be reproducible from the documented formulas and mesh algorithm.


## Fixture implementation status (2026-11-09)

The design was accepted and merged through PR #43 (merge commit
`0999f67058fe332a067667933cb30419e39910b8`). The deterministic fixture
generator and independent VTK verifier are now implemented on branch
`analytical-heat-diffusion-fixtures`, together with coarse and fine fixture
files and a fixture README.

- Generator: `tools/analytical-heat-diffusion/generate.py`.
- Independent verifier: `tools/analytical-heat-diffusion/verify.py`.
- Fixtures: `fixtures/analytical-heat-diffusion/heat-diffusion-coarse.vtk`
  and `heat-diffusion-fine.vtk`.
- The probe scalar is explicitly dimensionless, using coordinates normalized
  by one metre.
- The imported boundary temperature range is [0, 0], not a nonzero range
  endpoint. The renderer's existing constant-field policy is the expected
  behavior.
- The fine mesh's preliminary maximum continuous isosurface residual is
  approximately 0.0333587000791 K; the coarse result is approximately
  0.150410743745 K.
- Local execution of the generator/verifier and SVF has not yet been reported.
  Do not treat the challenge as passed until those checks are run.
