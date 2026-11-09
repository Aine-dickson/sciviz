#!/usr/bin/env python3
"""Independently parse and verify the committed analytical heat VTK fixtures."""

from __future__ import annotations

import itertools
import math
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2] / "fixtures" / "analytical-heat-diffusion"
ALPHA = 1.0
K = 1.0
T1 = math.log(2.0) / (3.0 * math.pi * math.pi * ALPHA)
VALUE_TOL = 1.0e-12
COORD_TOL = 1.0e-14
ISO_LEVEL = 0.2


def require(condition: bool, message: str):
    if not condition:
        raise AssertionError(message)


def close(actual: float, expected: float, tol: float = VALUE_TOL):
    return math.isfinite(actual) and abs(actual - expected) <= tol


def trig_unit(value: float):
    if value == 0.0:
        return 0.0, 1.0
    if value == 1.0:
        return 0.0, -1.0
    if value == 0.5:
        return 1.0, 0.0
    angle = math.pi * value
    return math.sin(angle), math.cos(angle)


def exact_fields(point):
    x, y, z = point
    sx, cx = trig_unit(x)
    sy, cy = trig_unit(y)
    sz, cz = trig_unit(z)
    temperature = sx * sy * sz
    flux = (
        -K * math.pi * cx * sy * sz,
        -K * math.pi * sx * cy * sz,
        -K * math.pi * sx * sy * cz,
    )
    return temperature, 0.5 * temperature, x + 2.0 * y + 3.0 * z, flux, tuple(0.5 * v for v in flux)


def parse_fixture(path: Path):
    lines = path.read_text(encoding="ascii").splitlines()
    require(len(lines) >= 8, f"{path}: file too short")
    require(lines[0] == "# vtk DataFile Version 3.0", f"{path}: unexpected VTK header")
    require(lines[2] == "ASCII", f"{path}: expected ASCII")
    require(lines[3] == "DATASET UNSTRUCTURED_GRID", f"{path}: expected UNSTRUCTURED_GRID")
    cursor = 4

    def take_header(prefix):
        nonlocal cursor
        require(cursor < len(lines) and lines[cursor].startswith(prefix), f"{path}: expected {prefix}")
        line = lines[cursor]
        cursor += 1
        return line.split()

    point_header = take_header("POINTS ")
    point_count = int(point_header[1])
    require(point_header[2].lower() == "double", f"{path}: points must be double")
    points = []
    for _ in range(point_count):
        values = [float(v) for v in lines[cursor].split()]
        cursor += 1
        require(len(values) == 3, f"{path}: malformed point tuple")
        points.append(tuple(values))

    cell_header = take_header("CELLS ")
    cell_count, cell_size = int(cell_header[1]), int(cell_header[2])
    require(cell_size == cell_count * 5, f"{path}: incorrect CELLS size")
    cells = []
    for _ in range(cell_count):
        values = [int(v) for v in lines[cursor].split()]
        cursor += 1
        require(len(values) == 5 and values[0] == 4, f"{path}: non-tetrahedral cell record")
        cells.append(tuple(values[1:]))

    type_header = take_header("CELL_TYPES ")
    require(int(type_header[1]) == cell_count, f"{path}: CELL_TYPES count mismatch")
    cell_types = [int(lines[cursor + i].strip()) for i in range(cell_count)]
    cursor += cell_count
    require(all(value == 10 for value in cell_types), f"{path}: found non-tetrahedral VTK cell type")

    data_header = take_header("POINT_DATA ")
    require(int(data_header[1]) == point_count, f"{path}: POINT_DATA count mismatch")

    scalars = {}
    for expected_name in ("temperature_t0", "temperature_t1", "probe_linear"):
        header = take_header("SCALARS ")
        require(len(header) == 4, f"{path}: malformed SCALARS header")
        require(header[1] == expected_name and header[2].lower() == "double" and header[3] == "1",
                f"{path}: unexpected scalar header {header}")
        lookup = take_header("LOOKUP_TABLE ")
        require(len(lookup) == 2 and lookup[1] == "default", f"{path}: unexpected lookup table")
        scalars[expected_name] = [float(lines[cursor + i].strip()) for i in range(point_count)]
        cursor += point_count

    vectors = {}
    for expected_name in ("heat_flux_t0", "heat_flux_t1"):
        header = take_header("VECTORS ")
        require(len(header) == 3 and header[1] == expected_name and header[2].lower() == "double",
                f"{path}: unexpected vector header {header}")
        values = []
        for _ in range(point_count):
            vector = tuple(float(v) for v in lines[cursor].split())
            cursor += 1
            require(len(vector) == 3, f"{path}: malformed vector tuple")
            values.append(vector)
        vectors[expected_name] = values

    require(cursor == len(lines), f"{path}: unexpected trailing records")
    return points, cells, scalars, vectors


