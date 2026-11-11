#!/usr/bin/env python3
"""Synthetic self-tests for compare.py and verify_glyphs.py (standard library only).

Usage: python -I tools/rodero-fibre-audit/test_tools.py
"""

from __future__ import annotations

import copy
import json
import os
import struct
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import audit  # noqa: E402
import compare  # noqa: E402
import verify_glyphs  # noqa: E402
from test_audit import write_vtk  # noqa: E402

STATS = {
    "schema": "svf-b1.2-fibre-stats/1",
    "parameters": {"tags": [1, 2], "bin_mm": 4.0, "numeric_conversion": audit.NUMERIC_CONVERSION},
    "eligible_by_tag": {"1": 3, "2": 4},
    "groups": {"1": {"endo": {"count": 3, "abs_cos_p50": 0.25, "alpha_deg_p50": None}}},
    "fractions": {"abs_cos_lt_0_1": 0.5, "abs_cos_lt_0_1_count": 3},
    "lv_axis": {"vector_mm": [1.0, 2.0, 3.0], "length_mm": 3.7416573867739413},
    "provenance": {"source_sha256": "aa", "input_sha256": "aa"},
    "run": {"runtime_s": 1.0},
}


def changed(path, value):
    document = copy.deepcopy(STATS)
    target = document
    for part in path[:-1]:
        target = target[part]
    if value is KeyError:
        del target[path[-1]]
    else:
        target[path[-1]] = value
    return document


class CompareTests(unittest.TestCase):
    def assertMismatch(self, svf, fragment):
        problems = compare.compare(STATS, svf)
        self.assertEqual(len(problems), 1, problems)
        self.assertIn(fragment, problems[0])

    def test_identical_documents_match(self):
        self.assertEqual(compare.compare(STATS, copy.deepcopy(STATS)), [])

    def test_counts_and_strings_are_exact(self):
        self.assertMismatch(changed(("eligible_by_tag", "2"), 5), "eligible_by_tag.2")
        self.assertMismatch(changed(("provenance", "source_sha256"), "bb"), "source_sha256")
        self.assertMismatch(changed(("schema",), "other"), "schema")

    def test_float_tolerance_and_exact_floats(self):
        self.assertEqual(compare.compare(STATS, changed(("groups", "1", "endo", "abs_cos_p50"), 0.25 + 5e-10)), [])
        self.assertMismatch(changed(("groups", "1", "endo", "abs_cos_p50"), 0.25 + 2e-9), "tolerance")
        self.assertEqual(compare.compare(STATS, changed(("lv_axis", "vector_mm"), [1.0, 2.0, 3.0 + 1e-10])), [])
        # Fractions and parameters are exact, however small the difference.
        self.assertMismatch(changed(("fractions", "abs_cos_lt_0_1"), 0.5 + 2**-52), "rule exact")
        self.assertMismatch(changed(("parameters", "bin_mm"), 4.0 + 1e-12), "rule exact")

    def test_missing_keys_types_and_lists(self):
        self.assertMismatch(changed(("lv_axis", "length_mm"), KeyError), "missing from the SVF output")
        extra = changed(("lv_axis", "extra"), 1)
        self.assertMismatch(extra, "missing from the audit baseline")
        self.assertMismatch(changed(("eligible_by_tag", "1"), 3.0), "type int (audit) vs float (SVF)")
        self.assertMismatch(changed(("groups", "1", "endo", "alpha_deg_p50"), 75.0), "type null")
        self.assertMismatch(changed(("lv_axis", "vector_mm"), [1.0, 2.0]), "vector_mm.2: missing")
        self.assertMismatch(changed(("parameters", "numeric_conversion"), "decimal-to-f64"), "numeric_conversion")
        self.assertMismatch(changed(("parameters", "numeric_conversion"), KeyError), "numeric_conversion")

    def test_only_input_hash_and_run_are_not_compared(self):
        svf = changed(("provenance", "input_sha256"), "bb")
        svf["run"] = {"runtime_s": 99.0, "anything": True}
        self.assertEqual(compare.compare(STATS, svf), [])
        self.assertEqual(compare.NOT_COMPARED, (("provenance", "input_sha256"), ("run",)))

    def test_main_exit_status(self):
        with tempfile.TemporaryDirectory() as folder:
            a, b = Path(folder, "a.json"), Path(folder, "b.json")
            a.write_text(json.dumps(STATS), encoding="utf-8")
            b.write_text(json.dumps(STATS), encoding="utf-8")
            self.assertEqual(compare.main([str(a), str(b)]), 0)
            b.write_text(json.dumps(changed(("eligible_by_tag", "1"), 4)), encoding="utf-8")
            self.assertEqual(compare.main([str(a), str(b)]), 1)
            self.assertEqual(compare.main([str(a)]), 2)


