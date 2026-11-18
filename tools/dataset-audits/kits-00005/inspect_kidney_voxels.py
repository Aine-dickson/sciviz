from pathlib import Path
from collections import defaultdict
import numpy as np
import pydicom
from pydicom.pixels import iter_pixels

ROOT = Path("datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005")

ct_dir = next(ROOT.glob("*/6-arterial-*"))
seg_path = next(ROOT.glob("*/300-Segmentation-*/*.dcm"))

# Inspect the CT slice nearest the middle of the volume.
ct_slices = []

for path in ct_dir.glob("*.dcm"):
    ds = pydicom.dcmread(path, stop_before_pixels=True)
    ct_slices.append((
        float(ds.ImagePositionPatient[2]),
        path
    ))

ct_slices.sort()
middle_z, middle_path = ct_slices[len(ct_slices) // 2]

ct = pydicom.dcmread(middle_path)
pixels = ct.pixel_array.astype(np.float32)
hu = pixels * float(ct.RescaleSlope) + float(ct.RescaleIntercept)

print("=== CT PIXEL DATA ===")
print("Middle slice Z:", middle_z)
print("Array shape:", pixels.shape)
print("Stored pixel range:", (pixels.min(), pixels.max()))
print("HU range:", (hu.min(), hu.max()))
print("HU percentiles:", np.percentile(hu, [1, 25, 50, 75, 99]))

# Inspect every segmentation frame without loading the entire
# decoded segmentation volume into memory.
seg = pydicom.dcmread(seg_path)
counts = defaultdict(list)
positions = defaultdict(list)
empty_frames = defaultdict(int)

print("\n=== SEGMENTATION VOXEL DATA ===")

for index, frame_pixels in enumerate(iter_pixels(seg)):
    group = seg.PerFrameFunctionalGroupsSequence[index]

    segment = group.SegmentIdentificationSequence[0].ReferencedSegmentNumber
    z = float(group.PlanePositionSequence[0].ImagePositionPatient[2])

    foreground = int(np.count_nonzero(frame_pixels))

    counts[segment].append(foreground)
    positions[segment].append(z)

    if foreground == 0:
        empty_frames[segment] += 1

for item in seg.SegmentSequence:
    number = item.SegmentNumber
    values = counts[number]
    nonempty = sum(v > 0 for v in values)

    print(f"\nSegment {number}: {item.SegmentLabel}")
    print("  Frames:", len(values))
    print("  Non-empty frames:", nonempty)
    print("  Empty frames:", empty_frames[number])
    print("  Total foreground pixels:", sum(values))
    print("  Minimum pixels per frame:", min(values))
    print("  Maximum pixels per frame:", max(values))
    print("  First Z:", min(positions[number]))
    print("  Last Z:", max(positions[number]))