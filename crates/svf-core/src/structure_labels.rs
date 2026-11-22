//! Labels and key entries for a structure picture: what they are and where
//! they point. Design: `docs/labels.md` (slice L1). Laying them out on screen
//! is a separate step, and drawing them belongs to the host.
//!
//! A label points at one part (or a group, standing for its descendants). Its
//! anchor is either authored (`at`) or the part's mesh vertex nearest to the
//! centre of its bounding box, so it lies on the surface. The anchor follows
//! the part's pose with the same transform as `StructureGeometry::apply_pose`.

use serde::Deserialize;

use crate::structure::{Structure, StructureGeometry};
use crate::structure_animation::{pose_point, range_vertices_and_centre, Pose};

/// Which column a label is placed in. Without one, the layout chooses by the
/// anchor's side of the picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum LabelSide {
    Left,
    Right,
}

/// A label shown at one instant (or in one view).
#[derive(Clone, Debug, PartialEq)]
pub struct PartLabel {
    /// Index of the labelled part.
    pub part: usize,
    pub text: String,
    /// Authored anchor in structure units, before the pose; `None` uses the default anchor.
    pub at: Option<[f64; 3]>,
    pub side: Option<LabelSide>,
}

/// One entry of the colour key: a swatch and its meaning.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyEntry {
    pub colour: [f64; 3],
    pub text: String,
}

/// File representation of a key entry in a view.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Key")]
pub struct KeyDocument {
    pub colour: (f64, f64, f64),
    pub text: String,
}

/// A checked key entry: non-empty text, colour values in 0 to 1.
pub(crate) fn key_entry(colour: (f64, f64, f64), text: &str) -> Result<KeyEntry, &'static str> {
    if text.trim().is_empty() {
        return Err("a key entry needs text");
    }
    let colour = [colour.0, colour.1, colour.2];
    if !colour.iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)) {
        return Err("key colour values must be in 0 to 1");
    }
    Ok(KeyEntry { colour, text: text.to_owned() })
}

/// A checked label text and anchor.
pub(crate) fn label_parts(text: Option<&String>, at: Option<(f64, f64, f64)>) -> Result<(), &'static str> {
    if text.is_some_and(|t| t.trim().is_empty()) {
        return Err("a label's text must not be empty (leave it out to use the part's name)");
    }
    if at.is_some_and(|a| !(a.0.is_finite() && a.1.is_finite() && a.2.is_finite())) {
        return Err("a label's anchor must be finite");
    }
    Ok(())
}

impl Structure {
    /// Whether `part` is `root` or one of its descendants.
    fn is_within(&self, part: usize, root: usize) -> bool {
        let mut current = Some(part);
        while let Some(c) = current {
            if c == root {
                return true;
            }
            current = self.parts().get(c).and_then(|p| p.parent());
        }
        false
    }

    /// Where `label` points under `pose`, in world units. `geometry` is the
    /// unposed geometry built from (at least) the labelled part and its
    /// descendants. `None` when none of them has geometry in it.
    #[must_use]
    pub fn label_anchor(&self, geometry: &StructureGeometry, label: &PartLabel, pose: &Pose) -> Option<[f64; 3]> {
        let points: Vec<[f64; 3]> = geometry.mesh().vertices().iter().map(|p| p.to_array()).collect();
        let indices = geometry.mesh().indices();
        let ranges: Vec<_> = geometry.ranges().iter().filter(|r| self.is_within(r.part_index, label.part)).collect();
        if ranges.is_empty() {
            return None;
        }
        let posed = |part: usize, point: [f64; 3], centre: [f64; 3]| match pose.parts.get(part) {
            Some(p) => pose_point(point, centre, p),
            None => point,
        };

        if let Some(at) = label.at {
            // On a part with geometry the anchor turns and scales with it; on a group it only moves.
            if let Some(own) = ranges.iter().find(|r| r.part_index == label.part) {
                let (_, centre) = range_vertices_and_centre(&points, indices, own);
                return Some(posed(label.part, at, centre));
            }
            let offset = pose.parts.get(label.part).map_or([0.0; 3], |p| p.offset);
            return Some([at[0] + offset[0], at[1] + offset[1], at[2] + offset[2]]);
        }

        // Default: the vertex nearest the centre of the bounding box of everything labelled.
        let per_range: Vec<(usize, Vec<usize>, [f64; 3])> = ranges
            .iter()
            .map(|r| {
                let (used, centre) = range_vertices_and_centre(&points, indices, r);
                (r.part_index, used, centre)
            })
            .filter(|(_, used, _)| !used.is_empty())
            .collect();
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for (_, used, _) in &per_range {
            for &i in used {
                for axis in 0..3 {
                    lo[axis] = lo[axis].min(points[i][axis]);
                    hi[axis] = hi[axis].max(points[i][axis]);
                }
            }
        }
        let middle = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
        let distance = |p: [f64; 3]| (p[0] - middle[0]).powi(2) + (p[1] - middle[1]).powi(2) + (p[2] - middle[2]).powi(2);
        let mut best: Option<(f64, usize, usize, [f64; 3])> = None; // (distance, vertex, part, part centre)
        for (part, used, centre) in &per_range {
            for &i in used {
                let d = distance(points[i]);
                if best.is_none_or(|(bd, bi, _, _)| d < bd || (d == bd && i < bi)) {
                    best = Some((d, i, *part, *centre));
                }
            }
        }
        best.map(|(_, i, part, centre)| posed(part, points[i], centre))
    }
}

