//! Opt-in integration test against locally exported KiTS-00005 DICOM SEG masks.
//!
//! Run from the workspace root:
//! SVF_KITS_MASK_DIR=datasets/validation-exports/kits-00005 \
//!   cargo test -p svf-core --test kits_real_masks -- --ignored --nocapture
//!
//! The raw files are local validation artifacts and are intentionally not
//! checked into the repository. Their layout is documented by the exporter.

use std::{env, fs, path::PathBuf, time::Instant};

use svf_core::{
    image_volume::ImageVolumeGeometry3,
    segmentation::{BinaryMask3, Segment3, SegmentId3, SegmentationVolume3},
    spatial::{DMat3, DVec3, WorldPoint3},
};

const DIMENSIONS: [usize; 3] = [512, 512, 834];
const VOXEL_COUNT: usize = DIMENSIONS[0] * DIMENSIONS[1] * DIMENSIONS[2];

fn load_mask(directory: &PathBuf, filename: &str, expected_count: usize) -> BinaryMask3 {
    let path = directory.join(filename);
    let bytes = fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    assert_eq!(
        bytes.len(),
        VOXEL_COUNT,
        "{} must contain exactly one byte per voxel",
        path.display()
    );
    assert!(
        bytes.iter().all(|value| *value <= 1),
        "{} contains values other than binary 0/1",
        path.display()
    );

    let foreground_indices = bytes
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (*value == 1).then_some(index))
        .collect::<Vec<_>>();
    drop(bytes);

    assert_eq!(
        foreground_indices.len(),
        expected_count,
        "{} foreground voxel count differs from the audited source",
        filename
    );

    BinaryMask3::from_indices(VOXEL_COUNT, foreground_indices)
        .expect("exported foreground indices must fit the declared volume")
}

#[test]
#[ignore = "requires locally exported KiTS-00005 masks; see exporter README"]
fn loads_real_kits_masks_into_svf_segmentation_volume() {
    let directory = env::var_os("SVF_KITS_MASK_DIR")
        .map(PathBuf::from)
        .expect("set SVF_KITS_MASK_DIR to the local KiTS mask export directory");
    // Cargo runs integration tests with the package directory as the working
    // directory. Interpret relative user paths from the workspace root so the
    // documented command works unchanged.
    let directory = if directory.is_absolute() {
        directory
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(directory)
    };

    let metadata_path = directory.join("metadata.json");
    assert!(
        metadata_path.is_file(),
        "missing exporter metadata: {}",
        metadata_path.display()
    );

    let geometry = ImageVolumeGeometry3::new(
        DIMENSIONS,
        WorldPoint3::new(-249.51171875, -466.51171875, -737.4),
        DMat3::from_cols(
            DVec3::new(0.9765625, 0.0, 0.0),
            DVec3::new(0.0, 0.9765625, 0.0),
            DVec3::new(0.0, 0.0, 0.5),
        ),
    )
    .expect("audited KiTS geometry must be valid");

    assert_eq!(geometry.voxel_count(), VOXEL_COUNT);

    let kidney = Segment3::new(
        SegmentId3::new(1),
        "Kidney",
        load_mask(&directory, "segment-1.raw", 902_167),
    )
    .expect("Kidney segment should be valid");
    let mass = Segment3::new(
        SegmentId3::new(2),
        "Mass",
        load_mask(&directory, "segment-2.raw", 126_444),
    )
    .expect("Mass segment should be valid");

    let segmentation = SegmentationVolume3::new(geometry, vec![kidney, mass])
        .expect("real masks must match the shared image geometry");

    let kidney_stats = segmentation.statistics(SegmentId3::new(1)).unwrap();
    let mass_stats = segmentation.statistics(SegmentId3::new(2)).unwrap();

    assert_eq!(kidney_stats.foreground_voxel_count, 902_167);
    assert_eq!(kidney_stats.index_bounds.min, [256, 256, 372]);
    assert_eq!(kidney_stats.index_bounds.max, [420, 347, 654]);

    assert_eq!(mass_stats.foreground_voxel_count, 126_444);
    assert_eq!(mass_stats.index_bounds.min, [383, 265, 466]);
    assert_eq!(mass_stats.index_bounds.max, [431, 315, 584]);

    assert!(kidney_stats.world_centroid.to_array().iter().all(|v| v.is_finite()));
    assert!(mass_stats.world_centroid.to_array().iter().all(|v| v.is_finite()));
    assert!(kidney_stats.physical_volume > 0.0);
    assert!(mass_stats.physical_volume > 0.0);

    println!("KiTS-00005 real-mask ingestion passed");
    println!("Dimensions (i,j,k): {:?}", DIMENSIONS);
    println!(
        "Kidney: {} voxels, bounds {:?}..{:?}, centroid {:?}, volume {:.3} mm^3",
        kidney_stats.foreground_voxel_count,
        kidney_stats.index_bounds.min,
        kidney_stats.index_bounds.max,
        kidney_stats.world_centroid.to_array(),
        kidney_stats.physical_volume
    );
    println!(
        "Mass: {} voxels, bounds {:?}..{:?}, centroid {:?}, volume {:.3} mm^3",
        mass_stats.foreground_voxel_count,
        mass_stats.index_bounds.min,
        mass_stats.index_bounds.max,
        mass_stats.world_centroid.to_array(),
        mass_stats.physical_volume
    );
}


