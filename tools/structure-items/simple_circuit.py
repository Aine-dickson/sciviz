"""Generate fixtures/structures/simple-circuit.ron (a cell, wires, a switch and a lamp).

Run from the repo root: python tools/structure-items/simple_circuit.py fixtures/structures/simple-circuit.ron
Then run: cargo test --release -p svf-core --test simple_circuit_fixture
"""
import math, sys

# Layout (cm, the XY plane, facing +Z). The loop is a rounded rectangle.
X0, X1, Y0, Y1, R = -10.0, 10.0, -6.0, 6.0, 1.5
CELL_NEG, CELL_POS, NUB = -2.5, 2.5, 2.9          # cell along the bottom wire: negative end left, positive (nub) right
PIVOT, CONTACT = (-10.0, -1.5), (-10.0, 1.5)      # switch on the left wire
LEVER = CONTACT[1] - PIVOT[1]
OPEN_DEG = 35.0                                   # open lever leans out of the loop (to -x)
HOLDER = (-1.0, 1.0)                              # lamp holder on the top wire
FIL_Y, SUP_X = 8.0, 0.7
WIRE_HW, WIRE_OPACITY = 0.3, 0.35
E_RADIUS, E_SPACING, E_SPEED = 0.2, 1.0, 2.0

def f(x): return f"{x:.6f}"
def v(p): return "(" + ", ".join(f(c) for c in (list(p) + [0.0] * (3 - len(p)))) + ")"

def line(a, b, step=1.0):
    n = max(1, math.ceil(math.dist(a, b) / step))
    return [(a[0] + (b[0] - a[0]) * i / n, a[1] + (b[1] - a[1]) * i / n) for i in range(n + 1)]

def arc(centre, r, a0, a1, step_deg=15.0):
    n = max(1, math.ceil(abs(a1 - a0) / step_deg))
    return [(centre[0] + r * math.cos(math.radians(a0 + (a1 - a0) * i / n)),
             centre[1] + r * math.sin(math.radians(a0 + (a1 - a0) * i / n))) for i in range(n + 1)]

def join(*pieces):
    out = []
    for p in pieces:
        for q in p:
            if not out or math.dist(out[-1], q) > 1e-9:
                out.append(q)
    return out

# The loop in the direction the electrons go: out of the negative terminal, left, up through the switch,
# right through the lamp, down, and back left into the positive terminal.
bottom_left = join(line((CELL_NEG, Y0), (X0 + R, Y0)), arc((X0 + R, Y0 + R), R, 270, 180), line((X0, Y0 + R), PIVOT))
left_top = join(line(CONTACT, (X0, Y1 - R)), arc((X0 + R, Y1 - R), R, 180, 90), line((X0 + R, Y1), (HOLDER[0], Y1)))
right = join(line((HOLDER[1], Y1), (X1 - R, Y1)), arc((X1 - R, Y1 - R), R, 90, 0), line((X1, Y1 - R), (X1, Y0 + R)),
             arc((X1 - R, Y0 + R), R, 0, -90), line((X1 - R, Y0), (NUB, Y0)))
filament = [(-SUP_X + 2 * SUP_X * i / 24, FIL_Y + 0.15 * math.sin(2 * math.pi * 2 * i / 24)) for i in range(25)]   # a coil seen side on: two waves
through_lamp = join([(HOLDER[0], Y1), (-SUP_X, Y1), (-SUP_X, FIL_Y)], filament, [(SUP_X, FIL_Y), (SUP_X, Y1), (HOLDER[1], Y1)])
through_cell = line((NUB, Y0), (CELL_NEG, Y0))
loop = join(bottom_left, [PIVOT, CONTACT], left_top, through_lamp, right, through_cell)
loop_length = sum(math.dist(a, b) for a, b in zip(loop, loop[1:]))
electrons = round(loop_length / E_SPACING)

def tube(path, hw):
    pts = ", ".join(v(p) for p in path)
    hws = ", ".join(f"({f(hw)}, {f(hw)})" for _ in path)
    return f"Tube(path: [{pts}], half_widths: [{hws}], up: (0.0, 0.0, 1.0))"