class VerifyGlyphTests(unittest.TestCase):
    """Cell 0 (tag 1) has three boundary triangles, all in bin (0, 0, 0) at h = 4.
    Cell 1 (tag 5) is outside the tag set."""

    def setUp(self):
        handle, self.vtk = tempfile.mkstemp(suffix=".vtk")
        os.close(handle)
        handle, self.csv = tempfile.mkstemp(suffix=".csv")
        os.close(handle)
        write_vtk(self.vtk, fibres=((0.6, 0.0, 0.8), (1, 0, 0)))
        data = audit.parse_vtk(self.vtk)
        self.expected = verify_glyphs.expected_glyphs(data, [1], 4.0)
        self.keys = {rank: key for key, (rank, _cell) in self.expected["by_key"].items()}

    def tearDown(self):
        os.remove(self.vtk)
        os.remove(self.csv)

    def row(self, rank, length=3.0):
        """The row SVF would export for this eligible triangle."""
        e = self.expected["eligible"][rank]
        offset = audit.scale(e["f_hat"], length / 2.0)
        start = audit.sub(e["centroid"], offset)
        end = tuple(c + o for c, o in zip(e["centroid"], offset))
        return [rank, *self.keys[rank], e["cell"], e["tag"], *e["bin"], *e["fibre"], e["theta"], *start, *end]

    def write(self, rows, glyphs=None, sha=None):
        sha = sha or audit.sha256(self.vtk)
        count = len(rows) if glyphs is None else glyphs
        lines = [f"# {verify_glyphs.GLYPH_SCHEMA} source_sha256={sha} input_sha256=xx tags=1 "
                 f"bin_mm=4.0 length_mm=3.0 glyphs={count}", ",".join(verify_glyphs.COLUMNS)]
        lines += [",".join(repr(v) for v in row) for row in rows]
        Path(self.csv).write_text("\n".join(lines) + "\n", encoding="utf-8")
        return verify_glyphs.verify(self.vtk, self.csv)

    def counts(self, report):
        return {k: len(v) for k, v in report["problems"].items() if v}

    def winner(self):
        (bin_index, rank), = self.expected["winners"].items()
        others = [r for r, e in self.expected["eligible"].items() if e["bin"] == bin_index and r != rank]
        return rank, others

    def test_correct_export_passes(self):
        winner, others = self.winner()
        self.assertEqual(len(others), 2)
        report = self.write([self.row(winner)])
        self.assertEqual(self.counts(report), {})
        self.assertEqual(report["expected_bins"], 1)

    def test_set_discrepancies(self):
        winner, others = self.winner()
        self.assertEqual(self.counts(self.write([])), {"missing": 1})
        self.assertEqual(self.counts(self.write([self.row(others[0])])), {"wrong_winner": 1})
        self.assertEqual(self.counts(self.write([self.row(winner), self.row(others[0])])),
                         {"duplicate_bin": 1, "wrong_winner": 1})
        self.assertEqual(self.counts(self.write([self.row(winner)], glyphs=2)), {"header": 1})
        moved = self.row(winner)
        moved[6] = 7  # bin_x: a bin with no eligible triangle
        report = self.write([moved])
        self.assertEqual(self.counts(report), {"missing": 1, "unexpected": 1, "row": 1})

    def test_row_checks(self):
        winner, _ = self.winner()
        base = self.row(winner)

        def mutate(index, value):
            row = list(base)
            row[index] = value
            return self.counts(self.write([row]))

        fibre_x = base[9]
        nudged = struct.unpack("<d", struct.pack("<Q", struct.unpack("<Q", struct.pack("<d", fibre_x))[0] + 1))[0]
        self.assertEqual(mutate(9, nudged), {"row": 1}, "fibre must match bit for bit")
        self.assertEqual(mutate(12, base[12] + 1e-7), {"row": 1}, "theta")
        self.assertEqual(mutate(13, base[13] + 1e-6), {"row": 2}, "start moves midpoint and length")
        self.assertEqual(mutate(0, winner + 1), {"wrong_winner": 1, "row": 1}, "surface index")
        self.assertEqual(mutate(4, 1), {"row": 1}, "source cell")
        self.assertEqual(mutate(5, 2), {"row": 2}, "tag outside the set and unlike the cell")
        self.assertEqual(self.counts(self.write([self.row(winner, length=2.0)])), {"row": 1}, "length")

    def test_tag_outside_the_set_is_never_expected(self):
        cell1 = [rank for rank, cell in self.expected["by_key"].values() if cell == 1]
        self.assertEqual(len(cell1), 3)
        self.assertTrue(all(rank not in self.expected["eligible"] for rank in cell1))

    def test_source_hash_must_match(self):
        winner, _ = self.winner()
        report = self.write([self.row(winner)], sha="0" * 64)
        self.assertIn("source_sha256 mismatch", report["fatal"])


if __name__ == "__main__":
    unittest.main()
