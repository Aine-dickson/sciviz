//! Explorer-side loading and preparation of VTK legacy ASCII datasets.
//!
//! File access and startup configuration live here; parsing, topology,
//! provenance, and scalar mapping remain in svf-core.

use std::{fs, path::PathBuf};

use crate::camera_setup::{apply_camera_setup, CameraSetup};

use svf_core::{
    boundary_axial_glyphs::{build_boundary_axial_glyphs, BoundaryAxialGlyphOptions3, BoundaryAxialGlyphSet3},
    camera::{Camera3, LookAtCamera, PerspectiveProjection, Projection3},
    vector_glyphs::{build_vector_glyphs, VectorGlyphSet3, VectorScalePolicy3},
    vector_visualization::{analyze_vector_sources, resolve_vector_sources},
    dataset::{Dataset, DatasetField, DatasetGeometry},
    fields::FieldAssociation,
    geometry::TriangleMesh3,
    isosurface::{extract_tetrahedral_isosurface, Isosurface3},
    isosurface_field_transfer::{map_transferred_scalar_field, transfer_point_scalar_field, MappedIsosurface3},
    scalar_visualization::{
        ColorMap, ColorRgba, MappedSurface3, ScalarRangeMode,
        SurfaceProvenance3,
    },
    spatial::{WorldPoint3, WorldVector3},
    surface_triangulation::TriangulatedSurface3,
    vtk_legacy::parse_legacy_ascii_unstructured_grid,
};

const FIT_VERTICAL_FOV: f64 = 45.0_f64.to_radians();
const INVALID_COLOR: [f64; 4] = [0.35, 0.05, 0.22, 1.0];

#[derive(Clone, Debug, PartialEq)]
pub struct VtkLoadConfig {
    pub path: PathBuf,
    pub scalar_field: Option<String>,
    pub isosurface_field: Option<String>,
    pub isovalue: Option<f64>,
    pub isosurface_color_field: Option<String>,
    pub vector_field: Option<String>,
    pub scalar_excluded_values: Vec<f64>,
    pub axial: Option<AxialGlyphConfig>,
    /// Deterministic view relative to the fitted camera (stage 5b).
    pub camera: CameraSetup,
    /// Render one frame to this PNG and exit (stage 5b).
    pub screenshot: Option<PathBuf>,
}

/// Boundary axial glyph request (design B1.2 section 6): a cell direction
/// field, a cell tag field with the selected tag values, and the thinning
/// parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct AxialGlyphConfig {
    pub direction_field: String,
    pub tag_field: String,
    pub tags: Vec<i64>,
    pub options: BoundaryAxialGlyphOptions3,
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
    pub isosurface: Option<Isosurface3>,
    pub isosurface_field_name: Option<String>,
    pub isosurface_color_field_name: Option<String>,
    pub mapped_isosurface: Option<MappedIsosurface3>,
    pub vector_field_name: Option<String>,
    pub vector_glyphs: Option<VectorGlyphSet3>,
    pub axial_glyphs: Option<BoundaryAxialGlyphSet3>,
}

impl ImportedScene {
    pub fn summary(&self) -> String {
        let representation = if let Some(isosurface) = self.isosurface.as_ref() {
            let color = self.isosurface_color_field_name.as_deref()
                .map(|name| format!(", colored by '{name}' (Viridis) with edge overlay"))
                .unwrap_or_else(|| ", uncolored".to_owned());
            format!(
                "extracted {} isosurface triangles from '{}' at {}{} ({} unsupported cells, {} skipped cells)",
                isosurface.mesh().triangle_count(),
                self.isosurface_field_name.as_deref().unwrap_or("<unknown>"),
                isosurface.isovalue(), color,
                isosurface.metadata().unsupported_cells().len(),
                isosurface.metadata().skipped_cells().len(),
            )
        } else {
            let scalar = self.scalar_field_name.as_deref()
                .map(|name| format!(", scalar field '{name}'"))
                .unwrap_or_default();
            format!("rendered {} boundary triangles{}", self.surface.triangle_count(), scalar)
        };
        let vectors = self.vector_glyphs.as_ref().map(|glyphs| {
            let displayable = glyphs.entries().iter().filter(|entry| entry.segments().is_some()).count();
            let length = match glyphs.policy() {
                VectorScalePolicy3::UniformLength { length } => length,
                VectorScalePolicy3::MagnitudeScaled { .. } => 0.0,
            };
            format!("; vector field '{}': {} source vectors, {} displayable arrows, uniform display length {:.6}",
                self.vector_field_name.as_deref().unwrap_or("<unknown>"), glyphs.entries().len(), displayable, length)
        }).unwrap_or_default();
        let axial = self.axial_glyphs.as_ref().map(axial_summary).unwrap_or_default();
        format!(
            "Loaded {} points and {} cells; {}; omitted {} lower-dimensional cells{}{}.",
            self.dataset.point_count(),
            self.dataset.cell_count(),
            representation,
            self.omitted_lower_dimension_cells,
            vectors,
            axial,
        )
    }
}

/// Per-tag counts, all taken from the core result.
fn axial_summary(set: &BoundaryAxialGlyphSet3) -> String {
    let options = set.options();
    let eligible = set.eligible_by_tag();
    let glyphs = set.glyphs_by_tag();
    let selected = set.selected_by_tag();
    let per_tag = set.tags().iter().map(|tag| {
        format!(
            "tag {tag}: {} selected, {} eligible, {} glyphs",
            selected.get(tag).copied().unwrap_or(0),
            eligible.get(tag).copied().unwrap_or(0),
            glyphs.get(tag).copied().unwrap_or(0),
        )
    }).collect::<Vec<_>>().join("; ");
    format!(
        "; axial glyphs (bin {}, length {}): {}; {} glyphs total; excluded {} zero-area, {} invalid-direction",
        options.bin_edge, options.length, per_tag, set.glyphs().len(),
        set.zero_area_count(), set.invalid_direction_count(),
    )
}

/// Parse `<field>=<v1,v2,...>` where every value is an integer tag.
fn parse_cell_tags(value: &str) -> Result<(String, Vec<i64>), String> {
    let (field, list) = value.split_once('=')
        .ok_or_else(|| format!("invalid --vtk-cell-tags '{value}': expected <field>=<v1,v2,...>"))?;
    if field.is_empty() {
        return Err("--vtk-cell-tags requires a field name before '='".to_owned());
    }
    let mut tags = Vec::new();
    for item in list.split(',') {
        let tag = item.trim().parse::<i64>()
            .map_err(|_| format!("invalid --vtk-cell-tags value '{item}': expected an integer tag"))?;
        tags.push(tag);
    }
    Ok((field.to_owned(), tags))
}

