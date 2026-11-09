# Analytical heat-diffusion fixtures

These files are manufactured analytical reference data for evaluating existing
SVF capabilities. They are not output from a numerical heat solver.

## Reproduce

From the repository root, with Python 3.10 or newer:

    python tools/analytical-heat-diffusion/generate.py
    python tools/analytical-heat-diffusion/verify.py

The generator has no third-party dependencies. It overwrites the two committed
VTK files deterministically. The verifier independently parses those files;
it does not import or call the generator.

## Files

| File | Points | Tetrahedra | Exterior triangles |
|---|---:|---:|---:|
| heat-diffusion-coarse.vtk | 125 | 384 | 192 |
| heat-diffusion-fine.vtk | 729 | 3,072 | 768 |

Each VTK file contains point-associated scalar arrays
temperature_t0, temperature_t1, probe_linear and vector arrays
heat_flux_t0, heat_flux_t1.

## Scientific definition

- Domain: \([0,1]^3\), coordinates in metres.
- Thermal diffusivity: \(\alpha=1\;\mathrm{m^2/s}\).
- Thermal conductivity: \(k=1\;\mathrm{W/(m\,K)}\).
- Temperature unit: K.
- Heat-flux unit: W/m².
- Probe unit: dimensionless.

The exact temperature is

\[
T=e^{-3\pi^2\alpha t}\sin(\pi x)\sin(\pi y)\sin(\pi z).
\]

The snapshots are \(t_0=0\) s and
\(t_1=\ln(2)/(3\pi^2\alpha)\approx0.02341016425756096\) s.
The second snapshot is exactly one half of the first.

The supplied heat flux is the analytical Fourier flux
\(\mathbf q=-k\nabla T\). SVF is not asked to derive it.

The independent probe is dimensionless:

\[
P=\frac{x}{1\;\mathrm m}
 +2\frac{y}{1\;\mathrm m}
 +3\frac{z}{1\;\mathrm m}.
\]

Here \(x,y,z\) are the numerical coordinates expressed in metres. This
linear field is included to test interpolation and transfer independently
of the temperature field.

## Mesh construction

Point order is X-fastest, then Y, then Z. Every grid cell is decomposed into
six tetrahedra by the Freudenthal construction: the six axis permutations
define paths from the cell's lower corner to its upper corner. A deterministic
orientation correction gives every tetrahedron positive signed volume.
Adjacent cells share the same face triangulation.

The generator evaluates known endpoint and centre trigonometric values
exactly. This makes the Dirichlet boundary temperature exactly zero and the
centre heat-flux vector exactly zero in the stored data, rather than leaving
small trigonometric round-off residues.

## Independent reference checks

The verifier checks:
- exact point/cell counts and deterministic point order;
- tetrahedron connectivity, positive signed volume and manifold face
  incidence;
- six exterior cube faces, expected boundary triangle counts and unit face
  areas;
- all five field names, tuple counts, associations and analytical values;
- zero boundary temperature, positive interior temperature, centre values,
  factor-of-two snapshot relations and outward normal heat flux;
- the continuous analytical temperature residual at edge intersections
  of the piecewise-linear \(T=0.2\) K isosurface.

Expected maximum continuous isosurface residuals are approximately:
- coarse: 0.150410743745 K;
- fine: 0.0333587000791 K.

The fine fixture must improve on the coarse fixture and remain below 0.04 K.
This is geometric approximation error; the piecewise-linear extracted
vertices lie on the interpolated isovalue by construction.

The verifier uses absolute tolerance \(10^{-12}\) for analytical field and
mesh checks. It checks the written decimal values after parsing them back
from the VTK files.

## SVF commands

From the repository root:

    cargo run -p svf-explorer -- --vtk fixtures/analytical-heat-diffusion/heat-diffusion-coarse.vtk

    cargo run -p svf-explorer -- --vtk fixtures/analytical-heat-diffusion/heat-diffusion-fine.vtk

    cargo run -p svf-explorer -- --vtk fixtures/analytical-heat-diffusion/heat-diffusion-fine.vtk --vtk-scalar temperature_t0

    cargo run -p svf-explorer -- --vtk fixtures/analytical-heat-diffusion/heat-diffusion-fine.vtk --vtk-isosurface temperature_t0 --vtk-isovalue 0.2 --vtk-isosurface-color probe_linear

    cargo run -p svf-explorer -- --vtk fixtures/analytical-heat-diffusion/heat-diffusion-fine.vtk --vtk-vectors heat_flux_t0

    cargo run -p svf-explorer -- --vtk fixtures/analytical-heat-diffusion/heat-diffusion-fine.vtk --vtk-scalar temperature_t0 --vtk-vectors heat_flux_t0

The boundary temperature field is identically zero, so the imported boundary
scalar range is constant \([0,0]\). The renderer's existing constant-field
mapping policy applies; do not interpret the boundary color as an interior
temperature gradient.

The two snapshots are separate static arrays, not time-series metadata.
