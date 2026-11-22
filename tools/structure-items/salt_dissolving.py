"""Generate fixtures/structures/salt-dissolving.ron (NaCl lattice and dissolving).

Run from the repo root: python tools/structure-items/salt_dissolving.py fixtures/structures/salt-dissolving.ron
Then run: cargo test --release -p svf-core --test salt_dissolving_fixture
"""
import math, sys

A = 5.64            # lattice constant, angstrom
D = A / 2           # Na-Cl distance 2.82
R_NA, R_CL = 1.02, 1.81   # Shannon ionic radii, 6-coordinate
SHRINK = 0.4
R_O, R_H = 1.52 * SHRINK, 1.20 * SHRINK   # van der Waals radii drawn at the same 0.4
OH, HOH = 0.96, 104.5
NA_O, CL_O = 2.4, 3.2      # ion to water-oxygen distance in solution
APPROACH = 4.0             # how far a water molecule travels to reach its place
LEAVE = 7.0                # how far a freed ion travels, along the corner diagonal
NA_COL, CL_COL = (0.60, 0.45, 0.90), (0.30, 0.80, 0.35)
O_COL, H_COL, GRID_COL = (0.85, 0.15, 0.15), (0.95, 0.95, 0.95), (0.62, 0.62, 0.68)

def f(x): return f"{x:.9f}"
def v(p): return "(" + ", ".join(f(c) for c in p) + ")"
def add(a, b): return [a[i] + b[i] for i in range(3)]
def mul(a, k): return [c * k for c in a]

def pos(i, j, k): return [(i - 1.5) * D, (j - 1.5) * D, (k - 1.5) * D]
def is_na(i, j, k): return (i + j + k) % 2 == 0
def ion_id(i, j, k): return f"{'na' if is_na(i, j, k) else 'cl'}-{i}{j}{k}"

AXES = {"px": [1, 0, 0], "nx": [-1, 0, 0], "py": [0, 1, 0], "ny": [0, -1, 0], "pz": [0, 0, 1], "nz": [0, 0, -1]}
SIDE = {"x": [0, 1, 0], "y": [0, 0, 1], "z": [1, 0, 0]}   # the plane each water molecule lies in

def water_atoms(centre, dname, na):
    """O and two H centres for a water molecule beside an ion at `centre`, in direction `dname`."""
    u = AXES[dname]; w = SIDE[dname[1]]
    o = add(centre, mul(u, NA_O if na else CL_O))
    th = math.radians(HOH / 2)
    s = 1.0 if na else -1.0   # hydrogens point away from Na+, towards Cl-
    h1 = add(o, add(mul(u, s * OH * math.cos(th)), mul(w, OH * math.sin(th))))
    h2 = add(o, add(mul(u, s * OH * math.cos(th)), mul(w, -OH * math.sin(th))))
    return o, h1, h2

# The four top corners leave, in this order: Na, Cl, then the other two together.
LEAVERS = [(3, 3, 0), (3, 3, 3), (0, 3, 3), (0, 3, 0)]

def outward(c):
    x, y, z = pos(*c)
    return [("p" if x > 0 else "n") + "x", ("p" if y > 0 else "n") + "y", ("p" if z > 0 else "n") + "z"]

def flip(d): return ("n" if d[0] == "p" else "p") + d[1]

def diag(c):
    p = pos(*c); n = math.sqrt(3)
    return [math.copysign(1, p[0]) / n * LEAVE, math.copysign(1, p[1]) / n * LEAVE, math.copysign(1, p[2]) / n * LEAVE]

parts, actions_by_beat = [], {}
P = parts.append
P('        Part(id: "salt", name: "Salt dissolving in water", parent: None,\n'
  '             description: "A small piece of a sodium chloride crystal, then water molecules pulling ions off its corners.",\n'
  '             colour: (0.5, 0.5, 0.5), geometry: None),')
P('        Part(id: "crystal", name: "Sodium chloride crystal", parent: Some("salt"),\n'
  '             description: "Sodium ions and chloride ions in a cubic lattice, alternating in every direction.",\n'
  '             colour: (0.5, 0.5, 0.5), geometry: None),')
P('        Part(id: "sodium", name: "Sodium ions", parent: Some("crystal"),\n'
  '             description: "Sodium ions, Na+ (purple). A sodium atom has lost one electron, so the ion has one positive charge.",\n'
  '             colour: (0.5, 0.5, 0.5), geometry: None),')
