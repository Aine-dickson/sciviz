//! The heart fixture (B2 step 5e): derived from Rodero et al. (2021) case 01 by
//! `examples/heart_adapt.rs`. Checks that it parses with its mesh container and
//! provenance, every part is closed, outward and the right volume, parts do not
//! overlap (empty allow-list), the frame is as documented, the lesson content is
//! there, and plane cuts on all three axes succeed. Design: `docs/heart-structure.md`.

use std::collections::{HashMap, HashSet};

use svf_core::{
    region_surface::{is_closed_manifold, signed_volume},
    spatial::{WorldPoint3, WorldVector3},
    structure::{RelationKind, Structure, StructureGeometry},
    structure_section::{SectionPlane, section_geometry},
};

const HEART: &str = include_str!("../../../fixtures/structures/heart-case01.ron");
const MESH: &[u8] = include_bytes!("../../../fixtures/structures/heart-case01.svfmesh");

fn load() -> (Structure, StructureGeometry) {
    let resolver = |name: &str| -> Result<Vec<u8>, String> {
        if name == "heart-case01.svfmesh" {
            Ok(MESH.to_vec())
        } else {
            Err(format!("unexpected mesh file {name}"))
        }
    };
    let structure =
        Structure::from_ron_with(HEART, &resolver).expect("heart-case01.ron parses and validates");
    let geometry = structure.build_geometry().expect("heart-case01.ron builds");
    (structure, geometry)
}

fn id(structure: &Structure, index: usize) -> &str {
    structure.parts()[index].id()
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
        assert_eq!(
            directed.get(&(b, a)),
            Some(&1),
            "edge ({a},{b}) has no opposite edge"
        );
    }
}

/// One part's triangles as index triples with its points.
fn body(g: &StructureGeometry, part: usize) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
    let range = g
        .ranges()
        .iter()
        .find(|r| r.part_index == part)
        .expect("part has geometry");
    let indices = g.mesh().indices();
    let slice =
        &indices[range.first_triangle * 3..(range.first_triangle + range.triangle_count) * 3];
    let mut used: Vec<u32> = slice.to_vec();
    used.sort_unstable();
    used.dedup();
    let points: Vec<[f64; 3]> = used
        .iter()
        .map(|&i| g.mesh().vertices()[i as usize].to_array())
        .collect();
    let tris = slice
        .chunks_exact(3)
        .map(|t| [0, 1, 2].map(|k| used.binary_search(&t[k]).unwrap() as u32))
        .collect();
    (points, tris)
}

fn centroid(points: &[[f64; 3]]) -> [f64; 3] {
    let n = points.len() as f64;
    [0, 1, 2].map(|k| points.iter().map(|p| p[k]).sum::<f64>() / n)
}

// ------------------------------------------------------------ structure

const PARTS: [&str; 21] = [
    "left-ventricle",
    "right-ventricle",
    "left-atrium",
    "right-atrium",
    "aorta",
    "pulmonary-artery",
    "mitral-valve-plane",
    "tricuspid-valve-plane",
    "aortic-valve-plane",
    "pulmonary-valve-plane",
    "left-atrial-appendage",
    "left-superior-pulmonary-vein",
    "left-inferior-pulmonary-vein",
    "right-inferior-pulmonary-vein",
    "right-superior-pulmonary-vein",
    "superior-vena-cava",
    "inferior-vena-cava",
    "left-ventricle-cavity",
    "right-ventricle-cavity",
    "left-atrium-cavity",
    "right-atrium-cavity",
];

#[test]
fn the_fixture_has_the_parts_the_budget_and_the_provenance() {
    let (s, g) = load();
    assert!(MESH.len() < 2_000_000, "mesh is {} bytes", MESH.len());
    assert!(HEART.len() < 30_000, "structure is {} bytes", HEART.len());
    for part in PARTS {
        let index = s
            .part_index(part)
            .unwrap_or_else(|| panic!("missing part {part}"));
        assert!(
            s.parts()[index].geometry().is_some(),
            "{part} has no geometry"
        );
        assert!(!s.parts()[index].description().is_empty());
    }
    assert_eq!(g.ranges().len(), PARTS.len());
    let triangles = g.mesh().indices().len() / 3;
    assert!(
        (40_000..=75_000).contains(&triangles),
        "{triangles} triangles"
    );

    let p = s.provenance().expect("provenance block");
    assert_eq!(p.doi, "10.5281/zenodo.4590294");
    assert_eq!(p.licence, "CC BY 4.0");
    assert!(p.modified);
    assert!(p.attribution.contains("Rodero") && p.attribution.contains("CC BY 4.0"));
    assert_eq!(
        p.source_sha256,
        "20bf29b8ec86454de4cfde2a0afc19992715107b6ef8a7dabe61f52e8f78f440"
    );
    assert!(p.tool_commit.len() >= 7 && p.tool_commit != "unknown");
    assert!(p.parameters.iter().any(|(k, _)| k == "deviation_mm"));
    assert_eq!(p.body_volumes_mm3.len(), PARTS.len());
    assert!(s.reference().contains("one individual") || s.reference().contains("One individual"));
}

