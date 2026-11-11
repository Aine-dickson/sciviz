#!/usr/bin/env python3
"""Glyph-level verification of an SVF fibre-diagnostic glyph export.

Standard library only. Implements docs/heart-fibre-orientation.md section
7.2 ("Glyph-level verification"). From the original, unmodified VTK file
alone (with audit.py's own parser, declared-type conversion and boundary),
it recomputes the expected glyph set -- for every occupied bin, the eligible
triangle with the lowest expected surface index -- and checks the exported
glyphs against it. It never trusts which glyphs SVF chose to export.

Usage:
    python -I tools/rodero-fibre-audit/verify_glyphs.py <01.vtk> <glyphs.csv>
Exit status: 0 when every check passes, 1 on any discrepancy, 2 on bad input.
"""

from __future__ import annotations

import csv
import math
import platform
import struct
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import audit  # noqa: E402

GLYPH_SCHEMA = "svf-b1.2-glyphs/1"
MIDPOINT_TOLERANCE_MM = 1.0e-9
DISPLACEMENT_RELATIVE_TOLERANCE = 1.0e-12
THETA_TOLERANCE_DEG = 1.0e-9
COLUMNS = [
    "surface_index", "p0", "p1", "p2", "cell", "tag", "bin_x", "bin_y", "bin_z",
    "fibre_x", "fibre_y", "fibre_z", "theta_deg",
    "start_x", "start_y", "start_z", "end_x", "end_y", "end_z",
]


class InputError(Exception):
    pass


def bits(value: float) -> bytes:
    return struct.pack("<d", value)


def read_glyphs(path: str) -> tuple[dict, list[dict]]:
    with open(path, encoding="utf-8", newline="") as handle:
        first = handle.readline().rstrip("\r\n")
        words = first.split()
        if len(words) < 2 or words[0] != "#" or words[1] != GLYPH_SCHEMA:
            raise InputError(f"first line must start with '# {GLYPH_SCHEMA}'")
        header = {}
        for word in words[2:]:
            key, sep, value = word.partition("=")
            if not sep or key in header:
                raise InputError(f"malformed header field {word!r}")
            header[key] = value
        required = {"source_sha256", "input_sha256", "tags", "bin_mm", "length_mm", "glyphs"}
        if missing := sorted(required - header.keys()):
            raise InputError(f"header lacks {', '.join(missing)}")
        reader = csv.DictReader(handle)
        if reader.fieldnames != COLUMNS:
            raise InputError(f"unexpected columns {reader.fieldnames}")
        rows = list(reader)
    try:
        header["tags"] = sorted({int(t) for t in header["tags"].split(",")})
        header["bin_mm"] = float(header["bin_mm"])
        header["length_mm"] = float(header["length_mm"])
        header["glyphs"] = int(header["glyphs"])
        parsed = []
        for row in rows:
            out = {k: int(row[k]) for k in COLUMNS[:9]}
            out.update({k: float(row[k]) for k in COLUMNS[9:]})
            parsed.append(out)
    except ValueError as error:
        raise InputError(f"unparseable value: {error}") from None
    return header, parsed