pub(crate) fn parse_finite_f64(flag: &str, value: Option<&String>) -> Result<f64, String> {
    let value = value.ok_or_else(|| format!("{flag} requires a finite f64 value"))?;
    let parsed = value.parse::<f64>()
        .map_err(|_| format!("invalid {flag} '{value}': expected a finite f64"))?;
    if !parsed.is_finite() {
        return Err(format!("{flag} must be finite"));
    }
    Ok(parsed)
}

pub(crate) fn parse_positive_f64(flag: &str, value: Option<&String>) -> Result<f64, String> {
    let value = value.ok_or_else(|| format!("{flag} requires a finite positive f64 value"))?;
    let parsed = value.parse::<f64>()
        .map_err(|_| format!("invalid {flag} '{value}': expected a finite positive f64"))?;
    if !(parsed.is_finite() && parsed > 0.0) {
        return Err(format!("{flag} must be finite and greater than zero"));
    }
    Ok(parsed)
}

/// Parse the optional VTK CLI workflow. Existing demo-only invocations are
/// left to the legacy mode selector and retain their previous behavior.
pub fn parse_vtk_args(args: &[String]) -> Result<Option<VtkLoadConfig>, String> {
    let has_vtk = args.iter().any(|arg| arg == "--vtk");
    let has_scalar = args.iter().any(|arg| arg == "--vtk-scalar");
    let has_isosurface = args.iter().any(|arg| arg == "--vtk-isosurface");
    let has_isovalue = args.iter().any(|arg| arg == "--vtk-isovalue");
    let has_isosurface_color = args.iter().any(|arg| arg == "--vtk-isosurface-color");
    let has_vectors = args.iter().any(|arg| arg == "--vtk-vectors");
    let has_scalar_exclusions = args.iter().any(|arg| arg == "--vtk-scalar-exclude");
    let has_axial_option = ["--vtk-cell-axial", "--vtk-cell-tags", "--vtk-axial-bin", "--vtk-axial-length",
        "--camera-yaw", "--camera-pitch", "--camera-zoom", "--screenshot"]
        .iter().any(|flag| args.iter().any(|arg| arg == flag));
    if !has_vtk {
        return if has_scalar || has_isosurface || has_isovalue || has_isosurface_color || has_vectors || has_scalar_exclusions || has_axial_option {
            Err("VTK field and isosurface options require --vtk <path>".to_owned())
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
    let mut isosurface_field = None;
    let mut isovalue = None;
    let mut isosurface_color_field = None;
    let mut vector_field = None;
    let mut scalar_excluded_values = Vec::new();
    let mut axial_field: Option<String> = None;
    let mut axial_tags: Option<(String, Vec<i64>)> = None;
    let mut axial_bin = None;
    let mut axial_length = None;
    let mut camera = CameraSetup::default();
    let mut screenshot: Option<PathBuf> = None;
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
            "--vtk-isosurface" => {
                if isosurface_field.is_some() {
                    return Err("--vtk-isosurface may only be supplied once".to_owned());
                }
                let value = args.get(index + 1)
                    .filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--vtk-isosurface requires a field name".to_owned())?;
                isosurface_field = Some(value.clone());
                index += 2;
            }
            "--vtk-isosurface-color" => {
                if isosurface_color_field.is_some() { return Err("--vtk-isosurface-color may only be supplied once".to_owned()); }
                let value = args.get(index + 1).filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--vtk-isosurface-color requires a point scalar field name".to_owned())?;
                isosurface_color_field = Some(value.clone());
                index += 2;
            }
            "--vtk-vectors" => {
                if vector_field.is_some() { return Err("--vtk-vectors may only be supplied once".to_owned()); }
                let value = args.get(index + 1).filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--vtk-vectors requires a field name".to_owned())?;
                vector_field = Some(value.clone());
                index += 2;
            }
            "--vtk-scalar-exclude" => {
                let value = args.get(index + 1).filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--vtk-scalar-exclude requires a finite f64 value".to_owned())?;
                let parsed = value.parse::<f64>()
                    .map_err(|_| format!("invalid --vtk-scalar-exclude value {value}: expected a finite f64"))?;
                if !parsed.is_finite() { return Err("--vtk-scalar-exclude values must be finite".to_owned()); }
                scalar_excluded_values.push(parsed);
                index += 2;
            }
            "--vtk-isovalue" => {
                if isovalue.is_some() {
                    return Err("--vtk-isovalue may only be supplied once".to_owned());
                }
                let value = args.get(index + 1)
                    .filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--vtk-isovalue requires a finite f64 value".to_owned())?;
                let parsed = value.parse::<f64>()
                    .map_err(|_| format!("invalid --vtk-isovalue '{value}': expected a finite f64"))?;
                if !parsed.is_finite() {
                    return Err("--vtk-isovalue must be finite".to_owned());
                }
                isovalue = Some(parsed);
                index += 2;
            }
            "--vtk-cell-axial" => {
                if axial_field.is_some() { return Err("--vtk-cell-axial may only be supplied once".to_owned()); }
                let value = args.get(index + 1).filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--vtk-cell-axial requires a cell vector field name".to_owned())?;
                axial_field = Some(value.clone());
                index += 2;
            }
            "--vtk-cell-tags" => {
                if axial_tags.is_some() { return Err("--vtk-cell-tags may only be supplied once".to_owned()); }
                let value = args.get(index + 1).filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--vtk-cell-tags requires <field>=<v1,v2,...>".to_owned())?;
                axial_tags = Some(parse_cell_tags(value)?);
                index += 2;
            }
            "--vtk-axial-bin" => {
                if axial_bin.is_some() { return Err("--vtk-axial-bin may only be supplied once".to_owned()); }
                axial_bin = Some(parse_positive_f64("--vtk-axial-bin", args.get(index + 1))?);
                index += 2;
            }
            "--vtk-axial-length" => {
                if axial_length.is_some() { return Err("--vtk-axial-length may only be supplied once".to_owned()); }
                axial_length = Some(parse_positive_f64("--vtk-axial-length", args.get(index + 1))?);
                index += 2;
            }
            "--camera-yaw" => {
                if camera.yaw_degrees.is_some() { return Err("--camera-yaw may only be supplied once".to_owned()); }
                camera.yaw_degrees = Some(parse_finite_f64("--camera-yaw", args.get(index + 1))?);
                index += 2;
            }
            "--camera-pitch" => {
                if camera.pitch_degrees.is_some() { return Err("--camera-pitch may only be supplied once".to_owned()); }
                camera.pitch_degrees = Some(parse_finite_f64("--camera-pitch", args.get(index + 1))?);
                index += 2;
            }
            "--camera-zoom" => {
                if camera.zoom_factor.is_some() { return Err("--camera-zoom may only be supplied once".to_owned()); }
                camera.zoom_factor = Some(parse_positive_f64("--camera-zoom", args.get(index + 1))?);
                index += 2;
            }
            "--screenshot" => {
                if screenshot.is_some() { return Err("--screenshot may only be supplied once".to_owned()); }
                let value = args.get(index + 1).filter(|value| !value.starts_with("--") && !value.is_empty())
                    .ok_or_else(|| "--screenshot requires an output PNG path".to_owned())?;
                screenshot = Some(PathBuf::from(value));
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

    if !scalar_excluded_values.is_empty() && scalar_field.is_none() {
        return Err("--vtk-scalar-exclude requires --vtk-scalar".to_owned());
    }
    if vector_field.is_some() && (isosurface_field.is_some() || isovalue.is_some() || isosurface_color_field.is_some()) {
        return Err("--vtk-vectors cannot be combined with --vtk-isosurface/--vtk-isovalue/--vtk-isosurface-color".to_owned());
    }
    if scalar_field.is_some() && (isosurface_field.is_some() || isovalue.is_some()) {
        return Err("--vtk-scalar cannot be combined with --vtk-isosurface/--vtk-isovalue".to_owned());
    }
    if isosurface_field.is_some() != isovalue.is_some() {
        return Err("--vtk-isosurface and --vtk-isovalue must be supplied together".to_owned());
    }

    if isosurface_color_field.is_some() && isosurface_field.is_none() {
        return Err("--vtk-isosurface-color requires --vtk-isosurface and --vtk-isovalue".to_owned());
    }

    let axial = match (axial_field, axial_tags) {
        (Some(direction_field), Some((tag_field, tags))) => {
            if vector_field.is_some() {
                return Err("--vtk-cell-axial cannot be combined with --vtk-vectors".to_owned());
            }
            if isosurface_field.is_some() || isovalue.is_some() || isosurface_color_field.is_some() {
                return Err("--vtk-cell-axial cannot be combined with --vtk-isosurface/--vtk-isovalue/--vtk-isosurface-color".to_owned());
            }
            let defaults = BoundaryAxialGlyphOptions3::default();
            Some(AxialGlyphConfig {
                direction_field,
                tag_field,
                tags,
                options: BoundaryAxialGlyphOptions3 {
                    bin_edge: axial_bin.unwrap_or(defaults.bin_edge),
                    length: axial_length.unwrap_or(defaults.length),
                },
            })
        }
        (None, None) if axial_bin.is_none() && axial_length.is_none() => None,
        (None, None) => {
            return Err("--vtk-axial-bin and --vtk-axial-length require --vtk-cell-axial and --vtk-cell-tags".to_owned());
        }
        _ => return Err("--vtk-cell-axial and --vtk-cell-tags must be supplied together".to_owned()),
    };

    Ok(Some(VtkLoadConfig {
        path: path.ok_or_else(|| "--vtk requires a file path".to_owned())?,
        scalar_field,
        isosurface_field,
        isovalue,
        isosurface_color_field,
        vector_field,
        scalar_excluded_values,
        axial,
        camera,
        screenshot,
    }))
}

pub fn load_vtk_scene(config: &VtkLoadConfig) -> Result<ImportedScene, String> {
    let input = fs::read_to_string(&config.path).map_err(|error| {
        format!("could not read VTK file '{}': {error}", config.path.display())
    })?;
    let dataset = parse_legacy_ascii_unstructured_grid(&input).map_err(|error| {
        format!("could not import VTK file '{}': {error}", config.path.display())
    })?;
    let mut scene = prepare_imported_scene_with_vectors_and_scalar_exclusions(
        dataset,
        config.scalar_field.as_deref(),
        config.isosurface_field.as_deref(),
        config.isovalue,
        config.isosurface_color_field.as_deref(),
        config.vector_field.as_deref(),
        &config.scalar_excluded_values,
    ).map_err(|error| {
        format!("could not prepare VTK file '{}': {error}", config.path.display())
    })?;
    if let Some(axial) = config.axial.as_ref() {
        attach_axial_glyphs(&mut scene, axial).map_err(|error| {
            format!("could not build axial glyphs for VTK file '{}': {error}", config.path.display())
        })?;
    }
    scene.camera = apply_camera_setup(&scene.camera, &config.camera)?;
    Ok(scene)
}

/// Build the thinned boundary axial glyphs on the scene's boundary surface.
/// Tag selection applies to glyphs only; the rendered surface is unchanged.
pub fn attach_axial_glyphs(scene: &mut ImportedScene, axial: &AxialGlyphConfig) -> Result<(), String> {
    if scene.isosurface.is_some() || scene.vector_glyphs.is_some() {
        return Err("axial glyphs cannot be combined with isosurfaces or point vector glyphs".to_owned());
    }
    let glyphs = build_boundary_axial_glyphs(
        &scene.dataset,
        &scene.surface,
        &axial.direction_field,
        &axial.tag_field,
        &axial.tags,
        axial.options,
    ).map_err(|error| error.to_string())?;
    scene.axial_glyphs = Some(glyphs);
    Ok(())
}

pub fn prepare_imported_scene_with_options(
    dataset: Dataset,
    scalar_field_name: Option<&str>,
    isosurface_field_name: Option<&str>,
    isovalue: Option<f64>,
    isosurface_color_field_name: Option<&str>,
) -> Result<ImportedScene, String> {
    prepare_imported_scene_with_vectors(dataset, scalar_field_name, isosurface_field_name, isovalue, isosurface_color_field_name, None)
}

pub fn prepare_imported_scene_with_vectors(
    dataset: Dataset,
    scalar_field_name: Option<&str>,
    isosurface_field_name: Option<&str>,
    isovalue: Option<f64>,
    isosurface_color_field_name: Option<&str>,
    vector_field_name: Option<&str>,
) -> Result<ImportedScene, String> {
    prepare_imported_scene_with_vectors_and_scalar_exclusions(dataset,scalar_field_name,isosurface_field_name,isovalue,isosurface_color_field_name,vector_field_name,&[])
}

pub fn prepare_imported_scene_with_vectors_and_scalar_exclusions(
    dataset: Dataset,
    scalar_field_name: Option<&str>,
    isosurface_field_name: Option<&str>,
    isovalue: Option<f64>,
    isosurface_color_field_name: Option<&str>,
    vector_field_name: Option<&str>,
    scalar_excluded_values: &[f64],
) -> Result<ImportedScene, String> {
    if isosurface_field_name.is_some() != isovalue.is_some() {
        return Err("isosurface field and isovalue must be supplied together".to_owned());
    }
    if isosurface_color_field_name.is_some() && isosurface_field_name.is_none() {
        return Err("isosurface color field requires isosurface extraction".to_owned());
    }
    if scalar_field_name.is_some() && isosurface_field_name.is_some() {
        return Err("scalar mapping and isosurface extraction are mutually exclusive".to_owned());
    }
    if vector_field_name.is_some() && isosurface_field_name.is_some() {
        return Err("vector glyphs cannot be combined with isosurface extraction".to_owned());
    }
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

    let isosurface = if let Some(name) = isosurface_field_name {
        let field = match dataset.field(name) {
            None => return Err(format!("field '{name}' was not found in the imported dataset")),
            Some(DatasetField::Vector3(_)) => {
                return Err(format!("field '{name}' is a vector field; isosurface extraction requires a point-associated scalar field"));
            }
            Some(DatasetField::Scalar(field)) => field,
        };
        if field.association() != FieldAssociation::Point {
            return Err(format!(
                "scalar field '{name}' has {:?} association; isosurface extraction requires Point association",
                field.association()
            ));
        }
        let value = isovalue.ok_or_else(|| "isosurface isovalue is required".to_owned())?;
        let extracted = extract_tetrahedral_isosurface(&dataset, name, value)
            .map_err(|error| format!("could not extract isosurface for field '{name}': {error}"))?;
        if extracted.mesh().triangle_count() == 0 {
            return Err(format!(
                "isosurface extraction for field '{name}' at {value} produced no triangles ({} unsupported cells, {} skipped cells)",
                extracted.metadata().unsupported_cells().len(),
                extracted.metadata().skipped_cells().len(),
            ));
        }
        Some(extracted)
    } else {
        None
    };

    let render_mesh = isosurface.as_ref().map_or(&mesh, Isosurface3::mesh);
    validate_render_mesh(render_mesh)?;
    let camera = fit_camera(render_mesh, 1280.0 / 800.0)?;

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
        let mapped = svf_core::scalar_visualization::map_scalar_field_with_exclusions(
            &provenance,
            field,
            dataset.point_count(),
            dataset.cell_count(),
            ScalarRangeMode::Automatic,
            &ColorMap::viridis(),
            invalid_color,
            scalar_excluded_values,
        ).map_err(|error| format!("could not map scalar field '{name}': {error}"))?;
        (Some(provenance), Some(mapped))
    } else {
        (None, None)
    };

    let vector_glyphs = if let Some(name) = vector_field_name {
        let field = match dataset.field(name) {
            None => return Err(format!("vector field '{name}' was not found")),
            Some(DatasetField::Scalar(_)) => return Err(format!("field '{name}' is a scalar field; --vtk-vectors requires a three-component vector field")),
            Some(DatasetField::Vector3(field)) => field,
        };
        if field.association() != FieldAssociation::Point {
            return Err(format!("vector field '{name}' has {:?} association; --vtk-vectors requires Point association", field.association()));
        }
        if field.basis() != svf_core::fields::VectorBasis3::World {
            return Err(format!("vector field '{name}' uses {:?} basis; --vtk-vectors requires World basis", field.basis()));
        }
        let sources = resolve_vector_sources(&dataset, name).map_err(|error| format!("could not resolve vector field '{name}': {error}"))?;
        let analyses = analyze_vector_sources(&sources).map_err(|error| format!("could not analyze vector field '{name}': {error}"))?;
        let diagonal = mesh_bounds_diagonal(&mesh)?;
        let length = (diagonal * 0.05).max(1.0e-6);
        let glyphs = build_vector_glyphs(&analyses, VectorScalePolicy3::UniformLength { length })
            .map_err(|error| format!("could not build glyphs for vector field '{name}': {error}"))?;
        validate_vector_glyphs(&glyphs)?;
        Some(glyphs)
    } else { None };

    let mapped_isosurface = if let Some(color_name) = isosurface_color_field_name {
        let extracted = isosurface.as_ref().ok_or_else(|| "--vtk-isosurface-color requires an extracted isosurface".to_owned())?;
        let field = match dataset.field(color_name) {
            None => return Err(format!("isosurface color field '{color_name}' was not found")),
            Some(DatasetField::Vector3(_)) => return Err(format!("isosurface color field '{color_name}' is a vector field; a point-associated scalar field is required")),
            Some(DatasetField::Scalar(field)) => field,
        };
        if field.association() != FieldAssociation::Point {
            return Err(format!("isosurface color field '{color_name}' has {:?} association; coloring requires Point association", field.association()));
        }
        let transferred = transfer_point_scalar_field(&dataset, extracted, color_name)
            .map_err(|error| format!("could not transfer isosurface color field '{color_name}': {error}"))?;
        let invalid_color = ColorRgba::new(INVALID_COLOR[0], INVALID_COLOR[1], INVALID_COLOR[2], INVALID_COLOR[3])
            .map_err(|error| format!("invalid fallback color: {error}"))?;
        Some(map_transferred_scalar_field(transferred, ScalarRangeMode::Automatic, &ColorMap::viridis(), invalid_color)
            .map_err(|error| format!("could not map isosurface color field '{color_name}': {error}"))?)
    } else { None };

    Ok(ImportedScene {
        dataset,
        surface,
        mesh: render_mesh.clone(),
        camera,
        _provenance: provenance,
        mapped_surface,
        omitted_lower_dimension_cells,
        scalar_field_name: scalar_field_name.map(str::to_owned),
        isosurface,
        isosurface_field_name: isosurface_field_name.map(str::to_owned),
        isosurface_color_field_name: isosurface_color_field_name.map(str::to_owned),
        mapped_isosurface,
        vector_field_name: vector_field_name.map(str::to_owned),
        vector_glyphs,
        axial_glyphs: None,
    })
}

fn mesh_bounds_diagonal(mesh: &TriangleMesh3) -> Result<f64, String> {
    let first = mesh.vertices().first().ok_or_else(|| "cannot determine vector scale from an empty mesh".to_owned())?.to_array();
    let mut minimum = first;
    let mut maximum = first;
    for point in &mesh.vertices()[1..] {
        let p = point.to_array();
        for axis in 0..3 { minimum[axis] = minimum[axis].min(p[axis]); maximum[axis] = maximum[axis].max(p[axis]); }
    }
    let diagonal = (maximum[0] - minimum[0]).hypot(maximum[1] - minimum[1]).hypot(maximum[2] - minimum[2]);
    if !diagonal.is_finite() { return Err("imported boundary bounds cannot produce a finite vector display length".to_owned()); }
    Ok(diagonal)
}

fn validate_vector_glyphs(glyphs: &VectorGlyphSet3) -> Result<(), String> {
    for entry in glyphs.entries() {
        if let Some(segments) = entry.segments() {
            for segment in segments {
                for point in [segment.start(), segment.end()] {
                    if point.to_array().iter().any(|value| !(*value as f32).is_finite()) {
                        return Err("vector glyph coordinates exceed the renderer's finite f32 range".to_owned());
                    }
                }
            }
        }
    }
    Ok(())
}

/// Reject render inputs that the existing f32 GPU buffers cannot represent,
/// before Explorer creates a native window.
fn validate_render_mesh(mesh: &TriangleMesh3) -> Result<(), String> {
    u32::try_from(mesh.vertices().len())
        .map_err(|_| "render mesh exceeds the renderer's u32 vertex limit".to_owned())?;
    u32::try_from(mesh.indices().len())
        .map_err(|_| "render mesh exceeds the renderer's u32 index limit".to_owned())?;
    if mesh.vertices().iter().any(|point| {
        point.to_array().iter().any(|coordinate| !(*coordinate as f32).is_finite())
    }) {
        return Err("render mesh coordinates exceed the renderer's finite f32 range".to_owned());
    }
    Ok(())
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
    let camera = Camera3::new(pose, Projection3::Perspective(projection));
    if camera.view_projection_matrix().to_cols_array().iter()
        .any(|value| !(*value as f32).is_finite())
    {
        return Err("fitted camera matrix exceeds the renderer's finite f32 uniform range".to_owned());
    }
    Ok(camera)
}

#[cfg(test)]
mod tests {
    use super::*;
    use svf_core::{
        fields::{FieldMetadata, ScalarField, VectorBasis3, VectorField3, VectorValue3},
        sampling::{NonFiniteValue, SampleValue},
        unstructured::{CellType3, UnstructuredGrid3},
    };

    const EXAMPLE_VTK: &str = include_str!("../examples/tetra-with-fields.vtk");

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
    fn transfers_independent_point_scalar_to_isosurface_for_coloring() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(EXAMPLE_VTK).unwrap();
        let scene = prepare_imported_scene_with_options(dataset, None, Some("temperature"), Some(25.0), Some("pressure")).unwrap();
        let mapped = scene.mapped_isosurface.as_ref().unwrap();
        assert_eq!(mapped.transferred().metadata().name(), "pressure");
        assert_eq!(mapped.colors().len(), mapped.isosurface().mesh().vertex_count());
        assert_eq!(mapped.isosurface().mesh().triangle_count(), 2);
        assert!(scene.summary().contains("colored by 'pressure'"));
    }

    #[test]
    fn rejects_invalid_independent_isosurface_color_fields() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        let missing = prepare_imported_scene_with_options(dataset.clone(), None, Some("temperature"), Some(25.0), Some("missing"));
        assert!(missing.unwrap_err().contains("was not found"));
        let cell = prepare_imported_scene_with_options(dataset, None, Some("temperature"), Some(25.0), Some("density"));
        assert!(cell.unwrap_err().contains("Point association"));
    }

    #[test]
    fn committed_smoke_fixture_loads_and_reports_omitted_line_cell() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(EXAMPLE_VTK).unwrap();
        let scene = prepare_imported_scene_with_options(dataset, None, None, None, None).unwrap();
        assert_eq!(scene.dataset.point_count(), 5);
        assert_eq!(scene.dataset.cell_count(), 2);
        assert_eq!(scene.surface.triangle_count(), 4);
        assert_eq!(scene.omitted_lower_dimension_cells, 1);
        assert!(scene.mapped_surface.is_none());
        assert!(scene.summary().contains("omitted 1 lower-dimensional cells"));
    }

    #[test]
    fn parses_vtk_path_and_optional_scalar_name() {
        assert_eq!(
            parse_vtk_args(&config(&["svf-explorer", "--vtk", "mesh with spaces.vtk", "--vtk-scalar", "temperature"])).unwrap(),
            Some(VtkLoadConfig {
                path: PathBuf::from("mesh with spaces.vtk"),
                scalar_field: Some("temperature".to_owned()),
                isosurface_field: None,
                isovalue: None,
                isosurface_color_field: None,
                vector_field: None,
                scalar_excluded_values: Vec::new(),
                axial: None,
                camera: CameraSetup::default(),
                screenshot: None,
            }),
        );
    }


    #[test]
    fn parses_repeated_scalar_exclusions_and_rejects_invalid_usage() {
        let parsed = parse_vtk_args(&config(&["svf-explorer","--vtk","heart.vtk","--vtk-scalar","Z.dat","--vtk-scalar-exclude","-10","--vtk-scalar-exclude","-9.5"])).unwrap().unwrap();
        assert_eq!(parsed.scalar_excluded_values,vec![-10.0,-9.5]);
        for args in [
            config(&["svf-explorer","--vtk","heart.vtk","--vtk-scalar-exclude","-10"]),
            config(&["svf-explorer","--vtk","heart.vtk","--vtk-scalar","Z.dat","--vtk-scalar-exclude"]),
            config(&["svf-explorer","--vtk","heart.vtk","--vtk-scalar","Z.dat","--vtk-scalar-exclude","NaN"]),
            config(&["svf-explorer","--vtk","heart.vtk","--vtk-scalar","Z.dat","--vtk-scalar-exclude","inf"]),
        ] { assert!(parse_vtk_args(&args).is_err(),"accepted invalid args: {args:?}"); }
    }

    #[test]
    fn parses_point_vectors_with_or_without_scalar_coloring() {
        let vector_only = parse_vtk_args(&config(&[
            "svf-explorer", "--vtk", "mesh.vtk", "--vtk-vectors", "velocity",
        ])).unwrap().unwrap();
        assert_eq!(vector_only.vector_field.as_deref(), Some("velocity"));
        assert_eq!(vector_only.scalar_field, None);

        let combined = parse_vtk_args(&config(&[
            "svf-explorer", "--vtk", "mesh.vtk",
            "--vtk-scalar", "pressure", "--vtk-vectors", "velocity",
        ])).unwrap().unwrap();
        assert_eq!(combined.scalar_field.as_deref(), Some("pressure"));
        assert_eq!(combined.vector_field.as_deref(), Some("velocity"));
    }

    #[test]
    fn rejects_invalid_vector_cli_combinations() {
        for args in [
            config(&["svf-explorer", "--vtk-vectors", "velocity"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-vectors"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-vectors", "a", "--vtk-vectors", "b"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-vectors", "velocity", "--vtk-isosurface", "temperature", "--vtk-isovalue", "25"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-vectors", "velocity", "--vtk-isosurface-color", "pressure"]),
        ] {
            assert!(parse_vtk_args(&args).is_err(), "accepted invalid args: {args:?}");
        }
    }

    #[test]
    fn prepares_point_vector_glyphs_without_changing_source_values() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(EXAMPLE_VTK).unwrap();
        let scene = prepare_imported_scene_with_vectors(
            dataset, None, None, None, None, Some("velocity"),
        ).unwrap();
        let glyphs = scene.vector_glyphs.as_ref().unwrap();
        assert_eq!(glyphs.entries().len(), 5);
        assert_eq!(glyphs.entries().iter().filter(|entry| entry.segments().is_some()).count(), 4);
        let expected_length = 3.0_f64.sqrt() * 0.05;
        match glyphs.policy() {
            VectorScalePolicy3::UniformLength { length } => assert!((length - expected_length).abs() < 1.0e-12),
            other => panic!("unexpected vector scale policy: {other:?}"),
        }
        let expected_points = [
            WorldPoint3::new(0.0, 0.0, 0.0),
            WorldPoint3::new(1.0, 0.0, 0.0),
            WorldPoint3::new(0.0, 1.0, 0.0),
            WorldPoint3::new(0.0, 0.0, 1.0),
            WorldPoint3::new(2.0, 0.0, 0.0),
        ];
        for (entry, expected) in glyphs.entries().iter().zip(expected_points) {
            assert_eq!(entry.analysis().source().anchor(), expected);
        }
        assert!(glyphs.entries()[4].segments().is_none());
        assert!(scene.summary().contains("5 source vectors, 4 displayable arrows"));
        assert!(scene.summary().contains("uniform display length"));
        assert!(matches!(
            scene.dataset.field("velocity"),
            Some(DatasetField::Vector3(field)) if field.values()[0] == VectorValue3::from_f64(1.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn combines_boundary_scalar_mapping_and_point_vector_glyphs() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(EXAMPLE_VTK).unwrap();
        let scene = prepare_imported_scene_with_vectors(
            dataset, Some("pressure"), None, None, None, Some("velocity"),
        ).unwrap();
        assert!(scene.mapped_surface.is_some());
        assert!(scene.vector_glyphs.is_some());
        assert_eq!(scene.scalar_field_name.as_deref(), Some("pressure"));
        assert_eq!(scene.vector_field_name.as_deref(), Some("velocity"));
    }

    #[test]
    fn rejects_missing_scalar_cell_and_dataset_vector_fields() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(EXAMPLE_VTK).unwrap();
        assert!(prepare_imported_scene_with_vectors(dataset.clone(), None, None, None, None, Some("missing"))
            .unwrap_err().contains("not found"));
        assert!(prepare_imported_scene_with_vectors(dataset.clone(), None, None, None, None, Some("pressure"))
            .unwrap_err().contains("scalar field"));

        let mut cell_dataset = dataset.clone();
        cell_dataset.add_vector_field3(VectorField3::new(
            FieldMetadata::new("cell_velocity", None).unwrap(),
            FieldAssociation::Cell, VectorBasis3::World,
            vec![VectorValue3::from_f64(1.0, 0.0, 0.0); 2],
        ).unwrap()).unwrap();
        assert!(prepare_imported_scene_with_vectors(cell_dataset, None, None, None, None, Some("cell_velocity"))
            .unwrap_err().contains("Point association"));

        let mut dataset_associated = dataset;
        dataset_associated.add_vector_field3(VectorField3::new(
            FieldMetadata::new("global_velocity", None).unwrap(),
            FieldAssociation::Dataset, VectorBasis3::World,
            vec![VectorValue3::from_f64(1.0, 0.0, 0.0)],
        ).unwrap()).unwrap();
        assert!(prepare_imported_scene_with_vectors(dataset_associated, None, None, None, None, Some("global_velocity"))
            .unwrap_err().contains("Dataset association"));
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
        let scene = prepare_imported_scene_with_options(dataset.clone(), None, None, None, None).unwrap();
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
        let mixed_scene = prepare_imported_scene_with_options(Dataset::new(mixed), None, None, None, None).unwrap();
        assert_eq!(mixed_scene.omitted_lower_dimension_cells, 1);
    }

    #[test]
    fn maps_point_and_cell_scalar_fields_onto_the_volume_boundary() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        let point_scene = prepare_imported_scene_with_options(dataset.clone(), Some("temperature"), None, None, None).unwrap();
        assert_eq!(point_scene.mapped_surface.as_ref().unwrap().triangle_count(), 4);
        assert_eq!(point_scene.mapped_surface.as_ref().unwrap().colors().len(), 12);
        assert_eq!(point_scene._provenance.as_ref().unwrap().source_cell_ids(), &[0, 0, 0, 0]);

        let cell_scene = prepare_imported_scene_with_options(dataset, Some("density"), None, None, None).unwrap();
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

        assert!(prepare_imported_scene_with_options(dataset.clone(), Some("absent"), None, None, None)
            .unwrap_err().contains("not found"));
        assert!(prepare_imported_scene_with_options(dataset, Some("velocity"), None, None, None)
            .unwrap_err().contains("vector field"));

        let no_volume = UnstructuredGrid3::new(
            grid.points().to_vec(),
            vec![CellType3::Triangle3],
            vec![0, 3],
            vec![0, 1, 2],
        ).unwrap();
        assert!(prepare_imported_scene_with_options(Dataset::new(no_volume), None, None, None, None)
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
        assert!(prepare_imported_scene_with_options(dataset, Some("invalid"), None, None, None)
            .unwrap_err().contains("no finite values"));
    }

    #[test]
    fn parses_vtk_isosurface_field_and_finite_isovalue() {
        let parsed = parse_vtk_args(&config(&[
            "svf-explorer", "--vtk", "mesh.vtk",
            "--vtk-isosurface", "temperature", "--vtk-isovalue", "25.5",
        ])).unwrap().unwrap();
        assert_eq!(parsed.path, PathBuf::from("mesh.vtk"));
        assert_eq!(parsed.isosurface_field.as_deref(), Some("temperature"));
        assert_eq!(parsed.isovalue, Some(25.5));
        assert!(parsed.scalar_field.is_none());
        let colored = parse_vtk_args(&config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-isosurface", "temperature", "--vtk-isovalue", "25", "--vtk-isosurface-color", "pressure"])).unwrap().unwrap();
        assert_eq!(colored.isosurface_color_field.as_deref(), Some("pressure"));
    }

    #[test]
    fn rejects_incomplete_conflicting_and_non_finite_isosurface_cli_options() {
        for args in [
            config(&["svf-explorer", "--vtk-isosurface", "temperature"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-isosurface", "temperature"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-isovalue", "25"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-isosurface", "temperature", "--vtk-isovalue", "NaN"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-isosurface", "temperature", "--vtk-isovalue", "inf"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-isosurface", "temperature", "--vtk-isovalue", "-inf"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-scalar", "temperature", "--vtk-isosurface", "temperature", "--vtk-isovalue", "25"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-isosurface", "a", "--vtk-isosurface", "b", "--vtk-isovalue", "25"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-isosurface", "temperature", "--vtk-isovalue", "bad"]),
            config(&["svf-explorer", "--vtk", "mesh.vtk", "--vtk-isosurface-color", "pressure"]),
        ] {
            assert!(parse_vtk_args(&args).is_err(), "accepted invalid args: {args:?}");
        }
    }

    #[test]
    fn extracts_imported_tetrahedral_isosurface_and_retains_provenance() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(EXAMPLE_VTK).unwrap();
        let scene = prepare_imported_scene_with_options(
            dataset.clone(), None, Some("temperature"), Some(25.0), None,
        ).unwrap();
        let iso = scene.isosurface.as_ref().unwrap();
        assert_eq!(iso.isovalue(), 25.0);
        assert_eq!(iso.mesh().triangle_count(), 2);
        assert_eq!(iso.mesh().vertex_count(), 4);
        assert_eq!(iso.vertex_provenance().len(), 4);
        assert_eq!(iso.triangle_source_cells(), &[0, 0]);
        assert_eq!(iso.metadata().unsupported_cells().len(), 1);
        assert_eq!(iso.metadata().unsupported_cells()[0].cell_id, 1);
        assert_eq!(scene.mesh, *iso.mesh());
        assert_eq!(scene.dataset, dataset);
        assert!(scene.mapped_surface.is_none());
        assert!(scene.camera.view_projection_matrix().to_cols_array().iter().all(|v| v.is_finite()));
        assert!(scene.summary().contains("extracted 2 isosurface triangles"));
    }

    #[test]
    fn rejects_missing_vector_and_non_point_associated_isosurface_fields() {
        let mut dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        dataset.add_vector_field3(VectorField3::new(
            FieldMetadata::new("velocity", None).unwrap(),
            FieldAssociation::Point,
            VectorBasis3::World,
            vec![VectorValue3::from_f64(1.0, 0.0, 0.0); 4],
        ).unwrap()).unwrap();
        assert!(prepare_imported_scene_with_options(dataset.clone(), None, Some("missing"), Some(25.0), None)
            .unwrap_err().contains("not found"));
        assert!(prepare_imported_scene_with_options(dataset, None, Some("velocity"), Some(25.0), None)
            .unwrap_err().contains("vector field"));

        let cell_dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        assert!(prepare_imported_scene_with_options(cell_dataset, None, Some("density"), Some(7.0), None)
            .unwrap_err().contains("Cell association"));
    }

    #[test]
    fn selects_an_alternate_point_scalar_field_by_exact_name() {
        let mut dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("pressure", Some("kPa".to_owned())).unwrap(),
            FieldAssociation::Point,
            vec![
                SampleValue::Finite(0.0),
                SampleValue::Finite(100.0),
                SampleValue::Finite(0.0),
                SampleValue::Finite(100.0),
            ],
        ).unwrap()).unwrap();
        let scene = prepare_imported_scene_with_options(
            dataset, None, Some("pressure"), Some(50.0), None,
        ).unwrap();
        assert_eq!(scene.isosurface_field_name.as_deref(), Some("pressure"));
        assert_eq!(scene.isosurface.as_ref().unwrap().isovalue(), 50.0);
        assert_eq!(scene.isosurface.as_ref().unwrap().mesh().triangle_count(), 2);
    }

    #[test]
    fn rejects_dataset_associated_isosurface_scalar_field() {
        let mut dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("ambient", None).unwrap(),
            FieldAssociation::Dataset,
            vec![SampleValue::Finite(25.0)],
        ).unwrap()).unwrap();
        let error = prepare_imported_scene_with_options(
            dataset, None, Some("ambient"), Some(25.0), None,
        ).unwrap_err();
        assert!(error.contains("Dataset association"));
    }

    const AXIAL_VTK: &str = "# vtk DataFile Version 3.0