parts = []
def part(pid, name, parent, desc, colour, geometry, opacity=None):
    op = f", opacity: {opacity}" if opacity is not None else ""
    par = f'Some("{parent}")' if parent else "None"
    geo = f"Some({geometry})" if geometry else "None"
    parts.append(f'        Part(id: "{pid}", name: "{name}", parent: {par},\n             description: "{desc}",\n             colour: {v(colour)}{op}, geometry: {geo}),')

GREY = (0.5, 0.5, 0.5)
part("circuit", "Simple circuit", None, "A cell, wires, a switch and a lamp joined in one loop.", GREY, None)
part("cell", "Cell", "circuit", "A cell (often called a battery). Its chemicals push charge round the circuit. Negative terminal on the left, positive terminal (the small bump) on the right.", GREY, None)
part("cell-body", "Cell body", "cell", "The case of the cell; the flat end is the negative terminal.", (0.18, 0.18, 0.20), tube([(CELL_NEG, Y0), (0.0, Y0), (1.2, Y0)], 1.0))
part("cell-top", "Positive end of the cell", "cell", "The positive end of the cell.", (0.80, 0.55, 0.20), tube([(1.2, Y0), (1.85, Y0), (CELL_POS, Y0)], 1.0))
part("cell-nub", "Positive terminal", "cell", "The positive terminal of the cell.", (0.75, 0.75, 0.78), tube([(CELL_POS, Y0), (2.7, Y0), (NUB, Y0)], 0.35))
part("wires", "Wires", "circuit", "Copper wires in a clear coat. The metal is full of electrons that are free to move.", GREY, None)
desc = "A copper wire in a clear coat."
part("wire-1", "Wire from the cell to the switch", "wires", desc, (0.85, 0.45, 0.20), tube(join(line((CELL_NEG, Y0), (X0 + R, Y0)), arc((X0 + R, Y0 + R), R, 270, 180), line((X0, Y0 + R), PIVOT)), WIRE_HW), WIRE_OPACITY)
part("wire-2", "Wire from the switch to the lamp", "wires", desc, (0.85, 0.45, 0.20), tube(left_top, WIRE_HW), WIRE_OPACITY)
part("wire-3", "Wire from the lamp to the cell", "wires", desc, (0.85, 0.45, 0.20), tube(right, WIRE_HW), WIRE_OPACITY)
part("switch", "Switch", "circuit", "A switch: a metal lever that closes or opens a gap in the circuit.", GREY, None)
part("switch-pivot", "Switch pivot", "switch", "The contact the lever turns about.", (0.70, 0.70, 0.74), f"Ellipsoid(centre: {v(PIVOT)}, radii: (0.4, 0.4, 0.4))")
part("switch-contact", "Switch contact", "switch", "The contact the lever touches when the switch is closed.", (0.70, 0.70, 0.74), f"Ellipsoid(centre: {v(CONTACT)}, radii: (0.4, 0.4, 0.4))")
s, c = math.sin(math.radians(OPEN_DEG)), math.cos(math.radians(OPEN_DEG))
lever_end = (PIVOT[0] - LEVER * s, PIVOT[1] + LEVER * c)
lever_mid = ((PIVOT[0] + lever_end[0]) / 2, (PIVOT[1] + lever_end[1]) / 2)
part("switch-lever", "Switch lever", "switch", "The metal lever. Down (closed): charge can flow across. Up (open): there is a gap.", (0.78, 0.78, 0.82), tube([PIVOT, lever_mid, lever_end], 0.15))
part("lamp", "Lamp", "circuit", "A filament lamp (bulb). Current through the very thin filament makes it hot enough to glow.", GREY, None)
part("lamp-holder", "Lamp holder", "lamp", "The holder joins the lamp to the wires.", (0.35, 0.35, 0.38), tube([(HOLDER[0], Y1), (0.0, Y1), (HOLDER[1], Y1)], 0.6))
part("lamp-supports", "Filament supports", "lamp", "Two wires that carry the current up to the filament and back.", (0.6, 0.6, 0.62), None)
part("lamp-support-1", "Filament support", "lamp-supports", "A wire that carries current to the filament.", (0.6, 0.6, 0.62), tube([(-SUP_X, Y1 + 0.6), (-SUP_X, (Y1 + FIL_Y) / 2), (-SUP_X, FIL_Y)], 0.06))
part("lamp-support-2", "Filament support", "lamp-supports", "A wire that carries current back from the filament.", (0.6, 0.6, 0.62), tube([(SUP_X, Y1 + 0.6), (SUP_X, (Y1 + FIL_Y) / 2), (SUP_X, FIL_Y)], 0.06))
part("filament", "Filament", "lamp", "A very thin coiled metal wire. With a current it gets hot and glows.", (0.35, 0.30, 0.30), tube(filament, 0.05))
part("glass", "Glass bulb", "lamp", "The glass bulb round the filament.", (0.80, 0.85, 0.90), f"Ellipsoid(centre: {v((0.0, FIL_Y))}, radii: (1.6, 1.9, 1.6))", 0.25)
part("glow", "Glow", "lamp", "The light given out by the hot filament.", (1.0, 0.85, 0.35), f"Ellipsoid(centre: {v((0.0, FIL_Y))}, radii: (1.1, 1.1, 1.1))", 0.45)