#[test]
#[ignore = "requires locally exported KiTS-00005 masks; surface extraction is a separate, heavier check"]
fn extracts_real_kits_mass_surface_and_checks_topology() {
    let directory = env::var_os("SVF_KITS_MASK_DIR")
        .map(PathBuf::from)
        .expect("set SVF_KITS_MASK_DIR to the local KiTS mask export directory");
    let directory = if directory.is_absolute() {
        directory
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(directory)
    };

    let geometry = ImageVolumeGeometry3::new(
        DIMENSIONS,
        WorldPoint3::new(-249.51171875, -466.51171875, -737.4),
        DMat3::from_cols(
            DVec3::new(0.9765625, 0.0, 0.0),
            DVec3::new(0.0, 0.9765625, 0.0),
            DVec3::new(0.0, 0.0, 0.5),
        ),
    )
    .expect("audited KiTS geometry must be valid");

    let mass = Segment3::new(
        SegmentId3::new(2),
        "Mass",
        load_mask(&directory, "segment-2.raw", 126_444),
    )
    .expect("Mass segment should be valid");
    let segmentation = SegmentationVolume3::new(geometry, vec![mass])
        .expect("real Mass mask must match the shared image geometry");

    let extraction_start = Instant::now();
    let mesh = segmentation
        .extract_voxel_boundary_surface(SegmentId3::new(2))
        .expect("Mass surface extraction should succeed")
        .expect("Mass segment should exist");
    let extraction_elapsed = extraction_start.elapsed();

    assert!(!mesh.is_empty());
    assert!(mesh.triangle_count() > 0);

    let topology_start = Instant::now();
    let topology = mesh.topology();
    let topology_elapsed = topology_start.elapsed();

    let boundary_edge_count = topology.boundary_edges().count();
    let non_manifold_edges: Vec<_> = topology.edges_with_more_than_two_faces().collect();
    let non_manifold_edge_count = non_manifold_edges.len();
    assert_eq!(boundary_edge_count, 0, "surface has boundary edges");

    println!("Boundary edges: {boundary_edge_count}");
    println!("Edges with >2 incident faces: {non_manifold_edge_count}");
    for (edge_id, edge) in &non_manifold_edges {
        let [a, b] = edge.vertices();
        let incident = topology.edge_faces(*edge_id).unwrap();
        let world_a = mesh.vertices()[a as usize].to_vector().to_array();
        let world_b = mesh.vertices()[b as usize].to_vector().to_array();
        println!(
            "Non-manifold edge {edge_id}: vertices {a}-{b}, world endpoints {world_a:?} -> {world_b:?}, incident faces {incident:?}"
        );
    }

    // The exact cubical boundary may have self-contact edges when diagonal
    // foreground voxels reconnect elsewhere. Preserve and report those edges
    // rather than silently changing the source geometry to force manifoldness.
    // A closed boundary must still have no boundary edges, and each edge's
    // incident faces must have balanced opposing winding.
    let indices = mesh.indices();
    for edge_id in 0..topology.edge_count() {
        let edge = topology.edges()[edge_id].vertices();
        let incident_faces = topology.edge_faces(edge_id).unwrap();
        assert!(
            incident_faces.len() >= 2,
            "closed voxel boundary has an edge with insufficient incidence: {edge:?}"
        );
        assert_eq!(
            incident_faces.len() % 2,
            0,
            "edge {edge:?} must have paired incident faces"
        );

        let mut forward = 0_usize;
        let mut backward = 0_usize;
        for &face_index in incident_faces {
            let face = &indices[face_index * 3..face_index * 3 + 3];
            let directed_edge = [
                (face[0], face[1]),
                (face[1], face[2]),
                (face[2], face[0]),
            ]
            .into_iter()
            .find(|(from, to)| [(*from).min(*to), (*from).max(*to)] == edge)
            .expect("incident face must traverse its edge");
            if directed_edge.0 == edge[0] {
                forward += 1;
            } else {
                backward += 1;
            }
        }
        assert_eq!(
            forward,
            backward,
            "edge {edge:?} must have balanced opposing face winding"
        );
    }

    // Compute signed volume relative to the segment centroid to reduce
    // cancellation from large absolute world coordinates.
    let mass_stats = segmentation.statistics(SegmentId3::new(2))
        .expect("Mass statistics should exist");
    let reference = mass_stats.world_centroid.to_vector();
    let mut signed_volume = 0.0_f64;
    let mut compensation = 0.0_f64;
    for [a, b, c] in mesh.triangles() {
        let p0 = mesh.vertices()[a as usize].to_vector() - reference;
        let p1 = mesh.vertices()[b as usize].to_vector() - reference;
        let p2 = mesh.vertices()[c as usize].to_vector() - reference;
        let term = p0.dot(p1.cross(p2)) / 6.0;
        let adjusted = term - compensation;
        let next = signed_volume + adjusted;
        compensation = (next - signed_volume) - adjusted;
        signed_volume = next;
    }

    let expected_volume = mass_stats.physical_volume;
    let relative_volume_error = (signed_volume - expected_volume).abs() / expected_volume;
    assert!(
        signed_volume > 0.0,
        "surface winding should produce positive signed volume, got {signed_volume}"
    );
    assert!(
        relative_volume_error < 1.0e-9,
        "mesh volume {signed_volume} differs from mask volume {expected_volume}"
    );

    println!("KiTS-00005 Mass surface extraction passed");
    println!("Vertices: {}", mesh.vertex_count());
    println!("Triangles: {}", mesh.triangle_count());
    println!("Unique edges: {}", topology.edge_count());
    println!("Boundary edges: {boundary_edge_count}");
    println!("Edges with >2 incident faces: {non_manifold_edge_count}");
    println!("Signed mesh volume: {signed_volume:.6} mm^3");
    println!("Mask voxel volume: {expected_volume:.6} mm^3");
    println!("Relative volume error: {relative_volume_error:.3e}");
    println!("Surface extraction time: {extraction_elapsed:?}");
    println!("Topology analysis time: {topology_elapsed:?}");
}