P('        Part(id: "chloride", name: "Chloride ions", parent: Some("crystal"),\n'
  '             description: "Chloride ions, Cl- (green). A chlorine atom has gained one electron, so the ion has one negative charge.",\n'
  '             colour: (0.5, 0.5, 0.5), geometry: None),')
ions = []
for i in range(4):
    for j in range(4):
        for k in range(4):
            na = is_na(i, j, k); r = R_NA if na else R_CL
            ions.append((i, j, k))
            P(f'        Part(id: "{ion_id(i, j, k)}", name: "{"Sodium" if na else "Chloride"} ion {i}{j}{k}", parent: Some("{"sodium" if na else "chloride"}"),\n'
              f'             description: "{"A sodium ion, Na+. Ionic radius 1.02 angstrom." if na else "A chloride ion, Cl-. Ionic radius 1.81 angstrom."}",\n'
              f'             colour: {v(NA_COL if na else CL_COL)},\n'
              f'             geometry: Some(Ellipsoid(centre: {v(pos(i, j, k))}, radii: ({f(r)}, {f(r)}, {f(r)})))),')
P('        Part(id: "grid", name: "Lattice lines", parent: Some("crystal"),\n'
  '             description: "Lines that show the cubic grid the ions sit on. They are a guide for the eye, not bonds.",\n'
  '             colour: (0.5, 0.5, 0.5), geometry: None),')
lo, hi = -1.5 * D, 1.5 * D
for axis, step in (("x", (0, 0, D)), ("y", (D, 0, 0)), ("z", (0, D, 0))):
    for n in range(4):
        c = (n - 1.5) * D
        if axis == "x": a, b = [lo, c, lo], [hi, c, lo]
        if axis == "y": a, b = [lo, lo, c], [lo, hi, c]
        if axis == "z": a, b = [c, lo, lo], [c, lo, hi]
        mid = mul(add(a, b), 0.5)
        up = "(0.0, 0.0, 1.0)" if axis != "z" else "(1.0, 0.0, 0.0)"
        P(f'        Part(id: "grid-{axis}{n}", name: "Lattice lines along {axis}", parent: Some("grid"),\n'
          f'             description: "Lines that show the cubic grid the ions sit on. They are a guide for the eye, not bonds.",\n'
          f'             colour: {v(GRID_COL)},\n'
          f'             geometry: Some(Tube(path: [{v(a)}, {v(mid)}, {v(b)}], half_widths: [(0.06, 0.06), (0.06, 0.06), (0.06, 0.06)], up: {up})),\n'
          f'             repeat: Some(Linear(count: 4, step: {v(step)}))),')
P('        Part(id: "water", name: "Water molecules", parent: Some("salt"),\n'
  '             description: "Water molecules, H2O: red oxygen, white hydrogen. The oxygen end is slightly negative and the hydrogen end slightly positive.",\n'
  '             colour: (0.5, 0.5, 0.5), geometry: None),')

# Water molecules: for each leaving ion, three on its outside (they arrive first) and three on its inside (they join once it is free).
waters = {}   # (ion, dir) -> (group id, authored [o,h1,h2])
for c in LEAVERS:
    na = is_na(*c); iid = ion_id(*c)
    for d in outward(c):
        o, h1, h2 = water_atoms(pos(*c), d, na)
        start = mul(AXES[d], APPROACH)
        waters[(c, d)] = [add(o, start), add(h1, start), add(h2, start)]
    final = add(pos(*c), diag(c))
    for d in [flip(d) for d in outward(c)]:
        o, h1, h2 = water_atoms(final, d, na)
        start = mul(AXES[d], APPROACH)
        waters[(c, d)] = [add(o, start), add(h1, start), add(h2, start)]
