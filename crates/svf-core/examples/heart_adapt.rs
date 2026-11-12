//! B2 step 5e: adapts a Rodero heart (case 01) into a light structure: a
//! `.svfmesh` container and a `.ron` structure with provenance
//! (`docs/heart-structure.md`, sections 3, 6 to 8, 11 and 19).
//!
//! Usage:
//! `cargo run -p svf-core --release --example heart_adapt -- <01-svf-order.vtk> --out-dir <dir> [--deviation 0.4] [--inspect]`
//!
//! `--inspect` prints the heart frame and the position of every tag group, then exits.

use std::{collections::BTreeMap, env, fs, path::PathBuf, process, time::Instant};

use svf_core::{
    dataset::DatasetGeometry,
    mesh_simplify::{simplify_within, SimplifyOptions},
    region_surface::{
        is_closed_manifold, relabel_islands, repair_pinches, signed_volume, LabelledFace, LabelledSurface, P3,
    },
    structure_mesh::{MeshBody, MeshContainer},
    sampling::SampleValue,
    unstructured::UnstructuredGrid3,
    vtk_legacy::parse_legacy_ascii_unstructured_grid,
};

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("heart_adapt failed: {message}");
    process::exit(1);
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: P3, b: P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: P3) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: P3) -> P3 {
    let l = norm(a);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// The heart frame: a rigid transform from scanner coordinates (section 3).
#[derive(Clone, Copy, Debug)]
struct Frame {
    origin: P3,
    /// Rows are the new X, Y, Z axes in scanner coordinates.
    axes: [P3; 3],
}

impl Frame {
    fn apply(&self, p: P3) -> P3 {
        let d = sub(p, self.origin);
        [dot(self.axes[0], d), dot(self.axes[1], d), dot(self.axes[2], d)]
    }
    /// Row-major 3x4 `[R | t]` with `p' = R p + t`.
    fn matrix(&self) -> [f64; 12] {
        let t = [-dot(self.axes[0], self.origin), -dot(self.axes[1], self.origin), -dot(self.axes[2], self.origin)];
        let a = self.axes;
        [a[0][0], a[0][1], a[0][2], t[0], a[1][0], a[1][1], a[1][2], t[1], a[2][0], a[2][1], a[2][2], t[2]]
    }
}

/// Mean of the distinct points of the cells with these tags.
fn tag_centroid(grid: &UnstructuredGrid3, tags: &[u32], wanted: &[u32]) -> P3 {
    let mut seen = vec![false; grid.points().len()];
    let (mut sum, mut n) = ([0.0; 3], 0.0);
    for (c, &t) in tags.iter().enumerate() {
        if !wanted.contains(&t) {
            continue;
        }
        for &i in grid.cell_point_indices(c).unwrap() {
            if !std::mem::replace(&mut seen[i as usize], true) {
                let p = grid.points()[i as usize].to_array();
                sum = [sum[0] + p[0], sum[1] + p[1], sum[2] + p[2]];
                n += 1.0;
            }
        }
    }
    if n == 0.0 {
        fail(format!("no cells with tags {wanted:?}"));
    }
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

fn heart_frame(grid: &UnstructuredGrid3, tags: &[u32]) -> Frame {
    let mitral = tag_centroid(grid, tags, &[7]);
    let aortic = tag_centroid(grid, tags, &[9]);
    let base = [(mitral[0] + aortic[0]) / 2.0, (mitral[1] + aortic[1]) / 2.0, (mitral[2] + aortic[2]) / 2.0];
    // Apex: the left ventricle point farthest from the base.
    let mut apex = base;
    let mut far = 0.0;
    for (c, &t) in tags.iter().enumerate() {
        if t != 1 {
            continue;
        }
        for &i in grid.cell_point_indices(c).unwrap() {
            let p = grid.points()[i as usize].to_array();
            let d = norm(sub(p, base));
            if d > far {
                far = d;
                apex = p;
            }
        }
    }
    let y = unit(sub(base, apex));
    let lv = tag_centroid(grid, tags, &[1]);
    let rv = tag_centroid(grid, tags, &[2]);
    let across = sub(rv, lv);
    let x = unit(sub(across, [y[0] * dot(across, y), y[1] * dot(across, y), y[2] * dot(across, y)]));
    let z = cross(x, y);
    let planes: Vec<P3> = [7, 8, 9, 10].iter().map(|&t| tag_centroid(grid, tags, &[t])).collect();
    let origin = [
        planes.iter().map(|p| p[0]).sum::<f64>() / 4.0,
        planes.iter().map(|p| p[1]).sum::<f64>() / 4.0,
        planes.iter().map(|p| p[2]).sum::<f64>() / 4.0,
    ];
    Frame { origin, axes: [x, y, z] }
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("Usage: heart_adapt <01-svf-order.vtk> --out-dir <dir> [--deviation 0.4] [--inspect]");
        process::exit(2);
    };
    let (mut out_dir, mut deviation, mut inspect) = (None::<PathBuf>, 0.4f64, false);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--out-dir" => out_dir = Some(PathBuf::from(args.next().unwrap_or_else(|| fail("--out-dir needs a path")))),
            "--deviation" => {
                deviation = args.next().and_then(|v| v.parse().ok()).unwrap_or_else(|| fail("--deviation needs a number"));
            }
            "--inspect" => inspect = true,
            other => fail(format!("unknown argument {other}")),
        }
    }

    let clock = Instant::now();
    let input = fs::read_to_string(&path).unwrap_or_else(|e| fail(format!("read failed: {e}")));
    let dataset =
        parse_legacy_ascii_unstructured_grid(&input).unwrap_or_else(|e| fail(format!("parse failed: {e}")));
    drop(input);
    let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { fail("not an unstructured grid") };
    let tags: Vec<u32> = dataset
        .scalar_field("ID")
        .unwrap_or_else(|| fail("missing scalar field ID"))
        .values()
        .iter()
        .map(|v| match *v {
            SampleValue::Finite(x) if x >= 1.0 && x.fract() == 0.0 => x as u32,
            other => fail(format!("tag {other:?} is not a positive integer")),
        })
        .collect();
    println!("Read and parse: {:.1} s | {} cells", clock.elapsed().as_secs_f64(), tags.len());
    let (tags, fixes) = relabel_islands(grid, &tags, 8).unwrap_or_else(|e| fail(e));
    println!("Islands relabelled: {}", fixes.len());

    let frame = heart_frame(grid, &tags);
    println!("Frame origin (scanner mm): {:.2?}", frame.origin);
    println!("Frame axes X {:.4?} Y {:.4?} Z {:.4?}", frame.axes[0], frame.axes[1], frame.axes[2]);
    println!("Tag centroids in the heart frame (mm):");
    let mut by_tag: BTreeMap<u32, P3> = BTreeMap::new();
    for t in 1..=24 {
        by_tag.insert(t, frame.apply(tag_centroid(grid, &tags, &[t])));
        println!("  tag {t:2}: {:8.1} {:8.1} {:8.1}", by_tag[&t][0], by_tag[&t][1], by_tag[&t][2]);
    }
    if inspect {
        return;
    }
    let out_dir = out_dir.unwrap_or_else(|| fail("--out-dir is required"));
    adapt(grid, &tags, &frame, deviation, &out_dir, &path);
}

