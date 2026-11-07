#![forbid(unsafe_code)]
//! Minimal native wgpu presentation layer.
//!
//! This crate deliberately knows about the host window in this first
//! milestone. Later milestones can separate a host/surface adapter from the
//! backend-independent renderer API without leaking wgpu types into svf-core.

use std::{error::Error, fmt, sync::Arc};

use wgpu::{CurrentSurfaceTexture, Surface, SurfaceConfiguration};
use winit::{dpi::PhysicalSize, window::Window};

/// Errors raised while initializing or using the rendering infrastructure.
#[derive(Debug)]
pub enum RenderError {
    SurfaceCreation(String),
    AdapterRequest(String),
    DeviceRequest(String),
    UnsupportedSurface,
    SurfaceValidation,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfaceCreation(message) => write!(f, "could not create GPU surface: {message}"),
            Self::AdapterRequest(message) => write!(f, "could not find a compatible GPU adapter: {message}"),
            Self::DeviceRequest(message) => write!(f, "could not create GPU device: {message}"),
            Self::UnsupportedSurface => write!(f, "the selected adapter does not support this window surface"),
            Self::SurfaceValidation => write!(f, "wgpu reported a surface validation failure"),
        }
    }
}

impl Error for RenderError {}

/// Owns the GPU resources required to clear and present a native window.
pub struct ClearRenderer {
    instance: wgpu::Instance,
    window: Arc<Window>,
    surface: Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
}

impl ClearRenderer {
    /// Initialize wgpu for the supplied window.
    ///
    /// The window is retained by `Arc`, allowing wgpu's surface to own a
    /// window handle and therefore use a `'static` surface lifetime safely.
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

        tracing_adapter_info(&adapter);

        Ok(Self {
            instance,
            window,
            surface,
            device,
            queue,
            config,
        })
    }

    /// Reconfigure the presentation surface after a non-zero resize.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            // Minimized windows may report a zero-sized drawable area.
            // Keep the last valid configuration until a non-zero size arrives.
            return;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Clear and present one frame.
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
            label: Some("svf-clear-frame-encoder"),
        });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("svf-clear-pass"),
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
        }

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(frame);
        Ok(())
    }

    fn reconfigure_surface(&mut self) {
        // Recreate the surface from the retained window handle, then apply
        // the last known non-zero configuration.
        if let Ok(surface) = self.instance.create_surface(self.window.clone()) {
            self.surface = surface;
            self.surface.configure(&self.device, &self.config);
        }
    }
}

fn tracing_adapter_info(adapter: &wgpu::Adapter) {
    let info = adapter.get_info();
    eprintln!(
        "SVF graphics adapter: {} ({:?}, {:?})",
        info.name, info.backend, info.device_type
    );
}