for (c, d), atoms in waters.items():
    gid = f"w-{ion_id(*c)}-{d}"
    P(f'        Part(id: "{gid}", name: "Water molecule", parent: Some("water"),\n'
      f'             description: "A water molecule beside {"a sodium" if is_na(*c) else "a chloride"} ion.",\n'
      f'             colour: (0.5, 0.5, 0.5), geometry: None),')
    for name, p, r, col in (("o", atoms[0], R_O, O_COL), ("h1", atoms[1], R_H, H_COL), ("h2", atoms[2], R_H, H_COL)):
        P(f'        Part(id: "{gid}-{name}", name: "{"Oxygen" if name == "o" else "Hydrogen"} atom of a water molecule", parent: Some("{gid}"),\n'
          f'             description: "{"The oxygen atom, the slightly negative end of the molecule." if name == "o" else "A hydrogen atom, at the slightly positive end of the molecule."}",\n'
          f'             colour: {v(col)},\n'
          f'             geometry: Some(Ellipsoid(centre: {v(p)}, radii: ({f(r)}, {f(r)}, {f(r)})))),')

def narr(t, s): return f'Narrate(text: "{t}", for_s: {s:.1f})'
beats = []

b = [narr("This is a tiny piece of a crystal of salt, sodium chloride (NaCl). It is made of ions packed tightly together: sodium ions, Na⁺ (small, purple), and chloride ions, Cl⁻ (large, green). One grain of table salt holds about a billion billion of them.", 10.0)]
beats.append(("crystal", b))

b = [narr("To see the pattern, the ions are now drawn smaller. They sit on a regular cubic grid, sodium and chloride taking turns in every direction. The grey lines only show the grid: they are not bonds.", 9.0)]
for c in ions:
    b.append(f'Grow(part: "{ion_id(*c)}", to: {SHRINK}, for_s: 3.0, ease: Smooth)')
b.append('Show(part: "grid", at_s: 3.5)')
beats.append(("smaller", b))

CENTRE = (1, 1, 2)
NEIGH = [(0, 1, 2), (2, 1, 2), (1, 0, 2), (1, 2, 2), (1, 1, 1), (1, 1, 3)]
assert is_na(*CENTRE) and not any(is_na(*n) for n in NEIGH)
FOCUS = [CENTRE] + NEIGH
b = [narr("Look at one sodium ion. Six chloride ions surround it: above, below, in front, behind, left and right. Each chloride ion is surrounded by six sodium ions in the same way. Opposite charges attract, and this pull in every direction holds the crystal together. This is ionic bonding.", 11.0)]
for c in ions:
    if c not in FOCUS:
        b.append(f'Fade(part: "{ion_id(*c)}", to: 0.1, for_s: 1.5, ease: Smooth)')
b.append('Fade(part: "grid", to: 0.15, for_s: 1.5, ease: Smooth)')
for c in FOCUS:
    b.append(f'Highlight(part: "{ion_id(*c)}", at_s: 1.5)')
beats.append(("neighbours", b))

b = [narr("Count them: 32 sodium ions and 32 chloride ions. There is always one sodium ion for each chloride ion, so the formula is NaCl.", 8.0)]
for c in ions:
    if c not in FOCUS:
        b.append(f'Fade(part: "{ion_id(*c)}", to: 1.0, for_s: 1.5, ease: Smooth)')
b.append('Fade(part: "grid", to: 1.0, for_s: 1.5, ease: Smooth)')
b.append('Highlight(part: "sodium", at_s: 2.0)')
beats.append(("count", b))

b = [narr("Now the crystal is put into water. Water molecules (red oxygen, white hydrogen) have a slightly negative oxygen end and a slightly positive hydrogen end. They crowd round the ions at the corners, which are held by only three neighbours. Oxygen ends turn to sodium ions, hydrogen ends to chloride ions.", 12.0),
     'Fade(part: "grid", to: 0.0, for_s: 1.5, ease: Smooth)']
for c in LEAVERS:
    for d in outward(c):
        gid = f"w-{ion_id(*c)}-{d}"
        b.append(f'Show(part: "{gid}", at_s: 1.5)')
        b.append(f'Move(part: "{gid}", by: {v(mul(AXES[d], -APPROACH))}, for_s: 4.0, at_s: 1.5, ease: Smooth)')
beats.append(("water", b))

def leave(c, at):
    out = []
    iid = ion_id(*c); dv = diag(c)
    out.append(f'Move(part: "{iid}", by: {v(dv)}, for_s: 5.0, at_s: {at:.1f}, ease: Smooth)')
    for d in outward(c):
        out.append(f'Move(part: "w-{iid}-{d}", by: {v(dv)}, for_s: 5.0, at_s: {at:.1f}, ease: Smooth)')
    for d in [flip(d) for d in outward(c)]:
        gid = f"w-{iid}-{d}"
        out.append(f'Show(part: "{gid}", at_s: {at + 2.5:.1f})')
        out.append(f'Move(part: "{gid}", by: {v(mul(AXES[d], -APPROACH))}, for_s: 4.0, at_s: {at + 2.5:.1f}, ease: Smooth)')
    out.append(f'Highlight(part: "{iid}")')
    return out

