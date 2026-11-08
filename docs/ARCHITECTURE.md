# Architecture

## Product definition

Scientific Visualization Framework (SVF) is a reusable Rust library for
converting mathematical models and scientific datasets into accurate,
interactive 2D and 3D visual representations.

It is not intended to be a general-purpose game engine, 3D modelling package,
or scene-composition editor.

## Architectural rules

1. **Scientific truth is independent of presentation.** Dataset values,
   coordinates, topology, and simulation state must not be stored only in GPU
   buffers.
2. **Pipelines transform data.** A renderer consumes prepared visualization
   data; it does not own the scientific algorithms that produced it.
3. **Rendering is replaceable.** Public scientific data types must not expose
   wgpu types.
4. **Precision is explicit.** Scientific calculations should preserve f64
   where required. Conversion to f32 is a rendering-boundary decision.
5. **Interaction addresses data.** Picking should resolve from a screen
   location to a scientific entity and its associated values.
6. **Build incrementally.** Crate boundaries are introduced when they express
   stable responsibilities, not merely to maximize the number of crates.

## Workspace responsibilities

### `svf-core`

Graphics-independent scientific/domain foundation: geometry, coordinates,
datasets, field associations, topology, metadata, and validation.

### `svf-render`

GPU infrastructure and presentation. It owns the wgpu instance, adapter,
device, queue, window surface, and surface configuration. Scientific dataset
and field types do not belong here.

### `svf-explorer`

A deliberately thin desktop host. It owns the event loop and window lifecycle,
then delegates rendering to `svf-render`. It is not the product's primary API.

## Dataset model

The dataset layer separates source geometry from attached scientific fields.
A `Dataset` owns one immutable `DatasetGeometry` and an insertion-ordered
collection of named scalar/vector fields.

Supported source geometry variants:
- `PointSet3`: independent points; zero logical cells.
- `Polyline3`: ordered points; one logical cell per consecutive segment.
- `TriangleMesh3`: indexed vertices; one logical cell per source triangle.
- `StructuredGrid2D`: row-major samples; one logical cell per grid rectangle.
- `UnstructuredGrid3`: ordered points plus CSR connectivity and an explicit
  linear cell type per source cell; mixed cell types/dimensions are allowed.

Point fields contain one value per source point/sample. Cell fields contain
one value per logical source cell. Dataset fields contain exactly one value.
For structured grids, a rectangle remains one source cell even if a later
surface representation emits two render triangles for that rectangle.
For unstructured grids, cell-field values follow source cell-array order.
Any later tessellation or triangulation must preserve the originating source
cell ID for picking and field lookup. Derived render geometry never changes
source dataset cardinality.

Geometry is immutable through the dataset API, so attached field cardinality
cannot be invalidated by in-place topology edits. Geometry changes produce a
new dataset; explicit remapping/transfer of fields between datasets is a
separate future operation.

## Planned pipeline

A future pipeline should express:

`source -> filter/transform -> geometry or representation -> visual mapping -> renderer`

The pipeline should eventually support dependency tracking, invalidation,
selective recomputation, caching, and deterministic evaluation. Those
mechanisms are intentionally deferred.

## Rendering boundary

The core remains f64 and graphics-independent. `svf-render` converts data to
GPU-compatible formats at the boundary. Render triangles and GPU buffers are
derived representations, not authoritative scientific topology or values.