def expected_glyphs(data: dict, tags: list[int], bin_mm: float) -> dict:
    """Recompute boundary ranks, eligibility, centroids, bins and winners."""
    points, conn = data["points"], data["conn"]
    ids, fibres = data["cell_data"]["ID"], data["cell_data"]["fibres"]
    boundary, _ = audit.boundary_faces(conn, data["cell_count"], data["point_count"])
    tag_set = {str(t) for t in tags}

    def point(i):
        return (points[3 * i], points[3 * i + 1], points[3 * i + 2])

    by_key = {}  # sorted point-ID triple -> (rank, cell)
    eligible = {}  # rank -> per-triangle expectations
    winners = {}  # bin -> rank of the lowest eligible rank
    near_face = set()
    for rank, (key, cell, local) in enumerate(boundary):
        by_key[key] = (rank, cell)
        tag = audit.tag_key(ids[cell])
        if tag not in tag_set:
            continue
        a, b, c = (point(conn[4 * cell + k]) for k in audit.TET_FACES[local])
        normal = audit.cross(audit.sub(b, a), audit.sub(c, a))
        normal_length = audit.norm(normal)
        if normal_length == 0.0 or not math.isfinite(normal_length):
            continue
        f = (fibres[3 * cell], fibres[3 * cell + 1], fibres[3 * cell + 2])
        f_length = audit.norm(f)
        if not audit.finite(f) or f_length == 0.0 or not math.isfinite(f_length):
            continue
        n_hat = audit.scale(normal, 1.0 / normal_length)
        f_hat = audit.scale(f, 1.0 / f_length)
        cos = abs(audit.dot(f_hat, n_hat))
        centroid = ((a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0)
        index = tuple(math.floor(x / bin_mm) for x in centroid)
        for x, k in zip(centroid, index):
            r = x - k * bin_mm
            if r < audit.NEAR_FACE_MM or bin_mm - r < audit.NEAR_FACE_MM:
                near_face.add(rank)
                break
        eligible[rank] = {
            "cell": cell, "tag": int(tag), "bin": index, "centroid": centroid, "fibre": f,
            "f_hat": f_hat, "theta": math.degrees(math.acos(min(1.0, cos))),
        }
        winners.setdefault(index, rank)
    return {"by_key": by_key, "eligible": eligible, "winners": winners, "near_face": near_face}


def verify(vtk_path: str, glyph_path: str) -> dict:
    header, rows = read_glyphs(glyph_path)
    digest = audit.sha256(vtk_path)
    if digest != header["source_sha256"]:
        return {"fatal": f"source_sha256 mismatch: {vtk_path} hashes to {digest}, "
                         f"the glyph header says {header['source_sha256']}"}
    data = audit.parse_vtk(vtk_path)  # also refuses any non-tetrahedral cell
    expected = expected_glyphs(data, header["tags"], header["bin_mm"])
    eligible, winners, near_face = expected["eligible"], expected["winners"], expected["near_face"]
    length = header["length_mm"]
    tags = set(header["tags"])

    problems: dict[str, list[str]] = {k: [] for k in (
        "header", "missing", "unexpected", "wrong_winner", "duplicate_bin", "row")}

    def flag(rank) -> str:
        return " [near-face centroid]" if rank in near_face else ""

    if header["glyphs"] != len(rows):
        problems["header"].append(f"header says {header['glyphs']} glyphs, file has {len(rows)} rows")

    by_bin: dict[tuple, list[dict]] = {}
    for row in rows:
        by_bin.setdefault((row["bin_x"], row["bin_y"], row["bin_z"]), []).append(row)
    for b, rank in sorted(winners.items()):
        if b not in by_bin:
            problems["missing"].append(f"bin {b}: expected winner rank {rank}{flag(rank)}, no glyph exported")
    for b, group in sorted(by_bin.items()):
        ranks = [row["surface_index"] for row in group]
        if len(group) > 1:
            problems["duplicate_bin"].append(f"bin {b}: {len(group)} glyphs, surface indices {ranks}")
        if b not in winners:
            problems["unexpected"].append(f"bin {b}: no eligible triangle expected, glyph surface indices {ranks}"
                                          + "".join(flag(r) for r in ranks))
            continue
        for rank in ranks:
            if rank != winners[b]:
                problems["wrong_winner"].append(
                    f"bin {b}: glyph surface index {rank}{flag(rank)}, expected winner "
                    f"{winners[b]}{flag(winners[b])}")

    for row in rows:
        rank, where = row["surface_index"], f"surface index {row['surface_index']}"
        key = (row["p0"], row["p1"], row["p2"])
        found = expected["by_key"].get(key)
        if found is None:
            problems["row"].append(f"{where}: point IDs {key} are not a boundary face")
            continue
        if found != (rank, row["cell"]):
            problems["row"].append(f"{where}: face {key} is rank {found[0]} of cell {found[1]}, "
                                   f"row says rank {rank} of cell {row['cell']}")
            continue
        if row["tag"] not in tags:
            problems["row"].append(f"{where}: tag {row['tag']} is outside the tag set")
        e = eligible.get(rank)
        if e is None:
            problems["row"].append(f"{where}: not an eligible triangle (tag or section 4.6 exclusion)")
            continue
        if row["tag"] != e["tag"]:
            problems["row"].append(f"{where}: tag {row['tag']}, source cell has {e['tag']}")
        if (row["bin_x"], row["bin_y"], row["bin_z"]) != e["bin"]:
            problems["row"].append(f"{where}: bin {(row['bin_x'], row['bin_y'], row['bin_z'])}, "
                                   f"expected {e['bin']}{flag(rank)}")
        fibre = (row["fibre_x"], row["fibre_y"], row["fibre_z"])
        if any(bits(x) != bits(y) for x, y in zip(fibre, e["fibre"])):
            problems["row"].append(f"{where}: fibre {fibre} differs from the source {e['fibre']} bit for bit")
        start = (row["start_x"], row["start_y"], row["start_z"])
        end = (row["end_x"], row["end_y"], row["end_z"])
        midpoint = tuple((s + t) / 2.0 for s, t in zip(start, end))
        offset = max(abs(m - c) for m, c in zip(midpoint, e["centroid"]))
        if not offset <= MIDPOINT_TOLERANCE_MM:
            problems["row"].append(f"{where}: midpoint is {offset!r} mm from the centroid")
        displacement = audit.sub(end, start)
        error = audit.norm(audit.sub(displacement, audit.scale(e["f_hat"], length)))
        if not error <= DISPLACEMENT_RELATIVE_TOLERANCE * length:
            problems["row"].append(f"{where}: end - start differs from l*f_hat by {error!r}")
        if not abs(row["theta_deg"] - e["theta"]) <= THETA_TOLERANCE_DEG:
            problems["row"].append(f"{where}: theta {row['theta_deg']!r}, expected {e['theta']!r}")

    return {
        "header": header,
        "rows": len(rows),
        "expected_bins": len(winners),
        "eligible": len(eligible),
        "near_face": sorted(near_face),
        "problems": problems,
    }


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    start = time.perf_counter()
    try:
        report = verify(argv[0], argv[1])
    except (OSError, InputError, audit.VtkError) as error:
        print(f"verify_glyphs failed: {error}", file=sys.stderr)
        return 2
    if "fatal" in report:
        print(f"FAIL: {report['fatal']}")
        return 1
    problems = report["problems"]
    print(f"source_sha256 matches: {report['header']['source_sha256']}")
    print(f"input_sha256 (not checked): {report['header']['input_sha256']}")
    print(f"eligible triangles: {report['eligible']}; expected glyphs (occupied bins): "
          f"{report['expected_bins']}; exported rows: {report['rows']}")
    print(f"near-face centroids (reported, no allowance): {len(report['near_face'])} {report['near_face']}")
    for kind, items in problems.items():
        print(f"{kind}: {len(items)}")
        for item in items:
            print(f"  {item}")
    peak, method = audit.peak_memory()
    print(f"runtime {time.perf_counter() - start:.1f} s; peak memory {peak} bytes ({method}); "
          f"Python {platform.python_version()}")
    total = sum(len(items) for items in problems.values())
    print("PASS: every glyph verified" if total == 0 else f"FAIL: {total} discrepancy(ies)")
    return 0 if total == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