def determinant(a, b, c, d):
    u = [b[i] - a[i] for i in range(3)]
    v = [c[i] - a[i] for i in range(3)]
    w = [d[i] - a[i] for i in range(3)]
    return (
        u[0] * (v[1] * w[2] - v[2] * w[1])
        - u[1] * (v[0] * w[2] - v[2] * w[0])
        + u[2] * (v[0] * w[1] - v[1] * w[0])
    )


def triangle_area(a, b, c):
    u = [b[i] - a[i] for i in range(3)]
    v = [c[i] - a[i] for i in range(3)]
    cross = (
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    )
    return 0.5 * math.sqrt(sum(value * value for value in cross))


def verify_topology(points, cells, n):
    require(len(points) == n**3, "point count mismatch")
    require(len(cells) == 6 * (n - 1) ** 3, "tetrahedron count mismatch")
    faces = Counter()
    total_volume = 0.0
    for cell_id, tet in enumerate(cells):
        require(len(set(tet)) == 4, f"tetrahedron {cell_id} repeats a point")
        require(all(0 <= index < len(points) for index in tet), f"tetrahedron {cell_id} index out of range")
        volume6 = determinant(*(points[index] for index in tet))
        require(volume6 > 0.0, f"tetrahedron {cell_id} has non-positive signed volume")
        total_volume += volume6 / 6.0
        for face in itertools.combinations(tet, 3):
            faces[tuple(sorted(face))] += 1
    require(abs(total_volume - 1.0) <= VALUE_TOL, "tetrahedral volumes do not sum to the unit-cube volume")
    require(all(count in (1, 2) for count in faces.values()), "non-manifold face incidence")
    boundary = [face for face, count in faces.items() if count == 1]
    expected_per_side = 2 * (n - 1) ** 2
    side_counts = Counter()
    side_areas = Counter()
    for face in boundary:
        tri = [points[index] for index in face]
        sides = []
        for axis in range(3):
            if all(point[axis] == 0.0 for point in tri):
                sides.append((axis, 0))
            if all(point[axis] == 1.0 for point in tri):
                sides.append((axis, 1))
        require(len(sides) == 1, "boundary triangle is not on exactly one cube face")
        side = sides[0]
        side_counts[side] += 1
        side_areas[side] += triangle_area(*tri)
    require(len(boundary) == 6 * expected_per_side, "unexpected exterior boundary triangle count")
    for side in itertools.product(range(3), range(2)):
        require(side_counts[side] == expected_per_side, f"incorrect triangle count on cube face {side}")
        require(abs(side_areas[side] - 1.0) <= VALUE_TOL, f"cube face {side} area is not one")
    return len(boundary)


def iso_residual(points, cells, nodal_temperature):
    maximum = 0.0
    intersections = 0
    for tet in cells:
        for ia, ib in itertools.combinations(tet, 2):
            va, vb = nodal_temperature[ia], nodal_temperature[ib]
            if (va < ISO_LEVEL <= vb) or (vb < ISO_LEVEL <= va):
                fraction = (ISO_LEVEL - va) / (vb - va)
                a, b = points[ia], points[ib]
                p = tuple(a[i] + fraction * (b[i] - a[i]) for i in range(3))
                exact = math.prod(math.sin(math.pi * coordinate) for coordinate in p)
                maximum = max(maximum, abs(exact - ISO_LEVEL))
                intersections += 1
    require(intersections > 0, "isosurface level produced no edge intersections")
    return maximum, intersections


