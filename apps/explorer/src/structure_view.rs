//! `--structure <file.ron>`: load a recipe structure (B2 step 2b), show it,
//! pick parts, isolate one part and show all again. Geometry and part
//! provenance come from `svf_core::structure`; this module only holds the
//! view state and the command line.

use std::{fs, path::PathBuf};

use svf_core::{
    camera::Camera3,
    geometry::TriangleMesh3,
    ray::{intersect_mesh, Ray3},
    spatial::{WorldPoint3, WorldVector3},
    structure::{Structure, StructureGeometry},
    structure_section::{section_geometry, SectionPlane},
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
    pub view: ViewOptions,
    /// `--view <id>`: start in a named view of the structure file.
    pub view_id: Option<String>,
}

/// Initial section and peel state from the command line. A flag that is given
/// overrides the same setting of a `--view`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewOptions {
    /// World axis index (0 = x, 1 = y, 2 = z) and the plane offset along it.
    pub section: Option<(usize, f64)>,
    pub flip: bool,
    pub peel: Option<usize>,
}

const AXIS_NAMES: [&str; 3] = ["x", "y", "z"];
/// The section plane moves by this fraction of the extent per key press.
const SECTION_STEPS: f64 = 50.0;

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
    let mut view = ViewOptions::default();
    let mut view_id: Option<String> = None;
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
            "--section" => {
                if view.section.is_some() {
                    return Err("--section may only be supplied once".to_owned());
                }
                let value = value.ok_or_else(|| "--section requires <x|y|z>=<offset>".to_owned())?;
                let (axis, offset) = value
                    .split_once('=')
                    .ok_or_else(|| format!("invalid --section '{value}': expected <x|y|z>=<offset>"))?;
                let axis = AXIS_NAMES
                    .iter()
                    .position(|name| *name == axis)
                    .ok_or_else(|| format!("invalid --section axis '{axis}': use x, y or z"))?;
                let offset = offset
                    .parse::<f64>()
                    .ok()
                    .filter(|o| o.is_finite())
                    .ok_or_else(|| format!("invalid --section offset '{offset}': expected a finite number"))?;
                view.section = Some((axis, offset));
            }
            "--section-flip" => {
                view.flip = true;
                index += 1;
                continue;
            }
            "--peel" => {
                if view.peel.is_some() {
                    return Err("--peel may only be supplied once".to_owned());
                }
                let value = value.ok_or_else(|| "--peel requires a layer count".to_owned())?;
                view.peel = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("invalid --peel '{value}': expected a whole number"))?,
                );
            }
            "--view" => {
                if view_id.is_some() {
                    return Err("--view may only be supplied once".to_owned());
                }
                let value = value
                    .filter(|v| !v.starts_with("--") && !v.is_empty())
                    .ok_or_else(|| "--view requires a view id".to_owned())?;
                view_id = Some(value.clone());
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
    if view.flip && view.section.is_none() {
        return Err("--section-flip requires --section".to_owned());
    }
    Ok(Some(StructureConfig {
        path: path.ok_or_else(|| "--structure requires a file path".to_owned())?,
        camera,
        screenshot,
        view,
        view_id,
    }))
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SectionState {
    axis: usize,
    offset: f64,
    flipped: bool,
}

impl SectionState {
    fn plane(&self) -> Result<SectionPlane, String> {
        let mut point = [0.0; 3];
        point[self.axis] = self.offset;
        let mut normal = [0.0; 3];
        normal[self.axis] = if self.flipped { -1.0 } else { 1.0 };
        SectionPlane::new(
            WorldPoint3::new(point[0], point[1], point[2]),
            WorldVector3::new(normal[0], normal[1], normal[2]),
        )
        .map_err(|e| e.to_string())
    }
}

/// The state a named view asks for. The view is already validated.
fn view_state(structure: &Structure, index: usize) -> ViewState {
    let view = &structure.views()[index];
    ViewState {
        setup: view.camera().map_or_else(CameraSetup::default, |c| CameraSetup {
            yaw_degrees: Some(c.yaw_deg),
            pitch_degrees: Some(c.pitch_deg),
            zoom_factor: Some(c.zoom),
        }),
        shown: view.show().to_vec(),
        peeled: view.peel(),
        section: view.section().map(|s| SectionState {
            axis: s.axis.index(),
            offset: s.offset,
            flipped: s.flip,
        }),
    }
}

/// A loaded structure plus what is currently visible: an optional isolated
/// part, the number of peeled layers and an optional section plane.
pub struct StructureScene {
    pub structure: Structure,
    pub camera: Camera3,
    geometry: StructureGeometry,
    /// The camera fitted to the whole structure; views are relative to it.
    fitted: Camera3,
    isolated: Option<usize>,
    peeled: usize,
    section: Option<SectionState>,
    /// Parts a view restricts the display to (empty = no restriction).
    shown: Vec<usize>,
    /// Index of the current named view, if one is applied.
    view: Option<usize>,
    bounds: ([f64; 3], [f64; 3]),
}

/// Everything a named view sets besides the camera.
struct ViewState {
    setup: CameraSetup,
    shown: Vec<usize>,
    peeled: usize,
    section: Option<SectionState>,
}

impl StructureScene {
    pub fn load(config: &StructureConfig) -> Result<Self, String> {
        let text = fs::read_to_string(&config.path)
            .map_err(|e| format!("cannot read structure '{}': {e}", config.path.display()))?;
        let structure = Structure::from_ron_str(&text)
            .map_err(|e| format!("invalid structure '{}': {e}", config.path.display()))?;
        Self::from_structure(structure, &config.camera, &config.view, config.view_id.as_deref())
    }

    pub fn from_structure(
        structure: Structure,
        setup: &CameraSetup,
        view: &ViewOptions,
        view_id: Option<&str>,
    ) -> Result<Self, String> {
        let full = structure
            .build_geometry()
            .map_err(|e| format!("cannot build structure geometry: {e}"))?;
        if full.mesh().indices().is_empty() {
            return Err("the structure has no part with geometry".to_owned());
        }
        // The camera is fitted to the whole structure, so cuts and peels do
        // not move it.
        let fitted = fit_camera(full.mesh(), 1280.0 / 800.0)?;
        let named = match view_id {
            None => None,
            Some(id) => Some(structure.views().iter().position(|v| v.id() == id).ok_or_else(|| {
                let ids: Vec<&str> = structure.views().iter().map(|v| v.id()).collect();
                format!("unknown view '{id}'; this structure has: {}", ids.join(", "))
            })?),
        };
        let from_view = named.map(|i| view_state(&structure, i));
        // Command-line flags override the view, field by field.
        let wanted = match &from_view {
            None => *setup,
            Some(state) => CameraSetup {
                yaw_degrees: setup.yaw_degrees.or(state.setup.yaw_degrees),
                pitch_degrees: setup.pitch_degrees.or(state.setup.pitch_degrees),
                zoom_factor: setup.zoom_factor.or(state.setup.zoom_factor),
            },
        };
        let camera = apply_camera_setup(&fitted, &wanted)?;
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for point in full.mesh().vertices() {
            let p = point.to_array();
            for axis in 0..3 {
                low[axis] = low[axis].min(p[axis]);
                high[axis] = high[axis].max(p[axis]);
            }
        }
        if let Some(peel) = view.peel {
            if peel > structure.layers().len() {
                return Err(format!(
                    "--peel {peel} but the structure has {} layers",
                    structure.layers().len()
                ));
            }
        }
        let (view_shown, view_peel, view_section) = match from_view {
            None => (Vec::new(), 0, None),
            Some(state) => (state.shown, state.peeled, state.section),
        };
        let mut scene = Self {
            structure,
            camera,
            geometry: full,
            fitted,
            isolated: None,
            peeled: 0,
            section: None,
            shown: Vec::new(),
            view: None,
            bounds: (low, high),
        };
        let section = match view.section {
            Some((axis, offset)) => Some(SectionState { axis, offset, flipped: view.flip }),
            None => view_section,
        };
        scene.set_full(None, view.peel.unwrap_or(view_peel), section, view_shown, named)?;
        Ok(scene)
    }

    /// Rebuild the visible geometry for a new state, keeping the current view
    /// restriction; the scene is unchanged on error.
    fn set(
        &mut self,
        isolated: Option<usize>,
        peeled: usize,
        section: Option<SectionState>,
    ) -> Result<(), String> {
        self.set_full(isolated, peeled, section, self.shown.clone(), self.view)
    }

    fn set_full(
        &mut self,
        isolated: Option<usize>,
        peeled: usize,
        section: Option<SectionState>,
        shown: Vec<usize>,
        view: Option<usize>,
    ) -> Result<(), String> {
        let mut visible = self.structure.visible_parts(isolated, peeled);
        if !shown.is_empty() {
            visible.retain(|part| shown.contains(part));
        }
        let mut geometry = self
            .structure
            .build_geometry_for(&visible)
            .map_err(|e| format!("cannot build structure geometry: {e}"))?;
        if geometry.mesh().indices().is_empty() {
            return Err("no part with geometry would be visible".to_owned());
        }
        if let Some(state) = section {
            geometry = section_geometry(&geometry, &state.plane()?)
                .map_err(|e| format!("cannot cut the structure: {e}"))?;
            if geometry.mesh().indices().is_empty() {
                return Err("the section would remove everything; move or flip the plane".to_owned());
            }
        }
        self.geometry = geometry;
        self.isolated = isolated;
        self.peeled = peeled;
        self.section = section;
        self.shown = shown;
        self.view = view;
        Ok(())
    }

    /// `V`: apply the next named view (camera, parts shown, peel, section).
    /// Unchanged on error.
    pub fn next_view(&mut self) -> Result<String, String> {
        let count = self.structure.views().len();
        if count == 0 {
            return Err("this structure has no named views".to_owned());
        }
        let next = self.view.map_or(0, |i| (i + 1) % count);
        self.apply_view(next)
    }

    fn apply_view(&mut self, index: usize) -> Result<String, String> {
        let state = view_state(&self.structure, index);
        let camera = apply_camera_setup(&self.fitted, &state.setup)?;
        self.set_full(None, state.peeled, state.section, state.shown, Some(index))?;
        self.camera = camera;
        Ok(format!(
            "View {}/{}: {}",
            index + 1,
            self.structure.views().len(),
            self.structure.views()[index].name()
        ))
    }

    /// Relations of `part`, as sentences ("Crop opens into Gizzard").
    pub fn relation_lines(&self, part: usize) -> Vec<String> {
        let name = |i: usize| self.part_title(i);
        self.structure
            .relations_of(part)
            .map(|r| format!("{} {} {}", name(r.from()), r.kind().phrase(), name(r.to())))
            .collect()
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

    pub fn peeled(&self) -> usize {
        self.peeled
    }

    pub fn section_axis(&self) -> Option<usize> {
        self.section.map(|s| s.axis)
    }

    /// Short state text for the window title; empty when nothing is applied.
    pub fn state_text(&self) -> String {
        let mut parts = Vec::new();
        if let Some(i) = self.view {
            parts.push(format!("view: {}", self.structure.views()[i].name()));
        }
        if let Some(i) = self.isolated {
            parts.push(format!("isolated: {}", self.part_title(i)));
        }
        if self.peeled > 0 {
            parts.push(format!("peeled {}/{}", self.peeled, self.structure.layers().len()));
        }
        if let Some(s) = self.section {
            parts.push(format!(
                "section {}={:.2}{}",
                AXIS_NAMES[s.axis],
                s.offset,
                if s.flipped { " (flipped)" } else { "" }
            ));
        }
        parts.join(" | ")
    }

    pub fn summary(&self) -> String {
        let with_geometry = self.structure.parts().iter().filter(|p| p.geometry().is_some()).count();
        format!(
            "Structure '{}' ({}): {} parts ({} with geometry), {} layers, {} triangles. Reference: {}",
            self.structure.name(),
            self.structure.units(),
            self.structure.parts().len(),
            with_geometry,
            self.structure.layers().len(),
            self.geometry.mesh().indices().len() / 3,
            self.structure.reference(),
        )
    }

    /// The part under the ray in the visible mesh, as a part index.
    pub fn pick(&self, ray: &Ray3) -> Option<usize> {
        let hit = intersect_mesh(ray, self.geometry.mesh())?;
        self.geometry.part_of_triangle(hit.triangle_index())
    }

    /// Show only `part`. Errors if it has no geometry or is hidden by a peel;
    /// the current view is unchanged on error.
    pub fn isolate(&mut self, part: usize) -> Result<String, String> {
        let name = self.structure.parts().get(part).ok_or("unknown part")?.name().to_owned();
        if self.structure.parts()[part].geometry().is_none() {
            return Err(format!("'{name}' has no geometry of its own to isolate"));
        }
        if !self.structure.visible_parts(None, self.peeled).contains(&part) {
            return Err(format!("'{name}' is in a peeled layer; restore it first (R)"));
        }
        self.set(Some(part), self.peeled, self.section)?;
        Ok(format!("Isolated: {}", self.part_path(part)))
    }

    /// `A`: clear isolation and peeling. The section plane stays (it has its own key).
    pub fn show_all(&mut self) -> Result<String, String> {
        self.set_full(None, 0, self.section, Vec::new(), None)?;
        Ok("Showing all parts".to_owned())
    }

    /// `S`: off, transverse (plane normal along the structure axis, world y),
    /// longitudinal (world x), off. A plane set from the command line on
    /// another axis goes to off.
    pub fn cycle_section(&mut self) -> Result<String, String> {
        let next = match self.section {
            None => Some(1),
            Some(s) if s.axis == 1 => Some(0),
            Some(_) => None,
        };
        let section = next.map(|axis| SectionState {
            axis,
            offset: 0.5 * (self.bounds.0[axis] + self.bounds.1[axis]),
            flipped: false,
        });
        self.set(self.isolated, self.peeled, section)?;
        Ok(match section {
            None => "Section off".to_owned(),
            Some(s) => format!(
                "Section {} at {}={:.2}",
                if s.axis == 1 { "across the axis" } else { "along the axis" },
                AXIS_NAMES[s.axis],
                s.offset
            ),
        })
    }

    /// `[` and `]`: move the plane by `steps` of 1/50 of the extent, kept inside the bounds.
    pub fn move_section(&mut self, steps: f64) -> Result<String, String> {
        let state = self.section.ok_or("no section plane; press S first")?;
        let (low, high) = (self.bounds.0[state.axis], self.bounds.1[state.axis]);
        let offset = (state.offset + steps * (high - low) / SECTION_STEPS).clamp(low, high);
        let moved = SectionState { offset, ..state };
        self.set(self.isolated, self.peeled, Some(moved))?;
        Ok(format!("Section {}={:.2}", AXIS_NAMES[state.axis], offset))
    }

    /// `F`: keep the other side of the cut.
    pub fn flip_section(&mut self) -> Result<String, String> {
        let state = self.section.ok_or("no section plane; press S first")?;
        let flipped = SectionState { flipped: !state.flipped, ..state };
        self.set(self.isolated, self.peeled, Some(flipped))?;
        Ok(if flipped.flipped { "Section flipped" } else { "Section restored" }.to_owned())
    }

    /// `P`: remove the outermost layer still shown.
    pub fn peel(&mut self) -> Result<String, String> {
        let total = self.structure.layers().len();
        if total == 0 {
            return Err("this structure has no layers to peel".to_owned());
        }
        if self.peeled >= total {
            return Err("every layer is already peeled".to_owned());
        }
        let layer = self.structure.layers()[self.peeled].name().to_owned();
        self.set(self.isolated, self.peeled + 1, self.section)?;
        Ok(format!("Peeled layer '{layer}' ({} of {total})", self.peeled))
    }

    /// `R`: put back the layer peeled last.
    pub fn restore(&mut self) -> Result<String, String> {
        if self.peeled == 0 {
            return Err("no layer is peeled".to_owned());
        }
        let layer = self.structure.layers()[self.peeled - 1].name().to_owned();
        self.set(self.isolated, self.peeled - 1, self.section)?;
        Ok(format!("Restored layer '{layer}' ({} peeled)", self.peeled))
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
        StructureScene::from_structure(structure, &CameraSetup::default(), &ViewOptions::default(), None).unwrap()
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

    fn above(scene: &StructureScene, x: f64, y: f64) -> Option<String> {
        let ray = Ray3::new(WorldPoint3::new(x, y, 0.0), WorldVector3::new(0.0, -1.0, 0.0)).unwrap();
        scene.pick(&ray).map(|p| scene.structure.parts()[p].id().to_owned())
    }

    #[test]
    fn parses_section_and_peel_flags() {
        let c = parse_structure_args(&args(&[
            "x", "--structure", "a.ron", "--section", "y=6.5", "--section-flip", "--peel", "2",
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(c.view, ViewOptions { section: Some((1, 6.5)), flip: true, peel: Some(2) });
        let bad = |extra: &[&str]| {
            let mut a = vec!["x", "--structure", "a.ron"];
            a.extend_from_slice(extra);
            parse_structure_args(&args(&a)).is_err()
        };
        assert!(bad(&["--section", "w=1"]));
        assert!(bad(&["--section", "y"]));
        assert!(bad(&["--section", "y=nan"]));
        assert!(bad(&["--section-flip"]));
        assert!(bad(&["--peel", "-1"]));
        assert!(bad(&["--peel", "1", "--peel", "2"]));
    }

    #[test]
    fn section_across_the_stem_makes_the_cap_pickable_by_tissue() {
        let mut scene = scene();
        scene.cycle_section().unwrap();
        assert_eq!(scene.section_axis(), Some(1));
        // Plane at mid-length, cap faces up: a ray from above hits the cap.
        assert_eq!(above(&scene, 4.85, 50.0).as_deref(), Some("epidermis"));
        assert_eq!(above(&scene, 4.0, 50.0).as_deref(), Some("cortex"));
        assert_eq!(above(&scene, 0.5, 50.0).as_deref(), Some("pith"));
        // The cap is at y = 6, not at the top end y = 12.
        let ray = Ray3::new(WorldPoint3::new(0.5, 50.0, 0.0), WorldVector3::new(0.0, -1.0, 0.0)).unwrap();
        let hit = intersect_mesh(&ray, scene.mesh()).unwrap();
        assert!((hit.position().to_array()[1] - 6.0).abs() < 1e-6);
    }

    #[test]
    fn section_keys_cycle_move_flip_and_clamp() {
        let mut scene = scene();
        assert!(scene.move_section(1.0).is_err());
        assert!(scene.flip_section().is_err());
        scene.cycle_section().unwrap();
        scene.move_section(5.0).unwrap();
        assert!(scene.state_text().contains("section y=7.20"), "{}", scene.state_text());
        scene.flip_section().unwrap();
        assert!(scene.state_text().contains("flipped"));
        // Far beyond the end the plane clamps to the structure's bounds.
        scene.move_section(-1000.0).unwrap();
        assert!(scene.state_text().contains("section y=0.00"), "{}", scene.state_text());
        scene.cycle_section().unwrap(); // flipped y -> longitudinal x
        assert_eq!(scene.section_axis(), Some(0));
        scene.cycle_section().unwrap();
        assert_eq!(scene.section_axis(), None);
    }

    #[test]
    fn peeling_removes_layers_from_the_outside_and_restores_them() {
        let mut scene = scene();
        scene.peel().unwrap();
        assert_eq!(above(&scene, 4.85, 50.0), None, "epidermis is gone");
        assert_eq!(above(&scene, 4.0, 50.0).as_deref(), Some("cortex"));
        scene.peel().unwrap();
        scene.peel().unwrap();
        assert_eq!(above(&scene, 0.5, 50.0).as_deref(), Some("pith"));
        assert_eq!(above(&scene, 4.0, 50.0), None);
        // Peeling the last layer would leave nothing, so it is refused.
        assert!(scene.peel().unwrap_err().contains("no part"));
        assert_eq!(scene.peeled(), 3);
        scene.restore().unwrap();
        assert_eq!(scene.peeled(), 2);
        scene.show_all().unwrap();
        assert_eq!(scene.peeled(), 0);
        assert!(scene.restore().is_err());
    }

    #[test]
    fn peel_and_section_compose_and_a_failed_command_changes_nothing() {
        let mut scene = scene();
        scene.peel().unwrap();
        scene.cycle_section().unwrap();
        let triangles = scene.mesh().indices().len();
        assert_eq!(scene.state_text(), "peeled 1/4 | section y=6.00");
        // Isolating a peeled part is refused and the view stays.
        let skin = scene.structure.part_index("epidermis").unwrap();
        assert!(scene.isolate(skin).is_err());
        assert_eq!(scene.mesh().indices().len(), triangles);
        // A plane that removes everything is refused and the view stays.
        let cut = SectionState { axis: 1, offset: -5.0, flipped: false };
        assert!(scene.set(None, 0, Some(cut)).is_err());
        assert_eq!(scene.mesh().indices().len(), triangles);
        assert_eq!(scene.peeled(), 1);
    }

    #[test]
    fn initial_view_options_apply_and_bad_peel_is_rejected() {
        let structure = Structure::from_ron_str(STEM).unwrap();
        let view = ViewOptions { section: Some((0, 0.0)), flip: false, peel: Some(2) };
        let scene = StructureScene::from_structure(structure.clone(), &CameraSetup::default(), &view, None).unwrap();
        assert_eq!(scene.peeled(), 2);
        assert_eq!(scene.section_axis(), Some(0));
        let bad = ViewOptions { peel: Some(9), ..ViewOptions::default() };
        assert!(StructureScene::from_structure(structure, &CameraSetup::default(), &bad, None).is_err());
    }

    const COCKROACH: &str = include_str!("../../../fixtures/structures/cockroach.ron");

    fn cockroach(view: Option<&str>, setup: &CameraSetup, options: &ViewOptions) -> Result<StructureScene, String> {
        let structure = Structure::from_ron_str(COCKROACH).unwrap();
        StructureScene::from_structure(structure, setup, options, view)
    }

    #[test]
    fn parses_the_view_flag() {
        let c = parse_structure_args(&args(&["x", "--structure", "a.ron", "--view", "gut"])).unwrap().unwrap();
        assert_eq!(c.view_id.as_deref(), Some("gut"));
        assert_eq!(parse_structure_args(&args(&["x", "--structure", "a.ron"])).unwrap().unwrap().view_id, None);
        assert!(parse_structure_args(&args(&["x", "--structure", "a.ron", "--view"])).is_err());
        assert!(parse_structure_args(&args(&["x", "--structure", "a.ron", "--view", "a", "--view", "b"])).is_err());
    }

    #[test]
    fn a_named_view_sets_peel_camera_and_parts_and_unknown_ids_list_the_choices() {
        let none = ViewOptions::default();
        let plain = cockroach(None, &CameraSetup::default(), &none).unwrap();
        let gut = cockroach(Some("gut"), &CameraSetup::default(), &none).unwrap();
        assert_eq!(gut.peeled(), 3);
        assert!(gut.state_text().contains("view: Heart and fat body removed: the gut"), "{}", gut.state_text());
        assert_ne!(gut.camera.look_at().position(), plain.camera.look_at().position());
        assert!(gut.mesh().indices().len() < plain.mesh().indices().len());

        let only = cockroach(Some("gut-only"), &CameraSetup::default(), &none).unwrap();
        let triangles = only.mesh().indices().len() / 3;
        let ids: std::collections::HashSet<_> = (0..triangles)
            .filter_map(|t| only.geometry.part_of_triangle(t))
            .map(|p| only.structure.parts()[p].id().to_owned())
            .collect();
        assert!(ids.contains("crop") && ids.contains("abdominal-sternites"), "{ids:?}");
        assert!(!ids.contains("heart") && !ids.contains("head-capsule") && !ids.contains("fore-femur"), "{ids:?}");

        let error = cockroach(Some("nope"), &CameraSetup::default(), &none).err().unwrap();
        assert!(error.contains("unknown view 'nope'") && error.contains("sagittal"), "{error}");
    }

    #[test]
    fn flags_override_the_views_own_values() {
        let by_view = cockroach(Some("gut"), &CameraSetup::default(), &ViewOptions::default()).unwrap();
        let peel = ViewOptions { peel: Some(1), ..ViewOptions::default() };
        assert_eq!(cockroach(Some("gut"), &CameraSetup::default(), &peel).unwrap().peeled(), 1);
        let zoom = CameraSetup { zoom_factor: Some(2.0), ..CameraSetup::default() };
        let zoomed = cockroach(Some("gut"), &zoom, &ViewOptions::default()).unwrap();
        let distance = |s: &StructureScene| {
            (s.camera.look_at().position().to_vector() - s.camera.look_at().target().to_vector()).length()
        };
        assert!((distance(&zoomed) * 2.0 - distance(&by_view)).abs() < 1e-6 * distance(&by_view));
        assert_eq!(zoomed.camera.look_at().target(), by_view.camera.look_at().target());
    }

    #[test]
    fn v_cycles_every_view_and_wraps() {
        let mut scene = cockroach(None, &CameraSetup::default(), &ViewOptions::default()).unwrap();
        let count = scene.structure.views().len();
        assert!(count >= 9);
        let mut seen = Vec::new();
        for _ in 0..count {
            scene.next_view().unwrap_or_else(|e| panic!("{e}"));
            seen.push(scene.state_text());
        }
        assert!(seen[0].contains("Whole animal from above"), "{}", seen[0]);
        assert_eq!(scene.view, Some(count - 1));
        scene.next_view().unwrap();
        assert_eq!(scene.view, Some(0), "wraps to the first view");
        // `A` leaves the view: everything shown, nothing peeled, no view named.
        scene.next_view().unwrap();
        scene.show_all().unwrap();
        assert_eq!((scene.view, scene.peeled()), (None, 0));
        assert!(!scene.state_text().contains("view:"));
    }

    #[test]
    fn peeling_works_inside_a_view_with_a_section() {
        let mut scene = cockroach(Some("sagittal"), &CameraSetup::default(), &ViewOptions::default()).unwrap();
        assert_eq!(scene.section_axis(), Some(2));
        scene.peel().unwrap();
        assert_eq!(scene.peeled(), 1);
        assert!(scene.state_text().contains("view:") && scene.state_text().contains("peeled 1/7"));
    }

    #[test]
    fn a_view_that_cannot_be_applied_changes_nothing() {
        let text = STEM.trim_end().trim_end_matches(')').trim_end().trim_end_matches(',').to_owned()
            + ", views: [View(id: \"up\", name: \"Straight up\", camera: Some((yaw_deg: 0.0, pitch_deg: 89.9, zoom: 1.0)))])";
        let structure = Structure::from_ron_str(&text).unwrap();
        let mut scene =
            StructureScene::from_structure(structure, &CameraSetup::default(), &ViewOptions::default(), None).unwrap();
        let before = scene.camera;
        let error = scene.next_view().unwrap_err();
        assert!(error.contains("elevation"), "{error}");
        assert_eq!(scene.view, None);
        assert_eq!(scene.camera, before);
    }

    #[test]
    fn relations_are_listed_as_sentences() {
        let scene = cockroach(None, &CameraSetup::default(), &ViewOptions::default()).unwrap();
        let crop = scene.structure.part_index("crop").unwrap();
        assert_eq!(scene.relation_lines(crop), ["Oesophagus opens into Crop", "Crop opens into Gizzard"]);
        let root = scene.structure.part_index("cockroach").unwrap();
        assert!(scene.relation_lines(root).is_empty());
    }
}
