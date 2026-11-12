//! B2 step 5a audit of a Rodero heart (`docs/heart-structure.md`, sections 5 to 7).
//!
//! Reads the working copy of a case (cell tags in the scalar field `ID`) and
//! answers the questions the adaptation design rests on, without building any
//! production geometry:
//!
//! * Is each tag region's boundary a closed, consistently oriented 2-manifold?
//!   (non-manifold edges and vertices, orientation, Euler characteristic)
//! * Does the geometric orientation rule (against the tetrahedron's opposite
//!   vertex) reproduce the exact tag volume (sum of tetrahedron volumes)?
//! * How many closed components does the exterior surface have, and which of
//!   them enclose the chambers? Their volumes in millilitres.
//! * How many patches (faces with the same pair of side tags, connected through
//!   manifold edges) are there, how long are the pinned curves, and what does a
//!   given target edge length imply for the triangle budget?
//!
//! Usage: `cargo run -p svf-core --release --example heart_audit -- <01-svf-order.vtk> [--tag-field ID]`

use std::{
    collections::{BTreeMap, HashMap},
    env, fs, process,
    time::Instant,
};

use svf_core::{
    dataset::DatasetGeometry, sampling::SampleValue, unstructured::CellType3,
    vtk_legacy::parse_legacy_ascii_unstructured_grid,
};

type P = [f64; 3];

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("heart_audit failed: {message}");
    process::exit(1);
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: P, b: P) -> P {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Union-find with path halving.
struct Sets(Vec<u32>);
impl Sets {
    fn new(n: usize) -> Self {
        Self((0..n as u32).collect())
    }
    fn find(&mut self, mut x: u32) -> u32 {
        while self.0[x as usize] != x {
            self.0[x as usize] = self.0[self.0[x as usize] as usize];
            x = self.0[x as usize];
        }
        x
    }
    fn union(&mut self, a: u32, b: u32) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[a as usize] = b;
        }
    }
    fn count(&mut self) -> usize {
        (0..self.0.len() as u32).filter(|&i| self.find(i) == i).count()
    }
}

struct Rec {
    cell0: u32,
    opposite0: u32,
    cell1: u32,
    count: u8,
}

/// One face of the labelled complex: oriented outward from side A.
struct Face {
    tri: [u32; 3],
    tag_a: u32,
    /// 0 means void.
    tag_b: u32,
}

struct RegionReport {
    faces: usize,
    edges: usize,
    vertices: usize,
    nonmanifold_edges: usize,
    /// Non-manifold edges whose forward and backward uses differ (cannot be paired).
    unbalanced_edges: usize,
    inconsistent_edges: usize,
    nonmanifold_vertices: usize,
    components: usize,
    /// (faces, signed volume) per connected component, largest first.
    component_sizes: Vec<(usize, f64)>,
    area: f64,
    volume: f64,
}

fn edge_key(a: u32, b: u32) -> ((u32, u32), usize) {
    if a < b {
        ((a, b), 0)
    } else {
        ((b, a), 1)
    }
}

