//! Small diagnostic: report a legacy VTK file's fields and check boundary
//! extraction. Not a viewer; see `docs/heart-fibre-orientation.md` §6.
//!
//! Usage: `cargo run -p svf-core --release --example vtk_inspect -- <file.vtk>`

use std::{env, fs, process, time::Instant};

use svf_core::{
    boundary_surface::BoundaryFaceVertices3,
    dataset::{DatasetField, DatasetGeometry},
    vtk_legacy::parse_legacy_ascii_unstructured_grid,
};

fn fail(message: String) -> ! {
    eprintln!("{message}");
    process::exit(1);
}

fn main() {
    let mut args = env::args().skip(1);
    let (Some(path), None) = (args.next(), args.next()) else {
        eprintln!("Usage: vtk_inspect <file.vtk>");
        process::exit(2);
    };

    let start = Instant::now();
    let input = fs::read_to_string(&path)
        .unwrap_or_else(|error| fail(format!("File read failed: {error}")));
    let dataset = parse_legacy_ascii_unstructured_grid(&input)
        .unwrap_or_else(|error| fail(format!("VTK parse failed: {error}")));
    println!("File: {path} ({} bytes)", input.len());
    println!("Read and parse: {:.2} s", start.elapsed().as_secs_f64());
    println!("Points: {}", dataset.point_count());
    println!("Cells: {}", dataset.cell_count());

    println!("Fields: {}", dataset.fields().len());
    for field in dataset.fields() {
        let (kind, name, association, len) = match field {
            DatasetField::Scalar(f) => ("scalar", f.metadata().name(), f.association(), f.len()),
            DatasetField::Vector3(f) => ("Vector3", f.metadata().name(), f.association(), f.len()),
        };
        println!("  {name} | {association:?} | {kind} | {len} values");
    }

    let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else {
        fail("Boundary: not an unstructured grid; skipped extraction.".to_owned());
    };

    let start = Instant::now();
    let surface = grid
        .boundary_surface()
        .unwrap_or_else(|error| fail(format!("Boundary extraction failed: {error}")));
    let triangulated = surface.triangulate();
    let elapsed = start.elapsed().as_secs_f64();

    let quads = surface
        .faces()
        .iter()
        .filter(|face| matches!(face.vertices(), BoundaryFaceVertices3::Quad(_)))
        .count();
    println!("Boundary extraction and triangulation: {elapsed:.2} s");
    println!("  Boundary points: {}", surface.point_count());
    println!(
        "  Boundary faces: {} ({} triangles, {quads} quads)",
        surface.face_count(),
        surface.face_count() - quads
    );
    println!("  Triangulated surface triangles: {}", triangulated.triangle_count());
}