beats.append(("sodium-leaves", [narr("The water molecules pull a sodium ion off the corner. More water molecules gather round it on the other side. It is now surrounded by six water molecules, their oxygen ends facing it.", 10.0)] + leave(LEAVERS[0], 0.0)))
beats.append(("chloride-leaves", [narr("A chloride ion is pulled away in the same way. Here the water molecules turn their hydrogen ends towards the ion.", 10.0)] + leave(LEAVERS[1], 0.0)))
beats.append(("more-leave", [narr("Ion by ion, starting at the corners and edges, the crystal comes apart.", 8.0), 'Camera(yaw_deg: 30.0, pitch_deg: -40.0, zoom: 1.25, for_s: 5.0, ease: Smooth)'] + leave(LEAVERS[2], 0.0) + leave(LEAVERS[3], 0.0)))
b = [narr("Given enough water, the whole crystal comes apart like this: the salt dissolves. The ions are not destroyed. They spread through the water as separate Na⁺ and Cl⁻ ions, each surrounded by water molecules. This is a salt solution. If the water evaporates, the ions join up again into crystals.", 12.0)]
for c in LEAVERS:
    b.append(f'Highlight(part: "{ion_id(*c)}")')
beats.append(("dissolved", b))

head = """// Salt (sodium chloride) as an ionic lattice, then dissolving in water. Schematic.
// Proof of concept for the item register (docs/item-output-bar.md); not for lessons as it stands.
// Generated by tools/structure-items/salt_dissolving.py; edit that, not this file. Units are angstrom.
// The lattice is a 4 x 4 x 4 piece of the rock-salt structure (32 Na+, 32 Cl-), lattice constant 5.64 angstrom, so
// Na-Cl is 2.82. Ions are authored at their ionic radii (Shannon, 6-coordinate: Na+ 1.02, Cl- 1.81), touching as in the
// real crystal, and are shrunk to 0.4 of that in the second beat. Water molecules are drawn at 0.4 of the van der Waals
// radii (O 1.52, H 1.20) with O-H 0.96 angstrom and H-O-H 104.5 degrees. In the solution the ion to water-oxygen
// distance is 2.4 (Na+) and 3.2 (Cl-) angstrom, with six water molecules round each ion on the three axes.
// Simplified: the real number of water molecules round an ion varies (about 5 to 6 for Na+, 6 to 7 for Cl-) and they
// move all the time; for Cl- one O-H points at the ion, here both hydrogens face it equally. Ions really leave from
// corners, edges and steps all over the crystal, helped by the random motion of the water; here four corner ions leave
// in turn along the corner diagonals.
Structure(
    version: 1,
    name: "Salt: an ionic lattice that dissolves in water",
    units: "angstrom",
    reference: "To be confirmed by the author: textbook account of ionic bonding and the sodium chloride lattice (each ion has six oppositely charged neighbours) and of dissolving as the separation of ions by polar water molecules. Lattice constant 5.64 angstrom; Shannon ionic radii Na+ 1.02, Cl- 1.81 angstrom; ion to water-oxygen distances about 2.4 and 3.2 angstrom. Schematic.",
    circle_segments: 20,
    parts: [
"""
out = [head, "\n".join(parts), "\n    ],\n    layers: [],\n    views: [],\n    timelines: [\n"]
out.append('        Timeline(id: "dissolving", name: "Salt dissolving in water",\n')
out.append('            camera: Some((yaw_deg: 30.0, pitch_deg: -40.0, zoom: 1.8)),\n')
out.append('            scenes: [\n                Scene(id: "salt", beats: [\n')
for bid, acts in beats:
    out.append(f'                    Beat(id: "{bid}", actions: [\n')
    for a in acts:
        out.append(f'                        {a},\n')
    out.append('                    ]),\n')
out.append('                ]),\n            ]),\n    ],\n)\n')
open(sys.argv[1], "w", encoding="utf-8", newline="\n").write("".join(out))
print("parts", len(parts))