fn analyse(tris: &[[u32; 3]], pts: &[P]) -> RegionReport {
    let mut edges: HashMap<(u32, u32), ([u16; 2], u32)> = HashMap::new();
    let mut sets = Sets::new(tris.len());
    let (mut area, mut volume) = (0.0, 0.0);
    let mut by_vertex: HashMap<u32, Vec<u32>> = HashMap::new();
    for (i, t) in tris.iter().enumerate() {
        let (a, b, c) = (pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]);
        let n = cross(sub(b, a), sub(c, a));
        area += 0.5 * dot(n, n).sqrt();
        volume += dot(a, cross(b, c)) / 6.0;
        for k in 0..3 {
            let (u, v) = (t[k], t[(k + 1) % 3]);
            let (key, dir) = edge_key(u, v);
            let entry = edges.entry(key).or_insert(([0, 0], i as u32));
            entry.0[dir] += 1;
            if entry.1 != i as u32 {
                sets.union(entry.1, i as u32);
            }
            by_vertex.entry(u).or_default().push(i as u32);
        }
    }
    let (mut nonmanifold_edges, mut inconsistent_edges, mut unbalanced_edges) = (0, 0, 0);
    for (counts, _) in edges.values() {
        match (counts[0], counts[1]) {
            (1, 1) => {}
            (a, b) if a + b == 2 => inconsistent_edges += 1,
            (a, b) => {
                nonmanifold_edges += 1;
                if a != b {
                    unbalanced_edges += 1;
                }
            }
        }
    }
    let mut sizes: HashMap<u32, (usize, f64)> = HashMap::new();
    for (i, t) in tris.iter().enumerate() {
        let root = sets.find(i as u32);
        let e = sizes.entry(root).or_default();
        e.0 += 1;
        e.1 += dot(pts[t[0] as usize], cross(pts[t[1] as usize], pts[t[2] as usize])) / 6.0;
    }
    let mut component_sizes: Vec<(usize, f64)> = sizes.into_values().collect();
    component_sizes.sort_by(|a, b| b.0.cmp(&a.0));
    let mut nonmanifold_vertices = 0;
    for (&v, faces) in &by_vertex {
        let mut local = Sets::new(faces.len());
        let mut owner: HashMap<u32, usize> = HashMap::new();
        for (slot, &f) in faces.iter().enumerate() {
            for &w in &tris[f as usize] {
                if w == v {
                    continue;
                }
                match owner.get(&w) {
                    Some(&other) => local.union(other as u32, slot as u32),
                    None => {
                        owner.insert(w, slot);
                    }
                }
            }
        }
        if local.count() > 1 {
            nonmanifold_vertices += 1;
        }
    }
    RegionReport {
        faces: tris.len(),
        edges: edges.len(),
        vertices: by_vertex.len(),
        nonmanifold_edges,
        unbalanced_edges,
        inconsistent_edges,
        nonmanifold_vertices,
        components: component_sizes.len(),
        component_sizes,
        area,
        volume,
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("Usage: heart_audit <01-svf-order.vtk> [--tag-field ID]");
        process::exit(2);
    };
    let mut tag_field = "ID".to_owned();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--tag-field" => tag_field = args.next().unwrap_or_else(|| fail("--tag-field needs a name")),
            other => fail(format!("unknown argument {other}")),
        }
    }

    let clock = Instant::now();
    let input = fs::read_to_string(&path).unwrap_or_else(|e| fail(format!("read failed: {e}")));
    let dataset =
        parse_legacy_ascii_unstructured_grid(&input).unwrap_or_else(|e| fail(format!("parse failed: {e}")));
    drop(input);
    let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else {
        fail("not an unstructured grid");
    };
    let pts: Vec<P> = grid.points().iter().map(|p| p.to_array()).collect();
    let tags: Vec<u32> = dataset
        .scalar_field(&tag_field)
        .unwrap_or_else(|| fail(format!("missing scalar field {tag_field}")))
        .values()
        .iter()
        .map(|v| match *v {
            SampleValue::Finite(x) if x >= 1.0 && x.fract() == 0.0 => x as u32,
            other => fail(format!("tag {other:?} is not a positive integer")),
        })
        .collect();
    let mut tets: Vec<[u32; 4]> = Vec::with_capacity(grid.cell_count());
    for c in 0..grid.cell_count() {
        if grid.cell_type(c) != Some(CellType3::Tetrahedron4) {
            fail(format!("cell {c} is not a tetrahedron"));
        }
        let ids = grid.cell_point_indices(c).unwrap();
        tets.push([ids[0], ids[1], ids[2], ids[3]]);
    }
    if tags.len() != tets.len() {
        fail("tag count differs from cell count");
    }
    println!("File: {path}");
    println!("Read and parse: {:.1} s | points {} | tetrahedra {}", clock.elapsed().as_secs_f64(), pts.len(), tets.len());

    // Exact tag volumes from the tetrahedra, and the face map.
    let mut tet_volume: BTreeMap<u32, f64> = BTreeMap::new();
    let mut tet_count: BTreeMap<u32, usize> = BTreeMap::new();
    let mut degenerate_tets = 0usize;
    let clock = Instant::now();
    let mut faces: HashMap<[u32; 3], Rec> = HashMap::with_capacity(tets.len() * 2);
    for (c, t) in tets.iter().enumerate() {
        let p = t.map(|i| pts[i as usize]);
        let v = dot(sub(p[1], p[0]), cross(sub(p[2], p[0]), sub(p[3], p[0]))).abs() / 6.0;
        if v <= 1e-12 {
            degenerate_tets += 1;
        }
        *tet_volume.entry(tags[c]).or_default() += v;
        *tet_count.entry(tags[c]).or_default() += 1;
        for opp in 0..4 {
            let mut key: Vec<u32> = (0..4).filter(|&k| k != opp).map(|k| t[k]).collect();
            key.sort_unstable();
            let key = [key[0], key[1], key[2]];
            match faces.get_mut(&key) {
                None => {
                    faces.insert(key, Rec { cell0: c as u32, opposite0: t[opp], cell1: u32::MAX, count: 1 });
                }
                Some(rec) => {
                    if rec.count == 1 {
                        rec.cell1 = c as u32;
                    }
                    rec.count = rec.count.saturating_add(1);
                }
            }
        }
    }
    println!("Face map: {} unique faces in {:.1} s | degenerate tetrahedra {degenerate_tets}", faces.len(), clock.elapsed().as_secs_f64());

    // Labelled complex: boundary faces and faces between different tags.
    let (mut volume_nonmanifold, mut ambiguous) = (0usize, 0usize);
    let mut complex: Vec<Face> = Vec::new();
    for (key, rec) in &faces {
        if rec.count > 2 {
            volume_nonmanifold += 1;
            continue;
        }
        let tag_a = tags[rec.cell0 as usize];
        let tag_b = if rec.count == 2 { tags[rec.cell1 as usize] } else { 0 };
        if rec.count == 2 && tag_a == tag_b {
            continue;
        }
        let [a, b, c] = *key;
        let (pa, pb, pc) = (pts[a as usize], pts[b as usize], pts[c as usize]);
        let toward_opposite = dot(cross(sub(pb, pa), sub(pc, pa)), sub(pts[rec.opposite0 as usize], pa));
        if toward_opposite.abs() <= 1e-12 {
            ambiguous += 1;
        }
        // Outward from side A means away from the opposite vertex of cell0.
        let tri = if toward_opposite > 0.0 { [a, c, b] } else { [a, b, c] };
        complex.push(Face { tri, tag_a, tag_b });
    }
    drop(faces);
    println!(
        "Labelled complex: {} faces | faces used by more than two tetrahedra {volume_nonmanifold} | orientation ambiguous {ambiguous}",
        complex.len()
    );
    let exterior = complex.iter().filter(|f| f.tag_b == 0).count();
    println!("  exterior (one tetrahedron) {exterior} | interfaces between tags {}", complex.len() - exterior);

    // Per tag region.
    println!("\n== Tag regions (boundary of the cells with that tag) ==");
    println!("tag | cells | tet volume mm3 | faces | edges | verts | comps | chi | nm-edges | unbal | bad-orient | nm-verts | surface volume mm3 | rel diff");
    let mut unbalanced_total = 0;
    let mut nm_edge_total = 0;
    let mut nm_vertex_total = 0;
    let mut multi: Vec<(u32, Vec<(usize, f64)>)> = Vec::new();
    let all_tags: Vec<u32> = tet_count.keys().copied().collect();
    let mut gate_regions_ok = true;
    for &tag in &all_tags {
        let tris: Vec<[u32; 3]> = complex
            .iter()
            .filter_map(|f| {
                if f.tag_a == tag {
                    Some(f.tri)
                } else if f.tag_b == tag {
                    Some([f.tri[0], f.tri[2], f.tri[1]])
                } else {
                    None
                }
            })
            .collect();
        let r = analyse(&tris, &pts);
        let chi = r.vertices as i64 - r.edges as i64 + r.faces as i64;
        let exact = tet_volume[&tag];
        let diff = (r.volume - exact) / exact;
        if r.nonmanifold_edges > 0 || r.inconsistent_edges > 0 || r.nonmanifold_vertices > 0 || diff.abs() > 1e-6 {
            gate_regions_ok = false;
        }
        println!(
            "{tag:>3} | {:>7} | {:>14.1} | {:>6} | {:>6} | {:>6} | {:>5} | {chi:>4} | {:>8} | {:>5} | {:>10} | {:>8} | {:>18.1} | {:+.2e}",
            tet_count[&tag], exact, r.faces, r.edges, r.vertices, r.components, r.nonmanifold_edges,
            r.unbalanced_edges, r.inconsistent_edges, r.nonmanifold_vertices, r.volume, diff
        );
        unbalanced_total += r.unbalanced_edges;
        nm_edge_total += r.nonmanifold_edges;
        nm_vertex_total += r.nonmanifold_vertices;
        if r.components > 1 {
            multi.push((tag, r.component_sizes.clone()));
        }
    }

    println!(
        "non-manifold edges over all regions {nm_edge_total} (unbalanced, so not pairable: {unbalanced_total}) | non-manifold vertices {nm_vertex_total}"
    );
    for (tag, sizes) in &multi {
        let list: Vec<String> =
            sizes.iter().take(8).map(|(n, v)| format!("{n} faces / {:.1} mm3", v)).collect();
        println!("tag {tag} has {} components: {}{}", sizes.len(), list.join("; "), if sizes.len() > 8 { "; ..." } else { "" });
    }

    // Exterior components: the outer surface and the enclosed cavities.
    println!("\n== Exterior surface components (faces with one tetrahedron, wound outward from the tetrahedra) ==");
    let ext: Vec<&Face> = complex.iter().filter(|f| f.tag_b == 0).collect();
    let mut sets = Sets::new(ext.len());
    let mut first: HashMap<(u32, u32), u32> = HashMap::new();
    let mut edge_faces: HashMap<(u32, u32), u32> = HashMap::new();
    for (i, f) in ext.iter().enumerate() {
        for k in 0..3 {
            let (key, _) = edge_key(f.tri[k], f.tri[(k + 1) % 3]);
            *edge_faces.entry(key).or_default() += 1;
            match first.get(&key) {
                Some(&other) => sets.union(other, i as u32),
                None => {
                    first.insert(key, i as u32);
                }
            }
        }
    }
    let nonmanifold_exterior_edges = edge_faces.values().filter(|&&n| n != 2).count();
    println!("exterior edges not used by exactly two exterior faces: {nonmanifold_exterior_edges}");
    struct Comp {
        faces: usize,
        volume: f64,
        area: f64,
        tags: BTreeMap<u32, usize>,
        low: P,
        high: P,
    }
    let mut comps: HashMap<u32, Comp> = HashMap::new();
    for (i, f) in ext.iter().enumerate() {
        let root = sets.find(i as u32);
        let c = comps.entry(root).or_insert(Comp {
            faces: 0,
            volume: 0.0,
            area: 0.0,
            tags: BTreeMap::new(),
            low: [f64::MAX; 3],
            high: [f64::MIN; 3],
        });
        let (a, b, d) = (pts[f.tri[0] as usize], pts[f.tri[1] as usize], pts[f.tri[2] as usize]);
        c.faces += 1;
        c.volume += dot(a, cross(b, d)) / 6.0;
        let n = cross(sub(b, a), sub(d, a));
        c.area += 0.5 * dot(n, n).sqrt();
        *c.tags.entry(f.tag_a).or_default() += 1;
        for p in [a, b, d] {
            for k in 0..3 {
                c.low[k] = c.low[k].min(p[k]);
                c.high[k] = c.high[k].max(p[k]);
            }
        }
    }
    let mut list: Vec<&Comp> = comps.values().collect();
    list.sort_by(|a, b| b.faces.cmp(&a.faces));
    let mut inner = 0;
    println!("components: {}", list.len());
    for (n, c) in list.iter().enumerate() {
        let kind = if c.volume < 0.0 { "INNER (encloses a void)" } else { "outer" };
        if c.volume < 0.0 {
            inner += 1;
        }
        let tags: Vec<String> = c.tags.iter().map(|(t, k)| format!("{t}:{k}")).collect();
        println!(
            "  #{n}: {:>7} faces | area {:>9.1} mm2 | volume {:>12.1} mm3 = {:>8.2} mL | {kind} | extent {:.0} x {:.0} x {:.0} mm | tags {}",
            c.faces,
            c.area,
            c.volume,
            c.volume.abs() / 1000.0,
            c.high[0] - c.low[0],
            c.high[1] - c.low[1],
            c.high[2] - c.low[2],
            tags.join(" ")
        );
    }

    // Patches and pinned curves.
    println!("\n== Patches (same pair of side tags, connected through edges shared by exactly two such faces) ==");
    let pair = |f: &Face| (f.tag_a.min(f.tag_b), f.tag_a.max(f.tag_b));
    let mut edges: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for (i, f) in complex.iter().enumerate() {
        for k in 0..3 {
            let (key, _) = edge_key(f.tri[k], f.tri[(k + 1) % 3]);
            edges.entry(key).or_default().push(i as u32);
        }
    }
    let mut patch_sets = Sets::new(complex.len());
    let (mut curve_edges, mut curve_length) = (0usize, 0.0);
    let mut curve_vertices: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for ((u, v), list) in &edges {
        let interior = list.len() == 2 && pair(&complex[list[0] as usize]) == pair(&complex[list[1] as usize]);
        if interior {
            patch_sets.union(list[0], list[1]);
        } else {
            curve_edges += 1;
            curve_vertices.insert(*u);
            curve_vertices.insert(*v);
            curve_length += dot(sub(pts[*u as usize], pts[*v as usize]), sub(pts[*u as usize], pts[*v as usize])).sqrt();
        }
    }
    let mut pair_stats: BTreeMap<(u32, u32), (usize, f64, std::collections::HashSet<u32>)> = BTreeMap::new();
    let mut total_area = 0.0;
    for (i, f) in complex.iter().enumerate() {
        let (a, b, c) = (pts[f.tri[0] as usize], pts[f.tri[1] as usize], pts[f.tri[2] as usize]);
        let n = cross(sub(b, a), sub(c, a));
        let area = 0.5 * dot(n, n).sqrt();
        total_area += area;
        let e = pair_stats.entry(pair(f)).or_default();
        e.0 += 1;
        e.1 += area;
        e.2.insert(patch_sets.find(i as u32));
    }
    println!("side-tag pairs (0 = void) | faces | area mm2 | patches");
    let mut patches = 0;
    for ((a, b), (n, area, roots)) in &pair_stats {
        patches += roots.len();
        println!("  ({a:>2}, {b:>2}) | {n:>7} | {area:>9.1} | {}", roots.len());
    }
    println!(
        "pairs {} | patches {patches} | pinned curve edges {curve_edges} | curve length {:.0} mm | curve vertices {}",
        pair_stats.len(),
        curve_length,
        curve_vertices.len()
    );
    let l0 = (4.0 * total_area / (3.0_f64.sqrt() * complex.len() as f64)).sqrt();
    let mut patch_faces: HashMap<u32, usize> = HashMap::new();
    for i in 0..complex.len() {
        *patch_faces.entry(patch_sets.find(i as u32)).or_default() += 1;
    }
    let bucket = |lo: usize, hi: usize| {
        let sel: Vec<usize> = patch_faces.values().copied().filter(|&n| n >= lo && n < hi).collect();
        (sel.len(), sel.iter().sum::<usize>())
    };
    for (lo, hi) in [(0, 10), (10, 100), (100, 1000), (1000, usize::MAX)] {
        let (count, faces) = bucket(lo, hi);
        println!("patches with {lo} to {} faces: {count} (together {faces} faces)", if hi == usize::MAX { "many".to_owned() } else { (hi - 1).to_string() });
    }
    println!("source: {} faces, area {:.0} mm2, mean edge {:.2} mm", complex.len(), total_area, l0);
    println!("target mean edge | naive area estimate of triangles | floor from pinned curves (2 per curve vertex)");
    for target in [1.5, 2.0, 3.0, 4.0, 6.0] {
        let estimate = (complex.len() as f64 * (l0 / target).powi(2)) as usize;
        println!("  {target:>4.1} mm | {estimate:>8} | {}", 2 * curve_vertices.len());
    }

    println!("\n== Gates ==");
    println!("tag regions closed, consistently oriented, manifold, and surface volume = tet volume: {}", if gate_regions_ok { "PASS" } else { "FAIL (see table)" });
    println!("inner exterior components (expect 4 sealed chambers): {inner} -> {}", if inner == 4 { "PASS" } else { "CHECK" });
}
