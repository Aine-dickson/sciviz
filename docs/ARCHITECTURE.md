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
5. **Interaction addresses data.** Picking should eventually resolve from a
   screen location to a scientific entity and its associated values.
6. **Build incrementally.** Crate boundaries are introduced when they express
   stable responsibilities, not merely to maximize the number of crates.

## Workspace responsibilities

### `svf-core`

Graphics-independent scientific/domain foundation. Future responsibilities:
coordinate systems, datasets, field associations, topology, metadata, and
validation.

### `svf-render`

GPU infrastructure and presentation. The first implementation owns a wgpu
instance, adapter, device, queue, window surface, and surface configuration.
It currently uses winit's `Window` as its native host integration point.
Scientific data structures will not be added here.

### `svf-explorer`

A deliberately thin desktop host. It owns the event loop and window lifecycle,
then delegates rendering to `svf-render`. It is not the product's primary API.

## Planned data model

The first scientific data abstractions will be designed around semantics,
not graphics primitives:

- point sets / point clouds
- polylines
- triangle meshes
- structured grids
- unstructured grids
- scalar fields
- vector fields
- cell-associated and point-associated attributes

The data model must distinguish point-associated, cell-associated, and
dataset-wide attributes. These associations are not interchangeable.

## Planned pipeline

A future pipeline should express:

`source -> filter/transform -> geometry or representation -> visual mapping -> renderer`

The pipeline should eventually support dependency tracking, invalidation,
selective recomputation, caching, and deterministic evaluation. Those
mechanisms are intentionally not implemented in M1.

## M1 rendering boundary

M1 establishes only the native graphics lifecycle:

- create a Windows window
- create a wgpu instance and compatible adapter
- request a device and queue
- create and configure a presentation surface
- clear the surface
- respond to resize events
- recover from outdated/lost surfaces where possible
- exit cleanly on close

No scientific geometry, shaders, camera, or scene graph is introduced in M1.