def narr(t, s, at=None): return f'Narrate(text: "{t}", for_s: {s:.1f}' + (f", at_s: {at:.1f}" if at is not None else "") + ")"
closed_mid = (PIVOT[0], (PIVOT[1] + CONTACT[1]) / 2)
close_by = (closed_mid[0] - lever_mid[0], closed_mid[1] - lever_mid[1])
def lever(sign, at, ease="Smooth"):
    by = (sign * close_by[0], sign * close_by[1])
    return [f'Turn(part: "switch-lever", axis: (0.0, 0.0, 1.0), deg: {f(-sign * OPEN_DEG)}, for_s: 1.5, at_s: {at:.1f}, ease: {ease})',
            f'Move(part: "switch-lever", by: {v(by)}, for_s: 1.5, at_s: {at:.1f}, ease: {ease})']

ON, OFF = (1.0, 0.95, 0.60), (0.35, 0.30, 0.30)
GLASS_ON, GLASS_OFF = (1.0, 0.95, 0.75), (0.80, 0.85, 0.90)
FLOW_FOR = 23.5
beats = []
beats.append(("parts", [narr("A simple circuit. The cell (bottom) pushes charge round. Copper wires join it to a switch (left) and a lamp (top). Every part is joined into one loop.", 12.0),
                        'Highlight(part: "cell")', 'Highlight(part: "wires", at_s: 3.0)', 'Highlight(part: "switch", at_s: 6.0)', 'Highlight(part: "lamp", at_s: 9.0)',
                        # Labels stay for the whole timeline, as in a textbook diagram.
                        f'Label(part: "cell", text: Some("Cell"), at: Some({v((-0.6, Y0 + 1.0, 0.0))}))',
                        f'Label(part: "cell-nub", text: Some("Positive terminal (+)"), side: Some(Right), at_s: 1.0)',
                        f'Label(part: "cell-body", text: Some("Negative terminal (−)"), at: Some({v((CELL_NEG, Y0, 0.0))}), side: Some(Left), at_s: 1.0)',
                        f'Label(part: "wire-3", text: Some("Copper wire"), at: Some({v((X1, 0.0, WIRE_HW))}), at_s: 3.0)',
                        'Label(part: "switch", text: Some("Switch"), at_s: 6.0)',
                        f'Label(part: "lamp", text: Some("Lamp"), at: Some({v((-1.1, FIL_Y + 1.35, 0.0))}), side: Some(Left), at_s: 9.0)',
                        f'Label(part: "filament", text: Some("Filament"), at: Some({v((0.0, FIL_Y + 0.15, 0.0))}), side: Some(Right), at_s: 9.0)']))
beats.append(("open", [narr("The switch is open: its lever is up, leaving a gap. The loop is broken, so no charge flows round. There is no current, and the lamp is off.", 9.0),
                       'Highlight(part: "switch-lever")']))
