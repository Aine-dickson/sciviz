use std::sync::Arc;

use svf_core::sampling::SampledSurface3;

use svf_core::sampling::{Domain1D, Domain2D, SamplingSpec2D, StructuredGrid2D, sample_scalar_2d};
use svf_render::SceneRenderer;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const INITIAL_AMPLITUDE: f64 = 1.0;
const AMPLITUDE_STEP: f64 = 0.25;
const MAX_AMPLITUDE: f64 = 3.0;
const MIN_AMPLITUDE: f64 = 0.0;

struct Explorer {
    window: Option<Arc<Window>>,
    renderer: Option<SceneRenderer>,
    amplitude: f64,
    surface: Option<SampledSurface3>,
    cursor_position: Option<[f64; 2]>,
}

impl Default for Explorer {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            amplitude: INITIAL_AMPLITUDE,
            surface: None,
            cursor_position: None,
        }
    }
}

impl Explorer {
    fn inspect_cursor_position(&mut self, log_details: bool) {
        let (Some(window), Some(renderer), Some(surface), Some([x, y])) = (
            self.window.as_ref(),
            self.renderer.as_ref(),
            self.surface.as_ref(),
            self.cursor_position,
        ) else {
            if log_details {
                eprintln!("Surface inspection click received, but no cursor position is available");
            }
            return;
        };

        let size = window.inner_size();
        if log_details {
            eprintln!(
                "Surface inspection click: pixel=({x:.1}, {y:.1}), viewport={}x{}",
                size.width, size.height
            );
        }

        let ray = match renderer
            .camera()
            .ray_from_viewport(x, y, size.width, size.height)
        {
            Ok(ray) => ray,
            Err(error) => {
                window.set_title(&format!(
                    "{} | inspection ray error: {error}",
                    window_title(self.amplitude)
                ));
                if log_details {
                    eprintln!("Surface inspection ray error: {error}");
                }
                return;
            }
        };

        match surface.inspect_ray(&ray) {
            Some(hit) => {
                let [domain_x, domain_y] = hit.domain_coordinates();
                let [grid_x, grid_y] = hit.grid_coordinates();
                let cell = hit.source_cell();
                window.set_title(&format!(
                    "{} | hover x={domain_x:.3}, y={domain_y:.3}, f={:.3}",
                    window_title(self.amplitude),
                    hit.scalar_value(),
                ));
                if log_details {
                    eprintln!(
                        "Surface inspection HIT: world={:?}, domain=({domain_x:.6}, {domain_y:.6}), \
                         grid=({grid_x:.3}, {grid_y:.3}), scalar={:.6}, cell=({}, {}), \
                         triangle={}, barycentric={:?}",
                        hit.world_position().to_array(),
                        hit.scalar_value(),
                        cell.x(),
                        cell.y(),
                        hit.triangle_index(),
                        hit.barycentric(),
                    );
                }
            }
            None => {
                window.set_title(&format!(
                    "{} | hover: no surface",
                    window_title(self.amplitude)
                ));
                if log_details {
                    eprintln!("Surface inspection MISS: ray did not intersect the sampled surface");
                }
            }
        }
    }

    fn adjust_amplitude(&mut self, delta: f64) -> Result<(), Box<dyn std::error::Error>> {
        let amplitude = (self.amplitude + delta).clamp(MIN_AMPLITUDE, MAX_AMPLITUDE);
        if (amplitude - self.amplitude).abs() <= f64::EPSILON {
            return Ok(());
        }

        // Re-evaluate the mathematical model and regenerate its CPU-side mesh.
        // The renderer keeps its pipelines, camera, grid, axes, and depth target.
        let surface = demo_surface(amplitude)?;
        let Some(renderer) = self.renderer.as_mut() else {
            return Ok(());
        };
        renderer.update_mesh(surface.mesh())?;
        self.surface = Some(surface);

        self.amplitude = amplitude;
        if let Some(window) = self.window.as_ref() {
            window.set_title(&window_title(amplitude));
            window.request_redraw();
        }
        Ok(())
    }
}

impl ApplicationHandler for Explorer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(window_title(self.amplitude))
            .with_inner_size(PhysicalSize::new(1280, 800));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("Failed to create SVF Explorer window: {error}");
                event_loop.exit();
                return;
            }
        };

        let surface = match demo_surface(self.amplitude) {
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
                self.surface = Some(surface);
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
        if !self
            .window
            .as_ref()
            .is_some_and(|window| window.id() == window_id)
        {
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
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some([position.x, position.y]);
                self.inspect_cursor_position(false);
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.inspect_cursor_position(true);
            }

            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                let delta = match event.physical_key {
                    PhysicalKey::Code(KeyCode::ArrowUp) => Some(AMPLITUDE_STEP),
                    PhysicalKey::Code(KeyCode::ArrowDown) => Some(-AMPLITUDE_STEP),
                    _ => None,
                };

                if let Some(delta) = delta {
                    if let Err(error) = self.adjust_amplitude(delta) {
                        eprintln!("Could not update surface amplitude: {error}");
                        event_loop.exit();
                    }
                }
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

fn window_title(amplitude: f64) -> String {
    format!("Scientific Visualization Framework — A={amplitude:.2} (↑/↓)")
}

/// Samples f(x, y) = A * sin(x) * cos(y) over a square XZ domain.
/// The first domain coordinate maps to world X, the second to world Z,
/// and the scalar function output maps to world Y.
fn demo_surface(
    amplitude: f64,
) -> Result<svf_core::sampling::SampledSurface3, Box<dyn std::error::Error>> {
    let domain = Domain2D::new(Domain1D::new(-5.0, 5.0)?, Domain1D::new(-5.0, 5.0)?);
    let samples = sample_scalar_2d(domain, SamplingSpec2D::new(41, 41)?, |x, y| {
        amplitude * x.sin() * y.cos()
    })?;
    Ok(StructuredGrid2D::from_samples(samples).to_surface_mesh()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amplitude_scales_surface_height_without_changing_topology() {
        let half_amplitude = demo_surface(0.5).unwrap();
        let full_amplitude = demo_surface(1.0).unwrap();

        assert_eq!(
            half_amplitude.mesh().indices(),
            full_amplitude.mesh().indices()
        );
        assert_eq!(half_amplitude.vertex_count(), full_amplitude.vertex_count());

        for (half, full) in half_amplitude
            .mesh()
            .vertices()
            .iter()
            .zip(full_amplitude.mesh().vertices())
        {
            assert!((full.to_array()[1] - 2.0 * half.to_array()[1]).abs() < 1.0e-12);
        }

        assert!(
            full_amplitude
                .mesh()
                .vertices()
                .iter()
                .any(|point| point.to_array()[1] > 0.5)
        );
        assert!(
            full_amplitude
                .mesh()
                .vertices()
                .iter()
                .any(|point| point.to_array()[1] < -0.5)
        );
    }
}
