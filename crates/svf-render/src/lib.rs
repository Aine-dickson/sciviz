#![forbid(unsafe_code)]
//! Minimal scientific geometry renderer for the native Explorer.
//!
//! Scientific coordinates and camera mathematics remain in svf-core at f64
//! precision. This crate converts the combined camera matrix to f32 only when
//! preparing the GPU uniform buffer.

use std::{error::Error, fmt, mem::size_of, num::NonZeroU64, sync::Arc};

use bytemuck::{Pod, Zeroable};
use svf_core::{
    boundary_axial_glyphs::BoundaryAxialGlyphSet3,
    camera::{Camera3, LookAtCamera, PerspectiveProjection, Projection3},
    geometry::TriangleMesh3,
    spatial::{WorldPoint3, WorldVector3},
    vector_glyphs::VectorGlyphSet3,
};
use wgpu::{CurrentSurfaceTexture, Surface, SurfaceConfiguration, util::DeviceExt};
use winit::{dpi::PhysicalSize, window::Window};

const GRID_EXTENT: i32 = 10;
const GRID_COLOR: [f32; 3] = [0.19, 0.22, 0.28];
const X_AXIS_COLOR: [f32; 3] = [0.95, 0.25, 0.22];
const Y_AXIS_COLOR: [f32; 3] = [0.28, 0.88, 0.38];
const Z_AXIS_COLOR: [f32; 3] = [0.25, 0.52, 0.98];
const VECTOR_GLYPH_COLOR: [f32; 3] = [0.98, 0.72, 0.20];
/// Fixed bright cyan for boundary axial glyphs; independent of any scalar mapping.
const AXIAL_GLYPH_COLOR: [f32; 3] = [0.0, 0.95, 1.0];

/// Errors raised while initializing or using the rendering infrastructure.
#[derive(Debug)]
pub enum RenderError {
    SurfaceCreation(String),
    AdapterRequest(String),
    DeviceRequest(String),
    CameraConfiguration(String),
    UnsupportedSurface,
    SurfaceValidation,
    MeshConversion,
    MeshBufferSizeChanged,
    VectorGlyphConversion,
    IsosurfaceConversion,
    InvalidCameraUniform,
    UnsupportedCaptureFormat(String),
    CaptureFailed(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfaceCreation(message) => write!(f, "could not create GPU surface: {message}"),
            Self::AdapterRequest(message) => {
                write!(f, "could not find a compatible GPU adapter: {message}")
            }
            Self::DeviceRequest(message) => write!(f, "could not create GPU device: {message}"),
            Self::CameraConfiguration(message) => {
                write!(f, "could not configure reference camera: {message}")
            }
            Self::UnsupportedSurface => write!(
                f,
                "the selected adapter does not support this window surface"
            ),
            Self::SurfaceValidation => write!(f, "wgpu reported a surface validation failure"),
            Self::MeshConversion => write!(
                f,
                "scientific mesh cannot be represented by the renderer's f32 vertex/index format"
            ),
            Self::MeshBufferSizeChanged => write!(
                f,
                "updated scientific mesh must preserve the existing vertex and index counts"
            ),
            Self::VectorGlyphConversion => write!(
                f,
                "vector glyph geometry cannot be represented by the renderer's f32 line-vertex format"
            ),
            Self::IsosurfaceConversion => write!(
                f,
                "mapped isosurface cannot be represented by the renderer's f32 mesh format"
            ),
            Self::InvalidCameraUniform => write!(
                f,
                "camera view-projection matrix cannot be represented by the renderer's f32 uniform format"
            ),
            Self::UnsupportedCaptureFormat(format) => write!(
                f,
                "cannot capture frames from the surface format {format}; expected an 8-bit RGBA or BGRA format"
            ),
            Self::CaptureFailed(message) => write!(f, "frame capture failed: {message}"),
        }
    }
}

impl Error for RenderError {}

/// One captured frame: tightly packed RGBA8, row 0 at the top, alpha 255.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LineVertex {
    position: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MeshVertex {
    position: [f32; 3],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    // Four columns, matching WGSL mat4x4 column-major storage.
    view_projection: [[f32; 4]; 4],
    // Camera position (xyz) and the headlight switch (w: 1.0 on, 0.0 off).
    eye_shading: [f32; 4],
}

impl CameraUniform {
    fn from_camera(camera: &Camera3, shaded: bool) -> Self {
        let columns = camera.view_projection_matrix().to_cols_array_2d();
        let eye = camera.look_at().position().to_array();
        Self {
            view_projection: columns.map(|column| column.map(|value| value as f32)),
            eye_shading: [
                eye[0] as f32,
                eye[1] as f32,
                eye[2] as f32,
                if shaded { 1.0 } else { 0.0 },
            ],
        }
    }

    fn is_finite(&self) -> bool {
        self.view_projection.iter().flatten().all(|value| value.is_finite())
            && self.eye_shading.iter().all(|value| value.is_finite())
    }
}