#[test]
#[ignore = "requires locally exported KiTS-00005 masks; surface extraction is a separate, heavier check"]
fn extracts_real_kits_kidney_surface_and_checks_topology() {
    let directory = env::var_os("SVF_KITS_MASK_DIR")
        .map(PathBuf::from)
        .expect("set SVF_KITS_MASK_DIR to the local KiTS mask export directory");
    let directory = if directory.is_absolute() {
        directory
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(directory)
    };

    let geometry = ImageVolumeGeometry3::new(
        DIMENSIONS,
        WorldPoint3::new(-249.51171875, -466.51171875, -737.4),
        DMat3::from_cols(
            DVec3::new(0.9765625, 0.0, 0.0),
            DVec3::new(0.0, 0.9765625, 0.0),
            DVec3::new(0.0, 0.0, 0.5),
        ),
    )
    .expect("audited KiTS geometry must be valid");

    let kidney = Segment3::new(
        SegmentId3::new(1),
        "Kidney",
        load_mask(&directory, "segment-1.raw", 902_167),
    )
    .expect("Kidney segment should be valid");
    let segmentation = SegmentationVolume3::new(geometry, vec![kidney])
        .expect("real Kidney mask must match the shared image geometry");

    let extraction_start = Instant::now();
    let mesh = segmentation
        .extract_voxel_boundary_surface(SegmentId3::new(1))
        .expect("Kidney surface extraction should succeed")
        .expect("Kidney segment should exist");
    let extraction_elapsed = extraction_start.elapsed();

    assert!(!mesh.is_empty());
    assert!(mesh.triangle_count() > 0);

    let topology_start = Instant::now();
    let topology = mesh.topology();
    let topology_elapsed = topology_start.elapsed();

    let boundary_edge_count = topology.boundary_edges().count();
    let non_manifold_edges: Vec<_> = topology.edges_with_more_than_two_faces().collect();
    let non_manifold_edge_count = non_manifold_edges.len();
    assert_eq!(boundary_edge_count, 0, "surface has boundary edges");

    println!("Boundary edges: {boundary_edge_count}");
    println!("Edges with >2 incident faces: {non_manifold_edge_count}");
    for (edge_id, edge) in &non_manifold_edges {
        let [a, b] = edge.vertices();
        let incident = topology.edge_faces(*edge_id).unwrap();
        let world_a = mesh.vertices()[a as usize].to_vector().to_array();
        let world_b = mesh.vertices()[b as usize].to_vector().to_array();
        println!(
            "Non-manifold edge {edge_id}: vertices {a}-{b}, world endpoints {world_a:?} -> {world_b:?}, incident faces {incident:?}"
        );
        for &face_id in incident {
            let triangle = &mesh.indices()[face_id * 3..face_id * 3 + 3];
            println!("  face {face_id}: {triangle:?}");
        }

        // Report the four source voxels surrounding this lattice edge. This
        // distinguishes a local checkerboard pairing case from fan merging
        // that occurs transitively around a lattice corner.
        let lattice = |point: [f64; 3]| {
            [
                ((point[0] + 249.51171875) / 0.9765625 + 0.5).round() as usize,
                ((point[1] + 466.51171875) / 0.9765625 + 0.5).round() as usize,
                ((point[2] + 737.4) / 0.5 + 0.5).round() as usize,
            ]
        };
        let la = lattice(world_a);
        let lb = lattice(world_b);
        let axis = (0..3).find(|&axis| la[axis] != lb[axis])
            .expect("mesh edge must span one lattice axis");
        let mut other_axes = (0..3).filter(|&candidate| candidate != axis);
        let u = other_axes.next().unwrap();
        let v = other_axes.next().unwrap();
        let base = la[axis].min(lb[axis]);
        let mask = segmentation.segment(SegmentId3::new(1)).unwrap().mask();
        for u_offset in [0_usize, 1] {
            for v_offset in [0_usize, 1] {
                let mut voxel = [0_usize; 3];
                voxel[axis] = base;
                voxel[u] = la[u].saturating_sub(1) + u_offset;
                voxel[v] = la[v].saturating_sub(1) + v_offset;
                println!(
                    "  surrounding voxel {voxel:?}: {:?}",
                    mask.get_voxel(segmentation.geometry(), voxel)
                );
            }
        }
    }

    assert_eq!(non_manifold_edge_count, 0, "surface has non-manifold edges");

    // A closed, consistently oriented triangle surface must have exactly two
    // oppositely directed incident faces at every undirected edge.
    let indices = mesh.indices();
    for edge_id in 0..topology.edge_count() {
        let edge = topology.edges()[edge_id].vertices();
        let incident_faces = topology.edge_faces(edge_id).unwrap();
        assert_eq!(incident_faces.len(), 2, "edge {edge:?} incidence");

        let mut directions = Vec::with_capacity(2);
        for &face_index in incident_faces {
            let face = &indices[face_index * 3..face_index * 3 + 3];
            let directed_edge = [
                (face[0], face[1]),
                (face[1], face[2]),
                (face[2], face[0]),
            ]
            .into_iter()
            .find(|(from, to)| {
                [(*from).min(*to), (*from).max(*to)] == edge
            })
            .expect("incident face must traverse its edge");
            directions.push(if directed_edge.0 == edge[0] { 1_i8 } else { -1_i8 });
        }
        assert_eq!(
            directions[0],
            -directions[1],
            "incident faces must traverse edge {edge:?} in opposite directions"
        );
    }

    // Compute signed volume relative to the segment centroid to reduce
    // cancellation from large absolute world coordinates.
    let kidney_stats = segmentation.statistics(SegmentId3::new(1))
        .expect("Kidney statistics should exist");
    let reference = kidney_stats.world_centroid.to_vector();
    let mut signed_volume = 0.0_f64;
    let mut compensation = 0.0_f64;
    for [a, b, c] in mesh.triangles() {
        let p0 = mesh.vertices()[a as usize].to_vector() - reference;
        let p1 = mesh.vertices()[b as usize].to_vector() - reference;
        let p2 = mesh.vertices()[c as usize].to_vector() - reference;
        let term = p0.dot(p1.cross(p2)) / 6.0;
        let adjusted = term - compensation;
        let next = signed_volume + adjusted;
        compensation = (next - signed_volume) - adjusted;
        signed_volume = next;
    }

    let expected_volume = kidney_stats.physical_volume;
    let relative_volume_error = (signed_volume - expected_volume).abs() / expected_volume;
    assert!(
        signed_volume > 0.0,
        "surface winding should produce positive signed volume, got {signed_volume}"
    );
    assert!(
        relative_volume_error < 1.0e-9,
        "mesh volume {signed_volume} differs from mask volume {expected_volume}"
    );

    println!("KiTS-00005 Kidney surface extraction passed");
    println!("Vertices: {}", mesh.vertex_count());
    println!("Triangles: {}", mesh.triangle_count());
    println!("Unique edges: {}", topology.edge_count());
    println!("Boundary edges: {boundary_edge_count}");
    println!("Edges with >2 incident faces: {non_manifold_edge_count}");
    println!("Signed mesh volume: {signed_volume:.6} mm^3");
    println!("Mask voxel volume: {expected_volume:.6} mm^3");
    println!("Relative volume error: {relative_volume_error:.3e}");
    println!("Surface extraction time: {extraction_elapsed:?}");
    println!("Topology analysis time: {topology_elapsed:?}");
}
