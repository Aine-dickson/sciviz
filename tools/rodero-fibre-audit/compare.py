#!/usr/bin/env python3
"""Compare an SVF fibre-diagnostic statistics file against the audit baseline.

Standard library only. Applies the matching rule of
docs/heart-fibre-orientation.md section 7.2:
- integers (counts) and strings (hashes, schema) must be equal exactly;
- the fractions abs_cos_lt_* and every value under "parameters" must be
  equal exactly (fractions are ratios of exactly matching counts);
- every other float (percentiles, angles, lengths) must agree within 1e-9
  absolute;
- a key or list element missing from either side is a mismatch, as is a
  type difference (for example an integer count against a float).

Only provenance.input_sha256 and the run record ("run") are not compared.
Their names are fixed below and both sides' values are printed, so nothing
is skipped silently.

Usage:
    python -I tools/rodero-fibre-audit/compare.py <audit.json> <svf.json>
Exit status: 0 when everything matches, 1 on any mismatch, 2 on bad input.
"""

from __future__ import annotations

import json
import sys

TOLERANCE = 1.0e-9
NOT_COMPARED = (("provenance", "input_sha256"), ("run",))
EXACT_FLOAT_KEYS = {"abs_cos_lt_0_1", "abs_cos_lt_0_2"}
EXACT_SECTIONS = {"parameters"}
MISSING = object()


def kind(value) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, int):
        return "int"
    if isinstance(value, float):
        return "float"
    if isinstance(value, str):
        return "str"
    if isinstance(value, list):
        return "list"
    if isinstance(value, dict):
        return "object"
    return type(value).__name__


def show(path: tuple) -> str:
    return ".".join(str(part) for part in path) or "<root>"


def compare(audit, svf, path: tuple = ()) -> list[str]:
    """Return one message per mismatch below path."""
    if path in NOT_COMPARED:
        return []
    if audit is MISSING or svf is MISSING:
        side = "SVF output" if svf is MISSING else "audit baseline"
        return [f"{show(path)}: missing from the {side}"]
    if kind(audit) != kind(svf):
        return [f"{show(path)}: type {kind(audit)} (audit) vs {kind(svf)} (SVF): {audit!r} vs {svf!r}"]
    if isinstance(audit, dict):
        problems = []
        for key in list(audit) + [k for k in svf if k not in audit]:
            problems += compare(audit.get(key, MISSING), svf.get(key, MISSING), path + (key,))
        return problems
    if isinstance(audit, list):
        problems = []
        for index in range(max(len(audit), len(svf))):
            a = audit[index] if index < len(audit) else MISSING
            s = svf[index] if index < len(svf) else MISSING
            problems += compare(a, s, path + (index,))
        return problems
    if isinstance(audit, float):
        exact = path[-1] in EXACT_FLOAT_KEYS or (path and path[0] in EXACT_SECTIONS)
        difference = abs(audit - svf)
        if (audit != svf) if exact else not difference <= TOLERANCE:
            rule = "exact" if exact else f"tolerance {TOLERANCE}"
            return [f"{show(path)}: {audit!r} (audit) vs {svf!r} (SVF), difference {difference!r}, rule {rule}"]
        return []
    if audit != svf:
        return [f"{show(path)}: {audit!r} (audit) vs {svf!r} (SVF)"]
    return []


def lookup(document, path: tuple):
    for part in path:
        if not isinstance(document, dict) or part not in document:
            return MISSING
        document = document[part]
    return document


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        documents = []
        for name in argv:
            with open(name, encoding="utf-8") as handle:
                documents.append(json.load(handle))
    except (OSError, json.JSONDecodeError) as error:
        print(f"compare failed: {error}", file=sys.stderr)
        return 2
    audit, svf = documents

    print("Not compared (printed for the record):")
    for path in NOT_COMPARED:
        for label, document in (("audit", audit), ("SVF", svf)):
            value = lookup(document, path)
            text = "<missing>" if value is MISSING else json.dumps(value, sort_keys=True)
            print(f"  {show(path)} [{label}]: {text}")

    problems = compare(audit, svf)
    if problems:
        print(f"MISMATCH: {len(problems)} difference(s)")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("MATCH: every compared key agrees under the section 7.2 rule")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
