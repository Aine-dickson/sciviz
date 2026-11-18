from pathlib import Path
from collections import Counter
import pydicom

ROOT = Path("datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005")
ct_dir = next(ROOT.glob("*/6-arterial-*"))
seg_path = next(ROOT.glob("*/300-Segmentation-*/*.dcm"))

ct_by_uid = {}

for path in ct_dir.glob("*.dcm"):
    ds = pydicom.dcmread(path, stop_before_pixels=True)
    ct_by_uid[str(ds.SOPInstanceUID)] = {
        "position": tuple(float(v) for v in ds.ImagePositionPatient),
        "orientation": tuple(float(v) for v in ds.ImageOrientationPatient),
    }

ct_z = sorted(x["position"][2] for x in ct_by_uid.values())

print("=== CT GEOMETRY ===")
print("Instances:", len(ct_by_uid))
print("Z range:", (ct_z[0], ct_z[-1]))
print("Z spacing:", Counter(round(b-a, 4) for a, b in zip(ct_z, ct_z[1:])))

seg = pydicom.dcmread(seg_path, stop_before_pixels=True)

shared = seg.SharedFunctionalGroupsSequence[0]
shared_orientation = tuple(
    float(v) for v in
    shared.PlaneOrientationSequence[0].ImageOrientationPatient
)

matched = 0
missing = 0
position_diffs = []
orientation_diffs = 0
segment_counts = Counter()

for frame in seg.PerFrameFunctionalGroupsSequence:
    segment = frame.SegmentIdentificationSequence[0].ReferencedSegmentNumber
    segment_counts[segment] += 1

    position = tuple(float(v) for v in
        frame.PlanePositionSequence[0].ImagePositionPatient)

    orientation_sequence = getattr(frame, "PlaneOrientationSequence", None)
    orientation = (
        tuple(float(v) for v in orientation_sequence[0].ImageOrientationPatient)
        if orientation_sequence
        else shared_orientation
    )

    for derivation in frame.DerivationImageSequence:
        for source in derivation.SourceImageSequence:
            uid = str(source.ReferencedSOPInstanceUID)
            ct = ct_by_uid.get(uid)

            if ct is None:
                missing += 1
                continue

            matched += 1
            position_diffs.append(max(
                abs(a-b) for a, b in zip(position, ct["position"])
            ))

            if orientation != ct["orientation"]:
                orientation_diffs += 1

print("\n=== CT / SEG ALIGNMENT ===")
print("Frames by segment:", dict(segment_counts))
print("Matched references:", matched)
print("Missing references:", missing)
print("Maximum position difference (mm):",
      max(position_diffs) if position_diffs else "Unavailable")
print("Orientation mismatches:", orientation_diffs)

print("\n=== SEG STORAGE METADATA ===")
for tag in [
    "SegmentationType", "BitsAllocated", "BitsStored",
    "HighBit", "PixelRepresentation", "NumberOfFrames",
    "Rows", "Columns", "TransferSyntaxUID"
]:
    value = getattr(seg, tag, "Not present")
    if tag == "TransferSyntaxUID":
        value = seg.file_meta.TransferSyntaxUID
    print(f"{tag}: {value}")