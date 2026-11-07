# Sampling quality and resolution

This note describes the guarantees and limitations of the current uniform scalar sampler.

## Points, intervals, and spacing

`SamplingSpec1D` and `SamplingSpec2D` counts are numbers of sample points, not intervals. A request for N points creates N - 1 intervals and includes both domain endpoints.

For a finite domain [a, b] with N >= 2 uniformly spaced points, nominal spacing is:

    h = (b - a) / (N - 1)

For a 2D domain, spacing is calculated independently:

- `h_x = (x_end - x_start) / (N_x - 1)`
- `h_y = (y_end - y_start) / (N_y - 1)`

The axes can have different point counts and spacing. Increasing a point count while holding the domain fixed reduces nominal spacing on that axis.

## Storage and geometry growth

A 1D sample set stores N values. A 2D sample set stores N_x * N_y values, subject to checked size arithmetic and available memory.

A structured 2D grid has (N_x - 1) * (N_y - 1) cells. A fully finite sampled graph surface emits at most two triangles per cell: 2 * (N_x - 1) * (N_y - 1). A cell containing any non-finite corner is omitted in full, so the actual triangle count can be lower.

Doubling both axis point counts roughly quadruples the sample count and, for large grids, the potential cell and triangle count. Resolution therefore affects CPU evaluation time, memory use, mesh size, and eventually rendering cost.

## What uniform sampling tells you

Uniform sampling gives known sample locations and deterministic coverage of the requested domain. It does not reveal what happens between samples by itself.

The generated graph-surface mesh maps the sampled domain onto the world XZ plane and scalar values onto world Y, connecting neighboring samples with planar triangles. It is a piecewise-linear approximation of the sampled function, not an exact representation of the underlying function.

Features narrower than the sample spacing can be missed or misrepresented. A sharp peak may fall between sample locations; rapid oscillation may be undersampled; discontinuities may appear as steep slopes or may be missed. Increasing resolution can improve an approximation when the function is sufficiently regular and relevant features are resolved, but density alone is not a proof of accuracy.

## Error considerations

Error depends on more than point count. Relevant factors include:

- function variation and smoothness over the domain;
- local curvature and higher derivatives;
- discontinuities, singularities, narrow peaks, and rapid oscillations;
- the interpolation or reconstruction method;
- floating-point evaluation and coordinate representation;
- whether grid spacing is fine enough to resolve relevant features.

For a sufficiently smooth 1D function with bounded second derivative, linear-interpolation error on an interval can be bounded in terms of that derivative and the square of the interval width. This is conditional, not a universal guarantee for arbitrary functions. The current framework does not estimate derivatives, calculate error bounds, or decide automatically whether a resolution is adequate.

Sampling theory also gives reconstruction conditions for particular classes of band-limited signals. Those assumptions matter: a generic user-provided function is not known to be band-limited, and the current sampler does not perform anti-alias filtering or certify a Nyquist condition.

## Practical resolution workflow

1. Choose a domain that includes the region of interest.
2. Start with a modest point count on each axis.
3. Inspect for missed extrema, abrupt changes, oscillation, and unexpectedly flat or distorted regions.
4. Increase resolution along the axis where features are insufficiently resolved. Use independent X/Y counts when the problem is anisotropic.
5. Compare successive resolutions. If the visible result changes materially, the coarser result was not stable enough for the intended use.
6. Keep the intended use in mind: exploratory educational visualization and quantitative scientific results may require different evidence of accuracy.

Visual convergence between two resolutions is a useful diagnostic, not a formal error certificate.

## Current limitations and future work

The current sampler uses uniform spacing and evaluates every requested sample. It does not currently provide:

- adaptive or error-driven sampling;
- derivative estimation or numerical error bounds;
- anti-alias filtering for oscillatory functions;
- interpolation-quality estimates;
- automatic selection of a suitable resolution.

These should be introduced as explicit capabilities with documented assumptions and tests, rather than inferred from a denser mesh.