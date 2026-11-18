#!/usr/bin/env python3
"""Export the audited KiTS-00005 DICOM SEG as SVF-ordered binary masks.

This is a local validation bridge, not a production DICOM importer. It resolves
each SEG frame through its referenced CT SOP Instance UID; SEG frame ordering is
never treated as CT slice ordering.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import pydicom


DEFAULT_ROOT = Path("datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005")
DEFAULT_OUTPUT = Path("datasets/validation-exports/kits-00005")
POSITION_TOLERANCE_MM = 1e-4
SPACING_TOLERANCE_MM = 1e-4


def one_sequence_item(owner, name):
    sequence = getattr(owner, name, None)
    if sequence is None or len(sequence) != 1:
        raise ValueError(f"Expected exactly one {name} item")
    return sequence[0]


def referenced_ct_uid(frame):
    references = set()
    for derivation in getattr(frame, "DerivationImageSequence", []):
        for source in getattr(derivation, "SourceImageSequence", []):
            uid = getattr(source, "ReferencedSOPInstanceUID", None)
            if uid:
                references.add(str(uid))
    if len(references) != 1:
        raise ValueError(
            "Each SEG frame must resolve to exactly one distinct source CT "
            f"instance; found {len(references)}"
        )
    return next(iter(references))


def vector(values):
    result = np.asarray([float(value) for value in values], dtype=np.float64)
    if result.shape != (3,) or not np.isfinite(result).all():
        raise ValueError(f"Invalid 3D vector: {values}")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dataset-root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--output-dir", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    study_dirs = sorted(path for path in args.dataset_root.iterdir() if path.is_dir())
    ct_dirs = [path for study in study_dirs for path in study.glob("6-arterial-*") if path.is_dir()]
    seg_files = [path for study in study_dirs for path in study.glob("300-Segmentation-*/*.dcm")]
    if len(ct_dirs) != 1 or len(seg_files) != 1:
        raise ValueError(
            f"Expected one arterial CT directory and one SEG file; "
            f"found {len(ct_dirs)} and {len(seg_files)}"
        )
    ct_dir, seg_path = ct_dirs[0], seg_files[0]

    ct_by_uid = {}
    for path in ct_dir.glob("*.dcm"):
        ds = pydicom.dcmread(path, stop_before_pixels=True)
        uid = str(ds.SOPInstanceUID)
        if uid in ct_by_uid:
            raise ValueError(f"Duplicate CT SOPInstanceUID: {uid}")
        ct_by_uid[uid] = {
            "position": vector(ds.ImagePositionPatient),
            "orientation": np.asarray(ds.ImageOrientationPatient, dtype=np.float64),
            "path": str(path),
        }
    if not ct_by_uid:
        raise ValueError("No CT DICOM instances found")

    seg = pydicom.dcmread(seg_path)
    if str(seg.SegmentationType).upper() != "BINARY":
        raise ValueError(f"Expected BINARY SEG, got {seg.SegmentationType}")
    rows, columns = int(seg.Rows), int(seg.Columns)
    frames = seg.PerFrameFunctionalGroupsSequence
    pixels = np.asarray(seg.pixel_array)
    if pixels.shape != (len(frames), rows, columns):
        raise ValueError(
            f"Decoded pixel array shape {pixels.shape} does not match "
            f"({len(frames)}, {rows}, {columns})"
        )
    if not np.isin(pixels, (0, 1)).all():
        raise ValueError("Decoded binary SEG contains values other than 0 and 1")

    shared = one_sequence_item(seg, "SharedFunctionalGroupsSequence")
    pixel_measures = one_sequence_item(shared, "PixelMeasuresSequence")
    orientation_item = one_sequence_item(shared, "PlaneOrientationSequence")
    pixel_spacing = np.asarray(pixel_measures.PixelSpacing, dtype=np.float64)
    if pixel_spacing.shape != (2,) or not np.isfinite(pixel_spacing).all() or np.any(pixel_spacing <= 0):
        raise ValueError(f"Invalid PixelSpacing: {pixel_measures.PixelSpacing}")

    i_direction = vector(orientation_item.ImageOrientationPatient[:3])
    j_direction = vector(orientation_item.ImageOrientationPatient[3:])
    if abs(np.linalg.norm(i_direction) - 1.0) > 1e-5 or abs(np.linalg.norm(j_direction) - 1.0) > 1e-5:
        raise ValueError("Image orientation direction cosines are not unit length")
    if abs(float(np.dot(i_direction, j_direction))) > 1e-5:
        raise ValueError("Image orientation direction cosines are not orthogonal")
    normal = np.cross(i_direction, j_direction)
    normal /= np.linalg.norm(normal)

    # Sort source CT instances by their physical projection onto the slice normal.
    projected = sorted(
        ((float(np.dot(item["position"], normal)), uid, item)
         for uid, item in ct_by_uid.items()),
        key=lambda entry: entry[0],
    )
    projections = np.asarray([entry[0] for entry in projected], dtype=np.float64)
    if len(projected) < 2:
        raise ValueError("At least two CT slices are required to determine slice spacing")
    differences = np.diff(projections)
    if np.any(differences <= 0):
        raise ValueError("CT slice positions are not uniquely ordered")
    slice_spacing = float(np.median(differences))
    if np.max(np.abs(differences - slice_spacing)) > SPACING_TOLERANCE_MM:
        raise ValueError(
            "CT slices are not regularly spaced; this validation exporter "
            "does not resample irregular geometry"
        )

    uid_to_k = {uid: k for k, (_, uid, _) in enumerate(projected)}
    ordered_positions = np.stack([entry[2]["position"] for entry in projected])
    origin = ordered_positions[0]
    expected_orientation = np.concatenate((i_direction, j_direction))
    for _, uid, item in projected:
        if np.max(np.abs(item["orientation"] - expected_orientation)) > 1e-5:
            raise ValueError(f"CT orientation differs from SEG shared orientation: {uid}")
        expected_position = origin + np.dot(item["position"] - origin, normal) * normal
        if np.linalg.norm(item["position"] - expected_position) > POSITION_TOLERANCE_MM:
            raise ValueError(f"CT slice has in-plane origin drift beyond tolerance: {uid}")

    segments = {}
    for item in seg.SegmentSequence:
        number = int(item.SegmentNumber)
        if number in segments:
            raise ValueError(f"Duplicate SEG segment number: {number}")
        segments[number] = str(item.SegmentLabel)

    # Arrays use NumPy (k, j, i) C-order, matching SVF's i-fastest linear order.
    masks = {
        number: np.zeros((len(projected), rows, columns), dtype=np.uint8)
        for number in segments
    }
    seen = {number: set() for number in segments}
    frame_positions = {}

    for frame_index, frame in enumerate(frames):
        identification = one_sequence_item(frame, "SegmentIdentificationSequence")
        segment_number = int(identification.ReferencedSegmentNumber)
        if segment_number not in masks:
            raise ValueError(f"Frame references unknown segment number {segment_number}")
        uid = referenced_ct_uid(frame)
        if uid not in uid_to_k:
            raise ValueError(f"SEG frame references unknown CT SOPInstanceUID: {uid}")
        k = uid_to_k[uid]
        if k in seen[segment_number]:
            raise ValueError(f"Duplicate frame for segment {segment_number}, CT slice {k}")
        seen[segment_number].add(k)

        position_item = one_sequence_item(frame, "PlanePositionSequence")
        frame_position = vector(position_item.ImagePositionPatient)
        ct_position = ct_by_uid[uid]["position"]
        if np.max(np.abs(frame_position - ct_position)) > POSITION_TOLERANCE_MM:
            raise ValueError(f"SEG frame position does not match referenced CT instance {uid}")
        frame_positions[frame_index] = frame_position.tolist()
        masks[segment_number][k] = pixels[frame_index]

    expected_slices = set(range(len(projected)))
    for number, label in segments.items():
        if seen[number] != expected_slices:
            raise ValueError(
                f"Segment {number} ({label}) does not have exactly one frame "
                f"for every CT slice: {len(seen[number])}/{len(projected)}"
            )

    args.output_dir.mkdir(parents=True, exist_ok=True)
    segment_metadata = []
    for number, label in sorted(segments.items()):
        mask = masks[number]
        foreground = int(mask.sum())
        occupied = np.argwhere(mask)
        if foreground:
            # np.argwhere returns (k,j,i); SVF coordinates are (i,j,k).
            bounds = {
                "min_ijk": [int(occupied[:, 2].min()), int(occupied[:, 1].min()), int(occupied[:, 0].min())],
                "max_ijk": [int(occupied[:, 2].max()), int(occupied[:, 1].max()), int(occupied[:, 0].max())],
            }
        else:
            bounds = None
        filename = f"segment-{number}.raw"
        mask.tofile(args.output_dir / filename)
        segment_metadata.append({
            "segment_number": number,
            "label": label,
            "file": filename,
            "foreground_voxels": foreground,
            "bounds": bounds,
        })

    basis_i = i_direction * float(pixel_spacing[1])
    basis_j = j_direction * float(pixel_spacing[0])
    basis_k = normal * slice_spacing
    metadata = {
        "format": "SVF local validation mask export v1",
        "source_case": "KiTS-00005",
        "source_ct_series_directory": ct_dir.name,
        "source_seg_file": seg_path.name,
        "segmentation_type": "BINARY",
        "axis_order": ["i_column", "j_row", "k_slice"],
        "array_shape_numpy": [len(projected), rows, columns],
        "dimensions_svf_ijk": [columns, rows, len(projected)],
        "storage_order": "i-fastest; raw files are uint8 0/1 in NumPy (k,j,i) C-order",
        "origin_voxel_000": origin.tolist(),
        "basis_i_mm": basis_i.tolist(),
        "basis_j_mm": basis_j.tolist(),
        "basis_k_mm": basis_k.tolist(),
        "pixel_spacing_row_column_mm": pixel_spacing.tolist(),
        "slice_spacing_mm": slice_spacing,
        "image_orientation_patient": expected_orientation.tolist(),
        "ct_instance_count": len(ct_by_uid),
        "seg_frame_count": len(frames),
        "segments": segment_metadata,
    }
    (args.output_dir / "metadata.json").write_text(
        json.dumps(metadata, indent=2) + "\n", encoding="utf-8"
    )

    print("Export complete")
    print(f"CT instances: {len(ct_by_uid)}")
    print(f"SEG frames: {len(frames)}")
    print(f"SVF dimensions (i,j,k): {columns}, {rows}, {len(projected)}")
    print(f"Origin: {origin.tolist()}")
    print(f"Basis i: {basis_i.tolist()}")
    print(f"Basis j: {basis_j.tolist()}")
    print(f"Basis k: {basis_k.tolist()}")
    for item in segment_metadata:
        print(
            f"Segment {item['segment_number']} ({item['label']}): "
            f"{item['foreground_voxels']} foreground voxels; "
            f"bounds={item['bounds']}"
        )
    print(f"Output: {args.output_dir}")


if __name__ == "__main__":
    main()
