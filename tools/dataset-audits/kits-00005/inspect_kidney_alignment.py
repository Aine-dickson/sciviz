from pathlib import Path
from collections import defaultdict
import numpy as np
import pydicom
from pydicom.pixels import iter_pixels

ROOT = Path("datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005")

ct_dir = next(ROOT.glob("*/6-arterial-*"))
seg_path = next(ROOT.glob("*/300-Segmentation-*/*.dcm"))

# Index CT instances by SOP Instance UID.
ct = {}

for path in ct_dir.glob("*.dcm"):
    ds = pydicom.dcmread(path, stop_before_pixels=True)
    ct[str(ds.SOPInstanceUID)] = {
        "position": np.array(ds.ImagePositionPatient, dtype=float),
        "rows": ds.Rows,
        "columns": ds.Columns,
    }

seg = pydicom.dcmread(seg_path)
stats = defaultdict(lambda: {
    "frames": 0,
    "nonempty": 0,
    "pixels": 0,
    "ct_refs": set(),
    "z_positions": [],
    "min_row": 512,
    "max_row": -1,
    "min_col": 512,
    "max_col": -1,
    "missing_refs": 0,
    "position_errors": 0,
})

for i, pixels in enumerate(iter_pixels(seg)):
    frame = seg.PerFrameFunctionalGroupsSequence[i]
    number = frame.SegmentIdentificationSequence[0].ReferencedSegmentNumber

    item = stats[number]
    item["frames"] += 1

    sources = [
        source
        for derivation in frame.DerivationImageSequence
        for source in derivation.SourceImageSequence
    ]

    if not sources:
        item["missing_refs"] += 1
        continue

    uid = str(sources[0].ReferencedSOPInstanceUID)
    ct_info = ct.get(uid)

    if ct_info is None:
        item["missing_refs"] += 1
        continue

    item["ct_refs"].add(uid)

    position = np.array(
        frame.PlanePositionSequence[0].ImagePositionPatient,
        dtype=float
    )

    if not np.allclose(position, ct_info["position"], atol=0.0001):
        item["position_errors"] += 1

    mask = pixels != 0
    count = int(np.count_nonzero(mask))

    item["pixels"] += count

    if count:
        item["nonempty"] += 1
        item["z_positions"].append(float(position[2]))

        rows, cols = np.where(mask)
        item["min_row"] = min(item["min_row"], int(rows.min()))
        item["max_row"] = max(item["max_row"], int(rows.max()))
        item["min_col"] = min(item["min_col"], int(cols.min()))
        item["max_col"] = max(item["max_col"], int(cols.max()))

print("=== SEGMENTATION / CT ALIGNMENT ===")

for segment in seg.SegmentSequence:
    number = segment.SegmentNumber
    label = segment.SegmentLabel
    item = stats[number]

    print(f"\n{number}: {label}")
    print("  Frames:", item["frames"])
    print("  Distinct referenced CT instances:", len(item["ct_refs"]))
    print("  Missing references:", item["missing_refs"])
    print("  Position mismatches:", item["position_errors"])
    print("  Non-empty frames:", item["nonempty"])
    print("  Total foreground pixels:", item["pixels"])

    if item["z_positions"]:
        print("  Non-empty Z range:", (
            min(item["z_positions"]),
            max(item["z_positions"])
        ))
        print("  Image row bounds:", (
            item["min_row"], item["max_row"]
        ))
        print("  Image column bounds:", (
            item["min_col"], item["max_col"]
        ))