#[test]
fn the_frame_is_a_rigid_rotation() {
    let (s, _) = load();
    let m = s
        .provenance()
        .unwrap()
        .frame_transform
        .expect("frame recorded");
    let rows = [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]];
    for i in 0..3 {
        for j in 0..3 {
            let d: f64 = (0..3).map(|k| rows[i][k] * rows[j][k]).sum();
            assert!(
                (d - if i == j { 1.0 } else { 0.0 }).abs() < 1e-6,
                "rows {i},{j}: {d}"
            );
        }
    }
    let det = rows[0][0] * (rows[1][1] * rows[2][2] - rows[1][2] * rows[2][1])
        - rows[0][1] * (rows[1][0] * rows[2][2] - rows[1][2] * rows[2][0])
        + rows[0][2] * (rows[1][0] * rows[2][1] - rows[1][1] * rows[2][0]);
    assert!((det - 1.0).abs() < 1e-6, "determinant {det}");
}

#[test]
fn every_part_is_closed_outward_and_has_the_source_volume() {
    let (s, g) = load();
    assert_closed(g.mesh().indices());
    let volumes: HashMap<&str, f64> = s
        .provenance()
        .unwrap()
        .body_volumes_mm3
        .iter()
        .map(|(n, v)| (n.as_str(), *v))
        .collect();
    for part in PARTS {
        let index = s.part_index(part).unwrap();
        let (points, tris) = body(&g, index);
        assert!(is_closed_manifold(&tris), "{part} is not a closed manifold");
        let volume = signed_volume(&points, &tris);
        assert!(volume > 0.0, "{part} is not wound outward");
        let source = volumes[part];
        // Walls and cavities within 1 percent; thin parts (about 1 mm thick) within 10 percent.
        let tolerance = if part.ends_with("-plane")
            || part.contains("vein")
            || part.contains("vena")
            || part.contains("appendage")
            || part == "aorta"
            || part == "pulmonary-artery"
        {
            0.10
        } else {
            0.01
        };
        assert!(
            ((volume - source) / source).abs() <= tolerance,
            "{part}: {volume:.1} against {source:.1} mm3"
        );
    }
}

#[test]
fn the_heart_frame_puts_the_left_ventricle_apex_down_and_the_right_ventricle_on_the_right() {
    let (s, g) = load();
    let c = |part: &str| centroid(&body(&g, s.part_index(part).unwrap()).0);
    let (lv, rv) = (c("left-ventricle-cavity"), c("right-ventricle-cavity"));
    assert!(
        lv[0] < rv[0],
        "the left ventricle lies at smaller X than the right"
    );
    let atria = c("left-atrium-cavity")[1].min(c("right-atrium-cavity")[1]);
    assert!(
        lv[1].min(rv[1]) < atria,
        "the ventricles lie toward the apex (-Y), the atria toward the base"
    );
    let planes: Vec<[f64; 3]> = ["mitral", "tricuspid", "aortic", "pulmonary"]
        .iter()
        .map(|n| c(&format!("{n}-valve-plane")))
        .collect();
    let mean = centroid(&planes);
    assert!(
        mean.iter().all(|v| v.abs() < 1.0),
        "valve planes centre at the origin, got {mean:?}"
    );
    // Named by position: the left-sided veins lie at smaller X than the right-sided ones.
    let x = |n: &str| c(n)[0];
    assert!(x("left-superior-pulmonary-vein") < x("right-superior-pulmonary-vein"));
    assert!(x("left-inferior-pulmonary-vein") < x("right-inferior-pulmonary-vein"));
    // The superior vena cava enters above (larger Y) the inferior one.
    assert!(c("superior-vena-cava")[1] > c("inferior-vena-cava")[1]);
}

