//! B2 step 5b check of `svf_core::region_surface` on a Rodero heart
//! (`docs/heart-structure.md` sections 5 to 7). Runs the whole region pipeline on
//! the real mesh: island relabelling, the labelled surface, cavities, patches,
//! and the closed 2-manifold repair of every part, and compares each part's
//! volume with the exact tetrahedron sum.
//!
//! Usage: `cargo run -p svf-core --release --example heart_regions -- <01-svf-order.vtk> [--tag-field ID]`

use std::{env, fs, process, time::Instant};

use svf_core::{
    dataset::DatasetGeometry,
    region_surface::{is_closed_manifold, relabel_islands, repair_pinches, signed_volume, LabelledSurface, P3},
    sampling::SampleValue,
    vtk_legacy::parse_legacy_ascii_unstructured_grid,
};

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("heart_regions failed: {message}");
    process::exit(1);
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("Usage: heart_regions <01-svf-order.vtk> [--tag-field ID]");
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
    let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { fail("not an unstructured grid") };
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
    println!("Read and parse: {:.1} s | {} cells", clock.elapsed().as_secs_f64(), tags.len());

    let clock = Instant::now();
    let (tags, fixes) = relabel_islands(grid, &tags, 8).unwrap_or_else(|e| fail(e));
    println!("Islands relabelled: {} in {:.1} s", fixes.len(), clock.elapsed().as_secs_f64());
    for fix in &fixes {
        println!("  {} cell(s) of tag {} -> tag {}", fix.cells.len(), fix.from, fix.to);
    }

    let clock = Instant::now();
    let surface = LabelledSurface::build(grid, &tags).unwrap_or_else(|e| fail(e));
    println!("Labelled surface: {} faces in {:.1} s", surface.faces().len(), clock.elapsed().as_secs_f64());
    println!("Cavities: {}", surface.cavities().len());
    for c in surface.cavities() {
        println!(
            "  label {} | {} faces | {:.1} mL | area {:.0} mm2 | bounded by tags {:?}",
            c.label,
            c.face_count,
            c.volume / 1000.0,
            c.area,
            c.bounding_tags
        );
    }
    let patches = surface.patches();
    println!("Patches: {}", patches.len());

    // Exact tag volumes from the tetrahedra.
    let pts: Vec<P3> = grid.points().iter().map(|p| p.to_array()).collect();
    let mut exact = std::collections::BTreeMap::<u32, f64>::new();
    for (c, &tag) in tags.iter().enumerate() {
        let ids = grid.cell_point_indices(c).unwrap();
        let p = [pts[ids[0] as usize], pts[ids[1] as usize], pts[ids[2] as usize], pts[ids[3] as usize]];
        let sub = |a: P3, b: P3| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let cr = |a: P3, b: P3| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let d = cr(sub(p[2], p[0]), sub(p[3], p[0]));
        let v = (sub(p[1], p[0])[0] * d[0] + sub(p[1], p[0])[1] * d[1] + sub(p[1], p[0])[2] * d[2]).abs() / 6.0;
        *exact.entry(tag).or_default() += v;
    }

    // Parts: each wall and plane, each vessel end (inlet tag plus border tag), each cavity.
    let mut parts: Vec<(String, Vec<u32>, f64)> = Vec::new();
    for tag in 1..=10 {
        parts.push((format!("tag {tag}"), vec![tag], exact[&tag]));
    }
    for inlet in 11..=17 {
        parts.push((
            format!("tags {inlet}+{}", inlet + 7),
            vec![inlet, inlet + 7],
            exact[&inlet] + exact[&(inlet + 7)],
        ));
    }
    for c in surface.cavities() {
        parts.push((format!("cavity {}", c.label), vec![c.label], c.volume));
    }

    println!("\npart | triangles | repaired triangles | nm edges | irregular | alternate | split vertices | volume mm3 | source mm3 | rel diff | closed");
    let (mut all_ok, mut total_triangles) = (true, 0usize);
    let clock = Instant::now();
    for (name, labels, source) in &parts {
        let tris = surface.region_triangles(labels);
        match repair_pinches(surface.points(), &tris) {
            Ok((mesh, report)) => {
                let volume = signed_volume(&mesh.points, &mesh.triangles);
                let diff = (volume - source) / source;
                let closed = is_closed_manifold(&mesh.triangles);
                if !closed || diff.abs() > 1e-6 {
                    all_ok = false;
                }
                total_triangles += mesh.triangles.len();
                println!(
                    "{name} | {} | {} | {} | {} | {} | {} | {volume:.1} | {source:.1} | {diff:+.1e} | {closed}",
                    tris.len(),
                    mesh.triangles.len(),
                    report.nonmanifold_edges,
                    report.irregular_pairings,
                    report.alternate_pairings,
                    report.split_vertices
                );
            }
            Err(error) => {
                all_ok = false;
                println!("{name} | {} | REPAIR FAILED: {error}", tris.len());
            }
        }
    }
    println!(
        "\nParts: {} | part triangles in all (before simplification): {total_triangles} | repair time {:.1} s",
        parts.len(),
        clock.elapsed().as_secs_f64()
    );
    println!("All parts closed manifolds with the exact source volume: {}", if all_ok { "PASS" } else { "FAIL" });
}
