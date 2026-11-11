#!/usr/bin/env python3
"""Synthetic self-tests for audit.py (standard library only).

Usage: python -I tools/rodero-fibre-audit/test_audit.py
"""

from __future__ import annotations

import math
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import audit  # noqa: E402

# Two tetrahedra sharing face (1, 2, 3). Cell 0 is tag 1 (LV), cell 1 tag 5.
POINTS = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1), (1, 1, 1)]


def write_vtk(path, points=POINTS, cells=((0, 1, 2, 3), (1, 2, 3, 4)), types=(10, 10), ids=(1, 5),
              fibres=((1, 0, 0), (1, 0, 0)), z=(0.0, 0.0, 1.0, 1.0, 0.5), v=(-1, -1, -1, -1, -10),
              rho=(0.1, 0.1, 0.1, 0.1, -10)):
    lines = ["# vtk DataFile Version 3.0", "synthetic", "ASCII", "DATASET UNSTRUCTURED_GRID",
             f"POINTS {len(points)} float"]
    lines += [" ".join(map(str, p)) for p in points]
    lines.append(f"CELL_TYPES {len(types)}")
    lines += [str(t) for t in types]
    lines.append(f"CELLS {len(cells)} {5 * len(cells)}")
    lines += ["4 " + " ".join(map(str, c)) for c in cells]
    lines += [f"CELL_DATA {len(cells)}", "SCALARS ID int 1", "LOOKUP_TABLE default"]
    lines += [str(i) for i in ids]
    lines.append("VECTORS fibres float")
    lines += [" ".join(map(str, f)) for f in fibres]
    lines.append(f"POINT_DATA {len(points)}")
    for name, values in (("RHO.dat", rho), ("Z.dat", z), ("V.dat", v)):
        lines += [f"SCALARS {name} float 1", "LOOKUP_TABLE default"] + [str(x) for x in values]
    Path(path).write_text("\n".join(lines) + "\n", encoding="ascii")