#[cfg(test)]
mod tests {
    use super::{LabelSide, PartLabel};
    use crate::structure::{Structure, StructureError};

    /// `root` (group) holding `a` (a ball at x = 4) and `b` (a ball at x = -4); `c` is a group with no geometry under it.
    fn structure(timelines: &str, views: &str) -> Result<Structure, StructureError> {
        let text = format!(
            "Structure(version: 1, name: \"S\", units: \"cm\", reference: \"r\", circle_segments: 16, parts: [\
             Part(id: \"root\", name: \"Root\", parent: None, description: \"d\", colour: (0.5, 0.5, 0.5), geometry: None), \
             Part(id: \"a\", name: \"Ball A\", parent: Some(\"root\"), description: \"d\", colour: (0.5, 0.5, 0.5), \
                  geometry: Some(Ellipsoid(centre: (4.0, 0.0, 0.0), radii: (1.0, 1.0, 1.0)))), \
             Part(id: \"b\", name: \"Ball B\", parent: Some(\"root\"), description: \"d\", colour: (0.5, 0.5, 0.5), \
                  geometry: Some(Ellipsoid(centre: (-4.0, 0.0, 0.0), radii: (1.0, 1.0, 1.0)))), \
             Part(id: \"c\", name: \"Empty\", parent: None, description: \"d\", colour: (0.5, 0.5, 0.5), geometry: None)], \
             layers: [], views: [{views}], timelines: [{timelines}])"
        );
        Structure::from_ron_str(&text)
    }

    fn timeline(actions: &str) -> String {
        format!("Timeline(id: \"t\", name: \"T\", scenes: [Scene(id: \"s\", beats: [Beat(id: \"b\", actions: [{actions}])])])")
    }

    fn invalid(actions: &str) -> String {
        match structure(&timeline(actions), "") {
            Err(e) => format!("{e:?}"),
            Ok(_) => panic!("expected {actions} to be refused"),
        }
    }

    fn label(part: usize) -> PartLabel {
        PartLabel { part, text: String::new(), at: None, side: None }
    }

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    #[test]
    fn labels_persist_until_removed_and_default_to_the_part_name() {
        let s = structure(
            &timeline(
                "Label(part: \"a\"), Label(part: \"b\", text: Some(\"Second ball\"), side: Some(Left), at_s: 2.0), \
                 Unlabel(part: \"a\", at_s: 4.0), Wait(for_s: 6.0)",
            ),
            "",
        )
        .unwrap();
        let t = &s.timelines()[0];
        let texts = |time: f64| t.pose_at(time).labels.iter().map(|l| l.text.clone()).collect::<Vec<_>>();
        assert_eq!(texts(0.0), ["Ball A"]);
        assert_eq!(texts(3.0), ["Ball A", "Second ball"]);
        assert_eq!(texts(5.0), ["Second ball"]);
        let second = &t.pose_at(3.0).labels[1];
        assert_eq!((second.part, second.side), (2, Some(LabelSide::Left)));
    }

