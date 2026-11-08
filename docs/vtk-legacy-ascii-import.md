# VTK Legacy ASCII Unstructured-Grid Import

## Status

Design contract for M9.1. No implementation is authorized by this document alone.
The first implementation target is a graphics-independent parser in
`svf-core`; Explorer file selection and rendering integration are deferred to
M9.2.

## Objective

Allow SVF to construct an authoritative `Dataset` from a useful, explicitly
bounded subset of the VTK legacy ASCII file format. This is the first external
scientific-data ingestion path. The reader must preserve source point order,
source cell order, supported topology, field association, field names, and
numeric meaning to the extent supported by SVF's existing f64 field model.

The reader is an importer, not a VTK compatibility layer. Unsupported content
must produce a precise error; it must never be silently discarded or guessed.

## Public API and responsibility

Proposed module: `svf_core::vtk_legacy`.

Proposed entry point:

```rust
pub fn parse_legacy_ascii_unstructured_grid(
    input: &str,
) -> Result<Dataset, VtkLegacyError>
```

- Accept UTF-8 text already read by the caller. The core parser performs no
  filesystem or UI operations.
- Return a fully validated `Dataset`, or one structured error. Do not return a
  partially populated dataset.
- Keep parsing and format-specific errors in `svf-core`; do not introduce
  VTK, wgpu, windowing, or file-dialog dependencies.
- The Explorer may later read a path and pass its text to this API (M9.2).
- Error variants should identify the failure category and, where practical,
  the offending section, token, or record index. Avoid relying only on
  free-form error strings.

## Accepted file envelope

Support the classic legacy text envelope:

1. `# vtk DataFile Version x.x` first line.
2. One title/header line (up to the format's 256-character limit).
3. `ASCII` format declaration.
4. `DATASET UNSTRUCTURED_GRID`.
5. Geometry/topology records, followed optionally by point and cell data.

Accept legacy versions 1.0, 2.0, and 3.0. Keywords are case-insensitive and
numeric records may be separated by arbitrary ASCII whitespace, including
line breaks. CRLF and LF line endings must work. The title is retained only
for diagnostics if useful; it is not part of the current Dataset model.

Reject BINARY input, other dataset kinds, unsupported versions, malformed
headers, missing required geometry records, and unexpected trailing records.
Do not infer a format from a filename extension.

## Geometry and topology

Required records:

- `POINTS n dataType`: exactly `3 * n` coordinate values.
- `CELLS n size`: exactly `n` cell records. Each record contains its
  declared node count followed by that many zero-based point indices.
  `size` must equal the total number of integers in the cell list, including
  each record's node-count prefix.
- `CELL_TYPES n`: exactly one VTK cell type ID per cell; its count must equal
  the `CELLS` count.

Supported VTK cell IDs and exact mappings:

| VTK ID | VTK type | SVF type |
|---:|---|---|
| 3 | LINE | `Line2` |
| 5 | TRIANGLE | `Triangle3` |
| 9 | QUAD | `Quad4` |
| 10 | TETRA | `Tetrahedron4` |
| 12 | HEXAHEDRON | `Hexahedron8` |

Preserve the point sequence and each cell's connectivity sequence exactly.
Do not reorder points or cells. The existing `UnstructuredGrid3` constructor
remains the authority for fixed arity, repeated indices, index bounds, and
finite point coordinates.

Cell types not listed above are unsupported and fail the import, including
pixel (8), voxel (11), vertex/polyvertex, polygon, wedge, pyramid, and all
higher-order cells. In particular, do not reinterpret PIXEL as QUAD or VOXEL
as HEXAHEDRON: their local point ordering differs. Do not skip unsupported
cells, because doing so would change cell IDs and invalidate cell-associated
field alignment.

An empty grid may be accepted if the required records are structurally valid.
Geometric degeneracy remains governed by existing SVF topology semantics; the
reader must not silently repair or reorder cells.

## Numeric types and conversion

- For point coordinates, accept VTK numeric types that the parser explicitly
  implements; at minimum `float` and `double`. Convert coordinates to f64.
- For scalar/vector arrays, initially support `float`, `double`, `short`,
  `unsigned_short`, `int`, `unsigned_int`, and `unsigned_char`.
  Reject other declared numeric types rather than guessing their width or
  signedness.
- Parse `float` values with f32 semantics, then widen to f64. Parse
  `double` as f64. Parse supported integer types at their declared width,
  then convert exactly to f64 (all supported integer widths fit exactly).
- Scalar and vector field values may be NaN or positive/negative infinity;
  convert them through `SampleValue` / `VectorValue3` so SVF's existing
  non-finite semantics are retained.
- Point coordinates must be finite; the existing geometry constructor should
  reject non-finite coordinates.
- Reject malformed numeric tokens, integer overflow, count overflow, and
  truncated arrays. Use checked arithmetic for all count/size calculations.

## Point and cell attributes

Support optional `POINT_DATA n` and `CELL_DATA n` sections, in either order.
Each declared count must exactly match the imported grid's point or cell
count. Multiple supported arrays may occur in either section.

Supported records:

### Scalars

```
SCALARS name dataType [numComponents]
[LOOKUP_TABLE tableName]
values...
```

- Accept only one component. The component count may be omitted or equal 1.
- The `LOOKUP_TABLE tableName` declaration immediately following the scalar
  header is optional, as permitted by the VTK format. If present, consume it;
  lookup-table colors do not override SVF's visual mapping.
- A separate lookup-table definition (`LOOKUP_TABLE tableName size`, followed
  by `size * 4` RGBA values) may be consumed and ignored; it is presentation
  metadata, not a scientific field.
- Construct a `ScalarField` with `Point` or `Cell` association according
  to the containing section.

Record headers (`POINTS`, `CELLS`, `CELL_TYPES`, `SCALARS`, `VECTORS`,
`POINT_DATA`, `CELL_DATA`, and `LOOKUP_TABLE`) occupy their own lines in the
accepted subset. Numeric payloads may span any number of subsequent lines.
This line-aware rule makes the optional scalar component count unambiguous
while still accepting arbitrary whitespace within data arrays.

### Vectors

```
VECTORS name dataType
x0 y0 z0
...
```

- Require exactly three components per tuple.
- Construct a `VectorField3` with `VectorBasis3::World`, preserving
  components and tuple order.

No dataset-associated attributes are imported in M9.1. The existing Dataset
model supports dataset fields, but this subset of the legacy attribute grammar
does not map them without adding separate FIELD-record semantics.

Field names are preserved exactly. SVF's existing uniqueness rule applies
across scalar and vector fields: duplicate names, including a scalar/vector
name collision, fail the import. VTK legacy scalar/vector records do not
provide a portable units field in this subset, so imported `FieldMetadata`
uses `units: None`; do not infer units from names or titles.

Reject multi-component SCALARS (including RGB/RGBA tuples), COLOR_SCALARS,
NORMALS, TEXTURE_COORDINATES, TENSORS, FIELD arrays, METADATA blocks, and
other unsupported attribute records. Do not silently skip them. Supporting
those requires explicit model semantics and is outside this increment.

## Parser behavior and errors

- Parse by bounded token consumption, not by assuming numeric arrays occupy
  one line. VTK ASCII records are whitespace-separated.
- Treat keywords case-insensitively, but preserve user-provided field names.
- Validate section order: dataset geometry/topology precedes attributes;
  `POINT_DATA` / `CELL_DATA` establish the association for following arrays.
- Reject duplicate point-data or cell-data sections unless a later design
  explicitly defines their merge semantics.
- Reject unknown keywords, unsupported attribute records, and unexplained
  trailing tokens with an UnsupportedRecord or UnexpectedToken error.
- Reject a field before its association section and any tuple-count
  mismatch.
- Report the section and useful record/tuple index for malformed data.
- Do not impose arbitrary dataset-size limits in this first design, but avoid
  unchecked arithmetic and avoid allocating from unvalidated counts. A future
  resource-limit policy can be introduced when real dataset requirements are
  known.

## Explicit non-goals

- Binary legacy VTK.
- VTK XML (`.vtu`, `.vtp`) and VTKHDF.
- Dataset kinds other than `UNSTRUCTURED_GRID`.
- Structured 1D/2D/3D grids, rectilinear grids, and polygonal data.
- Pixel/voxel remapping, unsupported linear cells, polyhedra, and higher-order
  cells.
- Tensor, normal, texture-coordinate, color, and general FIELD attributes.
- Dataset-associated field import and units inference.
- Time series, multiple pieces, parallel files, compression, or streaming.
- General reader/plugin registries, format auto-detection, or writing files.
- Explorer file dialogs, CLI path loading, and visualization selection
  (planned separately for M9.2).

## Test and acceptance contract

Core tests must cover at least:

1. A representative ASCII fixture containing mixed supported cell types,
   with exact point order, cell order, cell types, offsets, and connectivity.
2. Point-associated scalar and vector fields and cell-associated scalar and
   vector fields, including multiple arrays and both section orders.
3. Correct field names, associations, vector world basis, and value ordering.
4. LF and CRLF, arbitrary whitespace, and arrays split across lines.
5. Float32 widening, float64 values, supported integer conversion, and
   preservation of NaN and signed infinities in field values.
6. Optional scalar component count and lookup-table declaration handling.
7. Rejection of BINARY, unsupported dataset kinds/versions/cell types/data
   types/attribute records, duplicate names/sections, malformed headers,
   invalid counts, bad indices, repeated cell point IDs, non-finite
   coordinates, integer overflow, truncated values, and trailing garbage.
8. No partial Dataset escapes on any failure.

Acceptance on the user's Windows environment:

- Focused importer tests pass.
- `cargo test --workspace` passes.
- `cargo check --workspace` passes.
- No existing Explorer mode changes behavior (there is no Explorer integration
  in M9.1).
- No new graphics dependency is introduced.

## Reference

The accepted syntax is a deliberately limited subset of the official VTK
legacy file format documentation:

- [VTK 9.6.1 — Simple Legacy Formats](https://docs.vtk.org/en/v9.6.1/vtk_file_formats/vtk_legacy_file_format.html)

The VTK specification defines the legacy file envelope, unstructured-grid
records, zero-based indices, data associations, and scalar/vector attribute
records. This contract narrows those facilities to the geometry and field
semantics SVF can currently represent without information loss.
