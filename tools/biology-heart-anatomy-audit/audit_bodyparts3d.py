#!/usr/bin/env python3
"""Audit the official BodyParts3D 4.0 PART-OF archive for the B1 heart challenge.

Uses only the Python standard library. Downloads the official metadata and OBJ
archive into a cache, verifies concept-to-element mappings against the archive,
and reports basic OBJ integrity statistics. It does not extract or modify meshes.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import sys
import urllib.request
import zipfile
from collections import defaultdict
from pathlib import Path, PurePosixPath

BASE_URL = "https://dbarchive.biosciencedbc.jp/data/bodyparts3d/LATEST/"
ARCHIVE_NAME = "partof_BP3D_4.0_obj_99.zip"
SOURCES = {
    "parts": "partof_parts_list_e.txt",
    "elements": "partof_element_parts.txt",
    "relations": "partof_inclusion_relation_list.txt",
}
ATTRIBUTION = (
    "BodyParts3D, © The Database Center for Life Science licensed under "
    "CC Attribution 4.0 International"
)

# FMA concept IDs are stable semantic identifiers. Expected BP representation
# IDs are checked against the official English concept table, not used as OBJ
# filenames; OBJ members are ELEMENT file IDs (FJ...).
REQUIRED = {
    "FMA7088": ("BP9305", "heart"),
    "FMA7096": ("BP9831", "right atrium"),
    "FMA7097": ("BP9454", "left atrium"),
    "FMA7098": ("BP9598", "right ventricle"),
    "FMA7101": ("BP9448", "left ventricle"),
    "FMA3734": ("BP10374", "aorta"),
    "FMA4720": ("BP10386", "superior vena cava"),
    "FMA10951": ("BP10397", "inferior vena cava"),
    "FMA8612": ("BP9912", "pulmonary trunk"),
    "FMA50872": ("BP9427", "right pulmonary artery"),
    "FMA50873": ("BP9414", "left pulmonary artery"),
    "FMA49914": ("BP9365", "right superior pulmonary vein"),
    "FMA49911": ("BP9407", "right inferior pulmonary vein"),
    "FMA49916": ("BP9499", "left superior pulmonary vein"),
    "FMA49913": ("BP9712", "left inferior pulmonary vein"),
    "FMA7234": ("BP9830", "tricuspid valve"),
    "FMA7246": ("BP9868", "pulmonary valve"),
    "FMA7235": ("BP9455", "mitral valve"),
    "FMA7236": ("BP9447", "aortic valve"),
}


def download(url: str, path: Path) -> None:
    if path.exists() and path.stat().st_size:
        print(f"Using cached {path} ({path.stat().st_size:,} bytes)")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(
        url, headers={"User-Agent": "SVF-B1-BodyParts3D-Audit/1.0"}
    )
    print(f"Downloading {url}")
    with urllib.request.urlopen(request, timeout=60) as response, path.open("wb") as out:
        total = response.headers.get("Content-Length")
        total_bytes = int(total) if total and total.isdigit() else None
        written = 0
        while True:
            chunk = response.read(1024 * 1024)
            if not chunk:
                break
            out.write(chunk)
            written += len(chunk)
            if total_bytes:
                print(f"\r  {written:,}/{total_bytes:,} bytes", end="", flush=True)
    print()


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8-sig", newline="") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_obj(stream) -> dict:
    vertices: list[tuple[float, float, float]] = []
    face_count = 0
    triangle_count = 0
    invalid_faces = 0
    degenerate_triangles = 0
    min_xyz = [math.inf, math.inf, math.inf]
    max_xyz = [-math.inf, -math.inf, -math.inf]

    for raw in stream:
        line = raw.decode("utf-8", errors="replace").strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if parts[0] == "v":
            if len(parts) < 4:
                continue
            try:
                xyz = (float(parts[1]), float(parts[2]), float(parts[3]))
            except ValueError:
                continue
            if not all(math.isfinite(v) for v in xyz):
                continue
            vertices.append(xyz)
            for axis, value in enumerate(xyz):
                min_xyz[axis] = min(min_xyz[axis], value)
                max_xyz[axis] = max(max_xyz[axis], value)
        elif parts[0] == "f":
            face_count += 1
            tokens = parts[1:]
            indices = []
            for token in tokens:
                try:
                    raw_index = int(token.split("/", 1)[0])
                except (ValueError, IndexError):
                    indices.append(None)
                    continue
                index = raw_index - 1 if raw_index > 0 else len(vertices) + raw_index
                indices.append(index)
            if len(indices) < 3 or any(
                i is None or i < 0 or i >= len(vertices) for i in indices
            ):
                invalid_faces += 1
                continue
            triangle_count += len(indices) - 2
            anchor = vertices[indices[0]]
            for i in range(1, len(indices) - 1):
                b = vertices[indices[i]]
                c = vertices[indices[i + 1]]
                ab = tuple(b[j] - anchor[j] for j in range(3))
                ac = tuple(c[j] - anchor[j] for j in range(3))
                cross = (
                    ab[1] * ac[2] - ab[2] * ac[1],
                    ab[2] * ac[0] - ab[0] * ac[2],
                    ab[0] * ac[1] - ab[1] * ac[0],
                )
                if sum(v * v for v in cross) <= 1e-24:
                    degenerate_triangles += 1

    if not vertices:
        bounds = None
    else:
        bounds = {"min": min_xyz, "max": max_xyz}
    return {
        "vertices": len(vertices),
        "faces": face_count,
        "triangles_fan_estimate": triangle_count,
        "invalid_faces": invalid_faces,
        "degenerate_triangles_fan_estimate": degenerate_triangles,
        "bounds": bounds,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=Path(".cache/bodyparts3d-4.0"),
        help="download/cache directory (default: .cache/bodyparts3d-4.0)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("bodyparts3d-heart-audit.json"),
        help="JSON report path (default: bodyparts3d-heart-audit.json)",
    )
    args = parser.parse_args()
    args.cache_dir.mkdir(parents=True, exist_ok=True)

    local = {}
    for key, filename in SOURCES.items():
        path = args.cache_dir / filename
        download(BASE_URL + filename, path)
        local[key] = path
    archive_path = args.cache_dir / ARCHIVE_NAME
    download(BASE_URL + ARCHIVE_NAME, archive_path)

    parts_rows = read_tsv(local["parts"])
    element_rows = read_tsv(local["elements"])
    relation_rows = read_tsv(local["relations"])

    parts = {row["concept id"]: row for row in parts_rows}
    elements_by_concept: dict[str, list[str]] = defaultdict(list)
    for row in element_rows:
        elements_by_concept[row["concept id"]].append(row["element file id"])

    concept_report = {}
    expected_ids = set()
    inverse: dict[str, list[str]] = defaultdict(list)
    for fma_id, (expected_bp, expected_name) in REQUIRED.items():
        row = parts.get(fma_id)
        if row is None:
            concept_report[fma_id] = {
                "expected_name": expected_name,
                "status": "MISSING_FROM_CONCEPT_TABLE",
            }
            continue
        bp_id = row["representation id"]
        name = row["en"]
        element_ids = sorted(set(elements_by_concept.get(fma_id, [])))
        expected_ids.update(element_ids)
        for element_id in element_ids:
            inverse[element_id].append(fma_id)
        concept_report[fma_id] = {
            "name_in_source": name,
            "representation_id": bp_id,
            "expected_representation_id": expected_bp,
            "representation_id_matches": bp_id == expected_bp,
            "element_file_ids": element_ids,
            "element_count": len(element_ids),
            "status": "MAPPED" if element_ids else "NO_ELEMENT_MAPPING",
        }

    with zipfile.ZipFile(archive_path) as archive:
        obj_members = {}
        for info in archive.infolist():
            if info.is_dir() or not info.filename.lower().endswith(".obj"):
                continue
            stem = PurePosixPath(info.filename).stem
            obj_members[stem] = info.filename
        missing_meshes = sorted(expected_ids - set(obj_members))
        mesh_report = {}
        for element_id in sorted(expected_ids & set(obj_members)):
            member = obj_members[element_id]
            with archive.open(member) as stream:
                mesh_report[element_id] = parse_obj(stream)

    shared = {
        element_id: sorted(concepts)
        for element_id, concepts in inverse.items()
        if len(set(concepts)) > 1
    }
    report = {
        "source": {
            "dataset": "BodyParts3D 4.0 PART-OF tree",
            "archive": ARCHIVE_NAME,
            "archive_bytes": archive_path.stat().st_size,
            "archive_sha256": sha256_file(archive_path),
            "attribution": ATTRIBUTION,
            "license": "CC BY 4.0",
            "source_base_url": BASE_URL,
        },
        "archive_obj_member_count": len(obj_members),
        "metadata_row_counts": {
            "concepts": len(parts_rows),
            "concept_element_mappings": len(element_rows),
            "inclusion_relationships": len(relation_rows),
        },
        "required_concepts": concept_report,
        "unique_required_element_ids": len(expected_ids),
        "missing_required_obj_ids": missing_meshes,
        "shared_element_assignments": shared,
        "mesh_statistics": mesh_report,
        "limitations": [
            "This script verifies archive membership and basic OBJ integrity only.",
            "OBJ coordinates have no units or anatomical axis labels in the OBJ syntax itself.",
            "Confirm native units, axis directions, and anatomical left/right from primary source documentation before transforming coordinates.",
            "A concept-to-element mapping is not proof that each concept has a distinct, non-overlapping surface.",
            "Fan-triangulated polygon statistics are estimates; no mesh repair or geometric transformation is performed.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    print(f"Archive OBJ members: {len(obj_members)}")
    print(f"Required concepts: {len(REQUIRED)}")
    print(f"Unique required element IDs: {len(expected_ids)}")
    print(f"Missing OBJ files: {len(missing_meshes)}")
    print(f"Shared element assignments: {len(shared)}")
    for fma_id, item in concept_report.items():
        print(
            f"{fma_id:9} {item.get('name_in_source', item.get('expected_name')):38} "
            f"BP={item.get('representation_id', 'MISSING'):7} "
            f"elements={item.get('element_count', 0):2} "
            f"status={item['status']}"
        )
    if missing_meshes:
        print("Missing OBJ IDs:", ", ".join(missing_meshes))
    print(f"Report: {args.output}")
    print("NOTE: units and anatomical axis directions remain unverified by this script.")
    return 1 if missing_meshes or any(
        item["status"] != "MAPPED" or not item.get("representation_id_matches", False)
        for item in concept_report.values()
    ) else 0


if __name__ == "__main__":
    sys.exit(main())
