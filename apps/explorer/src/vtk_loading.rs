//! Explorer-side loading and preparation of VTK legacy ASCII datasets.
//!
//! File access and startup configuration live here; parsing, topology,
//! provenance, and scalar mapping remain in svf-core.

use std::{fs, path::PathBuf};

use svf_core::{
    camera::{Camera3, LookAtCamera, PerspectiveProjection, Projection3},
    dataset::{Dataset, DatasetField, DatasetGeometry},
    fields::FieldAssociation,
    geometry::TriangleMesh3,
    scalar_visualization::{
        map_scalar_field, ColorMap, ColorRgba, MappedSurface3, ScalarRangeMode,
        SurfaceProvenance3,
    },
    spatial::{WorldPoint3, WorldVector3},
    surface_triangulation::TriangulatedSurface3,
    unstructured::UnstructuredGrid3,
    vtk_legacy::parse_legacy_ascii_unstructured_grid,
};

const FIT_VERTICAL_FOV: f64 = 45.0_f64.to_radians();
const INVALID_COLOR: [f64; 4] = [0.35, 0.05, 0.22, 1.0];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VtkLoadConfig {
    pub path: PathBuf,
    pub scalar_field: Option<String>,
}

#[derive(Debug)]
pub struct ImportedScene {
    pub dataset: Dataset,
    pub surface: TriangulatedSurface3,
    pub mesh: TriangleMesh3,
    pub camera: Camera3,
    pub _provenance: Option<SurfaceProvenance3>,
    pub mapped_surface: Option<MappedSurface3>,
    pub omitted_lower_dimension_cells: usize,
    pub scalar_field_name: Option<String>,
}

impl ImportedScene {
    pub fn summary(&self) -> String {
        let scalar = self.scalar_field_name.as_deref()
            .map(|name| format!(", scalar field '{name}'"))
            .unwrap_or_default();
        format!(
            "Loaded {} points and {} cells; rendered {} boundary triangles{}; omitted {} lower-dimensional cells.",
            self.dataset.point_count(),
            self.dataset.cell_count(),
            self.surface.triangle_count(),
            scalar,
            self.omitted_lower_dimension_cells,
        )
    }
}

/// Parse the optional VTK CLI workflow. Existing demo-only invocations are
/// left to the legacy mode selector and retain their previous behavior.
pub fn parse_vtk_args(args: &[String]) -> Result<Option<VtkLoadConfig>, String> {
    let has_vtk = args.iter().any(|arg| arg == "--vtk");
    let has_scalar = args.iter().any(|arg| arg == "--vtk-scalar");
    if !has_vtk {
        return if has_scalar {
            Err("--vtk-scalar requires --vtk <path>".to_owned())
        } else {
            Ok(None)
        };
    }

    const DEMO_FLAGS: &[&str] = &[
        "--unstructured",
        "--scalar",
        "--unstructured-scalar",
        "--vectors",
        "--cell-vectors",
        "--isosurface",
        "--colored-isosurface",
    ];

    if let Some(flag) = args.iter().find(|arg| DEMO_FLAGS.contains(&arg.as_str())) {
        return Err(format!("'{flag}' cannot be combined with --vtk"));
    }

    let mut path = None;
    let mut scalar_field = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--vtk" => {
                if path.is_some() {
                    return Err("--vtk may only be supplied once".to_owned());
                }
                let value = args.get(index + 1)
                    .filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--vtk requires a file path".to_owned())?;
                path = Some(PathBuf::from(value));
                index += 2;
            }
            "--vtk-scalar" => {
                if scalar_field.is_some() {
                    return Err("--vtk-scalar may only be supplied once".to_owned());
                }
                let value = args.get(index + 1)
                    .filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--vtk-scalar requires a field name".to_owned())?;
                scalar_field = Some(value.clone());
                index += 2;
            }
            argument if argument.starts_with("--") => {
                return Err(format!("unknown option '{argument}' in VTK mode"));
            }
            argument => {
                return Err(format!("unexpected argument '{argument}' in VTK mode"));
            }
        }
    }

    Ok(Some(VtkLoadConfig {
        path: path.ok_or_else(|| "--vtk requires a file path".to_owned())?,
        scalar_field,
    }))
}

