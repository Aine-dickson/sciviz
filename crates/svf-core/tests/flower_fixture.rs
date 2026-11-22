//! The generic flower fixture (proof of concept, `docs/flower-structure.md`):
//! parses, every body is closed, plane cuts across it succeed and stay closed,
//! and the lengthwise cut shows the carpel's inside.

use std::collections::{HashMap, HashSet};

use svf_core::{
    spatial::{WorldPoint3, WorldVector3},
    structure::{Structure, StructureGeometry},
    structure_section::{section_geometry, SectionPlane},
};

const FLOWER: &str = include_str!("../../../fixtures/structures/flower.ron");

fn load() -> (Structure, StructureGeometry) {
    let structure = Structure::from_ron_str(FLOWER).expect("flower.ron parses and validates");
    let geometry = structure.build_geometry().expect("flower.ron builds");
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
fn the_fixture_has_the_flower_parts_and_a_citation_note() {
    let (s, g) = load();
    assert!(!s.reference().is_empty());
    for part in [
        "peduncle", "receptacle", "sepals", "petals", "filaments", "anthers", "stigma", "style", "ovary",
        "ovules", "fruit", "seeds",
    ] {
        let index = s.part_index(part).unwrap_or_else(|| panic!("missing part {part}"));
        assert!(s.parts()[index].geometry().is_some(), "{part} has no geometry");
    }
    assert_eq!(g.ranges().len(), s.parts().iter().filter(|p| p.geometry().is_some()).count());
}

#[test]
fn layers_and_views_are_valid() {
    let (s, _) = load();
    assert_eq!(s.layers().len(), 5);
    let ids: Vec<_> = s.views().iter().map(|v| v.id()).collect();
    assert_eq!(ids, ["flower", "carpel-only", "fruit"]);
    assert!(s.views().iter().all(|v| v.peel() <= s.layers().len()));
}

#[test]
fn every_body_is_closed() {
    let (_, g) = load();
    assert_closed(g.mesh().indices());
}

#[test]
fn plane_cuts_across_the_flower_succeed_and_stay_closed() {
    let (_, g) = load();
    // The flower is about x -12..12, y -13..10, z -12..12.
    let ranges = [(0usize, -11.0, 11.0, 1.1), (1, -12.0, 10.0, 0.9), (2, -11.0, 11.0, 1.1)];
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
fn the_lengthwise_cut_shows_the_inside_of_the_carpel() {
    let (s, g) = load();
    let cut = section_geometry(&g, &plane([0.0, 0.0, 0.0], [0.0, 0.0, -1.0])).unwrap();
    let names: HashSet<&str> = cut.ranges().iter().map(|r| s.parts()[r.part_index].id()).collect();
    for part in ["ovary", "ovules", "style", "stigma", "fruit", "seeds", "peduncle"] {
        assert!(names.contains(part), "{part} missing from the lengthwise cut");
    }
}
