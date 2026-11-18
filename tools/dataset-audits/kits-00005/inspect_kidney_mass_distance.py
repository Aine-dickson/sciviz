
from pathlib import Path
from collections import defaultdict

import numpy as np
import pydicom
from pydicom.pixels import iter_pixels
from scipy.ndimage import distance_transform_edt

ROOT = Path("datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005")

seg_path = next(ROOT.glob("*/300-Segmentation-*/*.dcm"))
seg = pydicom.dcmread(seg_path)

frames = defaultdict(dict)
uid_to_z = {}

for index, pixels in enumerate(iter_pixels(seg)):
    group = seg.PerFrameFunctionalGroupsSequence[index]

    segment = (
        group.SegmentIdentificationSequence[0]
        .ReferencedSegmentNumber
    )

    source = group.DerivationImageSequence[0].SourceImageSequence[0]
    uid = source.ReferencedSOPInstanceUID

    frames[segment][uid] = pixels.astype(bool)
    uid_to_z[uid] = float(
        group.PlanePositionSequence[0].ImagePositionPatient[2]
    )

uids = sorted(uid_to_z, key=lambda uid: uid_to_z[uid])

kidney = np.stack([frames[1][uid] for uid in uids])
mass = np.stack([frames[2][uid] for uid in uids])

# Physical voxel spacing: Z, row, column.
spacing = (0.5, 0.9765625, 0.9765625)

# Distance from every background voxel to the nearest kidney voxel.
distance = distance_transform_edt(~kidney, sampling=spacing)

mass_distances = distance[mass]

print("=== MASS TO KIDNEY DISTANCE ===")
print("Minimum (mm):", float(mass_distances.min()))
print("Maximum (mm):", float(mass_distances.max()))
print("Mean (mm):", float(mass_distances.mean()))
print("Median (mm):", float(np.median(mass_distances)))
print("Percentiles (mm):",
      np.percentile(mass_distances, [25, 50, 75, 90, 95, 99]))

print("\nMass voxels within distance:")
for threshold in (0.5, 1, 2, 3, 5, 10):
    count = int(np.count_nonzero(mass_distances <= threshold))
    percentage = 100 * count / len(mass_distances)
    print(f"  {threshold:4g} mm: {count:7d} ({percentage:.2f}%)")
