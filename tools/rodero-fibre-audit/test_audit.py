#!/usr/bin/env python3
"""Synthetic self-tests for audit.py (standard library only).

Usage: python -I tools/rodero-fibre-audit/test_audit.py
"""

from __future__ import annotations

import json
import math
import os
import struct
import sys
import tempfile
import unittest
from decimal import Decimal, localcontext
from fractions import Fraction
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import audit  # noqa: E402

# Two tetrahedra sharing face (1, 2, 3). Cell 0 is tag 1 (LV), cell 1 tag 5.
POINTS = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1), (1, 1, 1)]


def write_vtk(path, points=POINTS, cells=((0, 1, 2, 3), (1, 2, 3, 4)), types=(10, 10), ids=(1, 5),
              fibres=((1, 0, 0), (1, 0, 0)), z=(0.0, 0.0, 1.0, 1.0, 0.5), v=(-1, -1, -1, -1, -10),
              rho=(0.1, 0.1, 0.1, 0.1, -10), point_type="float"):
    lines = ["# vtk DataFile Version 3.0", "synthetic", "ASCII", "DATASET UNSTRUCTURED_GRID",
             f"POINTS {len(points)} {point_type}"]
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


NAN = object()  # sentinel: the value must be NaN


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
                  fibres=((0, 0, 1),), z=(0.0, 0.0, 1.0, 1.0), v=(-1, -1, -1, -1), rho=(0.1,) * 4,
                  point_type="double")  # binary64 points: these x values are used exactly
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

    # ------------------------------------------------ declared-type input contract

    @staticmethod
    def exact_text(value: Fraction) -> str:
        with localcontext() as context:
            context.prec = 400
            return str(Decimal(value.numerator) / Decimal(value.denominator))

    def test_binary32_differs_from_direct_f64(self):
        for text in ("0.1", "79.418297", "-0.569958", "0.883928"):
            via_f32 = struct.unpack("<f", struct.pack("<f", float(text)))[0]
            self.assertEqual(audit.binary32(text), via_f32, text)
            self.assertNotEqual(audit.binary32(text), float(text), text)
        self.assertEqual(audit.binary32("0.5"), 0.5)  # exactly representable
        self.assertEqual(audit.binary32("-0.0"), 0.0)
        self.assertEqual(math.copysign(1.0, audit.binary32("-0.0")), -1.0)

    def test_binary32_is_correctly_rounded_where_f64_lands_on_a_midpoint(self):
        tie = Fraction(1) + Fraction(1, 2**24)  # midpoint of 1 and 1 + 2**-23
        tiny = Fraction(1, 2**60)  # below half an f64 ulp at 1
        naive = lambda t: struct.unpack("<f", struct.pack("<f", float(t)))[0]  # noqa: E731
        above, below = self.exact_text(tie + tiny), self.exact_text(tie - tiny)
        self.assertEqual(float(above), float(tie))  # both round to the midpoint in f64
        self.assertEqual(naive(above), 1.0)  # double rounding goes the wrong way
        self.assertEqual(audit.binary32(above), 1.0 + 2.0**-23)
        self.assertEqual(audit.binary32(below), 1.0)
        self.assertEqual(audit.binary32("-" + above), -(1.0 + 2.0**-23))
        self.assertEqual(audit.binary32(self.exact_text(tie)), 1.0)  # true tie: to even
        odd_tie = Fraction(1) + Fraction(3, 2**24)
        self.assertEqual(audit.binary32(self.exact_text(odd_tie)), 1.0 + 2.0**-22)
        # Subnormal: f64 lands on 2**-150, the midpoint of 0 and 2**-149.
        sub = self.exact_text(Fraction(1, 2**150) + Fraction(1, 2**210))
        self.assertEqual(naive(sub), 0.0)
        self.assertEqual(audit.binary32(sub), 2.0**-149)

    def test_binary32_range_limits(self):
        self.assertEqual(audit.binary32("3.4028235e38"), audit.F32_MAX)
        self.assertEqual(audit.binary32("1e39"), math.inf)
        self.assertEqual(audit.binary32("-1e39"), -math.inf)
        limit = Fraction(2**128 - 2**103)
        self.assertEqual(audit.binary32(self.exact_text(limit - Fraction(1, 2**10))), audit.F32_MAX)
        self.assertEqual(audit.binary32(self.exact_text(limit)), math.inf)  # tie to even
        self.assertEqual(audit.binary32("1e-50"), 0.0)

    def test_parser_converts_by_declared_type(self):
        text = "\n".join([
            "# vtk DataFile Version 3.0", "types", "ASCII", "DATASET UNSTRUCTURED_GRID",
            "POINTS 4 {points}", "0.1 0 0", "0 0.1 0", "0 0 0.1", "0.1 0.1 0.1",
            "CELL_TYPES 1", "10", "CELLS 1 5", "4 0 1 2 3",
            "CELL_DATA 1", "SCALARS ID int 1", "LOOKUP_TABLE default", "7",
            "SCALARS c_float float 1", "LOOKUP_TABLE default", "0.1",
            "SCALARS c_double double 1", "LOOKUP_TABLE default", "0.1",
            "VECTORS v_float float", "0.1 0.2 0.3",
            "VECTORS v_double double", "0.1 0.2 0.3",
        ]) + "\n"
        f32 = audit.binary32
        for declared, expected in (("float", f32("0.1")), ("double", 0.1)):
            Path(self.path).write_text(text.format(points=declared), encoding="ascii")
            data = audit.parse_vtk(self.path)
            self.assertEqual(data["points"][0], expected, declared)
            cells = data["cell_data"]
            self.assertEqual(cells["ID"][0], 7.0)
            self.assertEqual(cells["c_float"][0], f32("0.1"))
            self.assertEqual(cells["c_double"][0], 0.1)
            self.assertEqual(list(cells["v_float"]), [f32("0.1"), f32("0.2"), f32("0.3")])
            self.assertEqual(list(cells["v_double"]), [0.1, 0.2, 0.3])
        Path(self.path).write_text(text.format(points="long_double"), encoding="ascii")
        with self.assertRaises(audit.VtkError):
            audit.parse_vtk(self.path)

    # Shared with fibre_diagnostic.rs (svf_parser_accepts_and_rejects_the_audit_edge_cases):
    # SVF's parser must give the same value, or reject the same text.
    EDGE_CASES = [  # expected None: rejected; NAN: parses to NaN
        ("short", "-32768", -32768.0), ("short", "32767", 32767.0),
        ("short", "-32769", None), ("short", "32768", None),
        ("unsigned_short", "0", 0.0), ("unsigned_short", "65535", 65535.0),
        ("unsigned_short", "65536", None), ("unsigned_short", "-1", None), ("unsigned_short", "-0", None),
        ("int", "-2147483648", -2147483648.0), ("int", "2147483647", 2147483647.0),
        ("int", "2147483648", None), ("int", "-2147483649", None),
        ("int", "+7", 7.0), ("int", "007", 7.0), ("int", "-0", 0.0),
        ("int", "1.0", None), ("int", "1_0", None), ("int", "٣", None), ("int", "+", None),
        ("unsigned_int", "4294967295", 4294967295.0), ("unsigned_int", "4294967296", None),
        ("unsigned_int", "+1", 1.0),
        ("unsigned_char", "255", 255.0), ("unsigned_char", "256", None), ("unsigned_char", "-0", None),
        ("double", "1e5", 100000.0), ("double", "-0.25", -0.25), ("double", "1_0.5", None),
        ("double", "١.5", None), ("double", "abc", None),
        ("float", "0.5", 0.5), ("float", "1_0", None), ("float", "1e39", math.inf),
        # Declared type names are ASCII case-insensitive, as in SVF.
        ("FLOAT", "0.5", 0.5), ("Double", "-0.25", -0.25), ("INT", "7", 7.0), ("Unsigned_Char", "255", 255.0),
        # Special values: exactly Rust's spellings (any case, optional sign).
        ("float", "inf", math.inf), ("float", "INFINITY", math.inf), ("float", "-Infinity", -math.inf),
        ("float", "+inf", math.inf), ("double", "Inf", math.inf), ("float", "NaN", NAN), ("float", "-nan", NAN),
        ("double", "nan", NAN), ("double", "+NAN", NAN),
        ("float", "infinit", None), ("float", "infinityy", None), ("float", "in", None), ("float", "nan(1)", None),
        ("float", "nan1", None), ("float", "snan", None), ("double", "qnan", None), ("int", "inf", None),
        # Other edges of the decimal grammar.
        ("double", "1.", 1.0), ("double", ".5", 0.5), ("double", "+.5", 0.5), ("double", "1E+5", 100000.0),
        ("double", ".", None), ("double", "-.", None), ("double", "1e", None), ("double", "1e+", None),
        ("double", "e5", None), ("double", "0x10", None), ("double", "1d5", None), ("double", "1.5f", None),
        ("double", "+-1", None), ("double", "1,5", None), ("double", "1e400", math.inf), ("float", "1e-400", 0.0),
    ]

    def test_declared_types_accept_and_reject_like_svf(self):
        for declared, text, expected in self.EDGE_CASES:
            convert = audit.converter(declared, 0)
            if expected is None:
                with self.assertRaises(audit.VtkError, msg=f"{declared} {text!r}"):
                    convert(text)
            elif expected is NAN:
                self.assertTrue(math.isnan(convert(text)), f"{declared} {text!r}")
            else:
                self.assertEqual(convert(text), expected, f"{declared} {text!r}")

    def test_float_grammar_is_rusts_not_pythons(self):
        # float() accepts these; Rust's from_str does not. A VTK token cannot
        # contain whitespace, so this is checked at the converter only.
        for text in (" 1", "1 ", "	2.5", "1_0", "٣"):
            for declared in ("float", "double"):
                with self.assertRaises(audit.VtkError, msg=f"{declared} {text!r}"):
                    audit.converter(declared, 0)(text)

    def test_declared_type_name_is_case_insensitive_in_a_file(self):
        write_vtk(self.path, point_type="FLOAT")
        upper = audit.parse_vtk(self.path)["points"]
        write_vtk(self.path, point_type="float")
        self.assertEqual(upper, audit.parse_vtk(self.path)["points"])

    def test_out_of_range_integer_in_a_file_is_rejected(self):
        write_vtk(self.path, ids=(1, 2**31))
        with self.assertRaises(audit.VtkError):
            audit.parse_vtk(self.path)

    def test_baseline_records_the_numeric_conversion(self):
        write_vtk(self.path)
        stats = audit.audit(self.path, [1], 1, 4.0)
        self.assertEqual(stats["parameters"]["numeric_conversion"], audit.NUMERIC_CONVERSION)
        self.assertIn("float=binary32", audit.NUMERIC_CONVERSION)
        baseline = Path(__file__).resolve().parent / "baseline" / "case01.json"
        recorded = json.loads(baseline.read_text(encoding="utf-8"))["parameters"].get("numeric_conversion")
        self.assertEqual(recorded, audit.NUMERIC_CONVERSION, "the committed baseline must use this conversion")

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