/// Owns the native surface and renders indexed scientific meshes with reference axes/grid.
pub struct SceneRenderer {
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
    camera: Camera3,
    /// Camera headlight on the mesh surface; see `docs/surface-headlight.md`.
    shaded: bool,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    mesh_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    mesh_vertex_buffer: wgpu::Buffer,
    mesh_index_buffer: wgpu::Buffer,
    vector_vertex_buffer: wgpu::Buffer,
    vector_vertex_count: u32,
    mesh_vertex_count: u32,
    mesh_index_count: u32,
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
}

impl SceneRenderer {
    /// The f64 camera state used for both rendering and CPU-side picking.
    #[must_use]
    pub const fn camera(&self) -> &Camera3 {
        &self.camera
    }

    /// Replaces the camera used by rendering and CPU-side picking.
    ///
    /// The GPU uniform is validated and written before the stored camera is
    /// replaced, so an unrepresentable matrix cannot leave the two out of sync.
    pub fn set_camera(&mut self, camera: Camera3) -> Result<(), RenderError> {
        let uniform = CameraUniform::from_camera(&camera, self.shaded);
        if !uniform.is_finite() {
            return Err(RenderError::InvalidCameraUniform);
        }
        self.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::bytes_of(&uniform),
        );
        self.camera = camera;
        Ok(())
    }

    /// Whether the camera headlight shades the mesh surface.
    #[must_use]
    pub const fn shaded(&self) -> bool {
        self.shaded
    }

    /// Switches the camera headlight on or off. Lines are never shaded.
    pub fn set_shaded(&mut self, shaded: bool) {
        self.shaded = shaded;
        let uniform = CameraUniform::from_camera(&self.camera, self.shaded);
        self.queue
            .write_buffer(&self.camera_uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Initialize wgpu and create the reference-scene pipeline.
    pub async fn new(window: Arc<Window>, mesh: &TriangleMesh3) -> Result<Self, RenderError> {
        let (mesh_vertices, mesh_indices) = convert_mesh(mesh)?;
        Self::new_with_mesh_data(window, mesh_vertices, mesh_indices, Vec::new(), true).await
    }

    /// Initialize the renderer with a mesh and prepared vector glyphs from svf-core.
    pub async fn new_with_vector_glyphs(
        window: Arc<Window>,
        mesh: &TriangleMesh3,
        glyphs: &VectorGlyphSet3,
    ) -> Result<Self, RenderError> {
        let (mesh_vertices, mesh_indices) = convert_mesh(mesh)?;
        let vector_vertices = convert_vector_glyphs(glyphs)?;
        Self::new_with_mesh_data(window, mesh_vertices, mesh_indices, vector_vertices, true).await
    }

    /// Initialize the renderer with a scalar-mapped surface and prepared vector glyphs.
    pub async fn new_with_mapped_surface_and_vectors(
        window: Arc<Window>,
        surface: &svf_core::scalar_visualization::MappedSurface3,
        glyphs: &VectorGlyphSet3,
    ) -> Result<Self, RenderError> {
        let (mesh_vertices, mesh_indices) = convert_mapped_surface(surface)?;
        let vector_vertices = convert_vector_glyphs(glyphs)?;
        // Colour encodes a value here, so the headlight starts off.
        Self::new_with_mesh_data(window, mesh_vertices, mesh_indices, vector_vertices, false).await
    }

    /// Initialize the renderer with a surface (optionally scalar-mapped) and
    /// the thinned boundary axial glyphs from svf-core. The full surface is
    /// always drawn; glyph endpoints are the exact core endpoints cast to f32.
    pub async fn new_with_axial_glyphs(
        window: Arc<Window>,
        mesh: &TriangleMesh3,
        mapped: Option<&svf_core::scalar_visualization::MappedSurface3>,
        glyphs: &BoundaryAxialGlyphSet3,
    ) -> Result<Self, RenderError> {
        let (mesh_vertices, mesh_indices) = match mapped {
            Some(surface) => convert_mapped_surface(surface)?,
            None => convert_mesh(mesh)?,
        };
        let vector_vertices = convert_axial_glyphs(glyphs)?;
        // Shade a plain surface; keep a scalar-mapped one unshaded so colour stays true.
        Self::new_with_mesh_data(
            window,
            mesh_vertices,
            mesh_indices,
            vector_vertices,
            mapped.is_none(),
        )
        .await
    }

    /// Initialize the renderer with a scalar-mapped surface produced by svf-core.
    pub async fn new_with_mapped_surface(
        window: Arc<Window>,
        surface: &svf_core::scalar_visualization::MappedSurface3,
    ) -> Result<Self, RenderError> {
        let (mesh_vertices, mesh_indices) = convert_mapped_surface(surface)?;
        Self::new_with_mesh_data(window, mesh_vertices, mesh_indices, Vec::new(), false).await
    }

    /// Initialize the renderer with a provenance-preserving mapped isosurface.
    pub async fn new_with_mapped_isosurface(
        window: Arc<Window>,
        surface: &svf_core::isosurface_field_transfer::MappedIsosurface3,
    ) -> Result<Self, RenderError> {
        let (mesh_vertices, mesh_indices) = convert_mapped_isosurface(surface)?;
        Self::new_with_mesh_data(window, mesh_vertices, mesh_indices, Vec::new(), false).await
    }

    /// Initialize a mapped isosurface with a depth-tested overlay of unique mesh edges.
    pub async fn new_with_mapped_isosurface_and_edges(
        window: Arc<Window>,
        surface: &svf_core::isosurface_field_transfer::MappedIsosurface3,
    ) -> Result<Self, RenderError> {
        let (mesh_vertices, mesh_indices) = convert_mapped_isosurface(surface)?;
        let edge_vertices = convert_isosurface_edges(surface)?;
        Self::new_with_mesh_data(window, mesh_vertices, mesh_indices, edge_vertices, false).await
    }

    async fn new_with_mesh_data(
        window: Arc<Window>,
        mesh_vertices: Vec<MeshVertex>,
        mesh_indices: Vec<u32>,
        vector_vertices: Vec<LineVertex>,
        shaded: bool,
    ) -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| RenderError::SurfaceCreation(error.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| RenderError::AdapterRequest(error.to_string()))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|error| RenderError::DeviceRequest(error.to_string()))?;

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or(RenderError::UnsupportedSurface)?;
        surface.configure(&device, &config);
        let (depth_texture, depth_view) = create_depth_target(&device, config.width, config.height);

        let aspect_ratio = config.width as f64 / config.height as f64;
        let camera = reference_camera(aspect_ratio)?;
        let camera_uniform = CameraUniform::from_camera(&camera, shaded);

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("svf-camera-bind-group-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(size_of::<CameraUniform>() as u64),
                    },
                    count: None,
                }],
            });

        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("svf-camera-uniform-buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("svf-camera-bind-group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("svf-reference-lines-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("line.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("svf-reference-lines-pipeline-layout"),
            bind_group_layouts: &[Some(&camera_bind_group_layout)],
            immediate_size: 0,
        });

        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        };
        let vertex_buffers = [Some(vertex_layout)];
        let color_targets = [Some(wgpu::ColorTargetState {
            format: config.format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("svf-reference-lines-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(line_depth_state()),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &color_targets,
            }),
            multiview_mask: None,
            cache: None,
        });

        let vertices = reference_geometry();
        let vertex_count = vertices.len() as u32;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("svf-reference-lines-vertex-buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("svf-indexed-mesh-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mesh.wgsl").into()),
        });
        const MESH_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        let mesh_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<MeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &MESH_ATTRIBUTES,
        };
        let mesh_buffers = [Some(mesh_layout)];
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("svf-indexed-triangle-mesh-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &mesh_buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(mesh_depth_state()),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &color_targets,
            }),
            multiview_mask: None,
            cache: None,
        });

        let mesh_vertex_count =
            u32::try_from(mesh_vertices.len()).map_err(|_| RenderError::MeshConversion)?;
        let mesh_index_count =
            u32::try_from(mesh_indices.len()).map_err(|_| RenderError::MeshConversion)?;
        let mesh_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("svf-scientific-mesh-vertex-buffer"),
            contents: bytemuck::cast_slice(&mesh_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let mesh_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("svf-scientific-mesh-index-buffer"),
            contents: bytemuck::cast_slice(&mesh_indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });
        let vector_vertex_count =
            u32::try_from(vector_vertices.len()).map_err(|_| RenderError::VectorGlyphConversion)?;
        let vector_vertex_buffer = if vector_vertices.is_empty() {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("svf-vector-glyph-vertex-buffer"),
                size: size_of::<LineVertex>() as u64,
                usage: wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            })
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("svf-vector-glyph-vertex-buffer"),
                contents: bytemuck::cast_slice(&vector_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        };

        tracing_adapter_info(&adapter);

        Ok(Self {
            instance,
            window,
            surface,
            device,
            queue,
            config,
            camera,
            shaded,
            camera_uniform_buffer,
            camera_bind_group,
            pipeline,
            mesh_pipeline,
            vertex_buffer,
            vertex_count,
            mesh_vertex_buffer,
            mesh_index_buffer,
            vector_vertex_buffer,
            vector_vertex_count,
            mesh_vertex_count,
            mesh_index_count,
            _depth_texture: depth_texture,
            depth_view,
        })
    }

    /// Update scientific mesh buffers without recreating the renderer.
    ///
    /// The replacement mesh must preserve the existing vertex and index
    /// counts so the allocated GPU buffers remain large enough. Both vertex
    /// positions and indices are updated; pipelines, camera, reference
    /// geometry, and depth resources remain untouched.
    pub fn update_mesh(&mut self, mesh: &TriangleMesh3) -> Result<(), RenderError> {
        let (vertices, indices) = convert_mesh(mesh)?;
        let vertex_count =
            u32::try_from(vertices.len()).map_err(|_| RenderError::MeshConversion)?;
        let index_count = u32::try_from(indices.len()).map_err(|_| RenderError::MeshConversion)?;

        if vertex_count != self.mesh_vertex_count || index_count != self.mesh_index_count {
            return Err(RenderError::MeshBufferSizeChanged);
        }

        self.queue
            .write_buffer(&self.mesh_vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.queue
            .write_buffer(&self.mesh_index_buffer, 0, bytemuck::cast_slice(&indices));
        Ok(())
    }

    /// Reconfigure the surface and camera projection after a non-zero resize.
    ///
    /// Only the camera aspect ratio changes; world-space reference geometry
    /// and scientific coordinates remain untouched.
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), RenderError> {
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }

        let aspect_ratio = size.width as f64 / size.height as f64;
        let projection = (*self.camera.projection())
            .with_aspect_ratio(aspect_ratio)
            .map_err(|error| RenderError::CameraConfiguration(error.to_string()))?;
        self.camera = Camera3::new(*self.camera.look_at(), projection);

        let camera_uniform = CameraUniform::from_camera(&self.camera, self.shaded);
        self.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::bytes_of(&camera_uniform),
        );

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        let (depth_texture, depth_view) =
            create_depth_target(&self.device, size.width, size.height);
        self._depth_texture = depth_texture;
        self.depth_view = depth_view;
        Ok(())
    }

    /// Draw the reference grid and axes, then present the frame.
    pub fn render(&mut self) -> Result<(), RenderError> {
        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => return Ok(()),
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.reconfigure_surface();
                return Ok(());
            }
            CurrentSurfaceTexture::Validation => return Err(RenderError::SurfaceValidation),
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("svf-reference-frame-encoder"),
            });

        self.encode_scene(&mut encoder, &view, &self.depth_view);

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);
        Ok(())
    }

    /// Record the scene pass into `encoder`. Shared by the window frame and
    /// the offscreen capture, so both draw exactly the same thing.
    fn encode_scene(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("svf-reference-lines-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.025,
                            g: 0.035,
                            b: 0.055,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Draw opaque mesh first so it establishes depth. Reference lines
            // follow with depth testing but without writing depth, allowing
            // visible grid/axes to appear over the surface only where nearer.
            pass.set_pipeline(&self.mesh_pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.mesh_vertex_buffer.slice(..));
            pass.set_index_buffer(self.mesh_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.mesh_index_count, 0, 0..1);

            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..self.vertex_count, 0..1);

            pass.set_vertex_buffer(0, self.vector_vertex_buffer.slice(..));
            pass.draw(0..self.vector_vertex_count, 0..1);
        }
    }

    /// Render one frame offscreen at `width` x `height` and return it as
    /// tightly packed RGBA8 bytes (alpha forced to 255).
    ///
    /// The frame uses the same pipelines, geometry and look-at as the window;
    /// only the aspect ratio follows the requested size. Bytes are kept in the
    /// surface's colour encoding, i.e. as they would appear in the window.
    pub fn capture_rgba8(&mut self, width: u32, height: u32) -> Result<CapturedImage, RenderError> {
        if width == 0 || height == 0 {
            return Err(RenderError::CaptureFailed("capture size must be non-zero".to_owned()));
        }
        let limit = self.device.limits().max_texture_dimension_2d;
        if width > limit || height > limit {
            return Err(RenderError::CaptureFailed(format!(
                "capture size {width}x{height} exceeds the GPU texture limit {limit}"
            )));
        }
        let swap_channels = match self.config.format {
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => false,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => true,
            other => return Err(RenderError::UnsupportedCaptureFormat(format!("{other:?}"))),
        };

        // Same look-at and projection as the window, at the capture aspect.
        let projection = (*self.camera.projection())
            .with_aspect_ratio(width as f64 / height as f64)
            .map_err(|error| RenderError::CameraConfiguration(error.to_string()))?;
        let capture_camera = Camera3::new(*self.camera.look_at(), projection);
        let uniform = CameraUniform::from_camera(&capture_camera, self.shaded);
        self.queue.write_buffer(&self.camera_uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let color_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("svf-capture-color-target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (_depth_texture, depth_view) = create_depth_target(&self.device, width, height);

        let unpadded_row = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_row = unpadded_row.div_ceil(align) * align;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("svf-capture-readback"),
            size: u64::from(padded_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("svf-capture-encoder"),
        });
        self.encode_scene(&mut encoder, &color_view, &depth_view);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);

        let (sender, receiver) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| RenderError::CaptureFailed(format!("GPU wait failed: {error}")))?;
        receiver
            .recv()
            .map_err(|_| RenderError::CaptureFailed("readback callback never ran".to_owned()))?
            .map_err(|error| RenderError::CaptureFailed(format!("could not map readback buffer: {error}")))?;

        let mut rgba = Vec::with_capacity(unpadded_row as usize * height as usize);
        {
            let mapped = readback.slice(..).get_mapped_range().map_err(|error| {
                RenderError::CaptureFailed(format!("could not read mapped buffer: {error}"))
            })?;
            for row in mapped.chunks(padded_row as usize).take(height as usize) {
                for pixel in row[..unpadded_row as usize].chunks_exact(4) {
                    if swap_channels {
                        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
                    } else {
                        rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
                    }
                }
            }
        }
        readback.unmap();

        // Restore the window's camera uniform.
        let window_uniform = CameraUniform::from_camera(&self.camera, self.shaded);
        self.queue.write_buffer(&self.camera_uniform_buffer, 0, bytemuck::bytes_of(&window_uniform));

        Ok(CapturedImage { width, height, rgba })
    }

    fn reconfigure_surface(&mut self) {
        if let Ok(surface) = self.instance.create_surface(self.window.clone()) {
            self.surface = surface;
            self.surface.configure(&self.device, &self.config);
        }
    }
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

