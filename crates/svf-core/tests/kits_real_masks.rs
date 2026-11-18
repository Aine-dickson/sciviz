//! Opt-in integration test against locally exported KiTS-00005 DICOM SEG masks.
//!
//! Run from the workspace root:
//! SVF_KITS_MASK_DIR=datasets/validation-exports/kits-00005 \
//!   cargo test -p svf-core --test kits_real_masks -- --ignored --nocapture
//!
//! The raw files are local validation artifacts and are intentionally not
//! checked into the repository. Their layout is documented by the exporter.

use std::{env, fs, path::PathBuf};

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
