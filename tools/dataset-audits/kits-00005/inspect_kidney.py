from pathlib import Path
import pydicom

ROOT = Path("datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005")

ct_dir = next(ROOT.glob("*/6-arterial-*"))
seg_path = next(ROOT.glob("*/300-Segmentation-*/*.dcm"))

print("=== CT SERIES ===")
ct_files = sorted(ct_dir.glob("*.dcm"))
first = pydicom.dcmread(ct_files[0], stop_before_pixels=True)

for tag in [
    "Modality", "SeriesDescription", "SeriesInstanceUID",
    "StudyInstanceUID", "Rows", "Columns", "PixelSpacing",
    "SliceThickness", "ImageOrientationPatient",
    "ImagePositionPatient", "RescaleSlope", "RescaleIntercept",
]:
    print(f"{tag}: {getattr(first, tag, 'Not present')}")

print("CT slice files:", len(ct_files))

print("\n=== SEGMENTATION ===")
seg = pydicom.dcmread(seg_path)

for tag in [
    "Modality", "SeriesDescription", "SeriesInstanceUID",
    "Rows", "Columns", "NumberOfFrames",
    "ImageOrientationPatient", "PixelSpacing",
]:
    print(f"{tag}: {getattr(seg, tag, 'Not present')}")

print("\nSegment labels:")
for item in getattr(seg, "SegmentSequence", []):
    print(
        "Number:", item.SegmentNumber,
        "| Label:", item.SegmentLabel,
        "| Algorithm:", getattr(item, "SegmentAlgorithmType", "Unknown")
    )

print("\nReferenced series:")
for ref in getattr(seg, "ReferencedSeriesSequence", []):
    print("Series UID:", ref.SeriesInstanceUID)
    for instance in getattr(ref, "ReferencedInstanceSequence", []):
        print("Referenced CT instances:", len(ref.ReferencedInstanceSequence))
        break