#[test]
fn layers_peel_to_the_chambers_and_the_valve_planes() {
    let (s, _) = load();
    let ids: Vec<_> = s.layers().iter().map(|l| l.id()).collect();
    assert_eq!(
        ids,
        ["vessels", "atria", "right-ventricle", "left-ventricle"]
    );
    let visible = |peeled: usize| -> HashSet<&str> {
        s.visible_parts(None, peeled)
            .into_iter()
            .map(|i| id(&s, i))
            .collect()
    };
    assert!(visible(0).contains("left-ventricle") && visible(0).contains("aorta"));
    assert!(!visible(1).contains("aorta") && !visible(1).contains("superior-vena-cava"));
    assert!(visible(1).contains("left-atrium") && !visible(2).contains("left-atrium"));
    assert!(!visible(3).contains("right-ventricle") && visible(3).contains("left-ventricle"));
    let bare = visible(4);
    for wall in [
        "left-ventricle",
        "right-ventricle",
        "left-atrium",
        "right-atrium",
        "aorta",
        "pulmonary-artery",
    ] {
        assert!(!bare.contains(wall), "{wall} must be peeled");
    }
    for kept in [
        "mitral-valve-plane",
        "left-ventricle-cavity",
        "right-atrium-cavity",
    ] {
        assert!(bare.contains(kept), "{kept} must survive every peel");
    }
}

#[test]
fn relations_and_views_are_sensible() {
    let (s, _) = load();
    let ra = s.part_index("right-atrium-cavity").unwrap();
    let rv = s.part_index("right-ventricle-cavity").unwrap();
    assert!(
        s.relations_of(ra)
            .any(|r| r.kind() == RelationKind::OpensInto && r.from() == ra && r.to() == rv)
    );
    assert!(s.relations().len() >= 12);
    let ids: Vec<_> = s.views().iter().map(|v| v.id()).collect();
    for wanted in [
        "anterior",
        "four-chamber",
        "short-axis",
        "walls-peeled",
        "valve-planes",
        "great-vessels",
    ] {
        assert!(ids.contains(&wanted), "missing view {wanted}");
    }
    assert!(s.views().iter().filter(|v| v.section().is_some()).count() >= 2);
}

// ------------------------------------------------------------- overlaps

