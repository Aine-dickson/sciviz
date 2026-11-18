from pathlib import Path
from collections import Counter, defaultdict
import pydicom

ROOT = Path("datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005")
seg_path = next(ROOT.glob("*/300-Segmentation-*/*.dcm"))

ds = pydicom.dcmread(seg_path)

frames = ds.PerFrameFunctionalGroupsSequence
counts = Counter()
positions = defaultdict(list)
references = defaultdict(set)

for i, frame in enumerate(frames):
    segment = frame.SegmentIdentificationSequence[0].ReferencedSegmentNumber
    counts[segment] += 1

    if hasattr(frame, "PlanePositionSequence"):
        position = frame.PlanePositionSequence[0].ImagePositionPatient
        positions[segment].append(tuple(float(v) for v in position))

    if hasattr(frame, "DerivationImageSequence"):
        for derivation in frame.DerivationImageSequence:
            for source in getattr(derivation, "SourceImageSequence", []):
                references[segment].add(source.ReferencedSOPInstanceUID)

print("=== FRAME COUNTS ===")
for segment, count in sorted(counts.items()):
    label = ds.SegmentSequence[segment - 1].SegmentLabel
    print(f"{segment} ({label}): {count} frames")

print("\n=== SPATIAL POSITIONS ===")
for segment, values in sorted(positions.items()):
    z_values = sorted(set(p[2] for p in values))
    print(f"Segment {segment}:")
    print("  Unique positions:", len(set(values)))
    print("  Z range:", (min(z_values), max(z_values)) if z_values else "Unavailable")
    print("  First position:", values[0] if values else "Unavailable")
    print("  Last position:", values[-1] if values else "Unavailable")

print("\n=== REFERENCED CT INSTANCES ===")
for segment, uids in sorted(references.items()):
    print(f"Segment {segment}: {len(uids)} distinct CT instances referenced")

print("\n=== SHARED SPATIAL METADATA ===")
for group in getattr(ds, "SharedFunctionalGroupsSequence", []):
    for name in ["PixelMeasuresSequence", "PlaneOrientationSequence"]:
        if hasattr(group, name):
            item = getattr(group, name)[0]
            print(name)
            for field in ["PixelSpacing", "SliceThickness", "SpacingBetweenSlices",
                          "ImageOrientationPatient"]:
                if hasattr(item, field):
                    print(f"  {field}: {getattr(item, field)}")
