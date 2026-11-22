//! The generic plant cell fixture (proof of concept, item register in
//! `docs/item-output-bar.md`): parses, every body is closed, the nucleus and the
//! vacuole keep clear of each other, plane cuts succeed and stay closed, and the middle cut shows the
//! organelles inside the wall.

use std::collections::{HashMap, HashSet};

use svf_core::{
    spatial::{WorldPoint3, WorldVector3},
    structure::{Structure, StructureGeometry},
    structure_section::{section_geometry, SectionPlane},
};

const CELL: &str = include_str!("../../../fixtures/structures/plant-cell.ron");

fn load() -> (Structure, StructureGeometry) {
    let structure = Structure::from_ron_str(CELL).expect("plant-cell.ron parses and validates");
    let geometry = structure.build_geometry().expect("plant-cell.ron builds");
    (structure, geometry)
}

fn assert_closed(indices: &[u32]) {
    let mut directed: HashMap<(u32, u32), usize> = HashMap::new();
    for t in indices.chunks_exact(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *directed.entry((a, b)).or_default() += 1;
        }
    }
    for (&(a, b), &n) in &directed {
        assert_eq!(n, 1, "directed edge ({a},{b}) used {n} times");
        assert_eq!(directed.get(&(b, a)), Some(&1), "edge ({a},{b}) has no opposite edge");
    }
}

fn plane(point: [f64; 3], normal: [f64; 3]) -> SectionPlane {
    SectionPlane::new(
        WorldPoint3::new(point[0], point[1], point[2]),
        WorldVector3::new(normal[0], normal[1], normal[2]),
    )
    .unwrap()
}

#[test]
fn the_fixture_has_the_cell_parts() {
    let (s, g) = load();
    assert!(!s.reference().is_empty());
    for part in [
        "cell-wall", "cell-membrane", "vacuole", "chloroplasts", "mitochondria", "nucleus", "nucleolus",
    ] {
        let index = s.part_index(part).unwrap_or_else(|| panic!("missing part {part}"));
        assert!(s.parts()[index].geometry().is_some(), "{part} has no geometry");
    }
    assert_eq!(g.ranges().len(), s.parts().iter().filter(|p| p.geometry().is_some()).count());
}

#[test]
fn layers_and_views_are_valid() {
    let (s, _) = load();
    assert_eq!(s.layers().len(), 6);
    let ids: Vec<_> = s.views().iter().map(|v| v.id()).collect();
    assert_eq!(ids, ["whole", "inside", "cut"]);
    assert!(s.views().iter().all(|v| v.peel() <= s.layers().len()));
}

#[test]
fn every_body_is_closed() {
    let (_, g) = load();
    assert_closed(g.mesh().indices());
}

#[test]
fn plane_cuts_across_the_cell_succeed_and_stay_closed() {
    let (_, g) = load();
    // The cell is about x -24..24, y -16..16, z -16..16.
    let ranges = [(0usize, -23.0, 23.0, 1.7), (1, -15.0, 15.0, 1.3), (2, -15.0, 15.0, 1.3)];
    for (axis, from, to, step) in ranges {
        let mut offset = from;
        while offset <= to {
            let (mut n, mut p) = ([0.0; 3], [0.0; 3]);
            n[axis] = 1.0;
            p[axis] = offset;
            let cut = section_geometry(&g, &plane(p, n))
                .unwrap_or_else(|e| panic!("axis {axis} offset {offset:.2}: {e}"));
            assert_closed(cut.mesh().indices());
            offset += step;
        }
    }
}

#[test]
fn the_middle_cut_shows_the_organelles_inside_the_wall() {
    let (s, g) = load();
    let cut = section_geometry(&g, &plane([0.0, 0.0, 0.0], [0.0, 0.0, -1.0])).unwrap();
    let names: HashSet<&str> = cut.ranges().iter().map(|r| s.parts()[r.part_index].id()).collect();
    for part in ["cell-wall", "cell-membrane", "vacuole", "chloroplasts", "mitochondria", "nucleus", "nucleolus"] {
        assert!(names.contains(part), "{part} missing from the middle cut");
    }
}

/// The two biggest bodies inside the cell must keep clear of each other (bounding boxes, built mesh).
/// The smaller organelle rings are placed by hand and are not checked here.
#[test]
fn nucleus_and_vacuole_bounding_boxes_do_not_overlap() {
    let (s, g) = load();
    let v = g.mesh().vertices();
    let indices = g.mesh().indices();
    let mut boxes: HashMap<&str, ([f64; 3], [f64; 3])> = HashMap::new();
    for r in g.ranges() {
        let id = s.parts()[r.part_index].id();
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        for t in r.first_triangle..r.first_triangle + r.triangle_count {
            for k in 0..3 {
                let p = v[indices[3 * t + k] as usize].to_array();
                for a in 0..3 {
                    lo[a] = lo[a].min(p[a]);
                    hi[a] = hi[a].max(p[a]);
                }
            }
        }
        boxes.insert(id, (lo, hi));
    }
    // The nucleus and the vacuole are the two big bodies; they must stay apart.
    let (nl, nh) = boxes["nucleus"];
    let (vl, vh) = boxes["vacuole"];
    let overlap = (0..3).all(|a| nl[a] < vh[a] && vl[a] < nh[a]);
    assert!(!overlap, "nucleus and vacuole boxes overlap");
}