type V3 = [f64; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

struct Solid {
    triangles: Vec<[V3; 3]>,
    min: V3,
    max: V3,
}

impl Solid {
    fn new(points: &[V3], tris: &[[u32; 3]]) -> Self {
        let triangles: Vec<[V3; 3]> = tris.iter().map(|t| t.map(|i| points[i as usize])).collect();
        let (mut min, mut max) = ([f64::MAX; 3], [f64::MIN; 3]);
        for p in points {
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        Self {
            triangles,
            min,
            max,
        }
    }

    /// Point in solid by ray parity along an off-axis direction.
    fn contains(&self, p: V3) -> bool {
        if (0..3).any(|k| p[k] < self.min[k] || p[k] > self.max[k]) {
            return false;
        }
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

#[test]
fn parts_do_not_overlap_with_an_empty_allow_list() {
    let (s, g) = load();
    let solids: Vec<(usize, Solid)> = PARTS
        .iter()
        .map(|n| {
            let index = s.part_index(n).unwrap();
            let (points, tris) = body(&g, index);
            (index, Solid::new(&points, &tris))
        })
        .collect();
    let limit: usize = std::env::var("HEART_OVERLAP_SAMPLES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    let mut found: HashMap<(String, String), usize> = HashMap::new();
    for (a_index, a) in &solids {
        let stride = a.triangles.len().div_ceil(limit).max(1);
        for t in a.triangles.iter().step_by(stride) {
            let normal = cross(sub(t[1], t[0]), sub(t[2], t[0]));
            let length = dot(normal, normal).sqrt();
            if length < 1e-12 {
                continue;
            }
            let centroid = [0, 1, 2].map(|k| (t[0][k] + t[1][k] + t[2][k]) / 3.0);
            // A little way into the material (the normal points out of it).
            let p = [0, 1, 2].map(|k| centroid[k] - 1e-3 * normal[k] / length);
            for (b_index, b) in &solids {
                if a_index != b_index && b.contains(p) {
                    *found
                        .entry((id(&s, *a_index).to_owned(), id(&s, *b_index).to_owned()))
                        .or_default() += 1;
                }
            }
        }
    }
    let mut report: Vec<_> = found.into_iter().collect();
    report.sort();
    assert!(
        report.is_empty(),
        "unexpected overlaps (part A has points inside part B):\n{}",
        report
            .iter()
            .map(|((a, b), n)| format!("  {a} in {b}: {n}"))
            .collect::<Vec<_>>()
            .join("\n")
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
fn plane_cuts_on_all_three_axes_succeed_and_stay_closed() {
    let (_, g) = load();
    let range = g.ranges().iter().find(|r| r.part_index == 8).unwrap();
let source = &g.mesh().indices()
    [range.first_triangle * 3..(range.first_triangle + range.triangle_count) * 3];

let target = [
    [-23.99432373046875, 49.658905029296875, -18.415678024291992],
    [-24.134376525878906, 49.166534423828125, -17.898147583007813],
];

for (i, t) in source.chunks_exact(3).enumerate() {
    let positions: Vec<_> = t.iter()
        .map(|&index| g.mesh().vertices()[index as usize].to_array())
        .collect();

    if target.iter().all(|q| positions.iter().any(|p| {
        (0..3).all(|axis| (p[axis] - q[axis]).abs() < 1e-5)
    })) {
        println!("SOURCE TRIANGLE {i}: {t:?} {positions:?}");
    }
}
    // The heart spans about x -66..58, y -93..64, z -53..72 in the heart frame.
    let k = if cfg!(debug_assertions) { 2.5 } else { 1.0 };
    let ranges = [
        (0usize, -64.0, 56.0, 4.0 * k),
        (1, -90.0, 62.0, 4.0 * k),
        (2, -50.0, 70.0, 4.0 * k),
    ];
    let mut slowest = std::time::Duration::ZERO;
    let mut cuts = 0;
    for (axis, from, to, step) in ranges {
        let mut offset = from + 0.137;
        while offset <= to {
            let (mut n, mut p) = ([0.0; 3], [0.0; 3]);
            n[axis] = 1.0;
            p[axis] = offset;
            let start = std::time::Instant::now();
            let cut = section_geometry(&g, &plane(p, n))
                .unwrap_or_else(|e| panic!("axis {axis} offset {offset:.2}: {e}"));
            slowest = slowest.max(start.elapsed());
            println!("Checking axis {axis}, offset {offset:.2}");
            let indices = cut.mesh().indices();
            let vertices = cut.mesh().vertices();

            for (triangle_index, t) in indices.chunks_exact(3).enumerate() {
                for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    if (a == 2279 && b == 2281) || (a == 2281 && b == 2279) {
                        let part = cut.ranges().iter().find(|r| {
                            triangle_index >= r.first_triangle
                                && triangle_index < r.first_triangle + r.triangle_count
                        });

                        println!(
                            "BAD EDGE: triangle={triangle_index}, edge=({a},{b}), part={:?}",
                            part.map(|r| r.part_index)
                        );
                        println!("  triangle indices: {t:?}");
                        println!(
                            "  positions: {:?}, {:?}, {:?}",
                            vertices[t[0] as usize],
                            vertices[t[1] as usize],
                            vertices[t[2] as usize]
                        );
                    }
                }
            }
            assert_closed(cut.mesh().indices());
            cuts += 1;
            offset += step;
        }
    }
    println!("{cuts} cuts, slowest {slowest:?}");
    if !cfg!(debug_assertions) {
        assert!(slowest.as_millis() <= 100, "slowest cut {slowest:?}");
    }
}

#[test]
fn the_named_cuts_show_all_four_chambers() {
    let (s, g) = load();
    for view in s.views().iter().filter(|v| v.section().is_some()) {
        let sec = view.section().unwrap();
        let (mut n, mut p) = ([0.0; 3], [0.0; 3]);
        n[sec.axis.index()] = if sec.flip { -1.0 } else { 1.0 };
        p[sec.axis.index()] = sec.offset;
        let cut = section_geometry(&g, &plane(p, n))
            .unwrap_or_else(|e| panic!("view {}: {e}", view.id()));
        assert_closed(cut.mesh().indices());
        let names: HashSet<&str> = cut.ranges().iter().map(|r| id(&s, r.part_index)).collect();
        if view.id() == "four-chamber" {
            for part in [
                "left-ventricle-cavity",
                "right-ventricle-cavity",
                "left-atrium-cavity",
                "right-atrium-cavity",
            ] {
                assert!(names.contains(part), "the four-chamber cut misses {part}");
            }
        }
        if view.id() == "short-axis" {
            for part in [
                "left-ventricle",
                "right-ventricle",
                "left-ventricle-cavity",
                "right-ventricle-cavity",
            ] {
                assert!(names.contains(part), "the short-axis cut misses {part}");
            }
        }
    }
}
