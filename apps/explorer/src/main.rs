use std::sync::Arc;

use svf_core::{
    camera_navigation::{self, CameraNavigationConfig},
    dataset::Dataset,
    fields::{
        FieldAssociation, FieldMetadata, ScalarField, VectorBasis3, VectorField3, VectorValue3,
    },
    geometry::{PointSet3, TriangleMesh3},
    isosurface::{extract_tetrahedral_isosurface, Isosurface3},
    sampling::{
        Domain1D, Domain2D, SampleValue, SampledSurface3, SamplingSpec2D, StructuredGrid2D,
        sample_scalar_2d,
    },
    scalar_visualization::{
        ColorMap, ColorRgba, MappedSurface3, ScalarRangeMode, SurfaceProvenance3,
        map_scalar_field,
    },
    spatial::WorldPoint3,
    surface_triangulation::TriangulatedSurface3,
    vector_glyphs::{build_vector_glyphs, VectorGlyphSet3, VectorScalePolicy3},
    vector_visualization::{analyze_vector_sources, resolve_vector_sources},
    unstructured::{CellType3, UnstructuredGrid3},
};
use svf_render::SceneRenderer;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
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
    StructuredScalar,
    UnstructuredScalar,
    Vectors,
    CellVectors,
    Isosurface,
}

