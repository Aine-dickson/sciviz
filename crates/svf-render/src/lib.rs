#![forbid(unsafe_code)]
//! Minimal scientific line renderer for the native Explorer.
//!
//! Scientific coordinates and camera mathematics remain in svf-core at f64
//! precision. This crate converts the combined camera matrix to f32 only when
//! preparing the GPU uniform buffer.

use std::{error::Error, fmt, mem::size_of, num::NonZeroU64, sync::Arc};

use bytemuck::{Pod, Zeroable};
use svf_core::{
    camera::{Camera3, LookAtCamera, PerspectiveProjection, Projection3},
    spatial::{WorldPoint3, WorldVector3},
};
use wgpu::{CurrentSurfaceTexture, Surface, SurfaceConfiguration, util::DeviceExt};
use winit::{dpi::PhysicalSize, window::Window};

const GRID_EXTENT: i32 = 10;
const GRID_COLOR: [f32; 3] = [0.19, 0.22, 0.28];
const X_AXIS_COLOR: [f32; 3] = [0.95, 0.25, 0.22];
const Y_AXIS_COLOR: [f32; 3] = [0.28, 0.88, 0.38];
const Z_AXIS_COLOR: [f32; 3] = [0.25, 0.52, 0.98];

/// Errors raised while initializing or using the rendering infrastructure.
#[derive(Debug)]
pub enum RenderError {
    SurfaceCreation(String),
    AdapterRequest(String),
    DeviceRequest(String),
    CameraConfiguration(String),
    UnsupportedSurface,
    SurfaceValidation,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfaceCreation(message) => write!(f, "could not create GPU surface: {message}"),
            Self::AdapterRequest(message) => write!(f, "could not find a compatible GPU adapter: {message}"),
            Self::DeviceRequest(message) => write!(f, "could not create GPU device: {message}"),
            Self::CameraConfiguration(message) => write!(f, "could not configure reference camera: {message}"),
            Self::UnsupportedSurface => write!(f, "the selected adapter does not support this window surface"),
            Self::SurfaceValidation => write!(f, "wgpu reported a surface validation failure"),
        }
    }
}

impl Error for RenderError {}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LineVertex {
    position: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    // Four columns, matching WGSL mat4x4 column-major storage.
    view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    fn from_camera(camera: &Camera3) -> Self {
        let columns = camera.view_projection_matrix().to_cols_array_2d();
        Self {
            view_projection: columns.map(|column| column.map(|value| value as f32)),
        }
    }
}

/// Owns the native surface and renders a reference grid with world axes.
pub struct SceneRenderer {
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
    camera: Camera3,
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl SceneRenderer {
    /// Initialize wgpu and create the reference-scene pipeline.
    pub async fn new(window: Arc<Window>) -> Result<Self, RenderError> {
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

        let aspect_ratio = config.width as f64 / config.height as f64;
        let camera = reference_camera(aspect_ratio)?;
        let camera_uniform = CameraUniform::from_camera(&camera);

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("svf-camera-bind-group-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
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
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
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
            depth_stencil: None,
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

        tracing_adapter_info(&adapter);

        Ok(Self {
            instance,
            window,
            surface,
            device,
            queue,
            config,
            camera,
            camera_uniform_buffer,
            camera_bind_group,
            pipeline,
            vertex_buffer,
            vertex_count,
        })
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

        let camera_uniform = CameraUniform::from_camera(&self.camera);
        self.queue.write_buffer(
            &self.camera_uniform_buffer,
            0,
            bytemuck::bytes_of(&camera_uniform),
        );

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    /// Draw the reference grid and axes, then present the frame.
    pub fn render(&mut self) -> Result<(), RenderError> {
        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame)
            | CurrentSurfaceTexture::Suboptimal(frame) => frame,
            CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => return Ok(()),
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.reconfigure_surface();
                return Ok(());
            }
            CurrentSurfaceTexture::Validation => return Err(RenderError::SurfaceValidation),
        };

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("svf-reference-frame-encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("svf-reference-lines-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..self.vertex_count, 0..1);
        }

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);
        Ok(())
    }

    fn reconfigure_surface(&mut self) {
        if let Ok(surface) = self.instance.create_surface(self.window.clone()) {
            self.surface = surface;
            self.surface.configure(&self.device, &self.config);
        }
    }
}

fn reference_camera(aspect_ratio: f64) -> Result<Camera3, RenderError> {
    let pose = LookAtCamera::new(
        WorldPoint3::new(14.0, 12.0, 18.0),
        WorldPoint3::new(0.0, 0.0, 0.0),
        WorldVector3::new(0.0, 1.0, 0.0),
    )
    .map_err(|error| RenderError::CameraConfiguration(error.to_string()))?;

    let projection = PerspectiveProjection::new(
        45.0_f64.to_radians(),
        aspect_ratio,
        0.1,
        100.0,
    )
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
    push_line(&mut vertices, [-extent, 0.0, 0.0], [extent, 0.0, 0.0], X_AXIS_COLOR);
    push_line(&mut vertices, [0.0, -extent, 0.0], [0.0, extent, 0.0], Y_AXIS_COLOR);
    push_line(&mut vertices, [0.0, 0.0, -extent], [0.0, 0.0, extent], Z_AXIS_COLOR);

    vertices
}

fn push_line(
    vertices: &mut Vec<LineVertex>,
    start: [f32; 3],
    end: [f32; 3],
    color: [f32; 3],
) {
    vertices.push(LineVertex { position: start, color });
    vertices.push(LineVertex { position: end, color });
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
        let uniform = CameraUniform::from_camera(&camera);

        assert!(uniform
            .view_projection
            .iter()
            .flatten()
            .all(|value| value.is_finite()));
    }
}