// ------------------------------------------------------------------- parts

struct Spec {
    id: &'static str,
    name: &'static str,
    parent: &'static str,
    description: String,
    colour: (f64, f64, f64),
    /// Cell tags or, for a cavity, a cavity label.
    labels: Vec<u32>,
    source_volume: f64,
}

/// The exact volume of each tag, from the tetrahedra.
fn tag_volumes(grid: &UnstructuredGrid3, tags: &[u32]) -> BTreeMap<u32, f64> {
    let mut out = BTreeMap::new();
    for (c, &tag) in tags.iter().enumerate() {
        let ids = grid.cell_point_indices(c).unwrap();
        let p = [0, 1, 2, 3].map(|k| grid.points()[ids[k] as usize].to_array());
        let v = dot(sub(p[1], p[0]), cross(sub(p[2], p[0]), sub(p[3], p[0]))).abs() / 6.0;
        *out.entry(tag).or_default() += v;
    }
    out
}

type Row = (&'static str, &'static str, &'static str, &'static str, (f64, f64, f64), &'static [u32]);

#[rustfmt::skip]
const TAGGED: &[Row] = &[
    ("left-ventricle", "Left ventricle wall", "chamber-walls",
     "The muscular wall of the left ventricle, the thickest wall of the heart, as segmented from this individual's CT scan. It pumps blood to the body through the aortic valve.",
     (0.78, 0.33, 0.31), &[1]),
    ("right-ventricle", "Right ventricle wall", "chamber-walls",
     "The wall of the right ventricle, thinner than the left, lying in front of it. It pumps blood to the lungs through the pulmonary valve.",
     (0.82, 0.56, 0.52), &[2]),
    ("left-atrium", "Left atrium wall", "chamber-walls",
     "The wall of the left atrium, which receives blood from the lungs through the pulmonary veins and passes it to the left ventricle.",
     (0.86, 0.55, 0.56), &[3]),
    ("right-atrium", "Right atrium wall", "chamber-walls",
     "The wall of the right atrium, which receives blood from the body through the venae cavae and passes it to the right ventricle.",
     (0.88, 0.66, 0.62), &[4]),
    ("aorta", "Aorta (wall near the heart)", "great-vessels",
     "The first part of the aorta where it leaves the left ventricle. The data holds the vessel wall near the heart only; its lumen is not modelled.",
     (0.86, 0.24, 0.20), &[5]),
    ("pulmonary-artery", "Pulmonary artery (wall near the heart)", "great-vessels",
     "The first part of the pulmonary artery where it leaves the right ventricle. The data holds the vessel wall near the heart only; its lumen is not modelled.",
     (0.36, 0.46, 0.82), &[6]),
    ("mitral-valve-plane", "Mitral valve plane", "valve-planes",
     "A one-element-thick plane that closes the opening between the left atrium and left ventricle. The data models the valve as a plane, not as leaflets.",
     (0.95, 0.84, 0.30), &[7]),
    ("tricuspid-valve-plane", "Tricuspid valve plane", "valve-planes",
     "A one-element-thick plane that closes the opening between the right atrium and right ventricle. The data models the valve as a plane, not as leaflets.",
     (0.95, 0.80, 0.28), &[8]),
    ("aortic-valve-plane", "Aortic valve plane", "valve-planes",
     "A one-element-thick plane at the exit of the left ventricle into the aorta. The data models the valve as a plane, not as cusps.",
     (0.96, 0.88, 0.40), &[9]),
    ("pulmonary-valve-plane", "Pulmonary valve plane", "valve-planes",
     "A one-element-thick plane at the exit of the right ventricle into the pulmonary artery. The data models the valve as a plane, not as cusps.",
     (0.96, 0.84, 0.36), &[10]),
    // Vessel ends: the inlet tag with the border tag that touches it (by adjacency and
    // position in the heart frame; the source's border names are not in this order).
    ("left-atrial-appendage", "Left atrial appendage (inlet segment)", "vessel-ends",
     "The opening of the left atrial appendage, a pouch off the left atrium, shown as the short inlet segment the data holds.",
     (0.84, 0.46, 0.46), &[11, 18]),
    ("left-superior-pulmonary-vein", "Left superior pulmonary vein (inlet segment)", "vessel-ends",
     "The upper left pulmonary vein, which brings oxygenated blood from the left lung. Shown as the short inlet segment the data holds.",
     (0.82, 0.22, 0.26), &[12, 19]),
    ("left-inferior-pulmonary-vein", "Left inferior pulmonary vein (inlet segment)", "vessel-ends",
     "The lower left pulmonary vein, which brings oxygenated blood from the left lung. Shown as the short inlet segment the data holds.",
     (0.80, 0.20, 0.24), &[13, 20]),
    ("right-inferior-pulmonary-vein", "Right inferior pulmonary vein (inlet segment)", "vessel-ends",
     "The lower right pulmonary vein, which brings oxygenated blood from the right lung. Shown as the short inlet segment the data holds.",
     (0.78, 0.20, 0.24), &[14, 21]),
    ("right-superior-pulmonary-vein", "Right superior pulmonary vein (inlet segment)", "vessel-ends",
     "The upper right pulmonary vein, which brings oxygenated blood from the right lung. Shown as the short inlet segment the data holds.",
     (0.84, 0.24, 0.28), &[15, 22]),
    ("superior-vena-cava", "Superior vena cava (inlet segment)", "vessel-ends",
     "The large vein that returns blood from the upper body to the right atrium. Shown as the short inlet segment the data holds.",
     (0.30, 0.40, 0.80), &[16, 23]),
    ("inferior-vena-cava", "Inferior vena cava (inlet segment)", "vessel-ends",
     "The large vein that returns blood from the lower body to the right atrium. Shown as the short inlet segment the data holds.",
     (0.26, 0.36, 0.76), &[17, 24]),
];

#[rustfmt::skip]
const CAVITIES: &[(&str, &str, u32, &str, (f64, f64, f64))] = &[
    ("left-ventricle-cavity", "Left ventricle cavity", 9,
     "The space inside the left ventricle, shown as a cast: empty in the data, closed by the mitral and aortic valve planes. Red by convention for oxygenated blood.",
     (0.74, 0.10, 0.10)),
    ("right-ventricle-cavity", "Right ventricle cavity", 10,
     "The space inside the right ventricle, shown as a cast: empty in the data, closed by the tricuspid and pulmonary valve planes. Blue by convention for deoxygenated blood.",
     (0.20, 0.30, 0.74)),
    ("left-atrium-cavity", "Left atrium cavity", 11,
     "The space inside the left atrium, shown as a cast, closed by the mitral valve plane and the pulmonary vein and appendage inlets. Red by convention.",
     (0.80, 0.16, 0.16)),
    ("right-atrium-cavity", "Right atrium cavity", 16,
     "The space inside the right atrium, shown as a cast, closed by the tricuspid valve plane and the vena cava inlets. Blue by convention.",
     (0.26, 0.36, 0.80)),
];

fn specs(grid: &UnstructuredGrid3, tags: &[u32], surface: &LabelledSurface) -> Vec<Spec> {
    let vol = tag_volumes(grid, tags);
    let mut out: Vec<Spec> = Vec::new();
    for &(id, name, parent, description, colour, labels) in TAGGED {
        let source_volume = labels.iter().map(|k| vol.get(k).copied().unwrap_or(0.0)).sum();
        out.push(Spec { id, name, parent, description: description.to_owned(), colour, labels: labels.to_vec(), source_volume });
    }
    // Cavities are found by which tags bound them.
    for &(id, name, bound, description, colour) in CAVITIES {
        let c = surface
            .cavities()
            .iter()
            .find(|c| c.bounding_tags.contains(&bound))
            .unwrap_or_else(|| fail(format!("no cavity bounded by tag {bound}")));
        out.push(Spec {
            id,
            name,
            parent: "chambers",
            description: description.to_owned(),
            colour,
            labels: vec![c.label],
            source_volume: c.volume,
        });
    }
    out
}

/// Faces with exactly one side in `labels`, wound outward from the set.
fn part_triangles(faces: &[LabelledFace], labels: &[u32]) -> Vec<[u32; 3]> {
    faces
        .iter()
        .filter_map(|f| match (labels.contains(&f.front), labels.contains(&f.back)) {
            (true, false) => Some(f.triangle),
            (false, true) => Some([f.triangle[0], f.triangle[2], f.triangle[1]]),
            _ => None,
        })
        .collect()
}

// ------------------------------------------------------------------- adapt

fn adapt(grid: &UnstructuredGrid3, tags: &[u32], frame: &Frame, deviation: f64, out_dir: &PathBuf, source_path: &str) {
    let clock = Instant::now();
    let surface = LabelledSurface::build(grid, tags).unwrap_or_else(|e| fail(e));
    let patches = surface.patches();
    println!(
        "Labelled surface: {} faces, {} patches, {} cavities",
        surface.faces().len(),
        patches.len(),
        surface.cavities().len()
    );
    let specs = specs(grid, tags, &surface);

    // Vertices shared by more than one patch are pinned in every patch.
    let mut patch_count = vec![0u8; surface.points().len()];
    for patch in &patches {
        let mut vs: Vec<u32> = patch.faces.iter().flat_map(|&f| surface.faces()[f].triangle).collect();
        vs.sort_unstable();
        vs.dedup();
        for v in vs {
            patch_count[v as usize] = patch_count[v as usize].saturating_add(1);
        }
    }

    let mut points = surface.points().to_vec();
    let mut faces: Vec<LabelledFace> = Vec::with_capacity(surface.faces().len() / 4);
    let (mut simplified, mut kept, mut refused) = (0usize, 0usize, 0usize);
    let (mut before, mut after) = (0usize, 0usize);
    let mut worst = 0.0f64;
    let mut worst_patch = String::new();
    for patch in &patches {
        let tris: Vec<[u32; 3]> = patch.faces.iter().map(|&f| surface.faces()[f].triangle).collect();
        before += tris.len();
        let keep = |faces: &mut Vec<LabelledFace>, tris: &[[u32; 3]]| {
            for &triangle in tris {
                faces.push(LabelledFace { triangle, front: patch.front, back: patch.back });
            }
        };
        if tris.len() < 10 {
            keep(&mut faces, &tris);
            kept += 1;
            after += tris.len();
            continue;
        }
        let mut local: Vec<u32> = tris.iter().flatten().copied().collect();
        local.sort_unstable();
        local.dedup();
        let lp: Vec<P3> = local.iter().map(|&g| surface.points()[g as usize]).collect();
        let lt: Vec<[u32; 3]> =
            tris.iter().map(|t| t.map(|g| local.binary_search(&g).expect("vertex present") as u32)).collect();
        let pinned: Vec<bool> = local.iter().map(|&g| patch_count[g as usize] > 1).collect();
        let options = SimplifyOptions { epsilon: deviation, ..Default::default() };
        match simplify_within(&lp, &lt, &pinned, deviation, 6, &options) {
            Ok(result) => {
                let d = result.report.measured_deviation.unwrap_or(0.0);
                if d > worst {
                    worst = d;
                    worst_patch = format!("{}|{}", patch.front, patch.back);
                }
                for (i, &lv) in result.vertex_map.iter().enumerate() {
                    points[local[lv as usize] as usize] = result.points[i];
                }
                let new_tris: Vec<[u32; 3]> = result
                    .triangles
                    .iter()
                    .map(|t| t.map(|i| local[result.vertex_map[i as usize] as usize]))
                    .collect();
                after += new_tris.len();
                keep(&mut faces, &new_tris);
                simplified += 1;
            }
            Err(error) => {
                println!("  patch {}|{} kept as is: {error}", patch.front, patch.back);
                keep(&mut faces, &tris);
                after += tris.len();
                refused += 1;
            }
        }
    }
    println!(
        "Patches: {simplified} simplified, {kept} small and kept, {refused} refused | triangles {before} -> {after} | worst measured deviation {worst:.3} mm (patch {worst_patch}) | {:.1} s",
        clock.elapsed().as_secs_f64()
    );

    println!("\npart | triangles | vertices | volume mm3 | source mm3 | rel diff | closed | repair (nm edges, alternate, split)");
    let mut container = MeshContainer::default();
    let mut ok = true;
    let mut total = 0usize;
    let mut volumes: Vec<(String, f64)> = Vec::new();
    for spec in &specs {
        let tris = part_triangles(&faces, &spec.labels);
        let (mesh, report) = repair_pinches(&points, &tris).unwrap_or_else(|e| fail(format!("{}: {e}", spec.id)));
        let volume = signed_volume(&mesh.points, &mesh.triangles);
        let diff = (volume - spec.source_volume) / spec.source_volume;
        let closed = is_closed_manifold(&mesh.triangles);
        if !closed || diff.abs() > 0.01 {
            ok = false;
        }
        total += mesh.triangles.len();
        println!(
            "{} | {} | {} | {volume:.1} | {:.1} | {diff:+.4} | {closed} | ({}, {}, {})",
            spec.id,
            mesh.triangles.len(),
            mesh.points.len(),
            spec.source_volume,
            report.nonmanifold_edges,
            report.alternate_pairings,
            report.split_vertices
        );
        volumes.push((spec.id.to_owned(), spec.source_volume));
        container.bodies.push(MeshBody {
            name: spec.id.to_owned(),
            points: mesh.points.iter().map(|&p| frame.apply(p)).collect(),
            triangles: mesh.triangles,
        });
    }
    println!("\nParts: {} | triangles in all {total}", specs.len());
    println!("Every part closed with volume within 1 percent: {}", if ok { "PASS" } else { "FAIL" });

    let bytes = container.to_bytes().unwrap_or_else(|e| fail(e));
    println!("Container: {} bytes ({:.2} MB)", bytes.len(), bytes.len() as f64 / 1e6);
    fs::create_dir_all(out_dir).unwrap_or_else(|e| fail(e));
    fs::write(out_dir.join("heart-case01.svfmesh"), &bytes).unwrap_or_else(|e| fail(e));
    let ron = structure_ron(&specs, &volumes, frame, deviation, source_path);
    fs::write(out_dir.join("heart-case01.ron"), ron).unwrap_or_else(|e| fail(e));
    println!("Wrote heart-case01.svfmesh and heart-case01.ron to {}", out_dir.display());
}

fn git_commit() -> String {
    let run = |args: &[&str]| {
        process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    let head = run(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());
    let dirty = run(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    if dirty {
        format!("{head}+uncommitted")
    } else {
        head
    }
}

/// SHA-256 of the original `01.vtk` that sits next to the reordered file (Windows `certutil`).
fn sha256_of_source(path: &str) -> String {
    let original = std::path::Path::new(path).with_file_name("01.vtk");
    match process::Command::new("certutil").arg("-hashfile").arg(&original).arg("SHA256").output() {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).lines().nth(1).map(|l| l.replace(' ', "")).unwrap_or_default()
        }
        _ => String::new(),
    }
}

fn group(id: &str, name: &str, parent: &str, desc: &str, colour: (f64, f64, f64)) -> String {
    let parent = if parent.is_empty() { "None".to_owned() } else { format!("Some(\"{parent}\")") };
    format!("        Part(id: \"{id}\", name: \"{name}\", parent: {parent}, description: \"{desc}\", colour: {colour:?}),\n")
}

fn structure_ron(specs: &[Spec], volumes: &[(String, f64)], frame: &Frame, deviation: f64, source_path: &str) -> String {
    let mut s = String::new();
    s.push_str("// Human heart, adapted from Rodero et al. (2021) case 01. CC BY 4.0, modified: see the provenance block.\n");
    s.push_str("// Heart frame: Y apex to base, X from the left toward the right ventricle, Z = X cross Y. Millimetres.\n");
    s.push_str("// Generated by crates/svf-core/examples/heart_adapt.rs; do not edit the Mesh parts by hand.\n");
    s.push_str("Structure(\n    version: 1,\n    name: \"Human heart (one individual, end-diastole)\",\n    units: \"mm\",\n");
    s.push_str("    reference: \"Rodero C. et al. (2021), Linking statistical shape models and simulated function in the healthy adult human heart, PLoS Comput Biol 17(4): e1008851; data: https://doi.org/10.5281/zenodo.4590294 (CC BY 4.0), case 01. One individual's CT-derived anatomy, not a canonical heart; valves are planes and chambers are the empty space between the walls.\",\n");
    s.push_str("    circle_segments: 24,\n    parts: [\n");
    s.push_str(&group("heart", "Human heart", "", "A human heart reconstructed from one healthy adult's CT scan: four chambers, the great vessels near the heart, and the four valve positions.", (0.8, 0.4, 0.4)));
    s.push_str(&group("chamber-walls", "Chamber walls", "heart", "The muscular walls of the four chambers.", (0.8, 0.4, 0.4)));
    s.push_str(&group("great-vessels", "Great vessels and vein ends", "heart", "The arteries leaving the heart and the veins entering it, as short segments next to the heart.", (0.6, 0.4, 0.6)));
    s.push_str(&group("vessel-ends", "Vein ends", "great-vessels", "The pulmonary veins, venae cavae and left atrial appendage opening into the atria.", (0.6, 0.3, 0.5)));
    s.push_str(&group("valve-planes", "Valve planes", "heart", "The four valves, each modelled in this data as a flat plane across its opening.", (0.95, 0.85, 0.35)));
    s.push_str(&group("chambers", "Chamber cavities", "heart", "The blood-filled space inside each chamber, shown as a cast. The data has no blood-pool mesh: these are the empty spaces enclosed by the walls and valve planes.", (0.5, 0.3, 0.6)));
    for p in specs {
        let c = p.colour;
        s.push_str(&format!(
            "        Part(id: \"{}\", name: \"{}\", parent: Some(\"{}\"), description: \"{}\", colour: ({:.2}, {:.2}, {:.2}), geometry: Some(Mesh(file: \"heart-case01.svfmesh\", body: \"{}\"))),\n",
            p.id, p.name, p.parent, p.description, c.0, c.1, c.2, p.id
        ));
    }
    s.push_str("    ],\n    layers: [\n");
    s.push_str("        Layer(id: \"vessels\", name: \"Great vessels and vein ends\", parts: [\"great-vessels\"]),\n");
    s.push_str("        Layer(id: \"atria\", name: \"Atria\", parts: [\"left-atrium\", \"right-atrium\"]),\n");
    s.push_str("        Layer(id: \"right-ventricle\", name: \"Right ventricle wall\", parts: [\"right-ventricle\"]),\n");
    s.push_str("        Layer(id: \"left-ventricle\", name: \"Left ventricle wall\", parts: [\"left-ventricle\"]),\n");
    s.push_str("    ],\n    relations: [\n");
    for (from, kind, to) in [
        ("right-atrium-cavity", "OpensInto", "right-ventricle-cavity"),
        ("left-atrium-cavity", "OpensInto", "left-ventricle-cavity"),
        ("right-ventricle-cavity", "OpensInto", "pulmonary-artery"),
        ("left-ventricle-cavity", "OpensInto", "aorta"),
        ("superior-vena-cava", "OpensInto", "right-atrium-cavity"),
        ("inferior-vena-cava", "OpensInto", "right-atrium-cavity"),
        ("left-superior-pulmonary-vein", "OpensInto", "left-atrium-cavity"),
        ("left-inferior-pulmonary-vein", "OpensInto", "left-atrium-cavity"),
        ("right-superior-pulmonary-vein", "OpensInto", "left-atrium-cavity"),
        ("right-inferior-pulmonary-vein", "OpensInto", "left-atrium-cavity"),
        ("left-atrial-appendage", "OpensInto", "left-atrium-cavity"),
        ("left-ventricle", "Contains", "left-ventricle-cavity"),
        ("right-ventricle", "Contains", "right-ventricle-cavity"),
        ("left-atrium", "Contains", "left-atrium-cavity"),
        ("right-atrium", "Contains", "right-atrium-cavity"),
    ] {
        s.push_str(&format!("        Relation(from: \"{from}\", kind: {kind}, to: \"{to}\"),\n"));
    }
    s.push_str("    ],\n    views: [\n");
    s.push_str("        View(id: \"anterior\", name: \"The whole heart from the front\", camera: Some((yaw_deg: 0.0, pitch_deg: 0.0, zoom: 1.0)), peel: 0),\n");
    s.push_str("        View(id: \"four-chamber\", name: \"Four-chamber cut\", camera: Some((yaw_deg: 0.0, pitch_deg: 0.0, zoom: 1.0)), peel: 0, section: Some((axis: Z, offset: 10.0, flip: false))),\n");
    s.push_str("        View(id: \"short-axis\", name: \"Short-axis cut through the ventricles\", camera: Some((yaw_deg: 0.0, pitch_deg: 60.0, zoom: 1.0)), peel: 0, section: Some((axis: Y, offset: -20.0, flip: false))),\n");
    s.push_str("        View(id: \"walls-peeled\", name: \"Walls removed: the chambers and valve planes\", camera: Some((yaw_deg: 0.0, pitch_deg: 0.0, zoom: 1.0)), peel: 4),\n");
    s.push_str("        View(id: \"valve-planes\", name: \"The four valve planes\", camera: Some((yaw_deg: 0.0, pitch_deg: 30.0, zoom: 1.0)), show: [\"valve-planes\"], peel: 0),\n");
    s.push_str("        View(id: \"great-vessels\", name: \"Great vessels and vein ends\", camera: Some((yaw_deg: 0.0, pitch_deg: 0.0, zoom: 1.0)), show: [\"great-vessels\"], peel: 0),\n");
    s.push_str("    ],\n");
    let vols: Vec<String> = volumes.iter().map(|(n, v)| format!("(\"{n}\", {v:.3})")).collect();
    let m = frame.matrix().map(|v| format!("{v:.9}")).join(", ");
    let sha = sha256_of_source(source_path);
    s.push_str(&format!(
        "    provenance: Some(Provenance(\n        dataset: \"Rodero et al. 2021, healthy four-chamber heart meshes, case 01\",\n        doi: \"10.5281/zenodo.4590294\",\n        licence: \"CC BY 4.0\",\n        attribution: \"Rodero C. et al. (2021), PLoS Comput Biol 17(4): e1008851, data at https://doi.org/10.5281/zenodo.4590294, licensed CC BY 4.0. Modified: re-framed, small stray regions relabelled, surfaces simplified.\",\n        source_sha256: \"{sha}\",\n        tool: \"svf-core example heart_adapt\",\n        tool_commit: \"{}\",\n        parameters: [(\"deviation_mm\", \"{deviation}\"), (\"island_limit_cells\", \"8\"), (\"min_patch_faces_simplified\", \"10\")],\n        frame_transform: Some(({m})),\n        body_volumes_mm3: [{}],\n        modified: true,\n    )),\n)\n",
        git_commit(),
        vols.join(", ")
    ));
    s
}
