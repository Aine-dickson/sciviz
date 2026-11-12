//! `--structure <file.ron>`: load a recipe structure (B2 step 2b), show it,
//! pick parts, isolate one part and show all again. Geometry and part
//! provenance come from `svf_core::structure`; this module only holds the
//! view state and the command line.

use std::{fs, path::PathBuf};

use svf_core::{
    camera::Camera3,
    geometry::TriangleMesh3,
    ray::{intersect_mesh, Ray3},
    structure::{Structure, StructureGeometry},
};

use crate::{
    camera_setup::{apply_camera_setup, CameraSetup},
    vtk_loading::{fit_camera, parse_finite_f64, parse_positive_f64},
};

#[derive(Clone, Debug, PartialEq)]
pub struct StructureConfig {
    pub path: PathBuf,
    pub camera: CameraSetup,
    pub screenshot: Option<PathBuf>,
}

/// `Ok(None)` when `--structure` is absent. The camera and screenshot flags
/// are the ones `--vtk` accepts, with the same meaning.
pub fn parse_structure_args(args: &[String]) -> Result<Option<StructureConfig>, String> {
    if !args.iter().any(|arg| arg == "--structure") {
        return Ok(None);
    }
    if args.iter().any(|arg| arg == "--vtk") {
        return Err("--structure cannot be combined with --vtk".to_owned());
    }
    let mut path = None;
    let mut camera = CameraSetup::default();
    let mut screenshot: Option<PathBuf> = None;
    let mut index = 1;
    while index < args.len() {
        let value = args.get(index + 1);
        match args[index].as_str() {
            "--structure" => {
                if path.is_some() {
                    return Err("--structure may only be supplied once".to_owned());
                }
                let value = value
                    .filter(|v| !v.starts_with("--") && !v.is_empty())
                    .ok_or_else(|| "--structure requires a file path".to_owned())?;
                path = Some(PathBuf::from(value));
            }
            "--camera-yaw" => {
                if camera.yaw_degrees.is_some() {
                    return Err("--camera-yaw may only be supplied once".to_owned());
                }
                camera.yaw_degrees = Some(parse_finite_f64("--camera-yaw", value)?);
            }
            "--camera-pitch" => {
                if camera.pitch_degrees.is_some() {
                    return Err("--camera-pitch may only be supplied once".to_owned());
                }
                camera.pitch_degrees = Some(parse_finite_f64("--camera-pitch", value)?);
            }
            "--camera-zoom" => {
                if camera.zoom_factor.is_some() {
                    return Err("--camera-zoom may only be supplied once".to_owned());
                }
                camera.zoom_factor = Some(parse_positive_f64("--camera-zoom", value)?);
            }
            "--screenshot" => {
                if screenshot.is_some() {
                    return Err("--screenshot may only be supplied once".to_owned());
                }
                let value = value
                    .filter(|v| !v.starts_with("--") && !v.is_empty())
                    .ok_or_else(|| "--screenshot requires an output PNG path".to_owned())?;
                screenshot = Some(PathBuf::from(value));
            }
            argument if argument.starts_with("--") => {
                return Err(format!("unknown option '{argument}' in structure mode"));
            }
            argument => {
                return Err(format!("unexpected argument '{argument}' in structure mode"));
            }
        }
        index += 2;
    }
    Ok(Some(StructureConfig {
        path: path.ok_or_else(|| "--structure requires a file path".to_owned())?,
        camera,
        screenshot,
    }))
}

/// A loaded structure plus what is currently visible.
pub struct StructureScene {
    pub structure: Structure,
    pub camera: Camera3,
    geometry: StructureGeometry,
    isolated: Option<usize>,
}

impl StructureScene {
    pub fn load(config: &StructureConfig) -> Result<Self, String> {
        let text = fs::read_to_string(&config.path)
            .map_err(|e| format!("cannot read structure '{}': {e}", config.path.display()))?;
        let structure = Structure::from_ron_str(&text)
            .map_err(|e| format!("invalid structure '{}': {e}", config.path.display()))?;
        Self::from_structure(structure, &config.camera)
    }

    pub fn from_structure(structure: Structure, setup: &CameraSetup) -> Result<Self, String> {
        let geometry = structure
            .build_geometry()
            .map_err(|e| format!("cannot build structure geometry: {e}"))?;
        if geometry.mesh().indices().is_empty() {
            return Err("the structure has no part with geometry".to_owned());
        }
        let fitted = fit_camera(geometry.mesh(), 1280.0 / 800.0)?;
        let camera = apply_camera_setup(&fitted, setup)?;
        Ok(Self { structure, camera, geometry, isolated: None })
    }

    pub fn mesh(&self) -> &TriangleMesh3 {
        self.geometry.mesh()
    }

    pub fn colours(&self) -> &[[f64; 3]] {
        self.geometry.colours()
    }

    pub fn isolated(&self) -> Option<usize> {
        self.isolated
    }

    pub fn summary(&self) -> String {
        let with_geometry = self.structure.parts().iter().filter(|p| p.geometry().is_some()).count();
        format!(
            "Structure '{}' ({}): {} parts ({} with geometry), {} triangles. Reference: {}",
            self.structure.name(),
            self.structure.units(),
            self.structure.parts().len(),
            with_geometry,
            self.geometry.mesh().indices().len() / 3,
            self.structure.reference(),
        )
    }

    /// The part under the ray in the visible mesh, as a part index.
    pub fn pick(&self, ray: &Ray3) -> Option<usize> {
        let hit = intersect_mesh(ray, self.geometry.mesh())?;
        self.geometry.part_of_triangle(hit.triangle_index())
    }