    #[test]
    fn a_new_label_on_the_same_part_replaces_the_old_one() {
        let s = structure(&timeline("Label(part: \"a\"), Label(part: \"a\", text: Some(\"Renamed\"), at_s: 1.0), Wait(for_s: 2.0)"), "")
            .unwrap();
        let labels = s.timelines()[0].pose_at(1.5).labels;
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].text, "Renamed");
    }

    #[test]
    fn key_entries_persist_until_removed_and_particles_add_their_own() {
        let s = structure(
            &timeline(
                "Key(id: \"k\", colour: (1.0, 0.0, 0.0), text: \"Red thing\"), Unkey(id: \"k\", at_s: 3.0), \
                 Flow(id: \"f\", count: 3, path: [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0)], speed: 1.0, lateral: 0.0, seed: 1, \
                      particle_radius: 0.1, colour: (0.0, 0.0, 1.0), for_s: 2.0, at_s: 1.0, key: Some(\"Electron\")), \
                 Flow(id: \"g\", count: 3, path: [(0.0, 1.0, 0.0), (1.0, 1.0, 0.0)], speed: 1.0, lateral: 0.0, seed: 2, \
                      particle_radius: 0.1, colour: (0.0, 0.0, 1.0), for_s: 2.0, at_s: 1.0, key: Some(\"Electron\")), \
                 Wait(for_s: 5.0)",
            ),
            "",
        )
        .unwrap();
        let t = &s.timelines()[0];
        let texts = |time: f64| t.pose_at(time).key.iter().map(|k| k.text.clone()).collect::<Vec<_>>();
        assert_eq!(texts(0.5), ["Red thing"]);
        // Two flows with the same key and colour make one entry.
        assert_eq!(texts(2.0), ["Red thing", "Electron"]);
        assert_eq!(t.pose_at(2.0).key[1].colour, [0.0, 0.0, 1.0]);
        assert!(texts(4.0).is_empty(), "the key is removed and the flows have ended");
    }

    #[test]
    fn bad_labels_and_keys_are_refused() {
        assert!(invalid("Label(part: \"nope\")").contains("nope"));
        assert!(invalid("Label(part: \"a\", text: Some(\" \"))").contains("text"));
        assert!(invalid("Label(part: \"a\", at: Some((1.0, 0.0, inf)))").contains("finite"));
        assert!(invalid("Key(id: \"k\", colour: (2.0, 0.0, 0.0), text: \"x\")").contains("0 to 1"));
        assert!(invalid("Key(id: \"k\", colour: (1.0, 0.0, 0.0), text: \"\")").contains("text"));
        assert!(invalid("Unkey(id: \"never\")").contains("never"));
        assert!(invalid("Unlabel(part: \"nope\")").contains("nope"));
    }

    #[test]
    fn views_carry_labels_and_a_key() {
        let s = structure(
            "",
            "View(id: \"v\", name: \"V\", labels: [\"a\", \"root\"], key: [Key(colour: (0.0, 1.0, 0.0), text: \"Green\")])",
        )
        .unwrap();
        let v = &s.views()[0];
        assert_eq!(v.labels().iter().map(|l| (l.part, l.text.as_str())).collect::<Vec<_>>(), [(1, "Ball A"), (0, "Root")]);
        assert_eq!(v.key()[0].text, "Green");
        assert!(structure("", "View(id: \"v\", name: \"V\", labels: [\"nope\"])").is_err());
        assert!(structure("", "View(id: \"v\", name: \"V\", key: [Key(colour: (0.0, 1.0, 0.0), text: \"\")])").is_err());
    }

    #[test]
    fn the_default_anchor_is_on_the_surface_and_a_group_points_at_its_descendants() {
        let s = structure(&timeline("Wait(for_s: 1.0)"), "").unwrap();
        let g = s.build_geometry().unwrap();
        let pose = s.timelines()[0].pose_at(0.0);
        let a = s.label_anchor(&g, &label(1), &pose).unwrap();
        assert!((dist(a, [4.0, 0.0, 0.0]) - 1.0).abs() < 0.05, "on ball A's surface: {a:?}");
        // The group's box centre is the origin; the nearest vertex is on one of the balls' inner sides.
        let r = s.label_anchor(&g, &label(0), &pose).unwrap();
        assert!(r[0].abs() > 2.9 && r[0].abs() < 3.1, "{r:?}");
        // A group with nothing built under it has no anchor.
        assert_eq!(s.label_anchor(&g, &label(3), &pose), None);
    }

    #[test]
    fn anchors_follow_moves_turns_and_growth() {
        let s = structure(
            &timeline(
                "Move(part: \"root\", by: (0.0, 10.0, 0.0), for_s: 1.0), Grow(part: \"a\", to: 2.0, for_s: 1.0), \
                 Turn(part: \"b\", axis: (0.0, 0.0, 1.0), deg: 90.0, for_s: 1.0)",
            ),
            "",
        )
        .unwrap();
        let g = s.build_geometry().unwrap();
        let t = &s.timelines()[0];
        let (before, after) = (t.pose_at(0.0), t.pose_at(1.0));
        // Ball A doubled about its centre and moved up 10: its default anchor is still on its (now radius 2) surface.
        let a = s.label_anchor(&g, &label(1), &after).unwrap();
        assert!((dist(a, [4.0, 10.0, 0.0]) - 2.0).abs() < 0.1, "{a:?}");
        // An authored anchor on ball B turns with it about B's centre: (-3, 0, 0) goes to (-4, 1, 0), then up 10.
        let at = PartLabel { at: Some([-3.0, 0.0, 0.0]), ..label(2) };
        assert!(dist(s.label_anchor(&g, &at, &before).unwrap(), [-3.0, 0.0, 0.0]) < 1e-9);
        assert!(dist(s.label_anchor(&g, &at, &after).unwrap(), [-4.0, 11.0, 0.0]) < 1e-9);
        // An authored anchor on a group only moves with it.
        let on_group = PartLabel { at: Some([0.0, 0.0, 0.0]), ..label(0) };
        assert!(dist(s.label_anchor(&g, &on_group, &after).unwrap(), [0.0, 10.0, 0.0]) < 1e-9);
    }
}
