//! The cockroach fixture (B2 step 4b): parses, every body is closed, parts do
//! not overlap except where the anatomy joins them, every plane cut works, and
//! the dissection order shows what it should. Design: `docs/cockroach-structure.md`.

use std::collections::{HashMap, HashSet};

use svf_core::{
    spatial::{WorldPoint3, WorldVector3},
    structure::{RelationKind, Structure, StructureGeometry},
    structure_section::{section_geometry, SectionPlane},
};

const COCKROACH: &str = include_str!("../../../fixtures/structures/cockroach.ron");

fn load() -> (Structure, StructureGeometry) {
    let structure = Structure::from_ron_str(COCKROACH).expect("cockroach.ron parses and validates");
    let geometry = structure.build_geometry().expect("cockroach.ron builds");
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

fn id(structure: &Structure, index: usize) -> &str {
    structure.parts()[index].id()
}

// ------------------------------------------------------------ structure

#[test]
fn the_fixture_has_the_lesson_content() {
    let (s, g) = load();
    assert!(COCKROACH.len() < 60_000, "fixture is {} bytes", COCKROACH.len());
    assert!(!s.reference().is_empty());
    for part in [
        "head-capsule", "eyes", "antennae", "mouthparts", "pronotum", "mesonotum", "metanotum",
        "abdominal-tergites", "cerci", "tegmina", "hind-wings", "fore-coxa", "fore-femur", "fore-tibia",
        "fore-tarsus", "mid-coxa", "hind-tarsus", "heart", "fat-body", "salivary-glands", "oesophagus",
        "crop", "gizzard", "gastric-caeca", "midgut", "malpighian-tubules", "hindgut", "rectum", "ovaries",
        "oviducts", "genital-chamber", "spermatheca", "colleterial-glands", "tracheal-trunks", "spiracles",
        "brain", "suboesophageal-ganglion", "nerve-cord", "thoracic-ganglia", "abdominal-ganglia",
    ] {
        let index = s.part_index(part).unwrap_or_else(|| panic!("missing part {part}"));
        assert!(s.parts()[index].geometry().is_some(), "{part} has no geometry");
    }
    let with_geometry = s.parts().iter().filter(|p| p.geometry().is_some()).count();
    assert!(with_geometry >= 40, "{with_geometry} parts have geometry");
    assert_eq!(g.ranges().len(), with_geometry);
    assert!(g.part_of_triangle(0).is_some());
}

#[test]
fn layers_follow_the_dorsal_dissection_order() {
    let (s, _) = load();
    let ids: Vec<_> = s.layers().iter().map(|l| l.id()).collect();
    assert_eq!(
        ids,
        ["wings", "dorsal-wall", "heart-fat", "digestive", "reproductive", "tracheae", "nervous"]
    );
    let visible = |peeled: usize| -> HashSet<&str> {
        s.visible_parts(None, peeled).into_iter().map(|i| id(&s, i)).collect()
    };
    // Nothing the lesson says is "never peeled" is in a layer.
    for always in ["head-capsule", "thoracic-sterna", "abdominal-sternites", "fore-femur", "antennae", "cerci"] {
        assert!(visible(7).contains(always), "{always} must survive every peel");
    }
    assert!(visible(0).contains("tegmina") && !visible(1).contains("tegmina"));
    assert!(visible(1).contains("pronotum") && !visible(2).contains("pronotum"));
    assert!(!visible(2).contains("abdominal-tergites"));
    // Removing the dorsal wall reveals the heart and fat body, which the wall hid.
    assert!(visible(2).contains("heart") && visible(2).contains("fat-body"));
    assert!(!visible(3).contains("heart") && visible(3).contains("crop"));
    assert!(!visible(4).contains("crop") && visible(4).contains("ovaries"));
    assert!(!visible(5).contains("ovaries") && visible(5).contains("tracheal-trunks"));
    assert!(!visible(6).contains("tracheal-trunks") && visible(6).contains("nerve-cord"));
    assert!(!visible(7).contains("nerve-cord"));
}

#[test]
fn relations_and_views_are_sensible() {
    let (s, _) = load();
    let crop = s.part_index("crop").unwrap();
    let gizzard = s.part_index("gizzard").unwrap();
    assert!(s
        .relations_of(crop)
        .any(|r| r.kind() == RelationKind::OpensInto && r.from() == crop && r.to() == gizzard));
    assert!(s.relations().len() >= 12);
    let layer_count = s.layers().len();
    assert!(s.views().len() >= 8);
    assert!(s.views().iter().all(|v| v.peel() <= layer_count));
    let peels: Vec<_> = s.views().iter().map(|v| v.peel()).collect();
    for expected in 0..layer_count {
        assert!(peels.contains(&expected), "no view at peel {expected}");
    }
    assert!(s.views().iter().any(|v| v.section().is_some()));
}

#[test]
fn every_body_is_closed() {
    let (_, g) = load();
    assert_closed(g.mesh().indices());
}

// ------------------------------------------------------------- overlaps

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// One part's triangles with its bounding box, for point-in-solid by ray parity.
struct Body {
    triangles: Vec<[V3; 3]>,
    min: V3,
    max: V3,
}

impl Body {
    fn contains(&self, p: V3) -> bool {
        if (0..3).any(|k| p[k] < self.min[k] || p[k] > self.max[k]) {
            return false;
        }
        // An irrational-looking direction avoids grazing edges and vertices.
        let d: V3 = [0.577_350_269, 0.318_309_886, 0.751_113_131];
        let mut crossings = 0;
        for t in &self.triangles {
            let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
            let h = cross(d, e2);
            let a = dot(e1, h);
            if a.abs() < 1e-14 {
                continue;
            }
            let f = 1.0 / a;
            let s = sub(p, t[0]);
            let u = f * dot(s, h);
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = cross(s, e1);
            let v = f * dot(d, q);
            if v < 0.0 || u + v > 1.0 {
                continue;
            }
            if f * dot(e2, q) > 0.0 {
                crossings += 1;
            }
        }
        crossings % 2 == 1
    }
}

fn bodies(g: &StructureGeometry) -> Vec<(usize, Body)> {
    let v = g.mesh().vertices();
    let indices = g.mesh().indices();
    g.ranges()
        .iter()
        .map(|r| {
            let triangles: Vec<[V3; 3]> = (r.first_triangle..r.first_triangle + r.triangle_count)
                .map(|t| {
                    let at = |k: usize| v[indices[3 * t + k] as usize].to_array();
                    [at(0), at(1), at(2)]
                })
                .collect();
            let (mut min, mut max) = ([f64::MAX; 3], [f64::MIN; 3]);
            for p in triangles.iter().flatten() {
                for k in 0..3 {
                    min[k] = min[k].min(p[k]);
                    max[k] = max[k].max(p[k]);
                }
            }
            (r.part_index, Body { triangles, min, max })
        })
        .collect()
}

/// Pairs of parts that are *meant* to share space: a joint, an attachment or a
/// duct opening into an organ. Everything else must keep clear.
fn joined(a: &str, b: &str) -> bool {
    const JOINTS: &[(&str, &str)] = &[
        // Appendages and organs embedded in the cuticle or head capsule.
        ("eyes", "head-capsule"), ("antennae", "head-capsule"), ("mouthparts", "head-capsule"),
        ("oesophagus", "head-capsule"), ("oesophagus", "mouthparts"), ("nerve-cord", "head-capsule"), ("head-capsule", "pronotum"),
        ("cerci", "abdominal-tergites"), ("cerci", "abdominal-sternites"),
        ("spiracles", "abdominal-tergites"), ("spiracles", "abdominal-sternites"),
        ("spiracles", "tracheal-trunks"),
        // Gut chain: each region opens into the next.
        ("oesophagus", "crop"), ("crop", "gizzard"), ("gizzard", "midgut"), ("gastric-caeca", "midgut"),
        ("gastric-caeca", "gizzard"), ("midgut", "hindgut"), ("malpighian-tubules", "hindgut"),
        ("malpighian-tubules", "midgut"), ("hindgut", "rectum"), ("salivary-glands", "mouthparts"),
        ("salivary-glands", "oesophagus"),
        // Ducts opening into the genital chamber.
        ("oviducts", "genital-chamber"), ("colleterial-glands", "genital-chamber"),
        ("spermatheca", "genital-chamber"), ("oviducts", "ovaries"),
        // Nervous connections.
        ("brain", "suboesophageal-ganglion"), ("suboesophageal-ganglion", "nerve-cord"),
        ("thoracic-ganglia", "nerve-cord"), ("abdominal-ganglia", "nerve-cord"),
    ];
    let leg = |s: &str| -> Option<(String, usize)> {
        let (pair, seg) = s.split_once('-')?;
        let order = ["coxa", "femur", "tibia", "tarsus"].iter().position(|x| *x == seg)?;
        Some((pair.to_owned(), order))
    };
    if let (Some((p, i)), Some((q, j))) = (leg(a), leg(b)) {
        if p == q && i.abs_diff(j) == 1 {
            return true; // consecutive segments of one leg overlap at the joint
        }
    }
    let attaches = |s: &str, t: &str| s.ends_with("-coxa") && t == "thoracic-sterna";
    if attaches(a, b) || attaches(b, a) {
        return true;
    }
    JOINTS.iter().any(|(x, y)| (a == *x && b == *y) || (a == *y && b == *x))
}

#[test]
fn parts_do_not_overlap_except_where_the_anatomy_joins_them() {
    let (s, g) = load();
    let all = bodies(&g);
    let limit: usize = std::env::var("COCKROACH_OVERLAP_SAMPLES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);

    let mut found: HashMap<(String, String), usize> = HashMap::new();
    for (part_a, a) in &all {
        let stride = a.triangles.len().div_ceil(limit).max(1);
        for t in a.triangles.iter().step_by(stride) {
            let normal = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            let length = dot(normal, normal).sqrt();
            if length < 1e-12 {
                continue;
            }
            let centroid = [
                (t[0][0] + t[1][0] + t[2][0]) / 3.0,
                (t[0][1] + t[1][1] + t[2][1]) / 3.0,
                (t[0][2] + t[1][2] + t[2][2]) / 3.0,
            ];
            // A little way into the material (the normal points out of it).
            let p = [
                centroid[0] - 1e-3 * normal[0] / length,
                centroid[1] - 1e-3 * normal[1] / length,
                centroid[2] - 1e-3 * normal[2] / length,
            ];
            for (part_b, b) in &all {
                if part_a == part_b || joined(id(&s, *part_a), id(&s, *part_b)) {
                    continue;
                }
                if b.contains(p) {
                    let key = (id(&s, *part_a).to_owned(), id(&s, *part_b).to_owned());
                    *found.entry(key).or_default() += 1;
                }
            }
        }
    }
    let mut report: Vec<_> = found.into_iter().collect();
    report.sort();
    assert!(
        report.is_empty(),
        "unexpected overlaps (part A has points inside part B):\n{}",
        report.iter().map(|((a, b), n)| format!("  {a} in {b}: {n}")).collect::<Vec<_>>().join("\n")
    );
}

// -------------------------------------------------------------- sections

fn plane(point: V3, normal: V3) -> SectionPlane {
    SectionPlane::new(
        WorldPoint3::new(point[0], point[1], point[2]),
        WorldVector3::new(normal[0], normal[1], normal[2]),
    )
    .unwrap()
}

#[test]
fn plane_cuts_across_the_whole_animal_succeed_and_stay_closed() {
    let (_, g) = load();
    // (axis, from, to, step): the body is about x 0..40, y -7..7, z -10..10.
    // Coarser in debug builds, where a cut of this scene takes a while.
    let k = if cfg!(debug_assertions) { 2.0 } else { 1.0 };
    let ranges = [(0usize, -1.0, 41.0, 1.3 * k), (1, -7.0, 7.0, 0.7 * k), (2, -9.0, 9.0, 0.7 * k)];
    let mut slowest = std::time::Duration::ZERO;
    for (axis, from, to, step) in ranges {
        let mut offset = from;
        while offset <= to {
            let (mut n, mut p) = ([0.0; 3], [0.0; 3]);
            n[axis] = 1.0;
            p[axis] = offset;
            let start = std::time::Instant::now();
            let cut = section_geometry(&g, &plane(p, n))
                .unwrap_or_else(|e| panic!("axis {axis} offset {offset:.2}: {e}"));
            slowest = slowest.max(start.elapsed());
            assert_closed(cut.mesh().indices());
            offset += step;
        }
    }
    println!("slowest cut {slowest:?}");
}

#[test]
fn the_sagittal_cut_shows_the_organs_inside_the_body_wall() {
    let (s, g) = load();
    let cut = section_geometry(&g, &plane([0.0, 0.0, 0.0], [0.0, 0.0, -1.0])).unwrap();
    // Keeping z <= 0 with the cap facing +z... the cut must cap the midline
    // organs (they are cut through) and still hold the body wall.
    let names: HashSet<&str> = cut.ranges().iter().map(|r| id(&s, r.part_index)).collect();
    for part in ["crop", "gizzard", "midgut", "hindgut", "nerve-cord", "heart", "brain", "head-capsule"] {
        assert!(names.contains(part), "{part} missing from the sagittal cut");
    }
}
