//! GPU side of the overlay (`docs/labels.md`, slice L3): one alpha-blended
//! pipeline of screen-space quads drawn last, with no depth test, reading glyph
//! coverage from an R8 atlas texture.

use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::overlay::{Overlay, OverlayVertex, TextKit, ATLAS_SIZE};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TargetUniform {
    size: [f32; 2],
    _pad: [f32; 2],
}

pub(crate) struct OverlayRenderer {
    pipeline: wgpu::RenderPipeline,
    target_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    atlas_texture: wgpu::Texture,
    /// The atlas generation last uploaded.
    uploaded: Option<u64>,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_count: u32,
}

impl OverlayRenderer {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, depth_format: wgpu::TextureFormat) -> Self {
        let target_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("svf-overlay-target"),
            contents: bytemuck::bytes_of(&TargetUniform { size: [1.0, 1.0], _pad: [0.0; 2] }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("svf-overlay-atlas"),
            size: wgpu::Extent3d { width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Nearest: glyph quads are placed on whole pixels, so text stays crisp.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("svf-overlay-sampler"), ..Default::default() });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("svf-overlay-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("svf-overlay-bind-group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: target_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("svf-overlay-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("overlay.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("svf-overlay-pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];
        let buffers = [Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<OverlayVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        })];
        let targets = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("svf-overlay-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            // The scene pass has a depth attachment; the overlay ignores it.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, target_buffer, bind_group, atlas_texture, uploaded: None, vertex_buffer: None, vertex_count: 0 }
    }

    /// Replace what the overlay draws. Uploads the atlas only when it changed.
    pub(crate) fn set(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, overlay: &Overlay, kit: &TextKit) {
        let (pixels, generation) = kit.atlas();
        if self.uploaded != Some(generation) {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(ATLAS_SIZE), rows_per_image: Some(ATLAS_SIZE) },
                wgpu::Extent3d { width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1 },
            );
            self.uploaded = Some(generation);
        }
        let size = [overlay.size[0].max(1.0), overlay.size[1].max(1.0)];
        queue.write_buffer(&self.target_buffer, 0, bytemuck::bytes_of(&TargetUniform { size, _pad: [0.0; 2] }));
        self.vertex_count = u32::try_from(overlay.vertices.len()).unwrap_or(0);
        self.vertex_buffer = (self.vertex_count > 0).then(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("svf-overlay-vertices"),
                contents: bytemuck::cast_slice(&overlay.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
    }

    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(buffer) = &self.vertex_buffer {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..self.vertex_count, 0..1);
        }
    }
}