pub fn load_vtk_scene(config: &VtkLoadConfig) -> Result<ImportedScene, String> {
    let input = fs::read_to_string(&config.path).map_err(|error| {
        format!("could not read VTK file '{}': {error}", config.path.display())
    })?;
    let dataset = parse_legacy_ascii_unstructured_grid(&input).map_err(|error| {
        format!("could not import VTK file '{}': {error}", config.path.display())
    })?;
    prepare_imported_scene(dataset, config.scalar_field.as_deref()).map_err(|error| {
        format!("could not prepare VTK file '{}': {error}", config.path.display())
    })
}

pub fn prepare_imported_scene(
    dataset: Dataset,
    scalar_field_name: Option<&str>,
) -> Result<ImportedScene, String> {
    let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else {
        return Err("VTK importer returned a non-unstructured dataset".to_owned());
    };

    let volume_cell_count = grid.cell_types().iter()
        .filter(|cell_type| cell_type.topological_dimension() == 3)
        .count();
    if volume_cell_count == 0 {
        return Err("no renderable volume boundary: the dataset contains no tetrahedral or hexahedral cells".to_owned());
    }

    let omitted_lower_dimension_cells = grid.cell_types().iter()
        .filter(|cell_type| cell_type.topological_dimension() < 3)
        .count();

    let boundary = grid.boundary_surface()
        .map_err(|error| format!("volume boundary extraction failed: {error}"))?;
    let surface = boundary.triangulate();
    if surface.triangle_count() == 0 || surface.point_count() == 0 {
        return Err("no renderable volume boundary: boundary extraction produced an empty surface".to_owned());
    }

    let indices = surface.triangles().iter()
        .flat_map(|triangle| triangle.vertices())
        .collect();
    let mesh = TriangleMesh3::new(surface.points().to_vec(), indices)
        .map_err(|error| format!("could not construct render mesh: {error}"))?;

    let camera = fit_camera(&mesh, 1280.0 / 800.0)?;

    let (provenance, mapped_surface) = if let Some(name) = scalar_field_name {
        let field = match dataset.field(name) {
            None => return Err(format!("scalar field '{name}' was not found in the imported dataset")),
            Some(DatasetField::Vector3(_)) => {
                return Err(format!("field '{name}' is a vector field; --vtk-scalar requires a scalar field"));
            }
            Some(DatasetField::Scalar(field)) => field,
        };
        if field.association() == FieldAssociation::Dataset {
            return Err(format!("scalar field '{name}' has dataset association, which cannot be mapped onto this surface"));
        }

        let provenance = SurfaceProvenance3::from_unstructured(&surface, grid)
            .map_err(|error| format!("could not build scalar surface provenance: {error}"))?;
        let invalid_color = ColorRgba::new(
            INVALID_COLOR[0], INVALID_COLOR[1], INVALID_COLOR[2], INVALID_COLOR[3],
        ).map_err(|error| format!("invalid fallback color: {error}"))?;
        let mapped = map_scalar_field(
            &provenance,
            field,
            dataset.point_count(),
            dataset.cell_count(),
            ScalarRangeMode::Automatic,
            &ColorMap::viridis(),
            invalid_color,
        ).map_err(|error| format!("could not map scalar field '{name}': {error}"))?;
        (Some(provenance), Some(mapped))
    } else {
        (None, None)
    };

    Ok(ImportedScene {
        dataset,
        surface,
        mesh,
        camera,
        _provenance: provenance,
        mapped_surface,
        omitted_lower_dimension_cells,
        scalar_field_name: scalar_field_name.map(str::to_owned),
    })
}