def verify(path: Path, n: int):
    points, cells, scalars, vectors = parse_fixture(path)
    boundary_count = verify_topology(points, cells, n)
    require(set(scalars) == {"temperature_t0", "temperature_t1", "probe_linear"}, "scalar field set mismatch")
    require(set(vectors) == {"heat_flux_t0", "heat_flux_t1"}, "vector field set mismatch")
    require(all(len(values) == len(points) for values in scalars.values()), "scalar tuple count mismatch")
    require(all(len(values) == len(points) for values in vectors.values()), "vector tuple count mismatch")

    interior_count = 0
    for point_id, point in enumerate(points):
        i = point_id % n
        j = (point_id // n) % n
        k = point_id // (n * n)
        expected_point = (i / (n - 1), j / (n - 1), k / (n - 1))
        require(all(abs(point[a] - expected_point[a]) <= COORD_TOL for a in range(3)),
                f"point ordering/coordinate mismatch at point {point_id}")
        expected = exact_fields(point)
        actual = (
            scalars["temperature_t0"][point_id],
            scalars["temperature_t1"][point_id],
            scalars["probe_linear"][point_id],
            vectors["heat_flux_t0"][point_id],
            vectors["heat_flux_t1"][point_id],
        )
        for field_index in range(3):
            require(close(actual[field_index], expected[field_index]),
                    f"scalar analytical mismatch at point {point_id}, field {field_index}")
        for field_index in (3, 4):
            require(all(close(actual[field_index][axis], expected[field_index][axis]) for axis in range(3)),
                    f"vector analytical mismatch at point {point_id}, field {field_index}")
        require(close(actual[1], 0.5 * actual[0]), f"temperature snapshot relation failed at {point_id}")
        require(all(close(actual[4][axis], 0.5 * actual[3][axis]) for axis in range(3)),
                f"heat-flux snapshot relation failed at {point_id}")
        x, y, z = point
        if x not in (0.0, 1.0) and y not in (0.0, 1.0) and z not in (0.0, 1.0):
            interior_count += 1
            require(actual[0] > 0.0, f"non-positive interior temperature at point {point_id}")
        else:
            require(actual[0] == 0.0 and actual[1] == 0.0, f"boundary temperature not exactly zero at {point_id}")
        if x == 0.0:
            require(actual[3][0] <= VALUE_TOL, "heat flux does not point outward on x=0")
        if x == 1.0:
            require(actual[3][0] >= -VALUE_TOL, "heat flux does not point outward on x=1")
        if y == 0.0:
            require(actual[3][1] <= VALUE_TOL, "heat flux does not point outward on y=0")
        if y == 1.0:
            require(actual[3][1] >= -VALUE_TOL, "heat flux does not point outward on y=1")
        if z == 0.0:
            require(actual[3][2] <= VALUE_TOL, "heat flux does not point outward on z=0")
        if z == 1.0:
            require(actual[3][2] >= -VALUE_TOL, "heat flux does not point outward on z=1")

    centre_id = (n // 2) * n * n + (n // 2) * n + (n // 2)
    require(points[centre_id] == (0.5, 0.5, 0.5), "centre point missing")
    require(close(scalars["temperature_t0"][centre_id], 1.0), "t0 centre temperature must be 1 K")
    require(close(scalars["temperature_t1"][centre_id], 0.5), "t1 centre temperature must be 0.5 K")
    require(vectors["heat_flux_t0"][centre_id] == (0.0, 0.0, 0.0), "centre heat flux must be zero")

    # Exact closed-form identity: dT/dt = -3*pi^2*alpha*T and
    # alpha*laplacian(T) = -3*pi^2*alpha*T.
    require(interior_count == (n - 2) ** 3, "interior point count mismatch")
    residual, intersections = iso_residual(points, cells, scalars["temperature_t0"])
    return {
        "points": len(points),
        "tetrahedra": len(cells),
        "boundary_triangles": boundary_count,
        "interior_points": interior_count,
        "isovalue_edge_intersections": intersections,
        "max_continuous_isosurface_residual_K": residual,
    }


def main():
    summaries = {}
    for label, n in (("coarse", 5), ("fine", 9)):
        path = ROOT / f"heat-diffusion-{label}.vtk"
        require(path.is_file(), f"missing fixture: {path}")
        summaries[label] = verify(path, n)
        print(f"{label}: {summaries[label]}")
    coarse = summaries["coarse"]["max_continuous_isosurface_residual_K"]
    fine = summaries["fine"]["max_continuous_isosurface_residual_K"]
    require(fine < coarse, "fine mesh did not improve the isosurface approximation")
    require(fine <= 0.04, "fine mesh isosurface residual exceeds 0.04 K")
    print("PASS: both fixtures satisfy analytical, field, and mesh invariants.")
    print(f"PASS: isosurface residual improves from {coarse:.12g} K to {fine:.12g} K.")


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise
