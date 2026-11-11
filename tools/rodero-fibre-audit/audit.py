#!/usr/bin/env python3
"""Independent B1.2 fibre audit of the original Rodero et al. (2021) VTK file.

Standard library only. Reads the original, unmodified legacy ASCII VTK file
with its own parser, derives the exterior boundary from tetrahedral face
incidence, and writes the statistics set defined in
docs/heart-fibre-orientation.md (sections 4-7) as JSON. It never reads SVF
output or SVF's extracted boundary.

Usage:
    python -I tools/rodero-fibre-audit/audit.py <01.vtk> --tags 1,2 \
        --lv-tag 1 --bin 4 --out <stats.json>
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import platform
import re
import struct
import sys
import time
from array import array
from collections import Counter
from fractions import Fraction

SCHEMA = "svf-b1.2-fibre-stats/1"
EPSILON = 1.0e-6  # section 4.6, on dimensionless unit-vector quantities
NEAR_FACE_MM = 1.0e-9  # section 7.2
ENDO_MAX = 0.2  # mean rho < 0.2 -> endocardial (section 5)
EPI_MIN = 0.8  # mean rho > 0.8 -> epicardial
APEX_Z_MAX = 0.02  # LV axis: apex points have Z < 0.02 (section 4.5)
BASE_Z_MIN = 0.98  # base points have Z > 0.98
LV_V = -1.0  # source: V is -1 in the LV and +1 in the RV
SENTINEL = -10.0  # source: non-ventricular labels use -10 for all UVCs
VTK_TETRA = 10

# Tetrahedron4 reference face cycles (docs/volume-boundary-extraction.md).
# Only the cycle order of the vertices matters here, for the cross product
# and the centroid sum; no step depends on the normal's sign.
TET_FACES = ((1, 2, 3), (0, 3, 2), (0, 1, 3), (0, 2, 1))


class VtkError(Exception):
    pass


# ---------------------------------------------------------------- parsing


class Lines:
    def __init__(self, handle):
        self.handle = handle
        self.number = 0

    def next(self) -> str:
        line = self.handle.readline()
        if not line:
            raise VtkError(f"unexpected end of file after line {self.number}")
        self.number += 1
        return line

    def next_nonblank(self) -> str | None:
        while True:
            line = self.handle.readline()
            if not line:
                return None
            self.number += 1
            if line.strip():
                return line


# Numeric input contract (design section 7.2): each array is converted
# according to its declared VTK type before any calculation, exactly as SVF's
# parser does. The baseline records this so it cannot silently revert.
NUMERIC_CONVERSION = "vtk-declared-type/1: float=binary32 promoted to f64; double=binary64; integer types exact"
F32_MAX = 3.4028234663852886e38
F32_OVERFLOW_TIE = 2.0**128 - 2.0**103  # midpoint between F32_MAX and 2**128


def _f32_bits(value: float) -> int:
    return struct.unpack("<I", struct.pack("<f", value))[0]


def _from_f32_bits(bits: int) -> float:
    return struct.unpack("<f", struct.pack("<I", bits))[0]


def binary32(text: str) -> float:
    """Decimal text -> nearest IEEE-754 binary32 (ties to even), as an f64.

    This is the conversion SVF applies to a VTK `float` value (Rust's
    correctly rounded `str::parse::<f32>`). Rounding the text to f64 first and
    then to binary32 gives the same result unless the f64 lands exactly on
    the midpoint between two binary32 values: every such midpoint is itself
    an f64, so no midpoint can lie strictly between the true value and its
    nearest f64. That one case is resolved with exact rational arithmetic.
    """
    d = _decimal(text)
    try:
        f = _from_f32_bits(_f32_bits(d))
    except OverflowError:  # the f64 rounds beyond the binary32 range
        if abs(d) == F32_OVERFLOW_TIE and abs(Fraction(text)) < F32_OVERFLOW_TIE:
            return math.copysign(F32_MAX, d)
        return math.copysign(math.inf, d)
    if d == f or not math.isfinite(d):
        return f
    # The binary32 neighbour of f on the side of d. Bits are sign-magnitude,
    # so +1 moves away from zero for either sign (and from ±0 to ±2**-149).
    bits = _f32_bits(f)
    other = _from_f32_bits(bits + 1 if abs(d) > abs(f) else bits - 1)
    if not math.isfinite(other):  # f is F32_MAX and d is above it, below the overflow tie
        return f
    midpoint = (f + other) / 2.0  # exact in f64
    if d != midpoint:
        return f
    exact, tie = Fraction(text), Fraction(midpoint)
    if exact == tie:
        return f  # a true tie: struct already rounded it to even
    # Pick whichever neighbour is on the same side of the midpoint as the text.
    return other if (exact > tie) == (other > midpoint) else f


def _decimal(text: str) -> float:
    """Decimal text -> nearest binary64, accepting only what Rust's parser accepts.

    Python's float() also takes underscores ("1_0") and non-ASCII digits,
    which SVF rejects, so those are refused here rather than read silently.
    """
    if not text.isascii() or "_" in text:
        raise VtkError(f"invalid floating-point value {text!r}")
    try:
        return float(text)
    except ValueError:
        raise VtkError(f"invalid floating-point value {text!r}") from None


# Declared integer types and their ranges, as SVF parses them (Rust i16, u16,
# i32, u32, u8). Values outside the range are errors, not wider integers.
INTEGER_RANGES = {
    "short": (-(2**15), 2**15 - 1),
    "unsigned_short": (0, 2**16 - 1),
    "int": (-(2**31), 2**31 - 1),
    "unsigned_int": (0, 2**32 - 1),
    "unsigned_char": (0, 2**8 - 1),
}
SIGNED_INTEGER = re.compile(r"[+-]?[0-9]+")
UNSIGNED_INTEGER = re.compile(r"\+?[0-9]+")  # Rust's unsigned parse has no '-', not even "-0"


def _integer_converter(declared: str):
    low, high = INTEGER_RANGES[declared]
    pattern = SIGNED_INTEGER if low < 0 else UNSIGNED_INTEGER

    def convert(text: str) -> float:
        if not pattern.fullmatch(text):
            raise VtkError(f"invalid {declared} value {text!r}")
        value = int(text)
        if not low <= value <= high:
            raise VtkError(f"{declared} value {text!r} is outside [{low}, {high}]")
        return float(value)

    return convert


CONVERTERS = {
    "float": binary32,
    "double": _decimal,
    **{declared: _integer_converter(declared) for declared in INTEGER_RANGES},
}


def converter(declared: str, line: int):
    try:
        return CONVERTERS[declared]
    except KeyError:
        raise VtkError(f"line {line}: unsupported data type {declared!r}") from None


def read_values(lines: Lines, count: int, typecode: str, convert) -> array:
    """Read exactly `count` whitespace-separated values, however they wrap."""
    out = array(typecode)
    while len(out) < count:
        out.extend(map(convert, lines.next().split()))
    if len(out) != count:
        raise VtkError(f"line {lines.number}: record overruns its declared size")
    return out


def parse_vtk(path: str) -> dict:
    with open(path, encoding="ascii") as handle:
        lines = Lines(handle)
        if not lines.next().startswith("# vtk DataFile Version"):
            raise VtkError("missing legacy VTK header")
        lines.next()  # title
        if lines.next().strip() != "ASCII":
            raise VtkError("only ASCII legacy VTK is supported")
        if lines.next().split() != ["DATASET", "UNSTRUCTURED_GRID"]:
            raise VtkError("expected DATASET UNSTRUCTURED_GRID")

        points = conn = cell_types = None
        cell_count = None
        cell_data: dict[str, array] = {}
        point_data: dict[str, array] = {}
        target = None
        target_size = None

        while (line := lines.next_nonblank()) is not None:
            words = line.split()
            keyword = words[0]
            if keyword == "POINTS":
                n = int(words[1])
                points = read_values(lines, 3 * n, "d", converter(words[2], lines.number))
            elif keyword == "CELLS":
                n, size = int(words[1]), int(words[2])
                raw = read_values(lines, size, "l", int)
                if size != 5 * n or any(raw[i] != 4 for i in range(0, size, 5)):
                    raise VtkError("audit requires every cell to have 4 points")
                conn = array("l", (raw[i] for i in range(size) if i % 5))
                cell_count = n
            elif keyword == "CELL_TYPES":
                n = int(words[1])
                cell_types = read_values(lines, n, "l", int)
            elif keyword in ("CELL_DATA", "POINT_DATA"):
                target = cell_data if keyword == "CELL_DATA" else point_data
                target_size = int(words[1])
            elif keyword == "SCALARS":
                if target is None or (len(words) > 3 and words[3] != "1"):
                    raise VtkError(f"line {lines.number}: unsupported SCALARS")
                if lines.next().split() != ["LOOKUP_TABLE", "default"]:
                    raise VtkError(f"line {lines.number}: expected LOOKUP_TABLE default")
                target[words[1]] = read_values(lines, target_size, "d", converter(words[2], lines.number))
            elif keyword == "VECTORS":
                if target is None:
                    raise VtkError(f"line {lines.number}: VECTORS outside a data section")
                target[words[1]] = read_values(lines, 3 * target_size, "d", converter(words[2], lines.number))
            else:
                raise VtkError(f"line {lines.number}: unsupported record {keyword}")

    if points is None or conn is None or cell_types is None:
        raise VtkError("missing POINTS, CELLS or CELL_TYPES")
    if len(cell_types) != cell_count:
        raise VtkError("CELL_TYPES count differs from CELLS count")
    if any(t != VTK_TETRA for t in cell_types):
        raise VtkError("audit requires an all-tetrahedron mesh (VTK type 10)")
    point_count = len(points) // 3
    if any(not 0 <= p < point_count for p in conn):
        raise VtkError("connectivity references a missing point")
    return {
        "points": points,
        "conn": conn,
        "cell_count": cell_count,
        "point_count": point_count,
        "cell_data": cell_data,
        "point_data": point_data,
    }


# ---------------------------------------------------------------- boundary


def boundary_faces(conn: array, cell_count: int, point_count: int):
    """Return (boundary, incidence histogram).

    Every tetrahedral face is keyed by its sorted point-ID triple, packed into
    one integer that orders lexicographically. Faces occurring exactly once
    are boundary faces. The result is sorted by key, so a face's position in
    it is its rank: the expected triangulated-surface index (section 7.2).
    Each entry is (sorted point-ID triple, source cell index, local face).
    """
    n = point_count
    stride = 4 * cell_count
    items = []
    append = items.append
    for cell in range(cell_count):
        base = 4 * cell
        p = conn[base : base + 4]
        for local, (i, j, k) in enumerate(TET_FACES):
            a, b, c = sorted((p[i], p[j], p[k]))
            append(((a * n + b) * n + c) * stride + base + local)
    items.sort()

    boundary = []
    histogram: Counter = Counter()
    index, total = 0, len(items)
    while index < total:
        key = items[index] // stride
        end = index + 1
        while end < total and items[end] // stride == key:
            end += 1
        histogram[end - index] += 1
        if end - index == 1:
            rest = items[index] % stride
            ab, c = divmod(key, n)
            a, b = divmod(ab, n)
            boundary.append(((a, b, c), rest // 4, rest % 4))
        index = end
    return boundary, histogram


# ---------------------------------------------------------------- statistics


def percentile(sorted_values: list[float], p: float):
    """Nearest rank: index round((n - 1) p), with halves rounded up."""
    if not sorted_values:
        return None
    return sorted_values[math.floor((len(sorted_values) - 1) * p + 0.5)]


def tag_key(value: float) -> str:
    if value != math.floor(value) or not math.isfinite(value):
        raise VtkError(f"non-integer ID tag {value!r}")
    return str(int(value))


def sub(u, v):
    return (u[0] - v[0], u[1] - v[1], u[2] - v[2])


def cross(u, v):
    return (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])


def dot(u, v):
    return u[0] * v[0] + u[1] * v[1] + u[2] * v[2]


def norm(u):
    return math.sqrt(dot(u, u))


def scale(u, s):
    return (u[0] * s, u[1] * s, u[2] * s)


def finite(u):
    return all(math.isfinite(x) for x in u)


def lv_axis(points: array, z: array, v: array) -> dict:
    apex = [0.0, 0.0, 0.0]
    base = [0.0, 0.0, 0.0]
    n_apex = n_base = 0
    for i in range(len(z)):
        if v[i] != LV_V or z[i] == SENTINEL:
            continue
        if z[i] < APEX_Z_MAX:
            target, n_apex = apex, n_apex + 1
        elif z[i] > BASE_Z_MIN:
            target, n_base = base, n_base + 1
        else:
            continue
        for k in range(3):
            target[k] += points[3 * i + k]
    if n_apex == 0 or n_base == 0:
        raise VtkError("cannot define the LV axis: no apex or base points")
    axis = tuple(base[k] / n_base - apex[k] / n_apex for k in range(3))
    return {"vector_mm": list(axis), "length_mm": norm(axis), "apex_points": n_apex, "base_points": n_base}


def audit(path: str, tags: list[int], lv_tag: int, bin_mm: float) -> dict:
    data = parse_vtk(path)
    points, conn = data["points"], data["conn"]
    for name, store in (("ID", data["cell_data"]), ("fibres", data["cell_data"])):
        if name not in store:
            raise VtkError(f"missing cell field {name}")
    for name in ("RHO.dat", "Z.dat", "V.dat"):
        if name not in data["point_data"]:
            raise VtkError(f"missing point field {name}")
    ids, fibres = data["cell_data"]["ID"], data["cell_data"]["fibres"]
    rho = data["point_data"]["RHO.dat"]

    axis = lv_axis(points, data["point_data"]["Z.dat"], data["point_data"]["V.dat"])
    axis_unit = scale(tuple(axis["vector_mm"]), 1.0 / axis["length_mm"])

    boundary, histogram = boundary_faces(conn, data["cell_count"], data["point_count"])

    tag_set = {str(t) for t in tags}
    lv = str(lv_tag)
    boundary_by_tag: Counter = Counter()
    selected_by_tag: Counter = Counter()
    eligible_by_tag: Counter = Counter()
    exclusions = Counter(
        {"zero_area": 0, "invalid_fibre": 0, "helix_zero_projection": 0, "helix_degenerate_circumferential": 0}
    )
    abs_cos: dict[str, dict[str, list[float]]] = {t: {"endo": [], "epi": [], "other": []} for t in tag_set}
    alpha: dict[str, list[float]] = {"endo": [], "epi": [], "other": []}
    sentinel_rho_triangles = 0
    bins: dict[tuple[int, int, int], str] = {}
    near_face = 0

    def point(i):
        return (points[3 * i], points[3 * i + 1], points[3 * i + 2])

    for (_key, cell, local) in boundary:
        tag = tag_key(ids[cell])
        boundary_by_tag[tag] += 1
        if tag not in tag_set:
            continue
        selected_by_tag[tag] += 1

        cycle = [conn[4 * cell + k] for k in TET_FACES[local]]
        a, b, c = (point(i) for i in cycle)
        normal = cross(sub(b, a), sub(c, a))
        normal_length = norm(normal)
        if normal_length == 0.0 or not math.isfinite(normal_length):
            exclusions["zero_area"] += 1
            continue
        f = (fibres[3 * cell], fibres[3 * cell + 1], fibres[3 * cell + 2])
        f_length = norm(f)
        if not finite(f) or f_length == 0.0 or not math.isfinite(f_length):
            exclusions["invalid_fibre"] += 1
            continue
        eligible_by_tag[tag] += 1
        n_hat = scale(normal, 1.0 / normal_length)
        f_hat = scale(f, 1.0 / f_length)
        cos = abs(dot(f_hat, n_hat))

        rhos = [rho[i] for i in cycle]
        if any(r == SENTINEL for r in rhos):
            sentinel_rho_triangles += 1
        mean_rho = (rhos[0] + rhos[1] + rhos[2]) / 3.0
        group = "endo" if mean_rho < ENDO_MAX else "epi" if mean_rho > EPI_MIN else "other"
        abs_cos[tag][group].append(cos)

        if tag == lv:
            t = sub(f_hat, scale(n_hat, dot(f_hat, n_hat)))
            t_length = norm(t)
            circ = cross(axis_unit, n_hat)
            circ_length = norm(circ)
            if t_length <= EPSILON:
                exclusions["helix_zero_projection"] += 1
            elif circ_length <= EPSILON:
                exclusions["helix_degenerate_circumferential"] += 1
            else:
                value = abs(dot(t, circ)) / (t_length * circ_length)
                alpha[group].append(math.degrees(math.acos(min(1.0, value))))

        centroid = ((a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0)
        index = tuple(math.floor(x / bin_mm) for x in centroid)
        for x, k in zip(centroid, index):
            r = x - k * bin_mm
            if r < NEAR_FACE_MM or bin_mm - r < NEAR_FACE_MM:
                near_face += 1
                break
        # Boundary is in rank order, so the first triangle seen in a bin has
        # the lowest expected surface index: it is the bin's winner.
        bins.setdefault(index, tag)

    groups = {}
    all_cos = []
    for tag in sorted(tag_set, key=int):
        groups[tag] = {}
        for group, values in abs_cos[tag].items():
            values.sort()
            all_cos.extend(values)
            entry = {
                "count": len(values),
                "abs_cos_p50": percentile(values, 0.50),
                "abs_cos_p90": percentile(values, 0.90),
                "abs_cos_p99": percentile(values, 0.99),
            }
            if tag == lv:
                angles = sorted(alpha[group])
                entry.update(
                    {
                        "alpha_count": len(angles),
                        "alpha_deg_p25": percentile(angles, 0.25),
                        "alpha_deg_p50": percentile(angles, 0.50),
                        "alpha_deg_p75": percentile(angles, 0.75),
                    }
                )
            groups[tag][group] = entry

    eligible = len(all_cos)
    lt01 = sum(1 for x in all_cos if x < 0.1)
    lt02 = sum(1 for x in all_cos if x < 0.2)
    glyphs_by_tag = Counter(bins.values())

    def ordered(counter):
        return {k: counter[k] for k in sorted(counter, key=int)}

    return {
        "schema": SCHEMA,
        "parameters": {
            "tags": sorted(tags),
            "lv_tag": lv_tag,
            "bin_mm": bin_mm,
            "grid_origin_mm": [0.0, 0.0, 0.0],
            "epsilon": EPSILON,
            "near_face_mm": NEAR_FACE_MM,
            "endo_rho_max": ENDO_MAX,
            "epi_rho_min": EPI_MIN,
            "numeric_conversion": NUMERIC_CONVERSION,
        },
        "mesh": {"points": data["point_count"], "cells": data["cell_count"]},
        "boundary_triangles": len(boundary),
        "boundary_by_tag": ordered(boundary_by_tag),
        "selected_by_tag": ordered(selected_by_tag),
        "eligible_by_tag": ordered(eligible_by_tag),
        "exclusions": dict(exclusions),
        "groups": groups,
        "fractions": {
            "eligible": eligible,
            "abs_cos_lt_0_1_count": lt01,
            "abs_cos_lt_0_2_count": lt02,
            "abs_cos_lt_0_1": lt01 / eligible if eligible else None,
            "abs_cos_lt_0_2": lt02 / eligible if eligible else None,
        },
        "lv_axis": axis,
        "thinning": {
            "occupied_bins": len(bins),
            "glyphs_by_tag": ordered(glyphs_by_tag),
            "near_face_centroids": near_face,
        },
        "audit_checks": {
            "face_incidence_histogram": {str(k): histogram[k] for k in sorted(histogram)},
            "eligible_triangles_with_sentinel_rho_vertex": sentinel_rho_triangles,
        },
    }


# ---------------------------------------------------------------- provenance


def sha256(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        while chunk := handle.read(1 << 20):
            digest.update(chunk)
    return digest.hexdigest()


def peak_memory() -> tuple[int | None, str]:
    """Peak process memory from the OS, through the standard library."""
    if sys.platform == "win32":
        import ctypes
        from ctypes import wintypes

        class Counters(ctypes.Structure):
            _fields_ = [
                ("cb", wintypes.DWORD),
                ("PageFaultCount", wintypes.DWORD),
                ("PeakWorkingSetSize", ctypes.c_size_t),
                ("WorkingSetSize", ctypes.c_size_t),
                ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
                ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
                ("PagefileUsage", ctypes.c_size_t),
                ("PeakPagefileUsage", ctypes.c_size_t),
            ]

        counters = Counters()
        counters.cb = ctypes.sizeof(Counters)
        kernel32 = ctypes.WinDLL("kernel32")
        process = kernel32.GetCurrentProcess
        process.restype = wintypes.HANDLE
        query = kernel32.K32GetProcessMemoryInfo
        query.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
        if query(process(), ctypes.byref(counters), counters.cb):
            return counters.PeakWorkingSetSize, "Windows PeakWorkingSetSize"
        return None, "unavailable"
    try:
        import resource

        peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        return (peak if sys.platform == "darwin" else peak * 1024), "getrusage ru_maxrss"
    except ImportError:
        return None, "unavailable"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("vtk", help="original, unmodified legacy ASCII VTK file")
    parser.add_argument("--tags", required=True, help="comma-separated ID tags, e.g. 1,2")
    parser.add_argument("--lv-tag", type=int, required=True, help="tag that receives the LV helix diagnostic")
    parser.add_argument("--bin", type=float, required=True, dest="bin_mm", help="spatial bin edge h in mm")
    parser.add_argument("--out", required=True, help="output JSON path")
    args = parser.parse_args(argv)

    tags = sorted({int(t) for t in args.tags.split(",")})
    if args.lv_tag not in tags:
        parser.error("--lv-tag must be one of --tags")
    if not (args.bin_mm > 0 and math.isfinite(args.bin_mm)):
        parser.error("--bin must be positive and finite")

    start = time.perf_counter()
    try:
        stats = audit(args.vtk, tags, args.lv_tag, args.bin_mm)
    except VtkError as error:
        print(f"audit failed: {error}", file=sys.stderr)
        return 1
    digest = sha256(args.vtk)
    stats["provenance"] = {"source_sha256": digest, "input_sha256": digest}
    peak, method = peak_memory()
    stats["run"] = {
        "runtime_s": round(time.perf_counter() - start, 1),
        "peak_memory_bytes": peak,
        "peak_memory_method": method,
        "python": platform.python_version(),
        "platform": platform.platform(),
    }
    with open(args.out, "w", encoding="utf-8", newline="\n") as handle:
        json.dump(stats, handle, indent=2)
        handle.write("\n")
    print(json.dumps({k: stats[k] for k in ("boundary_triangles", "eligible_by_tag", "thinning", "run")}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