fn line_depth_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        stencil: Default::default(),
        bias: Default::default(),
    }
}

fn mesh_depth_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Less),
        stencil: Default::default(),
        bias: Default::default(),
    }
}

fn create_depth_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("svf-scene-depth-target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn convert_mesh(mesh: &TriangleMesh3) -> Result<(Vec<MeshVertex>, Vec<u32>), RenderError> {
    let vertices = mesh
        .vertices()
        .iter()
        .map(|point| {
            let p = point.to_array();
            let position = [p[0] as f32, p[1] as f32, p[2] as f32];
            if !position.iter().all(|v| v.is_finite()) {
                return Err(RenderError::MeshConversion);
            }
            Ok(MeshVertex {
                position,
                color: [0.18, 0.62, 0.88, 1.0],
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((vertices, mesh.indices().to_vec()))
}

fn convert_mapped_surface(
    surface: &svf_core::scalar_visualization::MappedSurface3,
) -> Result<(Vec<MeshVertex>, Vec<u32>), RenderError> {
    if surface.positions().len() != surface.colors().len()
        || surface.indices().len() % 3 != 0
        || surface.indices().iter().any(|&index| index as usize >= surface.positions().len())
    {
        return Err(RenderError::MeshConversion);
    }

    let vertices = surface.positions().iter().zip(surface.colors()).map(|(point, color)| {
        let p = point.to_array();
        let position = [p[0] as f32, p[1] as f32, p[2] as f32];
        let color = color.channels().map(|channel| channel as f32);
        if !position.iter().all(|value| value.is_finite())
            || !color.iter().all(|value| value.is_finite())
        {
            return Err(RenderError::MeshConversion);
        }
        Ok(MeshVertex { position, color })
    }).collect::<Result<Vec<_>, _>>()?;

    Ok((vertices, surface.indices().to_vec()))
}

fn convert_mapped_isosurface(
    surface: &svf_core::isosurface_field_transfer::MappedIsosurface3,
) -> Result<(Vec<MeshVertex>, Vec<u32>), RenderError> {
    let mesh = surface.isosurface().mesh();
    if mesh.vertex_count() != surface.colors().len()
        || mesh.indices().len() % 3 != 0
        || mesh.indices().iter().any(|&index| index as usize >= mesh.vertex_count())
    {
        return Err(RenderError::IsosurfaceConversion);
    }
    let vertices = mesh.vertices().iter().zip(surface.colors()).map(|(point, color)| {
        let p = point.to_array();
        let position = [p[0] as f32, p[1] as f32, p[2] as f32];
        let color = color.channels().map(|channel| channel as f32);
        if !position.iter().all(|value| value.is_finite())
            || !color.iter().all(|value| value.is_finite())
        {
            return Err(RenderError::IsosurfaceConversion);
        }
        Ok(MeshVertex { position, color })
    }).collect::<Result<Vec<_>, _>>()?;
    Ok((vertices, mesh.indices().to_vec()))
}

fn convert_isosurface_edges(
    surface: &svf_core::isosurface_field_transfer::MappedIsosurface3,
) -> Result<Vec<LineVertex>, RenderError> {
    use std::collections::BTreeSet;
    let mesh = surface.isosurface().mesh();
    let mut edges = BTreeSet::new();
    for triangle in mesh.indices().chunks_exact(3) {
        for (a, b) in [(triangle[0], triangle[1]), (triangle[1], triangle[2]), (triangle[2], triangle[0])] {
            edges.insert((a.min(b), a.max(b)));
        }
    }
    let capacity = edges.len().checked_mul(2).ok_or(RenderError::IsosurfaceConversion)?;
    let mut vertices = Vec::new();
    vertices.try_reserve_exact(capacity).map_err(|_| RenderError::IsosurfaceConversion)?;
    const EDGE_COLOR: [f32; 3] = [0.035, 0.045, 0.065];
    for (a, b) in edges {
        for index in [a, b] {
            let point = mesh.vertices().get(index as usize).ok_or(RenderError::IsosurfaceConversion)?;
            let position = point.to_array().map(|value| value as f32);
            if !position.iter().all(|value| value.is_finite()) {
                return Err(RenderError::IsosurfaceConversion);
            }
            vertices.push(LineVertex { position, color: EDGE_COLOR });
        }
    }
    Ok(vertices)
}

fn convert_vector_glyphs(
    glyphs: &VectorGlyphSet3,
) -> Result<Vec<LineVertex>, RenderError> {
    let capacity = glyphs
        .entries()
        .len()
        .checked_mul(6)
        .ok_or(RenderError::VectorGlyphConversion)?;
    let mut vertices = Vec::new();
    vertices
        .try_reserve_exact(capacity)
        .map_err(|_| RenderError::VectorGlyphConversion)?;

    for entry in glyphs.entries() {
        let Some(segments) = entry.segments() else {
            continue;
        };

        for segment in segments {
            for point in [segment.start(), segment.end()] {
                let coordinates = point.to_array();
                let position = coordinates.map(|value| value as f32);
                if !position.iter().all(|value| value.is_finite()) {
                    return Err(RenderError::VectorGlyphConversion);
                }
                vertices.push(LineVertex {
                    position,
                    color: VECTOR_GLYPH_COLOR,
                });
            }
        }
    }

    Ok(vertices)
}

/// One line segment per glyph: the core `start()` and `end()` cast to f32,
/// with nothing normalised, offset or shortened.
fn convert_axial_glyphs(glyphs: &BoundaryAxialGlyphSet3) -> Result<Vec<LineVertex>, RenderError> {
    let capacity = glyphs
        .glyphs()
        .len()
        .checked_mul(2)
        .ok_or(RenderError::VectorGlyphConversion)?;
    let mut vertices = Vec::new();
    vertices
        .try_reserve_exact(capacity)
        .map_err(|_| RenderError::VectorGlyphConversion)?;
    for glyph in glyphs.glyphs() {
        for point in [glyph.start(), glyph.end()] {
            let position = point.to_array().map(|value| value as f32);
            if !position.iter().all(|value| value.is_finite()) {
                return Err(RenderError::VectorGlyphConversion);
            }
            vertices.push(LineVertex { position, color: AXIAL_GLYPH_COLOR });
        }
    }
    Ok(vertices)
}

fn reference_camera(aspect_ratio: f64) -> Result<Camera3, RenderError> {
    let pose = LookAtCamera::new(
        WorldPoint3::new(14.0, 12.0, 18.0),
        WorldPoint3::new(0.0, 0.0, 0.0),
        WorldVector3::new(0.0, 1.0, 0.0),
    )
    .map_err(|error| RenderError::CameraConfiguration(error.to_string()))?;

    let projection = PerspectiveProjection::new(45.0_f64.to_radians(), aspect_ratio, 0.1, 100.0)
        .map_err(|error| RenderError::CameraConfiguration(error.to_string()))?;

    Ok(Camera3::new(pose, Projection3::Perspective(projection)))
}

fn reference_geometry() -> Vec<LineVertex> {
    let extent = GRID_EXTENT as f32;
    let mut vertices = Vec::with_capacity((GRID_EXTENT as usize * 4 * 2) + 6);

    // The grid lies in the XZ plane (Y = 0), matching the +Y world-up axis.
    // Skip the two zero-offset grid lines because the colored axes replace them.
    for coordinate in -GRID_EXTENT..=GRID_EXTENT {
        if coordinate == 0 {
            continue;
        }

        let offset = coordinate as f32;
        push_line(
            &mut vertices,
            [-extent, 0.0, offset],
            [extent, 0.0, offset],
            GRID_COLOR,
        );
        push_line(
            &mut vertices,
            [offset, 0.0, -extent],
            [offset, 0.0, extent],
            GRID_COLOR,
        );
    }

    // Draw the three world axes last so they remain visually distinct.
    push_line(
        &mut vertices,
        [-extent, 0.0, 0.0],
        [extent, 0.0, 0.0],
        X_AXIS_COLOR,
    );
    push_line(
        &mut vertices,
        [0.0, -extent, 0.0],
        [0.0, extent, 0.0],
        Y_AXIS_COLOR,
    );
    push_line(
        &mut vertices,
        [0.0, 0.0, -extent],
        [0.0, 0.0, extent],
        Z_AXIS_COLOR,
    );

    vertices
}

fn push_line(vertices: &mut Vec<LineVertex>, start: [f32; 3], end: [f32; 3], color: [f32; 3]) {
    vertices.push(LineVertex {
        position: start,
        color,
    });
    vertices.push(LineVertex {
        position: end,
        color,
    });
}

fn tracing_adapter_info(adapter: &wgpu::Adapter) {
    let info = adapter.get_info();
    eprintln!(
        "SVF graphics adapter: {} ({:?}, {:?})",
        info.name, info.backend, info.device_type
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use svf_core::{
        dataset::Dataset,
        fields::{FieldAssociation, FieldMetadata, VectorBasis3, VectorField3, VectorValue3},
        geometry::PointSet3,
        vector_glyphs::{build_vector_glyphs, VectorScalePolicy3},
        vector_visualization::{analyze_vector_sources, resolve_vector_sources},
    };

    fn vector_glyphs(points: Vec<WorldPoint3>, values: Vec<VectorValue3>) -> VectorGlyphSet3 {
        vector_glyphs_with_length(points, values, 2.0)
    }

    fn vector_glyphs_with_length(
        points: Vec<WorldPoint3>,
        values: Vec<VectorValue3>,
        length: f64,
    ) -> VectorGlyphSet3 {
        let mut dataset = Dataset::new(PointSet3::new(points).unwrap());
        let field = VectorField3::new(
            FieldMetadata::new("velocity", None).unwrap(),
            FieldAssociation::Point,
            VectorBasis3::World,
            values,
        )
        .unwrap();
        dataset.add_vector_field3(field).unwrap();
        let sources = resolve_vector_sources(&dataset, "velocity").unwrap();
        let analyses = analyze_vector_sources(&sources).unwrap();
        build_vector_glyphs(
            &analyses,
            VectorScalePolicy3::UniformLength { length },
        )
        .unwrap()
    }

    #[test]
    fn vector_glyph_conversion_flattens_segments_and_skips_entries_without_geometry() {
        let glyphs = vector_glyphs(
            vec![
                WorldPoint3::new(1.0, 2.0, 3.0),
                WorldPoint3::new(4.0, 5.0, 6.0),
                WorldPoint3::new(7.0, 8.0, 9.0),
            ],
            vec![
                VectorValue3::from_f64(1.0, 0.0, 0.0),
                VectorValue3::from_f64(0.0, 0.0, 0.0),
                VectorValue3::from_f64(f64::NAN, 1.0, 0.0),
            ],
        );

        let vertices = convert_vector_glyphs(&glyphs).unwrap();
        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(vertices[1].position, [3.0, 2.0, 3.0]);
        assert!(vertices.iter().all(|vertex| vertex.color == VECTOR_GLYPH_COLOR));
    }

    fn axial_glyph_set(tags: &[f64], request: &[i64], length: f64) -> BoundaryAxialGlyphSet3 {
        use svf_core::{
            boundary_axial_glyphs::{build_boundary_axial_glyphs, BoundaryAxialGlyphOptions3},
            fields::ScalarField,
            sampling::SampleValue,
            unstructured::CellType3,
        };
        // Two tetrahedra sharing face (1, 2, 3).
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 1.0]];
        let cells = [[0u32, 1, 2, 3], [1, 2, 3, 4]];
        let grid = svf_core::unstructured::UnstructuredGrid3::new(
            points.iter().map(|p| WorldPoint3::new(p[0], p[1], p[2])).collect(),
            vec![CellType3::Tetrahedron4; 2],
            vec![0, 4, 8],
            cells.iter().flatten().copied().collect(),
        )
        .unwrap();
        let mut dataset = Dataset::new(grid);
        dataset
            .add_scalar_field(
                ScalarField::new(
                    FieldMetadata::new("ID", None).unwrap(),
                    FieldAssociation::Cell,
                    tags.iter().map(|&t| SampleValue::from_f64(t)).collect(),
                )
                .unwrap(),
            )
            .unwrap();
        dataset
            .add_vector_field3(
                VectorField3::new(
                    FieldMetadata::new("fibres", None).unwrap(),
                    FieldAssociation::Cell,
                    VectorBasis3::World,
                    vec![VectorValue3::from_f64(1.0, 0.0, 0.0), VectorValue3::from_f64(0.0, 1.0, 0.0)],
                )
                .unwrap(),
            )
            .unwrap();
        let svf_core::dataset::DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { unreachable!() };
        let surface = grid.boundary_surface().unwrap().triangulate();
        build_boundary_axial_glyphs(
            &dataset,
            &surface,
            "fibres",
            "ID",
            request,
            BoundaryAxialGlyphOptions3 { bin_edge: 0.25, length },
        )
        .unwrap()
    }

    #[test]
    fn axial_conversion_emits_one_segment_per_glyph_with_exact_f32_endpoints() {
        let set = axial_glyph_set(&[1.0, 5.0], &[1], 0.5);
        assert!(!set.glyphs().is_empty());
        let vertices = convert_axial_glyphs(&set).unwrap();
        assert_eq!(vertices.len(), 2 * set.glyphs().len());
        for (glyph, pair) in set.glyphs().iter().zip(vertices.chunks_exact(2)) {
            assert_eq!(pair[0].position, glyph.start().to_array().map(|v| v as f32));
            assert_eq!(pair[1].position, glyph.end().to_array().map(|v| v as f32));
        }
        assert!(vertices.iter().all(|v| v.color == AXIAL_GLYPH_COLOR));
    }

    #[test]
    fn axial_conversion_only_draws_requested_tags() {
        let both = convert_axial_glyphs(&axial_glyph_set(&[1.0, 5.0], &[1, 5], 0.5)).unwrap();
        let one = convert_axial_glyphs(&axial_glyph_set(&[1.0, 5.0], &[1], 0.5)).unwrap();
        assert!(!one.is_empty());
        assert!(one.len() < both.len());
    }

    #[test]
    fn axial_conversion_rejects_positions_outside_f32_range() {
        let set = axial_glyph_set(&[1.0, 1.0], &[1], 1.0e39);
        assert!(matches!(convert_axial_glyphs(&set), Err(RenderError::VectorGlyphConversion)));
    }

    #[test]
    fn vector_glyph_conversion_rejects_positions_outside_f32_range() {
        // The arrow must be representable in f64 before the renderer
        // rejects its world coordinates at the f32 GPU boundary.
        let glyphs = vector_glyphs_with_length(
            vec![WorldPoint3::new(1.0e100, 0.0, 0.0)],
            vec![VectorValue3::from_f64(1.0, 0.0, 0.0)],
            1.0e90,
        );
        assert!(matches!(
            convert_vector_glyphs(&glyphs),
            Err(RenderError::VectorGlyphConversion)
        ));
    }

    #[test]
    fn mesh_conversion_preserves_indices_and_converts_positions() {
        let mesh = TriangleMesh3::new(
            vec![
                WorldPoint3::new(0.0, 1.0, 2.0),
                WorldPoint3::new(1.0, 1.0, 2.0),
                WorldPoint3::new(0.0, 1.0, 3.0),
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        let (vertices, indices) = convert_mesh(&mesh).unwrap();
        assert_eq!(vertices[1].position, [1.0, 1.0, 2.0]);
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn mapped_surface_conversion_preserves_positions_indices_and_rgba() {
        use svf_core::{
            fields::{FieldAssociation, FieldMetadata, ScalarField},
            sampling::SampleValue,
            scalar_visualization::{ColorMap, ColorRgba, ScalarRangeMode, SurfaceProvenance3, map_scalar_field},
        };

        let mesh = TriangleMesh3::new(
            vec![
                WorldPoint3::new(0.0, 0.0, 0.0),
                WorldPoint3::new(1.0, 0.0, 0.0),
                WorldPoint3::new(0.0, 1.0, 0.0),
            ],
            vec![0, 1, 2],
        ).unwrap();
        let surface = SurfaceProvenance3::from_triangle_mesh(&mesh).unwrap();
        let field = ScalarField::new(
            FieldMetadata::new("temperature", Some("K".to_owned())).unwrap(),
            FieldAssociation::Point,
            vec![SampleValue::Finite(0.0), SampleValue::Finite(0.5), SampleValue::Finite(1.0)],
        ).unwrap();
        let mapped = map_scalar_field(
            &surface, &field, 3, 1, ScalarRangeMode::Automatic,
            &ColorMap::viridis(), ColorRgba::new(1.0, 0.0, 1.0, 1.0).unwrap(),
        ).unwrap();

        let (vertices, indices) = convert_mapped_surface(&mapped).unwrap();
        assert_eq!(vertices.len(), 3);
        assert_eq!(vertices[2].position, [0.0, 1.0, 0.0]);
        assert_eq!(vertices[1].color, mapped.colors()[1].channels().map(|channel| channel as f32));
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn mesh_conversion_rejects_f64_values_outside_f32_range() {
        let mesh = TriangleMesh3::new(
            vec![
                WorldPoint3::new(0.0, 0.0, 0.0),
                WorldPoint3::new(1.0e100, 0.0, 0.0),
                WorldPoint3::new(0.0, 1.0, 0.0),
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        assert!(matches!(
            convert_mesh(&mesh),
            Err(RenderError::MeshConversion)
        ));
    }

    #[test]
    fn reference_geometry_contains_grid_and_three_axes() {
        let vertices = reference_geometry();

        // Forty grid lines plus three axes, with two vertices per line.
        assert_eq!(vertices.len(), 86);
        assert_eq!(vertices[80].position, [-10.0, 0.0, 0.0]);
        assert_eq!(vertices[82].position, [0.0, -10.0, 0.0]);
        assert_eq!(vertices[84].position, [0.0, 0.0, -10.0]);
        assert_eq!(vertices[80].color, X_AXIS_COLOR);
        assert_eq!(vertices[82].color, Y_AXIS_COLOR);
        assert_eq!(vertices[84].color, Z_AXIS_COLOR);
    }

    #[test]
    fn camera_uniform_conversion_produces_finite_f32_values() {
        let camera = reference_camera(16.0 / 9.0).expect("valid reference camera");
        let uniform = CameraUniform::from_camera(&camera, true);

        assert!(uniform.is_finite());
    }

    #[test]
    fn camera_uniform_carries_eye_and_headlight_switch() {
        let camera = reference_camera(16.0 / 9.0).expect("valid reference camera");
        let eye = camera.look_at().position().to_array();

        let on = CameraUniform::from_camera(&camera, true);
        let off = CameraUniform::from_camera(&camera, false);

        assert_eq!(on.eye_shading[..3], [eye[0] as f32, eye[1] as f32, eye[2] as f32]);
        assert_eq!(on.eye_shading[3], 1.0);
        assert_eq!(off.eye_shading[3], 0.0);
        assert_eq!(on.view_projection, off.view_projection);
        assert_eq!(size_of::<CameraUniform>(), 80);
    }
}