two tets
ASCII
DATASET UNSTRUCTURED_GRID
POINTS 5 double
0 0 0
1 0 0
0 1 0
0 0 1
1 1 1
CELLS 2 10
4 0 1 2 3
4 1 2 3 4
CELL_TYPES 2
10
10
CELL_DATA 2
SCALARS ID double 1
LOOKUP_TABLE default
1 5
VECTORS fibres double
1 0 0
0 1 0
";

    fn axial_config(extra: &[&str]) -> Result<Option<VtkLoadConfig>, String> {
        let mut args = vec!["svf-explorer", "--vtk", "heart.vtk", "--vtk-cell-axial", "fibres", "--vtk-cell-tags", "ID=1,2"];
        args.extend_from_slice(extra);
        parse_vtk_args(&config(&args))
    }

    #[test]
    fn parses_axial_options_with_core_defaults_and_overrides() {
        let defaults = BoundaryAxialGlyphOptions3::default();
        let parsed = axial_config(&[]).unwrap().unwrap().axial.unwrap();
        assert_eq!(parsed.direction_field, "fibres");
        assert_eq!(parsed.tag_field, "ID");
        assert_eq!(parsed.tags, vec![1, 2]);
        assert_eq!(parsed.options, defaults);
        let parsed = axial_config(&["--vtk-axial-bin", "2.5", "--vtk-axial-length", "1"]).unwrap().unwrap().axial.unwrap();
        assert_eq!(parsed.options, BoundaryAxialGlyphOptions3 { bin_edge: 2.5, length: 1.0 });
        let with_scalar = axial_config(&["--vtk-scalar", "RHO.dat", "--vtk-scalar-exclude", "-10"]).unwrap().unwrap();
        assert!(with_scalar.axial.is_some() && with_scalar.scalar_field.is_some());
    }

    #[test]
    fn rejects_invalid_axial_cli_usage() {
        for bad in [
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-cell-axial", "fibres"]),
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-cell-tags", "ID=1"]),
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-axial-bin", "4"]),
            config(&["svf-explorer", "--vtk-cell-axial", "fibres", "--vtk-cell-tags", "ID=1"]),
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-cell-axial", "f", "--vtk-cell-tags", "ID"]),
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-cell-axial", "f", "--vtk-cell-tags", "=1"]),
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-cell-axial", "f", "--vtk-cell-tags", "ID=1,x"]),
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-cell-axial", "f", "--vtk-cell-tags", "ID=1.5"]),
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-cell-axial", "f", "--vtk-cell-tags", "ID=1", "--vtk-vectors", "v"]),
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-cell-axial", "f", "--vtk-cell-tags", "ID=1", "--vtk-isosurface", "t", "--vtk-isovalue", "1"]),
            config(&["svf-explorer", "--vtk", "m.vtk", "--vtk-cell-axial", "f", "--vtk-cell-tags", "ID=1", "--vtk-cell-axial", "g"]),
        ] {
            assert!(parse_vtk_args(&bad).is_err(), "expected error for {bad:?}");
        }
        for value in ["0", "-1", "NaN", "inf", "abc"] {
            assert!(axial_config(&["--vtk-axial-bin", value]).is_err(), "bin {value}");
            assert!(axial_config(&["--vtk-axial-length", value]).is_err(), "length {value}");
        }
        assert!(axial_config(&["--vtk-axial-bin"]).is_err());
    }

    #[test]
    fn axial_glyphs_follow_requested_tags_and_leave_the_surface_unchanged() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(AXIAL_VTK).unwrap();
        let mut scene = prepare_imported_scene_with_options(dataset, None, None, None, None).unwrap();
        let triangles = scene.surface.triangle_count();
        assert_eq!(triangles, 6);
        let options = BoundaryAxialGlyphOptions3 { bin_edge: 0.25, length: 0.5 };
        let request = |tags: Vec<i64>| AxialGlyphConfig {
            direction_field: "fibres".to_owned(), tag_field: "ID".to_owned(), tags, options,
        };
        attach_axial_glyphs(&mut scene, &request(vec![1])).unwrap();
        let one = scene.axial_glyphs.as_ref().unwrap();
        assert_eq!(one.eligible_by_tag().keys().copied().collect::<Vec<_>>(), vec![1]);
        assert_eq!(one.glyphs_by_tag().keys().copied().collect::<Vec<_>>(), vec![1]);
        assert!(one.glyphs().iter().all(|glyph| glyph.sample().tag() == 1));
        let one_count = one.glyphs().len();
        assert!(one_count > 0);
        assert_eq!(scene.surface.triangle_count(), triangles);
        assert_eq!(scene.mesh.triangle_count(), triangles);
        let summary = scene.summary();
        assert!(summary.contains("tag 1: 3 selected, 3 eligible,"), "{summary}");
        assert!(summary.contains("excluded 0 zero-area, 0 invalid-direction"), "{summary}");

        attach_axial_glyphs(&mut scene, &request(vec![1, 5])).unwrap();
        assert!(scene.axial_glyphs.as_ref().unwrap().glyphs().len() > one_count);
        assert_eq!(scene.surface.triangle_count(), triangles);

        let error = attach_axial_glyphs(&mut scene, &AxialGlyphConfig {
            tag_field: "missing".to_owned(), ..request(vec![1])
        }).unwrap_err();
        assert!(error.contains("missing"), "{error}");
    }

    #[test]
    fn rejects_isosurface_that_produces_no_triangles() {
        let dataset = svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid(TETRA_VTK).unwrap();
        let error = prepare_imported_scene_with_options(dataset, None, Some("temperature"), Some(1000.0), None)
            .unwrap_err();
        assert!(error.contains("produced no triangles"));
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

    fn view_config(extra: &[&str]) -> Result<Option<VtkLoadConfig>, String> {
        let mut args = vec!["svf-explorer", "--vtk", "heart.vtk"];
        args.extend_from_slice(extra);
        parse_vtk_args(&config(&args))
    }

    #[test]
    fn camera_and_screenshot_options_parse_and_default_to_unset() {
        let plain = view_config(&[]).unwrap().unwrap();
        assert_eq!(plain.camera, CameraSetup::default());
        assert_eq!(plain.screenshot, None);

        let parsed = view_config(&[
            "--camera-yaw", "-35.5", "--camera-pitch", "12", "--camera-zoom", "2.5",
            "--screenshot", "out/frame one.png",
        ]).unwrap().unwrap();
        assert_eq!(parsed.camera, CameraSetup {
            yaw_degrees: Some(-35.5),
            pitch_degrees: Some(12.0),
            zoom_factor: Some(2.5),
        });
        assert_eq!(parsed.screenshot, Some(PathBuf::from("out/frame one.png")));
    }

    #[test]
    fn camera_and_screenshot_options_combine_with_every_vtk_mode() {
        let tail = ["--camera-yaw", "10", "--screenshot", "f.png"];
        for mode in [
            vec!["--vtk-scalar", "Z"],
            vec!["--vtk-vectors", "v"],
            vec!["--vtk-isosurface", "t", "--vtk-isovalue", "1"],
            vec!["--vtk-cell-axial", "fibres", "--vtk-cell-tags", "ID=1"],
        ] {
            let mut extra = mode.clone();
            extra.extend_from_slice(&tail);
            assert!(view_config(&extra).unwrap().unwrap().screenshot.is_some(), "{mode:?}");
        }
    }

    #[test]
    fn invalid_camera_and_screenshot_options_are_rejected() {
        for bad in [
            vec!["--camera-yaw", "NaN"],
            vec!["--camera-yaw", "inf"],
            vec!["--camera-yaw", "abc"],
            vec!["--camera-yaw"],
            vec!["--camera-pitch", "-inf"],
            vec!["--camera-zoom", "0"],
            vec!["--camera-zoom", "-2"],
            vec!["--camera-zoom", "NaN"],
            vec!["--camera-yaw", "1", "--camera-yaw", "2"],
            vec!["--camera-pitch", "1", "--camera-pitch", "2"],
            vec!["--camera-zoom", "1", "--camera-zoom", "2"],
            vec!["--screenshot"],
            vec!["--screenshot", "--vtk-scalar"],
            vec!["--screenshot", "a.png", "--screenshot", "b.png"],
        ] {
            assert!(view_config(&bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn camera_and_screenshot_options_require_vtk() {
        for flag in [["--camera-yaw", "5"], ["--camera-pitch", "5"], ["--camera-zoom", "2"], ["--screenshot", "f.png"]] {
            let args = config(&["svf-explorer", flag[0], flag[1]]);
            assert!(parse_vtk_args(&args).unwrap_err().contains("require --vtk"), "{flag:?}");
        }
    }
}
