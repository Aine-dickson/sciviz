use std::sync::Arc;

use svf_core::sampling::{
    sample_scalar_2d, Domain1D, Domain2D, SamplingSpec2D, StructuredGrid2D,
};
use svf_render::SceneRenderer;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct Explorer {
    window: Option<Arc<Window>>,
    renderer: Option<SceneRenderer>,
}

impl ApplicationHandler for Explorer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Scientific Visualization Framework")
            .with_inner_size(PhysicalSize::new(1280, 800));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("Failed to create SVF Explorer window: {error}");
                event_loop.exit();
                return;
            }
        };

        let surface = match demo_surface() {
            Ok(surface) => surface,
            Err(error) => {
                eprintln!("Failed to generate demonstration surface: {error}");
                event_loop.exit();
                return;
            }
        };

        match pollster::block_on(SceneRenderer::new(window.clone(), surface.mesh())) {
            Ok(renderer) => {
                self.renderer = Some(renderer);
                self.window = Some(window.clone());
                window.request_redraw();
            }
            Err(error) => {
                eprintln!("Failed to initialize SVF graphics: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.resize(size) {
                        eprintln!("Renderer resize failed: {error}");
                        event_loop.exit();
                        return;
                    }
                }
                window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.render() {
                        eprintln!("Rendering failed: {error}");
                        event_loop.exit();
                    }
                }
            }

            _ => {}
        }
    }
}

fn main() {
    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            eprintln!("Failed to create event loop: {error}");
            return;
        }
    };

    // Wait when idle; the window requests redraws only when needed.
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = Explorer::default();
    if let Err(error) = event_loop.run_app(&mut app) {
        eprintln!("SVF Explorer terminated with an event-loop error: {error}");
    }
}

/// Creates a small flat graph surface to exercise indexed triangle rendering.
/// M5.2 will replace this fixture with the adjustable parametric function.
fn demo_surface() -> Result<svf_core::sampling::SampledSurface3, Box<dyn std::error::Error>> {
    let domain = Domain2D::new(
        Domain1D::new(-5.0, 5.0)?,
        Domain1D::new(-5.0, 5.0)?,
    );
    let samples = sample_scalar_2d(domain, SamplingSpec2D::new(21, 21)?, |_, _| 0.0)?;
    Ok(StructuredGrid2D::from_samples(samples).to_surface_mesh()?)
}