    /// Show only `part`. Errors if it has no geometry or is out of range;
    /// the current view is unchanged on error.
    pub fn isolate(&mut self, part: usize) -> Result<(), String> {
        let name = self.structure.parts().get(part).ok_or("unknown part")?.name().to_owned();
        let geometry = self
            .structure
            .build_geometry_for(&[part])
            .map_err(|e| format!("cannot isolate '{name}': {e}"))?;
        if geometry.mesh().indices().is_empty() {
            return Err(format!("'{name}' has no geometry of its own to isolate"));
        }
        self.geometry = geometry;
        self.isolated = Some(part);
        Ok(())
    }

    pub fn show_all(&mut self) -> Result<(), String> {
        self.geometry = self
            .structure
            .build_geometry()
            .map_err(|e| format!("cannot rebuild structure geometry: {e}"))?;
        self.isolated = None;
        Ok(())
    }

    /// "Dicot stem > Vascular bundles > Phloem".
    pub fn part_path(&self, part: usize) -> String {
        let mut names = Vec::new();
        let mut next = Some(part);
        while let Some(i) = next {
            let Some(p) = self.structure.parts().get(i) else { break };
            names.push(p.name());
            next = p.parent();
        }
        names.reverse();
        names.join(" > ")
    }

    pub fn part_title(&self, part: usize) -> String {
        self.structure.parts().get(part).map_or_else(String::new, |p| p.name().to_owned())
    }

    pub fn part_description(&self, part: usize) -> &str {
        self.structure.parts().get(part).map_or("", |p| p.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use svf_core::spatial::{WorldPoint3, WorldVector3};

    const STEM: &str = include_str!("../../../fixtures/structures/dicot-stem.ron");

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    fn scene() -> StructureScene {
        let structure = Structure::from_ron_str(STEM).unwrap();
        StructureScene::from_structure(structure, &CameraSetup::default()).unwrap()
    }

    #[test]
    fn parses_structure_flags_and_rejects_bad_ones() {
        assert_eq!(parse_structure_args(&args(&["x"])).unwrap(), None);
        let c = parse_structure_args(&args(&[
            "x", "--structure", "a.ron", "--camera-yaw", "90", "--screenshot", "o.png",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(c.path, PathBuf::from("a.ron"));
        assert_eq!(c.camera.yaw_degrees, Some(90.0));
        assert_eq!(c.screenshot, Some(PathBuf::from("o.png")));
        assert!(parse_structure_args(&args(&["x", "--structure"])).is_err());
        assert!(parse_structure_args(&args(&["x", "--structure", "a", "--structure", "b"])).is_err());
        assert!(parse_structure_args(&args(&["x", "--structure", "a", "--vtk", "b"])).is_err());
        assert!(parse_structure_args(&args(&["x", "--structure", "a", "--camera-zoom", "0"])).is_err());
        assert!(parse_structure_args(&args(&["x", "--structure", "a", "--bogus", "1"])).is_err());
    }

    #[test]
    fn pick_looking_down_the_axis_finds_the_part_at_each_radius() {
        let scene = scene();
        // Ray straight down -Y from above the top cap at a given x (z = 0).
        let pick = |x: f64| {
            let ray = Ray3::new(WorldPoint3::new(x, 50.0, 0.0), WorldVector3::new(0.0, -1.0, 0.0)).unwrap();
            scene.pick(&ray).map(|p| scene.structure.parts()[p].id().to_owned())
        };
        assert_eq!(pick(4.85).as_deref(), Some("epidermis"));
        assert_eq!(pick(4.0).as_deref(), Some("cortex"));
        // Bundle 0 is centred on angle 0: phloem 2.95..3.6 on the axis the angle is measured from.
        let ids: Vec<_> = [3.3, 2.9, 2.5].iter().map(|&x| pick(x)).collect();
        assert!(ids.iter().all(Option::is_some), "{ids:?}");
        assert_eq!(pick(0.5).as_deref(), Some("pith"));
        assert_eq!(pick(9.0), None);
    }

    #[test]
    fn isolate_then_show_all_restores_the_full_mesh() {
        let mut scene = scene();
        let full = scene.mesh().indices().len();
        let xylem = scene.structure.part_index("xylem").unwrap();
        scene.isolate(xylem).unwrap();
        assert_eq!(scene.isolated(), Some(xylem));
        assert!(scene.mesh().indices().len() < full);
        assert_eq!(scene.colours().len(), scene.mesh().vertices().len());
        // Only xylem is pickable now.
        let ray = Ray3::new(WorldPoint3::new(4.85, 50.0, 0.0), WorldVector3::new(0.0, -1.0, 0.0)).unwrap();
        assert_eq!(scene.pick(&ray), None);
        scene.show_all().unwrap();
        assert_eq!(scene.isolated(), None);
        assert_eq!(scene.mesh().indices().len(), full);
    }

    #[test]
    fn isolating_a_grouping_node_fails_and_keeps_the_view() {
        let mut scene = scene();
        let full = scene.mesh().indices().len();
        let group = scene.structure.part_index("vascular-bundles").unwrap();
        assert!(scene.isolate(group).is_err());
        assert!(scene.isolate(999).is_err());
        assert_eq!(scene.isolated(), None);
        assert_eq!(scene.mesh().indices().len(), full);
    }

    #[test]
    fn part_path_runs_from_the_root() {
        let scene = scene();
        let phloem = scene.structure.part_index("phloem").unwrap();
        assert_eq!(scene.part_path(phloem), "Dicot stem > Vascular bundles > Phloem");
    }
}
