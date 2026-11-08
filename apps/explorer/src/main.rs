use std::sync::Arc;

use svf_core::{
    geometry::TriangleMesh3,
    sampling::{
        Domain1D, Domain2D, SampledSurface3, SamplingSpec2D, StructuredGrid2D,
        sample_scalar_2d,
    },
    spatial::WorldPoint3,
    surface_triangulation::TriangulatedSurface3,
    unstructured::{CellType3, UnstructuredGrid3},
};
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ExplorerMode {
    #[default]
    Parametric,
    Unstructured,
}

impl ExplorerMode {
    fn window_title(self, amplitude: f64) -> String {
        match self {
            Self::Parametric => window_title(amplitude),
            Self::Unstructured => {
                "Scientific Visualization Framework — Unstructured volume (hex + tetra)"
                    .to_owned()
            }
        }
    }
}

struct Explorer {
    window: Option<Arc<Window>>,
    renderer: Option<SceneRenderer>,
    mode: ExplorerMode,
    amplitude: f64,
    surface: Option<SampledSurface3>,
    _triangulated_surface: Option<TriangulatedSurface3>,
    cursor_position: Option<[f64; 2]>,
}

impl Default for Explorer {
    fn default() -> Self {
        Self::new(ExplorerMode::default())
    }
}

impl Explorer {
    fn new(mode: ExplorerMode) -> Self {
        Self {
            window: None,
            renderer: None,
            mode,
            amplitude: INITIAL_AMPLITUDE,
            surface: None,
            _triangulated_surface: None,
            cursor_position: None,
        }
    }

    fn inspect_cursor_position(&mut self, log_details: bool) {
        if self.mode == ExplorerMode::Unstructured {
            if log_details {
                eprintln!(
                    "Unstructured surface inspection is not implemented in this increment"
                );
            }
            return;
        }

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
                    self.mode.window_title(self.amplitude)
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
                    self.mode.window_title(self.amplitude),
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
                    self.mode.window_title(self.amplitude)
                ));
                if log_details {
                    eprintln!("Surface inspection MISS: ray did not intersect the sampled surface");
                }
            }
        }
    }

    fn adjust_amplitude(&mut self, delta: f64) -> Result<(), Box<dyn std::error::Error>> {
        if self.mode != ExplorerMode::Parametric {
            return Ok(());
        }

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
            window.set_title(&self.mode.window_title(amplitude));
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
            .with_title(self.mode.window_title(self.amplitude))
            .with_inner_size(PhysicalSize::new(1280, 800));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("Failed to create SVF Explorer window: {error}");
                event_loop.exit();
                return;
            }
        };

        let (mesh, sampled_surface, triangulated_surface) = match self.mode {
            ExplorerMode::Parametric => match demo_surface(self.amplitude) {
                Ok(surface) => (surface.mesh().clone(), Some(surface), None),
                Err(error) => {
                    eprintln!("Failed to generate demonstration surface: {error}");
                    event_loop.exit();
                    return;
                }
            },
            ExplorerMode::Unstructured => match demo_unstructured_surface() {
                Ok((surface, mesh)) => (mesh, None, Some(surface)),
                Err(error) => {
                    eprintln!("Failed to generate unstructured demonstration surface: {error}");
                    event_loop.exit();
                    return;
                }
            },
        };

        match pollster::block_on(SceneRenderer::new(window.clone(), &mesh)) {
            Ok(renderer) => {
                self.renderer = Some(renderer);
                self.surface = sampled_surface;
                self._triangulated_surface = triangulated_surface;
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
    let mode = mode_from_args(std::env::args());

    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(error) => {
            eprintln!("Failed to create event loop: {error}");
            return;
        }
    };

    // Wait when idle; the window requests redraws only when needed.
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = Explorer::new(mode);
    if let Err(error) = event_loop.run_app(&mut app) {
        eprintln!("SVF Explorer terminated with an event-loop error: {error}");
    }
}

