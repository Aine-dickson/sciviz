
from pathlib import Path
from collections import defaultdict

import numpy as np
import pydicom
from pydicom.pixels import iter_pixels

ROOT = Path("datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005")

seg_path = next(ROOT.glob("*/300-Segmentation-*/*.dcm"))
seg = pydicom.dcmread(seg_path)

masks = defaultdict(dict)

for index, pixels in enumerate(iter_pixels(seg)):
    group = seg.PerFrameFunctionalGroupsSequence[index]

    segment = (
        group.SegmentIdentificationSequence[0]
        .ReferencedSegmentNumber
    )

    source = group.DerivationImageSequence[0].SourceImageSequence[0]
    ct_uid = source.ReferencedSOPInstanceUID

    masks[segment][ct_uid] = pixels.astype(bool)

kidney = masks[1]
mass = masks[2]

common_uids = set(kidney) & set(mass)

overlap = 0
mass_outside_kidney = 0
kidney_pixels = 0
mass_pixels = 0

for uid in common_uids:
    k = kidney[uid]
    m = mass[uid]

    overlap += int(np.count_nonzero(k & m))
    mass_outside_kidney += int(np.count_nonzero(m & ~k))
    kidney_pixels += int(np.count_nonzero(k))
    mass_pixels += int(np.count_nonzero(m))

print("=== MASK RELATIONSHIP ===")
print("Common CT slices:", len(common_uids))
print("Kidney foreground pixels:", kidney_pixels)
print("Mass foreground pixels:", mass_pixels)
print("Overlapping pixels:", overlap)
print("Mass pixels outside kidney:", mass_outside_kidney)

if mass_pixels:
    print(
        "Mass outside kidney (%):",
        round(100 * mass_outside_kidney / mass_pixels, 3)
    )

if overlap:
    print(
        "Mass overlapping kidney (%):",
        round(100 * overlap / mass_pixels, 3)
    )