/// Fit a perspective camera to the actual render-mesh bounds using a
/// conservative bounding sphere, including horizontal and vertical FOV.
pub fn fit_camera(mesh: &TriangleMesh3, aspect_ratio: f64) -> Result<Camera3, String> {
    if mesh.vertices().is_empty() {
        return Err("cannot fit camera to an empty render mesh".to_owned());
    }
    if !aspect_ratio.is_finite() || aspect_ratio <= 0.0 {
        return Err("camera aspect ratio must be finite and positive".to_owned());
    }

    let first = mesh.vertices()[0].to_array();
    let mut minimum = first;
    let mut maximum = first;
    for point in &mesh.vertices()[1..] {
        let p = point.to_array();
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(p[axis]);
            maximum[axis] = maximum[axis].max(p[axis]);
        }
    }

    let center = WorldPoint3::new(
        minimum[0] * 0.5 + maximum[0] * 0.5,
        minimum[1] * 0.5 + maximum[1] * 0.5,
        minimum[2] * 0.5 + maximum[2] * 0.5,
    );
    let diagonal = (maximum[0] - minimum[0])
        .hypot(maximum[1] - minimum[1])
        .hypot(maximum[2] - minimum[2]);
    let radius = (diagonal * 0.5).max(1.0e-6);

    let vertical_half_angle = FIT_VERTICAL_FOV * 0.5;
    let horizontal_half_angle = (vertical_half_angle.tan() * aspect_ratio).atan();
    let limiting_half_angle = vertical_half_angle.min(horizontal_half_angle);
    let distance = radius / limiting_half_angle.sin() * 1.15;
    let near = (distance - radius * 1.5).max(distance * 0.01).max(1.0e-6);
    let far = distance + radius * 1.5;

    if !center.to_vector().is_finite()
        || !distance.is_finite()
        || !near.is_finite()
        || !far.is_finite()
        || far <= near
    {
        return Err("render-mesh bounds cannot produce a finite camera configuration".to_owned());
    }

    let direction_length = 1.0_f64.hypot(0.8).hypot(1.0);
    let direction = [
        1.0 / direction_length,
        0.8 / direction_length,
        1.0 / direction_length,
    ];
    let center_coordinates = center.to_array();
    let position = WorldPoint3::new(
        center_coordinates[0] + direction[0] * distance,
        center_coordinates[1] + direction[1] * distance,
        center_coordinates[2] + direction[2] * distance,
    );
    let pose = LookAtCamera::new(
        position,
        center,
        WorldVector3::new(0.0, 1.0, 0.0),
    ).map_err(|error| format!("camera fit failed: {error}"))?;
    let projection = PerspectiveProjection::new(FIT_VERTICAL_FOV, aspect_ratio, near, far)
        .map_err(|error| format!("camera projection fit failed: {error}"))?;
    Ok(Camera3::new(pose, Projection3::Perspective(projection)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use svf_core::{
        fields::{FieldMetadata, ScalarField, VectorBasis3, VectorField3, VectorValue3},
        sampling::{NonFiniteValue, SampleValue},
        unstructured::CellType3,
    };

    const TETRA_VTK: &str = "# vtk DataFile Version 3.0
tetra test
ASCII
DATASET UNSTRUCTURED_GRID
POINTS 4 double
0 0 0  1 0 0  0 1 0  0 0 1
CELLS 1 5
4 0 1 2 3
CELL_TYPES 1
10
POINT_DATA 4
SCALARS temperature double
LOOKUP_TABLE default
10 20 30 40
CELL_DATA 1
SCALARS density double 1
LOOKUP_TABLE default
7
";

    fn config(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parses_vtk_path_and_optional_scalar_name() {
        assert_eq!(
            parse_vtk_args(&config(&["svf-explorer", "--vtk", "mesh with spaces.vtk", "--vtk-scalar", "temperature"])).unwrap(),
            Some(VtkLoadConfig {
                path: PathBuf::from("mesh with spaces.vtk"),
                scalar_field: Some("temperature".to_owned()),
            }),
        );
    }

    #[test]
    fn rejects_vtk_cli_errors() {
        for args in [
            config(&["svf-explorer", "--vtk"]),
            config(&["svf-explorer", "--vtk", "--vtk-scalar", "temperature"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-scalar"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-scalar", "a", "--vtk-scalar", "b"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--unknown"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--scalar"]),
            config(&["svf-explorer", "--vtk-scalar", "temperature"]),
        ] {
            assert!(parse_vtk_args(&args).is_err(), "accepted invalid args: {args:?}");
        }
    }

    #[test]
    fn legacy_demo_arguments_do_not_enter_vtk_mode() {
        assert_eq!(
            parse_vtk_args(&config(&["svf-explorer", "--cell-vectors"])).unwrap(),
            None,
        );
    }

    #[test]
    fn prepares_uncolored_volume_and_counts_omitted_lower_dimensional_cells() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        let grid = match dataset.geometry() {
            DatasetGeometry::UnstructuredGrid3(grid) => grid,
            _ => unreachable!(),
        };
        assert_eq!(grid.cell_types(), &[CellType3::Tetrahedron4]);
        let scene = prepare_imported_scene(dataset.clone(), None).unwrap();
        assert_eq!(scene.dataset.point_count(), 4);
        assert_eq!(scene.dataset.cell_count(), 1);
        assert_eq!(scene.surface.triangle_count(), 4);
        assert_eq!(scene.mesh.triangle_count(), 4);
        assert_eq!(scene.omitted_lower_dimension_cells, 0);
        assert!(scene.mapped_surface.is_none());

        // Add a lower-dimensional cell while keeping the tetrahedron intact.
        let mut points = match dataset.geometry() {
            DatasetGeometry::UnstructuredGrid3(grid) => grid.points().to_vec(),
            _ => unreachable!(),
        };
        points.push(WorldPoint3::new(2.0, 0.0, 0.0));
        let mixed = UnstructuredGrid3::new(
            points,
            vec![CellType3::Tetrahedron4, CellType3::Line2],
            vec![0, 4, 6],
            vec![0, 1, 2, 3, 0, 4],
        ).unwrap();
        let mixed_scene = prepare_imported_scene(Dataset::new(mixed), None).unwrap();
        assert_eq!(mixed_scene.omitted_lower_dimension_cells, 1);
    }

    #[test]
    fn maps_point_and_cell_scalar_fields_onto_the_volume_boundary() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        let point_scene = prepare_imported_scene(dataset.clone(), Some("temperature")).unwrap();
        assert_eq!(point_scene.mapped_surface.as_ref().unwrap().triangle_count(), 4);
        assert_eq!(point_scene.mapped_surface.as_ref().unwrap().colors().len(), 12);
        assert_eq!(point_scene.provenance.as_ref().unwrap().source_cell_ids(), &[0, 0, 0, 0]);

        let cell_scene = prepare_imported_scene(dataset, Some("density")).unwrap();
        assert_eq!(cell_scene.mapped_surface.as_ref().unwrap().triangle_count(), 4);
        assert!(cell_scene.mapped_surface.as_ref().unwrap().colors().iter().all(|color| {
            *color == cell_scene.mapped_surface.as_ref().unwrap().colors()[0]
        }));
    }

    #[test]
    fn reports_missing_and_vector_fields_distinctly() {
        let mut dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { unreachable!() };
        let grid = grid.clone();
        dataset.add_vector_field3(VectorField3::new(
            FieldMetadata::new("velocity", None).unwrap(),
            FieldAssociation::Point,
            VectorBasis3::World,
            vec![VectorValue3::from_f64(1.0, 0.0, 0.0); 4],
        ).unwrap()).unwrap();

        assert!(prepare_imported_scene(dataset.clone(), Some("absent"))
            .unwrap_err().contains("not found"));
        assert!(prepare_imported_scene(dataset, Some("velocity"))
            .unwrap_err().contains("vector field"));

        let no_volume = UnstructuredGrid3::new(
            grid.points().to_vec(),
            vec![CellType3::Triangle3],
            vec![0, 3],
            vec![0, 1, 2],
        ).unwrap();
        assert!(prepare_imported_scene(Dataset::new(no_volume), None)
            .unwrap_err().contains("no renderable volume boundary"));
    }

    #[test]
    fn reports_all_invalid_scalar_range() {
        let mut dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("invalid", None).unwrap(),
            FieldAssociation::Point,
            vec![SampleValue::NonFinite(NonFiniteValue::NaN); 4],
        ).unwrap()).unwrap();
        assert!(prepare_imported_scene(dataset, Some("invalid"))
            .unwrap_err().contains("no finite values"));
    }

    #[test]
    fn camera_fit_centers_mesh_and_handles_degenerate_bounds() {
        let mesh = TriangleMesh3::new(
            vec![
                WorldPoint3::new(100.0, 20.0, -40.0),
                WorldPoint3::new(102.0, 20.0, -40.0),
                WorldPoint3::new(100.0, 24.0, -40.0),
                WorldPoint3::new(100.0, 20.0, -34.0),
            ],
            vec![0, 1, 2, 0, 3, 1],
        ).unwrap();
        let camera = fit_camera(&mesh, 16.0 / 9.0).unwrap();
        assert_eq!(camera.look_at().target(), WorldPoint3::new(101.0, 22.0, -37.0));
        assert!(camera.view_projection_matrix().to_cols_array().iter().all(|value| value.is_finite()));

        let point_mesh = TriangleMesh3::new(
            vec![WorldPoint3::new(1.0, 2.0, 3.0); 3],
            vec![0, 1, 2],
        ).unwrap();
        assert!(fit_camera(&point_mesh, 1.0).is_ok());
    }
}
