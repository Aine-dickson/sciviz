
from pathlib import Path
from collections import defaultdict

import numpy as np
import pydicom
from pydicom.pixels import iter_pixels

ROOT = Path("datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005")

seg_path = next(ROOT.glob("*/300-Segmentation-*/*.dcm"))
seg = pydicom.dcmread(seg_path)

frames = defaultdict(dict)

for index, pixels in enumerate(iter_pixels(seg)):
    group = seg.PerFrameFunctionalGroupsSequence[index]

    segment = (
        group.SegmentIdentificationSequence[0]
        .ReferencedSegmentNumber
    )

    source = group.DerivationImageSequence[0].SourceImageSequence[0]
    uid = source.ReferencedSOPInstanceUID

    frames[segment][uid] = pixels.astype(bool)

uid_to_z = {}

for group in seg.PerFrameFunctionalGroupsSequence:
    source = group.DerivationImageSequence[0].SourceImageSequence[0]
    uid = source.ReferencedSOPInstanceUID
    uid_to_z[uid] = float(
        group.PlanePositionSequence[0].ImagePositionPatient[2]
    )

uids = sorted(uid_to_z, key=lambda uid: uid_to_z[uid])

kidney = np.stack([frames[1][uid] for uid in uids])
mass = np.stack([frames[2][uid] for uid in uids])

# Pad the kidney volume to avoid wrapping at volume boundaries.
padded = np.pad(kidney, 1, constant_values=False)

adjacent_26 = np.zeros(mass.shape, dtype=bool)
adjacent_6 = np.zeros(mass.shape, dtype=bool)

for dz in (-1, 0, 1):
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            if dz == dy == dx == 0:
                continue

            neighbour = padded[
                1 + dz:1 + dz + mass.shape[0],
                1 + dy:1 + dy + mass.shape[1],
                1 + dx:1 + dx + mass.shape[2],
            ]

            touching = mass & neighbour
            adjacent_26 |= touching

            if abs(dz) + abs(dy) + abs(dx) == 1:
                adjacent_6 |= touching

print("=== KIDNEY / MASS ADJACENCY ===")
print("Mass voxels:", int(mass.sum()))
print("Mass voxels with 6-neighbour kidney contact:",
      int(adjacent_6.sum()))
print("Mass voxels with 26-neighbour kidney contact:",
      int(adjacent_26.sum()))
print("Mass voxels without 26-neighbour contact:",
      int((mass & ~adjacent_26).sum()))

# Report the physical Z range of mass voxels touching the kidney.
z_indices = np.where(adjacent_26.any(axis=(1, 2)))[0]

if len(z_indices):
    print("Contact Z range:",
          (uid_to_z[uids[z_indices[0]]],
           uid_to_z[uids[z_indices[-1]]]))
else:
    print("No direct voxel contact detected.")