impl ExplorerMode {
    fn window_title(self, amplitude: f64) -> String {
        match self {
            Self::Parametric => window_title(amplitude),
            Self::Unstructured => {
                "Scientific Visualization Framework — Unstructured volume (hex + tetra)"
                    .to_owned()
            }
            Self::StructuredScalar => {
                "Scientific Visualization Framework — Structured point scalar".to_owned()
            }
            Self::UnstructuredScalar => {
                "Scientific Visualization Framework — Unstructured cell scalar".to_owned()
            }
            Self::Vectors => {
                "Scientific Visualization Framework — 3D vector field glyphs".to_owned()
            }
            Self::CellVectors => {
                "Scientific Visualization Framework — structured cell vector glyphs".to_owned()
            }
            Self::Isosurface => {
                "Scientific Visualization Framework — tetrahedral isosurface (iso=0)".to_owned()
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
    _scalar_dataset: Option<Dataset>,
    _surface_provenance: Option<SurfaceProvenance3>,
    _mapped_surface: Option<MappedSurface3>,
    _vector_dataset: Option<Dataset>,
    _vector_glyphs: Option<VectorGlyphSet3>,
    _isosurface_dataset: Option<Dataset>,
    _isosurface: Option<Isosurface3>,
    cursor_position: Option<[f64; 2]>,
    active_drag: Option<MouseButton>,
    drag_origin: Option<[f64; 2]>,
    last_drag_position: Option<[f64; 2]>,
    drag_moved: bool,
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
            _scalar_dataset: None,
            _surface_provenance: None,
            _mapped_surface: None,
            _vector_dataset: None,
            _vector_glyphs: None,
            _isosurface_dataset: None,
            _isosurface: None,
            cursor_position: None,
            active_drag: None,
            drag_origin: None,
            last_drag_position: None,
            drag_moved: false,
        }
    }

    fn inspect_cursor_position(&mut self, log_details: bool) {
        if matches!(
            self.mode,
            ExplorerMode::Unstructured | ExplorerMode::UnstructuredScalar
        ) {
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

    fn navigate_camera(
        &mut self,
        button: Option<MouseButton>,
        delta_x: f64,
        delta_y: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(renderer) = self.renderer.as_mut() else {
            return Ok(());
        };
        let camera = *renderer.camera();
        let config = CameraNavigationConfig::default();
        let updated = match button {
            Some(MouseButton::Left) => {
                camera_navigation::orbit(&camera, delta_x, delta_y, config)?
            }
            Some(MouseButton::Middle) => {
                let height = self.window.as_ref().map_or(0, |window| window.inner_size().height);
                camera_navigation::pan(&camera, delta_x, delta_y, height)?
            }
            None => camera_navigation::zoom(&camera, delta_y, config)?,
            _ => return Ok(()),
        };
        renderer.set_camera(updated)?;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        Ok(())
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

        let renderer_result: Result<SceneRenderer, Box<dyn std::error::Error>> = (|| {
            match self.mode {
                ExplorerMode::Parametric => {
                    let surface = demo_surface(self.amplitude)?;
                    let renderer =
                        pollster::block_on(SceneRenderer::new(window.clone(), surface.mesh()))?;
                    self.surface = Some(surface);
                    Ok(renderer)
                }
                ExplorerMode::Unstructured => {
                    let (surface, mesh) = demo_unstructured_surface()?;
                    let renderer =
                        pollster::block_on(SceneRenderer::new(window.clone(), &mesh))?;
                    self._triangulated_surface = Some(surface);
                    Ok(renderer)
                }
                ExplorerMode::StructuredScalar => {
                    let (dataset, surface, provenance, mapped) =
                        demo_structured_scalar_surface()?;
                    let renderer = pollster::block_on(
                        SceneRenderer::new_with_mapped_surface(window.clone(), &mapped),
                    )?;
                    self.surface = Some(surface);
                    self._scalar_dataset = Some(dataset);
                    self._surface_provenance = Some(provenance);
                    self._mapped_surface = Some(mapped);
                    Ok(renderer)
                }
                ExplorerMode::UnstructuredScalar => {
                    let (dataset, surface, provenance, mapped) =
                        demo_unstructured_scalar_surface()?;
                    let renderer = pollster::block_on(
                        SceneRenderer::new_with_mapped_surface(window.clone(), &mapped),
                    )?;
                    self._triangulated_surface = Some(surface);
                    self._scalar_dataset = Some(dataset);
                    self._surface_provenance = Some(provenance);
                    self._mapped_surface = Some(mapped);
                    Ok(renderer)
                }
                ExplorerMode::Vectors => {
                    let surface = demo_surface(self.amplitude)?;
                    let (dataset, glyphs) = demo_vector_glyphs()?;
                    let renderer = pollster::block_on(SceneRenderer::new_with_vector_glyphs(
                        window.clone(),
                        surface.mesh(),
                        &glyphs,
                    ))?;
                    self.surface = Some(surface);
                    self._vector_dataset = Some(dataset);
                    self._vector_glyphs = Some(glyphs);
                    Ok(renderer)
                }
                ExplorerMode::CellVectors => {
                    let (dataset, surface, glyphs) = demo_cell_vector_glyphs()?;
                    let renderer = pollster::block_on(SceneRenderer::new_with_vector_glyphs(
                        window.clone(),
                        surface.mesh(),
                        &glyphs,
                    ))?;
                    self.surface = Some(surface);
                    self._vector_dataset = Some(dataset);
                    self._vector_glyphs = Some(glyphs);
                    Ok(renderer)
                }
                ExplorerMode::Isosurface => {
                    let (dataset, isosurface) = demo_tetrahedral_isosurface()?;
                    let renderer = pollster::block_on(SceneRenderer::new(
                        window.clone(),
                        isosurface.mesh(),
                    ))?;
                    self._isosurface_dataset = Some(dataset);
                    self._isosurface = Some(isosurface);
                    Ok(renderer)
                }
            }
        })();

        match renderer_result {
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
                let next = [position.x, position.y];
                self.cursor_position = Some(next);
                if let Some(button) = self.active_drag {
                    if let Some(previous) = self.drag_origin {
                        let total_x = next[0] - previous[0];
                        let total_y = next[1] - previous[1];
                        if total_x.hypot(total_y) > 3.0 {
                            self.drag_moved = true;
                        }
                    }
                    let previous = self.last_drag_position.replace(next).unwrap_or(next);
                    let delta_x = next[0] - previous[0];
                    let delta_y = next[1] - previous[1];
                    if let Err(error) = self.navigate_camera(Some(button), delta_x, delta_y) {
                        eprintln!("Camera navigation failed: {error}");
                    }
                } else {
                    self.inspect_cursor_position(false);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    self.active_drag = Some(button);
                    self.drag_origin = self.cursor_position;
                    self.last_drag_position = self.cursor_position;
                    self.drag_moved = false;
                }
                ElementState::Released => {
                    let was_click = self.active_drag == Some(MouseButton::Left)
                        && button == MouseButton::Left
                        && !self.drag_moved;
                    self.active_drag = None;
                    self.drag_origin = None;
                    self.last_drag_position = None;
                    self.drag_moved = false;
                    if was_click {
                        self.inspect_cursor_position(true);
                    }
                }
            },

            WindowEvent::MouseWheel { delta, .. } => {
                let wheel_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                    MouseScrollDelta::PixelDelta(position) => position.y / 100.0,
                };
                if let Err(error) = self.navigate_camera(None, 0.0, wheel_delta) {
                    eprintln!("Camera zoom failed: {error}");
                }
            }

            WindowEvent::Focused(false) | WindowEvent::CursorLeft { .. } => {
                self.active_drag = None;
                self.drag_origin = None;
                self.last_drag_position = None;
                self.drag_moved = false;
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
    let arguments = args.into_iter().collect::<Vec<_>>();
    if arguments.iter().any(|argument| argument == "--cell-vectors") {
        ExplorerMode::CellVectors
    } else if arguments.iter().any(|argument| argument == "--vectors") {
        ExplorerMode::Vectors
    } else if arguments.iter().any(|argument| argument == "--isosurface") {
        ExplorerMode::Isosurface
    } else if arguments.iter().any(|argument| argument == "--unstructured-scalar") {
        ExplorerMode::UnstructuredScalar
    } else if arguments.iter().any(|argument| argument == "--scalar") {
        ExplorerMode::StructuredScalar
    } else if arguments.iter().any(|argument| argument == "--unstructured") {
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

/// Builds a small point-associated vector field over deterministic world-space anchors.
fn demo_vector_glyphs() -> Result<(Dataset, VectorGlyphSet3), Box<dyn std::error::Error>> {
    let mut points = Vec::new();
    let mut values = Vec::new();
    for z_index in 0..7 {
        for x_index in 0..7 {
            let x = -4.0 + (x_index as f64 * 8.0 / 6.0);
            let z = -4.0 + (z_index as f64 * 8.0 / 6.0);
            let y = x.sin() * z.cos();
            points.push(WorldPoint3::new(x, y, z));
            values.push(VectorValue3::from_f64(-0.25 * z, 0.4, 0.25 * x));
        }
    }

    let mut dataset = Dataset::new(PointSet3::new(points)?);
    dataset.add_vector_field3(VectorField3::new(
        FieldMetadata::new("velocity", Some("world units / time".to_owned()))?,
        FieldAssociation::Point,
        VectorBasis3::World,
        values,
    )?)?;
    let sources = resolve_vector_sources(&dataset, "velocity")?;
    let analyses = analyze_vector_sources(&sources)?;
    let glyphs = build_vector_glyphs(
        &analyses,
        VectorScalePolicy3::UniformLength { length: 0.8 },
    )?;
    Ok((dataset, glyphs))
}

/// Builds a sparse cell-associated vector field on the same structured grid
/// used to generate its visible graph surface.
fn demo_cell_vector_glyphs(
) -> Result<(Dataset, SampledSurface3, VectorGlyphSet3), Box<dyn std::error::Error>> {
    let domain = Domain2D::new(Domain1D::new(-4.0, 4.0)?, Domain1D::new(-4.0, 4.0)?);
    let samples = sample_scalar_2d(domain, SamplingSpec2D::new(9, 9)?, |x, z| {
        x.sin() * z.cos()
    })?;
    let grid = StructuredGrid2D::from_samples(samples);
    let surface = grid.to_surface_mesh()?;

    let cell_count = (grid.x_sample_count() - 1) * (grid.y_sample_count() - 1);
    let mut values = Vec::with_capacity(cell_count);
    for z_index in 0..(grid.y_sample_count() - 1) {
        for x_index in 0..(grid.x_sample_count() - 1) {
            let x0 = grid.sample(x_index, z_index).expect("cell corner exists").x();
            let x1 = grid.sample(x_index + 1, z_index).expect("cell corner exists").x();
            let z0 = grid.sample(x_index, z_index).expect("cell corner exists").y();
            let z1 = grid.sample(x_index, z_index + 1).expect("cell corner exists").y();
            let center_x = (x0 + x1) * 0.5;
            let center_z = (z0 + z1) * 0.5;
            values.push(VectorValue3::from_f64(
                -0.3 * center_z,
                0.75,
                0.3 * center_x,
            ));
        }
    }

    let mut dataset = Dataset::new(grid);
    dataset.add_vector_field3(VectorField3::new(
        FieldMetadata::new("cell velocity", Some("world units / time".to_owned()))?,
        FieldAssociation::Cell,
        VectorBasis3::World,
        values,
    )?)?;
    let sources = resolve_vector_sources(&dataset, "cell velocity")?;
    let analyses = analyze_vector_sources(&sources)?;
    let glyphs = build_vector_glyphs(
        &analyses,
        VectorScalePolicy3::UniformLength { length: 1.0 },
    )?;
    Ok((dataset, surface, glyphs))
}

fn demo_structured_scalar_surface(
) -> Result<(Dataset, SampledSurface3, SurfaceProvenance3, MappedSurface3), Box<dyn std::error::Error>> {
    let domain = Domain2D::new(Domain1D::new(-5.0, 5.0)?, Domain1D::new(-5.0, 5.0)?);
    let samples = sample_scalar_2d(domain, SamplingSpec2D::new(41, 41)?, |x, y| {
        x.sin() * y.cos()
    })?;
    let values = samples
        .as_slice()
        .iter()
        .map(|sample| sample.value())
        .collect();
    let grid = StructuredGrid2D::from_samples(samples);
    let surface = grid.to_surface_mesh()?;

    let mut dataset = Dataset::new(grid.clone());
    dataset.add_scalar_field(ScalarField::new(
        FieldMetadata::new("height", Some("world units".to_owned()))?,
        FieldAssociation::Point,
        values,
    )?)?;
    let provenance = SurfaceProvenance3::from_structured(&surface, &grid)?;
    let mapped = map_scalar_field(
        &provenance,
        dataset.scalar_field("height").expect("field was just attached"),
        dataset.point_count(),
        dataset.cell_count(),
        ScalarRangeMode::Automatic,
        &ColorMap::viridis(),
        invalid_color()?,
    )?;

    Ok((dataset, surface, provenance, mapped))
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

fn demo_unstructured_scalar_surface(
) -> Result<(Dataset, TriangulatedSurface3, SurfaceProvenance3, MappedSurface3), Box<dyn std::error::Error>> {
    let grid = demo_unstructured_grid()?;
    let surface = grid.boundary_surface()?.triangulate();

    let mut dataset = Dataset::new(grid.clone());
    dataset.add_scalar_field(ScalarField::new(
        FieldMetadata::new("cell intensity", Some("relative".to_owned()))?,
        FieldAssociation::Cell,
        vec![SampleValue::Finite(0.18), SampleValue::Finite(0.92)],
    )?)?;
    let provenance = SurfaceProvenance3::from_unstructured(&surface, &grid)?;
    let mapped = map_scalar_field(
        &provenance,
        dataset.scalar_field("cell intensity").expect("field was just attached"),
        dataset.point_count(),
        dataset.cell_count(),
        ScalarRangeMode::Automatic,
        &ColorMap::viridis(),
        invalid_color()?,
    )?;

    Ok((dataset, surface, provenance, mapped))
}

/// Two tetrahedra share a face. The point scalar field crosses that face,
/// producing a connected isosurface patch on both sides of the shared face.
fn demo_tetrahedral_isosurface(
) -> Result<(Dataset, Isosurface3), Box<dyn std::error::Error>> {
    let points = vec![
        WorldPoint3::new(0.0, -2.0, -2.0),
        WorldPoint3::new(0.0, 2.0, -2.0),
        WorldPoint3::new(0.0, 0.0, 2.0),
        WorldPoint3::new(3.0, 0.0, 0.0),
        WorldPoint3::new(-3.0, 0.0, 0.0),
    ];
    let grid = UnstructuredGrid3::new(
        points,
        vec![CellType3::Tetrahedron4, CellType3::Tetrahedron4],
        vec![0, 4, 8],
        vec![0, 1, 2, 3, 0, 2, 1, 4],
    )?;
    let mut dataset = Dataset::new(grid);
    dataset.add_scalar_field(ScalarField::new(
        FieldMetadata::new("signed distance", Some("world units".to_owned()))?,
        FieldAssociation::Point,
        vec![
            SampleValue::Finite(-1.0),
            SampleValue::Finite(1.0),
            SampleValue::Finite(-1.0),
            SampleValue::Finite(1.0),
            SampleValue::Finite(-1.0),
        ],
    )?)?;
    let isosurface = extract_tetrahedral_isosurface(&dataset, "signed distance", 0.0)?;
    Ok((dataset, isosurface))
}

fn invalid_color() -> Result<ColorRgba, svf_core::scalar_visualization::ColorMapError> {
    ColorRgba::new(0.35, 0.05, 0.22, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use svf_core::boundary_surface::BoundaryFaceVertices3;

    #[test]
    fn tetrahedral_isosurface_demo_builds_connected_provenance_mapped_geometry() {
        let (dataset, isosurface) = demo_tetrahedral_isosurface().unwrap();
        assert_eq!(dataset.point_count(), 5);
        assert_eq!(dataset.cell_count(), 2);
        assert_eq!(isosurface.isovalue(), 0.0);
        assert_eq!(isosurface.mesh().triangle_count(), 2);
        assert_eq!(isosurface.triangle_source_cells().len(), 2);
        assert!(isosurface.triangle_source_cells().iter().all(|cell| *cell < 2));
        assert_eq!(isosurface.vertex_provenance().len(), isosurface.mesh().vertex_count());
        assert!(isosurface.metadata().unsupported_cells().is_empty());
        assert!(isosurface.metadata().skipped_cells().is_empty());
    }

    #[test]
    fn isosurface_mode_is_selected_from_command_line() {
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned(), "--isosurface".to_owned()]),
            ExplorerMode::Isosurface
        );
    }

    #[test]
    fn vector_demo_builds_a_point_associated_field_and_glyphs() {
        let (dataset, glyphs) = demo_vector_glyphs().unwrap();
        assert_eq!(dataset.point_count(), 49);
        assert_eq!(glyphs.entries().len(), 49);
        assert!(glyphs.entries().iter().all(|entry| entry.segments().is_some()));
        assert!(glyphs.entries().iter().all(|entry| entry.display_length() == Some(0.8)));
    }

    #[test]
    fn cell_vector_demo_builds_structured_cell_anchors_and_glyphs() {
        use svf_core::vector_visualization::VectorEntity3;

        let (dataset, surface, glyphs) = demo_cell_vector_glyphs().unwrap();
        assert_eq!(dataset.point_count(), 81);
        assert_eq!(dataset.cell_count(), 64);
        assert_eq!(surface.mesh().triangle_count(), 128);
        assert_eq!(glyphs.entries().len(), 64);
        let field = dataset.vector_field3("cell velocity").unwrap();
        assert_eq!(field.association(), FieldAssociation::Cell);
        assert_eq!(field.len(), dataset.cell_count());
        assert!(glyphs.entries().iter().all(|entry| {
            entry.segments().is_some() && entry.display_length() == Some(1.0)
        }));

        let svf_core::dataset::DatasetGeometry::StructuredGrid2D(grid) = dataset.geometry() else {
            panic!("cell vector fixture must use a structured grid");
        };
        for (cell_id, entry) in glyphs.entries().iter().enumerate() {
            let analysis = entry.analysis();
            assert_eq!(analysis.source().entity(), VectorEntity3::Cell(cell_id));
            let x_count = grid.x_sample_count() - 1;
            let x = cell_id % x_count;
            let z = cell_id / x_count;
            let corners = [
                grid.sample(x, z).unwrap(),
                grid.sample(x + 1, z).unwrap(),
                grid.sample(x, z + 1).unwrap(),
                grid.sample(x + 1, z + 1).unwrap(),
            ];
            let expected = WorldPoint3::new(
                corners.iter().map(|sample| sample.x()).sum::<f64>() / 4.0,
                corners.iter().map(|sample| sample.value().as_finite().unwrap()).sum::<f64>() / 4.0,
                corners.iter().map(|sample| sample.y()).sum::<f64>() / 4.0,
            );
            let actual = analysis.source().anchor().to_array();
            let expected = expected.to_array();
            assert!(actual.iter().zip(expected).all(|(actual, expected)| {
                (actual - expected).abs() < 1.0e-12
            }));
        }
    }

    #[test]
    fn cell_vectors_mode_is_selected_from_command_line() {
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned(), "--cell-vectors".to_owned()]),
            ExplorerMode::CellVectors
        );
    }

    #[test]
    fn vectors_mode_is_selected_from_command_line() {
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned(), "--vectors".to_owned()]),
            ExplorerMode::Vectors
        );
    }

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
    fn structured_scalar_demo_maps_point_values_to_surface_corners() {
        let (dataset, surface, provenance, mapped) =
            demo_structured_scalar_surface().unwrap();

        assert_eq!(dataset.point_count(), 41 * 41);
        assert_eq!(dataset.cell_count(), 40 * 40);
        assert_eq!(surface.mesh().triangle_count(), 40 * 40 * 2);
        assert_eq!(mapped.triangle_count(), surface.mesh().triangle_count());
        assert_eq!(mapped.positions().len(), mapped.colors().len());
        assert_eq!(mapped.source_point_ids().len(), mapped.positions().len());
        assert_eq!(provenance.source_point_ids().len(), surface.mesh().vertex_count());
        assert_eq!(dataset.scalar_field("height").unwrap().association(), FieldAssociation::Point);
        assert!(mapped.colors().iter().any(|color| color != &mapped.colors()[0]));
    }

    #[test]
    fn unstructured_scalar_demo_maps_each_source_cell_to_its_surface_triangles() {
        let (dataset, surface, provenance, mapped) =
            demo_unstructured_scalar_surface().unwrap();

        assert_eq!(dataset.point_count(), 12);
        assert_eq!(dataset.cell_count(), 2);
        assert_eq!(surface.triangle_count(), 16);
        assert_eq!(mapped.triangle_count(), 16);
        assert_eq!(dataset.scalar_field("cell intensity").unwrap().association(), FieldAssociation::Cell);
        assert_eq!(provenance.source_cell_ids(), mapped.source_cell_ids());
        let hexahedron_triangles = mapped.source_cell_ids().iter().filter(|&&id| id == 0).count();
        let tetrahedron_triangles = mapped.source_cell_ids().iter().filter(|&&id| id == 1).count();
        assert_eq!(hexahedron_triangles, 12);
        assert_eq!(tetrahedron_triangles, 4);

        for (triangle, _) in mapped.source_cell_ids().iter().enumerate() {
            let colors = &mapped.colors()[triangle * 3..triangle * 3 + 3];
            assert!(colors.iter().all(|color| color == &colors[0]));
        }
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
    fn command_line_selects_each_optional_mode_only_for_exact_flags() {
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned()]),
            ExplorerMode::Parametric
        );
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned(), "--unstructured".to_owned()]),
            ExplorerMode::Unstructured
        );
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned(), "--scalar".to_owned()]),
            ExplorerMode::StructuredScalar
        );
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned(), "--unstructured-scalar".to_owned()]),
            ExplorerMode::UnstructuredScalar
        );
        assert_eq!(
            mode_from_args(vec!["svf-explorer".to_owned(), "--unstructured-extra".to_owned()]),
            ExplorerMode::Parametric
        );
    }

    #[test]
    fn optional_modes_do_not_change_parametric_default_or_existing_unstructured_mode() {
        assert_eq!(ExplorerMode::default(), ExplorerMode::Parametric);
        assert!(ExplorerMode::Parametric.window_title(1.0).contains("A=1.00"));
        assert!(ExplorerMode::Unstructured
            .window_title(1.0)
            .contains("Unstructured volume"));
        assert!(ExplorerMode::StructuredScalar
            .window_title(1.0)
            .contains("Structured point scalar"));
        assert!(ExplorerMode::UnstructuredScalar
            .window_title(1.0)
            .contains("Unstructured cell scalar"));
    }
}
