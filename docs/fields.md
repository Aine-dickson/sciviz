# M6.1 — Scientific field semantics

## Objective

Introduce graphics-independent field types that describe scientific quantities
and their association with dataset entities. This increment does not introduce
a dataset container, geometry attachment, visualization mapping, or renderer
integration.

## Public API

Implemented in `svf-core::fields`:

- `FieldAssociation::{Point, Cell, Dataset}`
- `FieldMetadata`: non-empty name and optional non-empty unit label
- `ScalarField`: ordered `SampleValue` values
- `VectorField3`: ordered three-component vector values and explicit basis
- `VectorValue3`: independently classified X/Y/Z components
- `VectorBasis3::World`: the only supported vector basis in M6.1
- `FieldError`: metadata and cardinality validation failures

`SampleValue::from_f64` is public so callers can classify finite values,
NaN, and positive/negative infinity consistently with the existing sampling
API.

## Association semantics

- Point association means one value per dataset point.
- Cell association means one value per logical dataset cell, not per
  render-generated triangle.
- Dataset association means exactly one value for the dataset as a whole.
- Point/cell field cardinality is checked against point/cell counts when
  attached to a concrete dataset in a later increment.
- Dataset-associated fields are validated to contain exactly one value at
  construction.

Fields preserve value order. Non-finite scalar values remain explicit
`SampleValue` entries. Vector components are classified independently, so
one invalid component does not erase the other two. A vector can be converted
to `WorldVector3` only when all three components are finite.

## Coordinate and unit semantics

Unit labels are descriptive strings; M6.1 does not perform unit conversion,
dimensional analysis, or enforce a unit vocabulary.

M6.1 supports world-coordinate vector components only. Dataset-local,
coordinate-system transforms, basis conversion, and vector/tensor field
families are deferred until coordinate-frame semantics are designed.

## Deliberate exclusions

- No universal dataset container or field registry.
- No implicit conversion between point and cell associations.
- No assumptions that cell values correspond to render triangles.
- No GPU buffers, colors, transfer functions, or rendering dependencies.
- No automatic interpolation or averaging between associations.
- No unit conversion or physical-dimension inference.

## Validation

Unit tests cover metadata validation, association preservation, point/cell
cardinality, dataset-wide single-value semantics, non-finite scalar retention,
component-wise vector validity, world-vector conversion, and vector-field
association/basis behavior.

Local Windows formatting, compilation, and test results must be recorded
separately after the user pulls this implementation.