class AuditTests(unittest.TestCase):
    def setUp(self):
        handle, self.path = tempfile.mkstemp(suffix=".vtk")
        os.close(handle)

    def tearDown(self):
        os.remove(self.path)

    def test_boundary_from_face_incidence(self):
        write_vtk(self.path)
        data = audit.parse_vtk(self.path)
        boundary, histogram = audit.boundary_faces(data["conn"], data["cell_count"], data["point_count"])
        self.assertEqual(dict(histogram), {1: 6, 2: 1})
        keys = [face[0] for face in boundary]
        self.assertEqual(keys, sorted(keys), "boundary must be in sorted-key (rank) order")
        self.assertNotIn((1, 2, 3), keys, "shared face is interior")
        self.assertEqual(sorted(face[1] for face in boundary), [0, 0, 0, 1, 1, 1])

    def test_selection_is_by_tag_not_value(self):
        write_vtk(self.path)
        stats = audit.audit(self.path, [1], 1, 4.0)
        self.assertEqual(stats["boundary_by_tag"], {"1": 3, "5": 3})
        self.assertEqual(stats["eligible_by_tag"], {"1": 3})
        # Fibre (1, 0, 0) inside the tag set is used.
        self.assertEqual(stats["fractions"]["eligible"], 3)

    def test_axial_angle_semantics(self):
        write_vtk(self.path)
        plus = audit.audit(self.path, [1], 1, 4.0)
        write_vtk(self.path, fibres=((-1, 0, 0), (1, 0, 0)))
        minus = audit.audit(self.path, [1], 1, 4.0)
        self.assertEqual(plus["groups"], minus["groups"])
        # Faces of cell 0: x=0 face (normal along x, |cos| 1), y=0 and z=0 (|cos| 0).
        group = plus["groups"]["1"]["endo"]
        self.assertEqual(group["count"], 3)
        self.assertEqual(plus["fractions"]["abs_cos_lt_0_1_count"], 2)

    def test_degenerate_cases_counted_and_excluded(self):
        write_vtk(self.path, fibres=((0, 0, 0), (1, 0, 0)))
        stats = audit.audit(self.path, [1], 1, 4.0)
        self.assertEqual(stats["exclusions"]["invalid_fibre"], 3)
        self.assertEqual(stats["eligible_by_tag"], {})
        self.assertEqual(stats["thinning"]["occupied_bins"], 0)
        # Fibre along +x: on the x=0 face t = 0 (zero projection).
        write_vtk(self.path)
        stats = audit.audit(self.path, [1], 1, 4.0)
        self.assertEqual(stats["exclusions"]["helix_zero_projection"], 1)
        # Axis is apex (Z<0.02: points 0, 1) to base (Z>0.98: points 2, 3).
        self.assertEqual(stats["lv_axis"]["apex_points"], 2)
        self.assertEqual(stats["lv_axis"]["base_points"], 2)

    def test_zero_area_face(self):
        flat = [(0, 0, 0), (1, 0, 0), (2, 0, 0), (0, 0, 1), (1, 1, 1)]
        write_vtk(self.path, points=flat, fibres=((0, 1, 0), (1, 0, 0)))
        stats = audit.audit(self.path, [1], 1, 4.0)
        # Points 0, 1, 2 are collinear: local face 3 (0, 2, 1) has zero area.
        self.assertEqual(stats["exclusions"]["zero_area"], 1)

    def test_floor_binning_and_lowest_rank_winner(self):
        shifted = [(x - 10.0, y - 10.0, z - 10.0) for x, y, z in POINTS]
        write_vtk(self.path, points=shifted)
        stats = audit.audit(self.path, [1], 1, 4.0)
        # All three centroids lie in [-12, -8) on each axis -> bin (-3, -3, -3).
        self.assertEqual(stats["thinning"]["occupied_bins"], 1)
        self.assertEqual(stats["thinning"]["glyphs_by_tag"], {"1": 1})
        stats = audit.audit(self.path, [1], 1, 0.25)
        self.assertGreater(stats["thinning"]["occupied_bins"], 1)

    def test_pinned_centroid_arithmetic_and_vertex_order(self):
        # Contract (design §7.2): p_c = ((a + b) + c) / 3.0 per coordinate in f64,
        # with a, b, c in the triangle's stored vertex order. For a tetrahedral
        # boundary face that is the Tetrahedron4 template cycle; local face 1
        # is [0, 3, 2]. These x values sum to exactly 396 in stored order, but
        # not in reversed order or as a/3 + b/3 + c/3, so the bin depends on it.
        xa, xb, xc = 132.211618, 131.696712, 132.09167  # points 0, 3, 2
        self.assertEqual(((xa + xb) + xc) / 3.0, 132.0)
        self.assertLess(((xc + xb) + xa) / 3.0, 132.0)
        self.assertLess(xa / 3.0 + xb / 3.0 + xc / 3.0, 132.0)
        points = [(xa, 1.0, 1.0), (131.0, 2.0, 1.0), (xc, 1.0, 2.0), (xb, 2.0, 2.0)]
        write_vtk(self.path, points=points, cells=((0, 1, 2, 3),), types=(10,), ids=(1,),
                  fibres=((0, 0, 1),), z=(0.0, 0.0, 1.0, 1.0), v=(-1, -1, -1, -1), rho=(0.1,) * 4)
        stats = audit.audit(self.path, [1], 1, 4.0)
        self.assertEqual(stats["exclusions"]["zero_area"], 0)
        # Face [0, 3, 2] lands exactly on x = 132 -> bin x 33. The other three
        # faces have x < 132 -> bin x 32. Any other order would give one bin.
        self.assertEqual(stats["thinning"]["occupied_bins"], 2)
        self.assertEqual(stats["thinning"]["near_face_centroids"], 1)

    def test_near_face_centroid(self):
        # Face (0,1,2) of a tet with points 0..3 at z=0 -> centroid z = 0, a bin face.
        write_vtk(self.path)
        stats = audit.audit(self.path, [1], 1, 4.0)
        self.assertGreaterEqual(stats["thinning"]["near_face_centroids"], 1)

    def test_percentile_nearest_rank_half_up(self):
        self.assertIsNone(audit.percentile([], 0.5))
        self.assertEqual(audit.percentile([1.0, 2.0, 3.0], 0.25), 2.0)  # (n-1)p = 0.5 -> 1
        self.assertEqual(audit.percentile([1.0, 2.0, 3.0, 4.0], 0.5), 3.0)  # 1.5 -> 2
        self.assertEqual(audit.percentile([5.0], 0.99), 5.0)

    def test_rejects_non_tetrahedral_input(self):
        write_vtk(self.path, types=(10, 12))
        with self.assertRaises(audit.VtkError):
            audit.parse_vtk(self.path)

    def test_sentinel_points_excluded_from_lv_axis(self):
        # Point 4 is a sentinel point (V = Z = -10, as the source writes them).
        # A bare "V < 0 and Z < 0.02" test would wrongly count it as apex.
        write_vtk(self.path, z=(0.0, 0.0, 1.0, 1.0, -10.0))
        stats = audit.audit(self.path, [1], 1, 4.0)
        self.assertEqual(stats["lv_axis"]["apex_points"], 2)
        # Apex mean (0.5, 0, 0), base mean (0, 0.5, 0.5).
        self.assertEqual(stats["lv_axis"]["vector_mm"], [-0.5, 0.5, 0.5])
        self.assertTrue(math.isclose(stats["lv_axis"]["length_mm"], math.sqrt(0.75)))


if __name__ == "__main__":
    unittest.main()
