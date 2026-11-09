use std::sync::Arc;

mod vtk_loading;

use vtk_loading::ImportedScene;

use svf_core::{
    camera_navigation::{self, CameraNavigationConfig},
    dataset::Dataset,
    fields::{
        FieldAssociation, FieldMetadata, ScalarField, VectorBasis3, VectorField3, VectorValue3,
    },
    geometry::{PointSet3, TriangleMesh3},
    isosurface::{extract_tetrahedral_isosurface, Isosurface3},
    isosurface_field_transfer::{map_transferred_scalar_field, transfer_point_scalar_field, MappedIsosurface3},
    sampling::{
        Domain1D, Domain2D, SampleValue, SampledSurface3, SamplingSpec2D, StructuredGrid2D,
        sample_scalar_2d,
    },
    scalar_visualization::{
        ColorMap, ColorRgba, MappedSurface3, ScalarRangeMode, SurfaceProvenance3,
        map_scalar_field,
    },
    spatial::WorldPoint3,
    surface_inspection::{inspect_boundary_surface, inspect_isosurface, InspectedEntity3},
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
    ColoredIsosurface,
    Imported,
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
            Self::ColoredIsosurface => {
                "Scientific Visualization Framework — colored tetrahedral isosurface".to_owned()
            }
            Self::Imported => "Scientific Visualization Framework — imported VTK dataset".to_owned(),
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
    _mapped_isosurface: Option<MappedIsosurface3>,
    imported_scene: Option<ImportedScene>,
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
            _mapped_isosurface: None,
            imported_scene: None,
            cursor_position: None,
            active_drag: None,
            drag_origin: None,
            last_drag_position: None,
            drag_moved: false,
        }
    }

    fn inspect_cursor_position(&mut self, log_details: bool) {
        if self.mode == ExplorerMode::Imported {
            self.inspect_imported_cursor_position(log_details);
            return;
        }

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
        if size.width == 0 || size.height == 0 {
            if log_details {
                eprintln!("Surface inspection unavailable: viewport has zero area");
            }
            return;
        }
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

    fn inspect_imported_cursor_position(&self, log_details: bool) {
        let (Some(window), Some(renderer), Some(scene), Some([x, y])) = (
            self.window.as_ref(),
            self.renderer.as_ref(),
            self.imported_scene.as_ref(),
            self.cursor_position,
        ) else {
            if log_details {
                eprintln!("Imported-surface inspection unavailable: cursor, window, renderer, or dataset is missing");
            }
            return;
        };

        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            if log_details {
                eprintln!("Imported-surface inspection unavailable: viewport has zero area");
            }
            return;
        }
        let ray = match renderer.camera().ray_from_viewport(x, y, size.width, size.height) {
            Ok(ray) => ray,
            Err(error) => {
                window.set_title(&format!("{} | inspection ray error", self.mode.window_title(self.amplitude)));
                if log_details {
                    eprintln!("Imported-surface inspection ray error: {error}");
                }
                return;
            }
        };

        let result = if let Some(isosurface) = scene.isosurface.as_ref() {
            inspect_isosurface(&scene.dataset, isosurface, &ray)
        } else {
            inspect_boundary_surface(&scene.dataset, &scene.surface, &ray)
        };

        match result {
            Err(error) => {
                window.set_title(&format!("{} | inspection error", self.mode.window_title(self.amplitude)));
                if log_details {
                    eprintln!("Imported-surface inspection error: {error}");
                }
            }
            Ok(None) => {
                window.set_title(&format!("{} | hover: no surface", self.mode.window_title(self.amplitude)));
                if log_details {
                    eprintln!("Imported-surface inspection MISS: ray did not intersect the visible scientific surface");
                }
            }
            Ok(Some(hit)) => {
                let (kind, cell_id) = match hit.entity() {
                    InspectedEntity3::BoundaryFace { source_cell_id, .. } => ("boundary", source_cell_id),
                    InspectedEntity3::IsosurfaceTriangle { source_cell_id } => ("isosurface", source_cell_id),
                };
                let first_value = hit.scalar_values().first()
                    .map(|value| format!(" | {}={:?}", value.name(), value.value()))
                    .unwrap_or_default();
                window.set_title(&format!(
                    "{} | {kind} cell {cell_id} at ({:.3}, {:.3}, {:.3}){}",
                    self.mode.window_title(self.amplitude),
                    hit.world_position().to_array()[0],
                    hit.world_position().to_array()[1],
                    hit.world_position().to_array()[2],
                    first_value,
                ));
                if log_details {
                    eprintln!(
                        "Imported scientific surface HIT: representation={kind}, entity={:?}, \
                         triangle={}, distance={:.9}, world={:?}, barycentric={:?}",
                        hit.entity(),
                        hit.triangle_index(),
                        hit.distance(),
                        hit.world_position().to_array(),
                        hit.barycentric(),
                    );
                    for value in hit.scalar_values() {
                        eprintln!(
                            "  scalar '{}' association={:?} units={:?} value={:?}",
                            value.name(), value.association(), value.units(), value.value()
                        );
                    }
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
                ExplorerMode::ColoredIsosurface => {
                    let (dataset, isosurface, mapped) = demo_colored_tetrahedral_isosurface()?;
                    let renderer = pollster::block_on(SceneRenderer::new_with_mapped_isosurface(
                        window.clone(),
                        &mapped,
                    ))?;
                    self._isosurface_dataset = Some(dataset);
                    self._isosurface = Some(isosurface);
                    self._mapped_isosurface = Some(mapped);
                    Ok(renderer)
                }
                ExplorerMode::Imported => {
                    let scene = self.imported_scene.as_ref()
                        .ok_or("imported VTK scene was not prepared before window creation")?;
                    let mut renderer = if let Some(mapped) = scene.mapped_isosurface.as_ref() {
                        pollster::block_on(SceneRenderer::new_with_mapped_isosurface_and_edges(window.clone(), mapped))?
                    } else if let Some(mapped) = scene.mapped_surface.as_ref() {
                        pollster::block_on(SceneRenderer::new_with_mapped_surface(window.clone(), mapped))?
                    } else {
                        pollster::block_on(SceneRenderer::new(window.clone(), &scene.mesh))?
                    };
                    renderer.set_camera(scene.camera)?;
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
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => f64::from(y) * 40.0,
                    MouseScrollDelta::PixelDelta(position) => position.y,
                };
                if let Err(error) = self.navigate_camera(None, 0.0, scroll) {
                    eprintln!("Camera zoom failed: {error}");
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ArrowUp) | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                            if let Err(error) = self.adjust_amplitude(AMPLITUDE_STEP) {
                                eprintln!("Amplitude update failed: {error}");
                            }
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown) | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                            if let Err(error) = self.adjust_amplitude(-AMPLITUDE_STEP) {
                                eprintln!("Amplitude update failed: {error}");
                            }
                        }
                        _ => {}
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.render() {
                        eprintln!("Render failed: {error}");
                        event_loop.exit();
                    }
                }
            }

            _ => {}
        }
    }
}

