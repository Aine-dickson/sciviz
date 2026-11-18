
from pathlib import Path
from collections import defaultdict, deque

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

# Sort slices by their physical Z position.
uid_to_z = {}

for group in seg.PerFrameFunctionalGroupsSequence:
    source = group.DerivationImageSequence[0].SourceImageSequence[0]
    uid = source.ReferencedSOPInstanceUID
    z = float(group.PlanePositionSequence[0].ImagePositionPatient[2])
    uid_to_z[uid] = z

uids = sorted(uid_to_z, key=lambda uid: uid_to_z[uid])

for item in seg.SegmentSequence:
    number = item.SegmentNumber
    label = item.SegmentLabel

    volume = np.stack([
        frames[number][uid] for uid in uids
    ])

    print(f"\n=== {label} ===")
    print("Volume shape (Z, rows, columns):", volume.shape)
    print("Foreground voxels:", int(volume.sum()))

    # Count 3D connected components using 26-connectivity.
    visited = np.zeros(volume.shape, dtype=bool)
    components = []

    nz, ny, nx = volume.shape

    for z, y, x in np.argwhere(volume):
        if visited[z, y, x]:
            continue

        queue = deque([(z, y, x)])
        visited[z, y, x] = True
        count = 0

        while queue:
            cz, cy, cx = queue.popleft()
            count += 1

            for dz in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    for dx in (-1, 0, 1):
                        if dz == dy == dx == 0:
                            continue

                        zz, yy, xx = cz + dz, cy + dy, cx + dx

                        if (
                            0 <= zz < nz
                            and 0 <= yy < ny
                            and 0 <= xx < nx
                            and volume[zz, yy, xx]
                            and not visited[zz, yy, xx]
                        ):
                            visited[zz, yy, xx] = True
                            queue.append((zz, yy, xx))

        components.append(count)

    components.sort(reverse=True)

    print("Connected components:", len(components))
    print("Largest component sizes:", components[:10])
    print("Smallest component size:", components[-1] if components else 0)
