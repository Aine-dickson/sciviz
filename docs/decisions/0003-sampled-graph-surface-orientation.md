# 0003 — Sampled graph-surface world orientation

- Status: Accepted
- Date: 2026-11-08

## Context

SVF's accepted world convention is right-handed with +Y up. A two-dimensional
scalar function is sampled over a mathematical domain with independent X
and Y coordinates. Converting its samples into a 3D graph surface must preserve
the framework's world-axis semantics rather than treating the third array
component as an implicit vertical axis.

## Decision

Map a sampled domain point `(x, y)` and scalar value `f(x, y)` to world
coordinates:

```text
world X = x
world Y = f(x, y)
world Z = y
```

The domain therefore lies on the world XZ plane and scalar values represent
height along +Y. Generated triangle winding faces +Y for a regular grid whose
sample indices increase along the positive domain axes.

This is a visualization mapping. The original mathematical sample records
retain their original two domain coordinates and scalar values; no scientific
sample is rewritten or reordered.

## Consequences

- Generated graph surfaces agree with the +Y-up world convention and the
  Explorer's XZ reference grid.
- Source sample and source cell identity remain available for inspection.
- Renderer camera placement can view graph surfaces using the same world
  convention as other geometry.
- A different orientation, such as a graph over another plane, must be an
  explicit transform or mapping rather than an undocumented axis swap.