FLOW_AT = 1.5                      # the current starts the moment the lever arrives
OPEN_AT = FLOW_AT + FLOW_FOR       # and stops the moment it starts to lift
cur = [narr("Close the switch. The lever touches the contact and the loop is complete.", 3.0, 0.0)] + lever(+1, 0.0) + [
       narr("Now there is a current. Electrons were already in all the metal. They all start to move at once, out of the negative terminal of the cell, round the loop and back into its positive terminal. So the lamp lights straight away.", 9.0, 3.0),
       narr("The current is the same everywhere in the loop: in every wire, through the switch, through the lamp's filament and through the cell. No charge is used up; it goes round and round.", 7.0, 12.0),
       narr("The cell gives the charge energy. In the thin filament that energy is transferred as heat and light. (Real electrons drift slowly, under a millimetre a second; here they are shown much faster.)", 6.0, 19.0),
       f'Flow(id: "electrons", count: {electrons}, path: [' + ", ".join(v(p) for p in loop) + f'], speed: {E_SPEED}, lateral: 0.05, seed: 3, particle_radius: {E_RADIUS}, colour: (0.25, 0.55, 1.00), for_s: {FLOW_FOR:.1f}, at_s: {FLOW_AT:.1f}, key: Some("Electron"))',
       f'Colour(part: "filament", to: {v(ON)}, for_s: 0.3, at_s: {FLOW_AT:.1f})', f'Colour(part: "glass", to: {v(GLASS_ON)}, for_s: 0.3, at_s: {FLOW_AT:.1f})', f'Show(part: "glow", at_s: {FLOW_AT:.1f})',
       f'Colour(part: "filament", to: {v(OFF)}, for_s: 0.3, at_s: {OPEN_AT:.1f})', f'Colour(part: "glass", to: {v(GLASS_OFF)}, for_s: 0.3, at_s: {OPEN_AT:.1f})',
       f'Hide(part: "glow", at_s: {OPEN_AT:.1f})'] + lever(-1, OPEN_AT)
beats.append(("current", cur))
beats.append(("off", [narr("Open the switch again. The gap breaks the loop: the current stops everywhere at once, and the lamp goes out.", 8.0), 'Highlight(part: "switch-lever")']))
beats.append(("summary", [narr("A current flows only round a complete circuit, and a switch makes or breaks the loop. Circuit diagrams show the conventional current, from the positive terminal to the negative, the opposite way to the electrons.", 10.0)]))

head = """// A simple circuit: a cell, wires, a switch and a filament lamp. Schematic.
// Proof of concept for the item register (docs/item-output-bar.md); not for lessons as it stands.
// Generated by tools/structure-items/simple_circuit.py; edit that, not this file. Units are centimetres; the circuit lies in the XY plane.
// The moving particles are electrons in a closed loop (one Flow), shown only while the switch is closed, at the same speed everywhere
// (the current is the same all round a series loop). The speed is a picture, not the drift speed (really well under 1 mm/s). The path runs
// through the switch, both filament supports and the filament, and through the inside of the cell, where the opaque case hides it.
// The switch lever has no hinge: it closes by a Turn about its own centre plus a Move of that centre, so its end stays near the pivot.
Structure(
    version: 1,
    name: "A simple circuit with a switch",
    units: "cm",
    reference: "To be confirmed by the author: textbook account of a simple series circuit (cell, switch, filament lamp): current flows only in a complete circuit, is the same all round a series loop, and is carried in metals by electrons moving from the negative to the positive terminal; conventional current is drawn from positive to negative. Schematic.",
    circle_segments: 20,
    parts: [
"""
out = [head, "\n".join(parts), "\n    ],\n    layers: [],\n    views: [],\n    timelines: [\n",
       '        Timeline(id: "circuit", name: "Closing and opening the switch",\n',
       '            camera: Some((yaw_deg: 40.0, pitch_deg: -20.0, zoom: 1.3)),\n',
       '            scenes: [\n                Scene(id: "circuit", beats: [\n']
for bid, acts in beats:
    out.append(f'                    Beat(id: "{bid}", actions: [\n')
    out += [f"                        {a},\n" for a in acts]
    out.append("                    ]),\n")
out.append("                ]),\n            ]),\n    ],\n)\n")
open(sys.argv[1], "w", encoding="utf-8", newline="\n").write("".join(out))
print(f"parts {len(parts)}, loop {loop_length:.2f} cm, electrons {electrons}")