fn mode_from_args(args: impl IntoIterator<Item = String>) -> ExplorerMode {
    if args.into_iter().any(|argument| argument == "--unstructured") {
        ExplorerMode::Unstructured
    } else {
        ExplorerMode::Parametric
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
) -> Result<SampledSurface3, Box<dyn std::error::Error>> {
    let domain = Domain2D::new(Domain1D::new(-5.0, 5.0)?, Domain1D::new(-5.0, 5.0)?);
    let samples = sample_scalar_2d(domain, SamplingSpec2D::new(41, 41)?, |x, y| {
        amplitude * x.sin() * y.cos()
    })?;
    Ok(StructuredGrid2D::from_samples(samples).to_surface_mesh()?)
}

fn demo_unstructured_grid() -> Result<UnstructuredGrid3, Box<dyn std::error::Error>> {
    let points = vec![
        // Hexahedron centered at the origin.
        WorldPoint3::new(-2.0, -1.0, -2.0),
        WorldPoint3::new(2.0, -1.0, -2.0),
        WorldPoint3::new(2.0, 1.0, -2.0),
        WorldPoint3::new(-2.0, 1.0, -2.0),
        WorldPoint3::new(-2.0, -1.0, 2.0),
        WorldPoint3::new(2.0, -1.0, 2.0),
        WorldPoint3::new(2.0, 1.0, 2.0),
        WorldPoint3::new(-2.0, 1.0, 2.0),
        // A disconnected tetrahedron to the right of the hexahedron.
        WorldPoint3::new(4.0, -1.0, -1.0),
        WorldPoint3::new(6.0, -1.0, -1.0),
        WorldPoint3::new(5.0, 1.0, -1.0),
        WorldPoint3::new(5.0, 0.0, 1.0),
    ];

    Ok(UnstructuredGrid3::new(
        points,
        vec![CellType3::Hexahedron8, CellType3::Tetrahedron4],
        vec![0, 8, 12],
        (0..12).collect(),
    )?)
}

fn triangulated_surface_to_mesh(
    surface: &TriangulatedSurface3,
) -> Result<TriangleMesh3, svf_core::geometry::GeometryError> {
    let indices = surface
        .triangles()
        .iter()
        .flat_map(|triangle| triangle.vertices())
        .collect();
    TriangleMesh3::new(surface.points().to_vec(), indices)
}

fn demo_unstructured_surface(
) -> Result<(TriangulatedSurface3, TriangleMesh3), Box<dyn std::error::Error>> {
    let grid = demo_unstructured_grid()?;
    let boundary = grid.boundary_surface()?;
    let surface = boundary.triangulate();
    let mesh = triangulated_surface_to_mesh(&surface)?;
    Ok((surface, mesh))
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

    #[test]
    fn unstructured_fixture_has_one_hexahedron_and_one_tetrahedron() {
        let grid = demo_unstructured_grid().unwrap();
        assert_eq!(grid.point_count(), 12);
        assert_eq!(grid.cell_count(), 2);
        assert_eq!(
            grid.cell_types(),
            &[CellType3::Hexahedron8, CellType3::Tetrahedron4]
        );
        assert_eq!(grid.cell_offsets(), &[0, 8, 12]);
        assert_eq!(grid.connectivity(), &(0..12).collect::<Vec<_>>());
        assert!(grid.points().iter().all(|point| point.to_vector().is_finite()));
    }

    #[test]
    fn unstructured_fixture_extracts_six_quads_and_four_triangles() {
        let grid = demo_unstructured_grid().unwrap();
        let boundary = grid.boundary_surface().unwrap();

        assert_eq!(boundary.point_count(), 12);
        assert_eq!(boundary.face_count(), 10);
        assert_eq!(
            boundary
                .faces()
                .iter()
                .filter(|face| matches!(face.vertices(), BoundaryFaceVertices3::Quad(_)))
                .count(),
            6
        );
        assert_eq!(
            boundary
                .faces()
                .iter()
                .filter(|face| matches!(face.vertices(), BoundaryFaceVertices3::Triangle(_)))
                .count(),
            4
        );
    }

    #[test]
    fn unstructured_fixture_triangulates_to_sixteen_provenance_mapped_triangles() {
        let grid = demo_unstructured_grid().unwrap();
        let boundary = grid.boundary_surface().unwrap();
        let surface = boundary.triangulate();

        assert_eq!(surface.point_count(), 12);
        assert_eq!(surface.triangle_count(), 16);
        assert_eq!(surface.source_point_ids(), &(0..12).collect::<Vec<_>>());

        let from_hexahedron = surface
            .triangles()
            .iter()
            .filter(|triangle| triangle.source_cell_index() == 0)
            .count();
        let from_tetrahedron = surface
            .triangles()
            .iter()
            .filter(|triangle| triangle.source_cell_index() == 1)
            .count();
        assert_eq!(from_hexahedron, 12);
        assert_eq!(from_tetrahedron, 4);

        for triangle in surface.triangles() {
            let source_face = &boundary.faces()[triangle.boundary_face_index()];
            assert_eq!(triangle.source_face_index(), source_face.source_face_index());
            assert_eq!(triangle.source_cell_index(), source_face.source_cell_index());
            assert_eq!(
                triangle.source_local_face_index(),
                source_face.source_local_face_index()
            );
            assert!(triangle.source_triangle_index() < 2);
        }
    }

    #[test]
    fn unstructured_render_mesh_preserves_points_and_triangle_connectivity() {
        let (surface, mesh) = demo_unstructured_surface().unwrap();

        assert_eq!(mesh.vertices(), surface.points());
        assert_eq!(mesh.vertex_count(), 12);
        assert_eq!(mesh.triangle_count(), 16);
        assert_eq!(
            mesh.triangles().collect::<Vec<_>>(),
            surface
                .triangles()
                .iter()
                .map(|triangle| triangle.vertices())
                .collect::<Vec<_>>()
        );
        assert!(mesh
            .indices()
            .iter()
            .all(|&index| (index as usize) < mesh.vertex_count()));
        assert!(mesh.vertices().iter().all(|point| point.to_vector().is_finite()));
    }

    #[test]
    fn command_line_selects_unstructured_mode_only_for_exact_flag() {
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned()]),
            ExplorerMode::Parametric
        );
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned(), "--unstructured".to_owned()]),
            ExplorerMode::Unstructured
        );
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned(), "--unstructured-extra".to_owned()]),
            ExplorerMode::Parametric
        );
    }

    #[test]
    fn unstructured_mode_does_not_change_parametric_default() {
        assert_eq!(ExplorerMode::default(), ExplorerMode::Parametric);
        assert!(ExplorerMode::Parametric.window_title(1.0).contains("A=1.00"));
        assert!(ExplorerMode::Unstructured
            .window_title(1.0)
            .contains("Unstructured volume"));
    }
}
