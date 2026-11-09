#!/usr/bin/env python3
"""Generate deterministic analytical heat-diffusion VTK legacy ASCII fixtures."""

from __future__ import annotations

import argparse
import itertools
import math
from pathlib import Path

ALPHA = 1.0
CONDUCTIVITY = 1.0
T1 = math.log(2.0) / (3.0 * math.pi * math.pi * ALPHA)


def trig_unit(value: float) -> tuple[float, float]:
    """Return exact endpoint/centre values where the analytical values are known."""
    if value == 0.0:
        return 0.0, 1.0
    if value == 1.0:
        return 0.0, -1.0
    if value == 0.5:
        return 1.0, 0.0
    angle = math.pi * value
    return math.sin(angle), math.cos(angle)


def point_index(i: int, j: int, k: int, n: int) -> int:
    return k * n * n + j * n + i


def determinant(a, b, c, d) -> float:
    u = [b[i] - a[i] for i in range(3)]
    v = [c[i] - a[i] for i in range(3)]
    w = [d[i] - a[i] for i in range(3)]
    return (
        u[0] * (v[1] * w[2] - v[2] * w[1])
        - u[1] * (v[0] * w[2] - v[2] * w[0])
        + u[2] * (v[0] * w[1] - v[1] * w[0])
    )


def build_mesh(n: int):
    points = [
        (i / (n - 1), j / (n - 1), k / (n - 1))
        for k in range(n)
        for j in range(n)
        for i in range(n)
    ]
    tetrahedra = []
    for k in range(n - 1):
        for j in range(n - 1):
            for i in range(n - 1):
                lower = (i, j, k)
                for permutation in itertools.permutations(range(3)):
                    lattice = [lower]
                    current = list(lower)
                    for axis in permutation:
                        current = current.copy()
                        current[axis] += 1
                        lattice.append(tuple(current))
                    ids = [point_index(*p, n) for p in lattice]
                    if determinant(*(points[p] for p in ids)) < 0.0:
                        ids[1], ids[2] = ids[2], ids[1]
                    tetrahedra.append(tuple(ids))
    return points, tetrahedra


def evaluate_fields(points):
    fields = {
        "temperature_t0": [],
        "temperature_t1": [],
        "probe_linear": [],
        "heat_flux_t0": [],
        "heat_flux_t1": [],
    }
    for x, y, z in points:
        sx, cx = trig_unit(x)
        sy, cy = trig_unit(y)
        sz, cz = trig_unit(z)
        base = sx * sy * sz
        q0 = (
            -CONDUCTIVITY * math.pi * cx * sy * sz,
            -CONDUCTIVITY * math.pi * sx * cy * sz,
            -CONDUCTIVITY * math.pi * sx * sy * cz,
        )
        fields["temperature_t0"].append(base)
        fields["temperature_t1"].append(0.5 * base)
        fields["probe_linear"].append(x + 2.0 * y + 3.0 * z)
        fields["heat_flux_t0"].append(q0)
        fields["heat_flux_t1"].append(tuple(0.5 * value for value in q0))
    return fields


def fmt(value: float) -> str:
    if value == 0.0:
        value = 0.0
    return format(value, ".17g")


def render_vtk(name: str, points, tetrahedra, fields) -> str:
    lines = [
        "# vtk DataFile Version 3.0",
        f"SVF analytical heat diffusion {name} fixture",
        "ASCII",
        "DATASET UNSTRUCTURED_GRID",
        f"POINTS {len(points)} double",
    ]
    lines.extend(" ".join(fmt(v) for v in point) for point in points)
    lines.append(f"CELLS {len(tetrahedra)} {len(tetrahedra) * 5}")
    lines.extend("4 " + " ".join(str(v) for v in tet) for tet in tetrahedra)
    lines.append(f"CELL_TYPES {len(tetrahedra)}")
    lines.extend("10" for _ in tetrahedra)
    lines.append(f"POINT_DATA {len(points)}")
    for name in ("temperature_t0", "temperature_t1", "probe_linear"):
        lines.extend((f"SCALARS {name} double 1", "LOOKUP_TABLE default"))
        lines.extend(fmt(value) for value in fields[name])
    for name in ("heat_flux_t0", "heat_flux_t1"):
        lines.append(f"VECTORS {name} double")
        lines.extend(" ".join(fmt(v) for v in vector) for vector in fields[name])
    return "\n".join(lines) + "\n"


def validate_generated(n: int, points, tetrahedra, fields):
    expected_points = n**3
    expected_cells = 6 * (n - 1) ** 3
    assert len(points) == expected_points
    assert len(tetrahedra) == expected_cells
    assert all(len(values) == expected_points for values in fields.values())
    for tet in tetrahedra:
        assert len(set(tet)) == 4
        assert all(0 <= index < expected_points for index in tet)
        assert determinant(*(points[index] for index in tet)) > 0.0
    for point, t0, t1, q0, q1 in zip(
        points,
        fields["temperature_t0"],
        fields["temperature_t1"],
        fields["heat_flux_t0"],
        fields["heat_flux_t1"],
    ):
        x, y, z = point
        if x in (0.0, 1.0) or y in (0.0, 1.0) or z in (0.0, 1.0):
            assert t0 == 0.0 and t1 == 0.0
        assert t1 == 0.5 * t0
        assert q1 == tuple(0.5 * v for v in q0)
    centre = point_index(n // 2, n // 2, n // 2, n)
    assert points[centre] == (0.5, 0.5, 0.5)
    assert fields["temperature_t0"][centre] == 1.0
    assert fields["temperature_t1"][centre] == 0.5
    assert fields["heat_flux_t0"][centre] == (0.0, 0.0, 0.0)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path(__file__).resolve().parents[2] / "fixtures" / "analytical-heat-diffusion",
    )
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    for label, n in (("coarse", 5), ("fine", 9)):
        points, tetrahedra = build_mesh(n)
        fields = evaluate_fields(points)
        validate_generated(n, points, tetrahedra, fields)
        output = args.output_dir / f"heat-diffusion-{label}.vtk"
        output.write_text(
            render_vtk(label, points, tetrahedra, fields),
            encoding="ascii",
            newline="\n",
        )
        print(
            f"{output}: {len(points)} points, {len(tetrahedra)} tetrahedra, "
            f"{output.stat().st_size} bytes"
        )
    print(f"alpha={ALPHA:.17g} m^2/s")
    print(f"k={CONDUCTIVITY:.17g} W/(m K)")
    print(f"t0=0 s; t1={T1:.17g} s")


if __name__ == "__main__":
    main()
