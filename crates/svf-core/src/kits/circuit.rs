//! The circuit kit (`docs/item-document.md` section 6): cells, switches and
//! lamps joined in one series loop. The kit places the objects, routes the
//! wires, and owns the rule that a current flows round the loop if and only if
//! every switch in it is closed. Authors cannot animate the current or the lamp.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::item::{Fidelity, ItemError, KitBuild, LabelDefault, Level, Named, Timed, Verb};
use crate::structure::{Geometry, PartDocument};
use crate::structure_animation::{ActionDocument, Ease};
use crate::structure_labels::LabelSide;
use crate::structure_lesson::ViewCamera;
use crate::structure_route::{point_along, rounded_route};

/// How long a lever takes to swing open or shut.
pub const LEVER_SWING_S: f64 = 1.5;
/// How far an open lever leans out of the loop.
pub const OPEN_DEG: f64 = 35.0;

// The loop: a rounded rectangle in the XY plane, in centimetres.
const X0: f64 = -8.0;
const X1: f64 = 8.0;
const Y0: f64 = -5.0;
const Y1: f64 = 5.0;
const CORNER: f64 = 1.5;
const SPACING: f64 = 0.25;
const CLEARANCE: f64 = 0.4;
const WIRE_HW: f64 = 0.25;
const WIRE_OPACITY: f64 = 0.35;
const COPPER: (f64, f64, f64) = (0.85, 0.45, 0.20);

// LR6 (AA) cell, IEC 60086-2 limits: at most 14.5 mm across and 50.5 mm long; positive contact at most 5.5 mm across.
pub const CELL_DIAMETER: f64 = 1.45;
pub const CELL_LENGTH: f64 = 5.05;
const NUB_DIAMETER: f64 = 0.55;
const NUB_HEIGHT: f64 = 0.15;
const TOP_BAND: f64 = 0.8;

// MES (E10) filament lamp: 10 mm screw cap, 11 mm glass bulb, about 24 mm overall; and a holder.
const HOLDER_HALF: f64 = 1.0;
const HOLDER_R: f64 = 0.5;
const CAP_R: f64 = 0.5;
const CAP_TOP: f64 = HOLDER_R + 0.9;
pub const BULB_DIAMETER: f64 = 1.1;
const BULB_HALF: f64 = 0.75;
const FIL_Y: f64 = CAP_TOP + BULB_HALF;
const SUP_X: f64 = 0.22;
const COIL_AMPLITUDE: f64 = 0.035;

// Knife switch (schematic: no standard size).
const GAP: f64 = 3.0;
const BALL: f64 = 0.4;
const LEVER_HW: f64 = 0.15;

// Electrons, drawn much faster than they drift.
const E_SPACING: f64 = 1.0;
pub const E_SPEED: f64 = 2.0;
const E_RADIUS: f64 = 0.12;
const E_LATERAL: f64 = 0.05;
pub const E_COLOUR: (f64, f64, f64) = (0.25, 0.55, 1.0);
pub const FILAMENT_ON: (f64, f64, f64) = (1.0, 0.95, 0.60);
pub const FILAMENT_OFF: (f64, f64, f64) = (0.35, 0.30, 0.30);
const GLASS_ON: (f64, f64, f64) = (1.0, 0.95, 0.75);
const GLASS_OFF: (f64, f64, f64) = (0.80, 0.85, 0.90);
const WARM_UP_S: f64 = 0.3;
const EPS: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum CellKind {
    #[default]
    #[serde(rename = "AA")]
    Aa,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum LampKind {
    #[default]
    #[serde(rename = "MES")]
    Mes,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum SwitchState {
    #[default]
    Open,
    Closed,
}

/// The kit's objects, as the item compiler hands them over.
#[derive(Clone, Debug, PartialEq)]
pub enum CircuitObject {
    Cell { id: String, kind: CellKind },
    Switch { id: String, starts: SwitchState },
    Lamp { id: String, kind: LampKind },
    Loop(Vec<String>),
}

/// The kit's verbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CircuitVerb {
    Close,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Cell,
    Switch(SwitchState),
    Lamp,
}

/// A member's place: centre, the electrons' direction `u` and the outward normal `n`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    c: [f64; 2],
    u: [f64; 2],
    n: [f64; 2],
}

impl Frame {
    fn at(&self, x: f64, y: f64) -> [f64; 3] {
        [self.c[0] + x * self.u[0] + y * self.n[0], self.c[1] + x * self.u[1] + y * self.n[1], 0.0]
    }
}

/// The four sides in the electrons' order (clockwise seen from +Z): start corner, direction, outward normal.
const SIDES: [([f64; 2], [f64; 2], [f64; 2]); 4] = [
    ([X1, Y0], [-1.0, 0.0], [0.0, -1.0]), // bottom, right to left
    ([X0, Y0], [0.0, 1.0], [-1.0, 0.0]),  // left, upwards
    ([X0, Y1], [1.0, 0.0], [0.0, 1.0]),   // top, left to right
    ([X1, Y1], [0.0, -1.0], [1.0, 0.0]),  // right, downwards
];

fn side_length(side: usize) -> f64 {
    if side % 2 == 0 { X1 - X0 } else { Y1 - Y0 }
}

fn half_length(kind: Kind) -> f64 {
    match kind {
        Kind::Cell => CELL_LENGTH / 2.0,
        Kind::Switch(_) => GAP / 2.0,
        Kind::Lamp => HOLDER_HALF,
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Member {
    id: String,
    kind: Kind,
    side: usize,
    frame: Frame,
}

impl Member {
    fn entry(&self) -> [f64; 3] {
        self.frame.at(-half_length(self.kind), 0.0)
    }

    fn exit(&self) -> [f64; 3] {
        self.frame.at(half_length(self.kind), 0.0)
    }

    /// The electrons' path through the member, entry to exit.
    fn inside(&self) -> Vec<[f64; 3]> {
        let f = &self.frame;
        match self.kind {
            Kind::Cell | Kind::Switch(_) => vec![self.entry(), self.exit()],
            Kind::Lamp => {
                let mut p = vec![self.entry(), f.at(-SUP_X, 0.0)];
                p.extend(coil().into_iter().map(|(x, y)| f.at(x, y)));
                p.extend([f.at(SUP_X, 0.0), self.exit()]);
                p
            }
        }
    }
}

/// The filament seen side on: two waves of a coil, from one support to the other.
fn coil() -> Vec<(f64, f64)> {
    (0..=24)
        .map(|i| {
            let s = f64::from(i) / 24.0;
            (-SUP_X + 2.0 * SUP_X * s, FIL_Y + COIL_AMPLITUDE * (std::f64::consts::TAU * 2.0 * s).sin())
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
struct SwitchTrack {
    closed: bool,
    initially_closed: bool,
    /// `(time, closed)`: when the switch counts as closed or open.
    events: Vec<(f64, bool)>,
    busy_until: f64,
    pivot: [f64; 3],
    /// Signed angle from the open lever to the closed one, about +Z.
    close_deg: f64,
}

/// The built circuit and the state of its switches through the story.
#[derive(Clone, Debug, PartialEq)]
pub struct CircuitKit {
    switches: BTreeMap<String, SwitchTrack>,
    lamps: Vec<String>,
    loop_path: Vec<[f64; 3]>,
}

fn tube(path: Vec<[f64; 3]>, hw: f64) -> Geometry {
    let half_widths = vec![(hw, hw); path.len()];
    Geometry::Tube {
        path,
        half_widths,
        up: [0.0, 0.0, 1.0],
        steps_per_span: 8,
        wall: None,
        arc_deg: None,
        corner_radius: None,
        point_spacing: None,
    }
}

fn straight(a: [f64; 3], b: [f64; 3], hw: f64) -> Geometry {
    let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, 0.0];
    tube(vec![a, mid, b], hw)
}

fn ball(centre: [f64; 3], r: f64) -> Geometry {
    Geometry::Ellipsoid { centre, radii: [r, r, r], rotate_deg: [0.0; 3], wall: None }
}

fn part(id: &str, name: &str, parent: &str, description: &str, colour: (f64, f64, f64), geometry: Option<Geometry>) -> PartDocument {
    PartDocument {
        id: id.to_owned(),
        name: name.to_owned(),
        parent: Some(parent.to_owned()),
        description: description.to_owned(),
        colour,
        opacity: 1.0,
        geometry,
        repeat: None,
        mirror: None,
    }
}

fn side_of(p: [f64; 3]) -> Option<LabelSide> {
    if p[0] > 1.0 {
        Some(LabelSide::Right)
    } else if p[0] < -1.0 {
        Some(LabelSide::Left)
    } else {
        None
    }
}

fn fidelity(form: Level, behaviour: Level, appearance: Level, source: &str) -> Fidelity {
    Fidelity { form, behaviour, appearance, source: source.to_owned() }
}

struct Builder {
    out: KitBuild,
}

impl Builder {
    fn name(&mut self, name: String, part: Option<&str>, point: Option<[f64; 3]>, label: Option<(&str, &str, Option<[f64; 3]>)>) {
        let label = label.map(|(on, text, at)| LabelDefault {
            part: on.to_owned(),
            text: text.to_owned(),
            at,
            side: side_of(at.or(point).unwrap_or([0.0; 3])),
        });
        self.out.names.push((name, Named { part: part.map(str::to_owned), point, label }));
    }

    fn add(&mut self, p: PartDocument, f: Fidelity) {
        if p.geometry.is_some() {
            self.out.fidelity.push((p.id.clone(), f));
        }
        self.out.parts.push(p);
    }
}

impl CircuitKit {
    /// Place the objects round the loop, route the wires, and name everything.
    pub fn build(objects: &[CircuitObject], root: &str) -> Result<(Self, KitBuild), ItemError> {
        let invalid = |s: String| ItemError::Invalid(s);
        let mut kinds: BTreeMap<String, Kind> = BTreeMap::new();
        let mut loops: Vec<&Vec<String>> = Vec::new();
        for o in objects {
            let (id, kind) = match o {
                CircuitObject::Cell { id, kind: CellKind::Aa } => (id, Kind::Cell),
                CircuitObject::Switch { id, starts } => (id, Kind::Switch(*starts)),
                CircuitObject::Lamp { id, kind: LampKind::Mes } => (id, Kind::Lamp),
                CircuitObject::Loop(members) => {
                    loops.push(members);
                    continue;
                }
            };
            if !crate::item::is_valid_id(id) || id == root || id == "wire" {
                return Err(invalid(format!(
                    "the object id '{id}' must be lowercase letters, digits and hyphens, and not '{root}' or 'wire'"
                )));
            }
            if kinds.insert(id.clone(), kind).is_some() {
                return Err(invalid(format!("two objects have the id '{id}'")));
            }
        }
        let [members] = loops.as_slice() else {
            return Err(invalid(format!("a circuit needs exactly one Loop (this item has {})", loops.len())));
        };
        for (i, id) in members.iter().enumerate() {
            if !kinds.contains_key(id) {
                return Err(invalid(format!("the loop names '{id}', which is not a cell, switch or lamp")));
            }
            if members[..i].contains(id) {
                return Err(invalid(format!("'{id}' is in the loop twice")));
            }
        }
        if let Some(id) = kinds.keys().find(|id| !members.contains(id)) {
            return Err(invalid(format!("'{id}' is not in the loop: every circuit object must be joined in")));
        }
        let cells = members.iter().filter(|id| kinds[*id] == Kind::Cell).count();
        if cells != 1 || kinds[&members[0]] != Kind::Cell {
            return Err(invalid(format!("the loop must start at its one cell (it has {cells} cells)")));
        }

        // Sides: the cell on the bottom; the others go left, top, right, with any extras on top.
        let rest = members.len() - 1;
        let counts: [usize; 3] = match rest {
            0 => return Err(invalid("a loop needs something besides the cell".to_owned())),
            1 => [0, 1, 0],
            2 => [1, 1, 0],
            n => [1, n - 2, 1],
        };
        let mut placed: Vec<Member> = Vec::new();
        let mut assign = vec![(0usize, 0usize, 1usize)]; // (side, index on side, count on side)
        for (s, &count) in counts.iter().enumerate() {
            for i in 0..count {
                assign.push((s + 1, i, count));
            }
        }
        for (id, &(side, index, count)) in members.iter().zip(&assign) {
            let (start, u, n) = SIDES[side];
            let len = side_length(side);
            #[allow(clippy::cast_precision_loss)]
            let along = len * (index + 1) as f64 / (count + 1) as f64;
            let kind = kinds[id];
            let h = half_length(kind);
            let room = if count == 1 { (len / 2.0 - CORNER - CLEARANCE).min(f64::INFINITY) } else {
                #[allow(clippy::cast_precision_loss)]
                let spacing = len / (count + 1) as f64;
                ((spacing - 2.0 * CLEARANCE) / 2.0).min(along - CORNER - CLEARANCE)
            };
            if h > room + EPS {
                return Err(invalid(format!("there is no room for '{id}' on its side of the loop: use fewer objects")));
            }
            let c = [start[0] + u[0] * along, start[1] + u[1] * along];
            placed.push(Member { id: id.clone(), kind, side, frame: Frame { c, u, n } });
        }

        let mut b = Builder { out: KitBuild { units: "cm", ..KitBuild::default() } };
        let mut switches = BTreeMap::new();
        let mut lamps = Vec::new();
        let datasheet = "nominal size from the LR6 (AA) limits of IEC 60086-2 (to be checked)";
        let lamp_sheet = "nominal MES (E10) lamp: 10 mm cap, 11 mm bulb (to be checked against a datasheet)";
        for m in &placed {
            let f = m.frame;
            let id = m.id.as_str();
            let pid = |piece: &str| format!("{id}-{piece}");
            let named = |piece: &str| format!("{id}.{piece}");
            match m.kind {
                Kind::Cell => {
                    let l = CELL_LENGTH / 2.0;
                    let (nub_end, band_end) = (-l + NUB_HEIGHT, -l + NUB_HEIGHT + TOP_BAND);
                    b.add(part(id, "Cell", root, "A cell (often called a battery). Its chemicals push charge round the circuit. The flat end is the negative terminal; the small bump is the positive terminal.", (0.5, 0.5, 0.5), None),
                          fidelity(Level::F0, Level::F0, Level::F0, ""));
                    b.add(part(&pid("body"), "Cell body", id, "The case of the cell; the flat end is the negative terminal.", (0.18, 0.18, 0.20),
                               Some(straight(f.at(band_end, 0.0), f.at(l, 0.0), CELL_DIAMETER / 2.0))),
                          fidelity(Level::F2, Level::F1, Level::F1, datasheet));
                    b.add(part(&pid("top"), "Positive end of the cell", id, "The positive end of the cell.", (0.80, 0.55, 0.20),
                               Some(straight(f.at(nub_end, 0.0), f.at(band_end, 0.0), CELL_DIAMETER / 2.0))),
                          fidelity(Level::F2, Level::F1, Level::F1, datasheet));
                    b.add(part(&pid("positive"), "Positive terminal", id, "The positive terminal of the cell.", (0.75, 0.75, 0.78),
                               Some(straight(f.at(-l, 0.0), f.at(nub_end, 0.0), NUB_DIAMETER / 2.0))),
                          fidelity(Level::F2, Level::F1, Level::F1, datasheet));
                    let top_surface = f.at(0.0, -CELL_DIAMETER / 2.0);
                    b.name(id.to_owned(), Some(id), Some(f.at(0.0, 0.0)), Some((id, "Cell", Some(top_surface))));
                    b.name(named("body"), Some(&pid("body")), None, Some((&pid("body"), "Cell body", None)));
                    b.name(named("top"), Some(&pid("top")), None, Some((&pid("top"), "Positive end", None)));
                    b.name(named("positive"), Some(&pid("positive")), Some(m.entry()), Some((&pid("positive"), "Positive terminal (+)", Some(m.entry()))));
                    b.name(named("negative"), Some(&pid("body")), Some(m.exit()), Some((&pid("body"), "Negative terminal (−)", Some(m.exit()))));
                }
                Kind::Switch(starts) => {
                    let (pivot, contact) = (m.entry(), m.exit());
                    let (s, c) = OPEN_DEG.to_radians().sin_cos();
                    let open_dir = [c * f.u[0] + s * f.n[0], c * f.u[1] + s * f.n[1]];
                    let close_deg = (open_dir[0] * f.u[1] - open_dir[1] * f.u[0]).atan2(c).to_degrees();
                    let lever_end = match starts {
                        SwitchState::Closed => contact,
                        SwitchState::Open => [pivot[0] + GAP * open_dir[0], pivot[1] + GAP * open_dir[1], 0.0],
                    };
                    let metal = (0.70, 0.70, 0.74);
                    b.add(part(id, "Switch", root, "A switch: a metal lever that closes or opens a gap in the circuit.", (0.5, 0.5, 0.5), None),
                          fidelity(Level::F0, Level::F0, Level::F0, ""));
                    b.add(part(&pid("pivot"), "Switch pivot", id, "The contact the lever turns about.", metal, Some(ball(pivot, BALL))),
                          fidelity(Level::F0, Level::F1, Level::F1, "schematic knife switch"));
                    b.add(part(&pid("contact"), "Switch contact", id, "The contact the lever touches when the switch is closed.", metal, Some(ball(contact, BALL))),
                          fidelity(Level::F0, Level::F1, Level::F1, "schematic knife switch"));
                    b.add(part(&pid("lever"), "Switch lever", id, "The metal lever. Shut (closed): charge can flow across. Up (open): there is a gap.", (0.78, 0.78, 0.82),
                               Some(straight(pivot, lever_end, LEVER_HW))),
                          fidelity(Level::F0, Level::F1, Level::F1, "schematic knife switch; hinged turn"));
                    b.name(id.to_owned(), Some(id), Some(f.at(0.0, 0.0)), Some((id, "Switch", None)));
                    b.name(named("lever"), Some(&pid("lever")), None, Some((&pid("lever"), "Lever", None)));
                    b.name(named("pivot"), Some(&pid("pivot")), Some(pivot), Some((&pid("pivot"), "Pivot", None)));
                    b.name(named("contact"), Some(&pid("contact")), Some(contact), Some((&pid("contact"), "Contact", None)));
                    let closed = starts == SwitchState::Closed;
                    switches.insert(id.to_owned(), SwitchTrack { closed, initially_closed: closed, events: Vec::new(), busy_until: 0.0, pivot, close_deg });
                }
                Kind::Lamp => {
                    let along_n = f.n[0].abs() > 0.5;
                    let bulb_radii = if along_n { [BULB_HALF, BULB_DIAMETER / 2.0, BULB_DIAMETER / 2.0] } else { [BULB_DIAMETER / 2.0, BULB_HALF, BULB_DIAMETER / 2.0] };
                    let glow_r = BULB_DIAMETER / 2.0 * 0.8;
                    let centre = f.at(0.0, FIL_Y);
                    b.add(part(id, "Lamp", root, "A filament lamp (bulb). Current through the very thin filament makes it hot enough to glow.", (0.5, 0.5, 0.5), None),
                          fidelity(Level::F0, Level::F0, Level::F0, ""));
                    b.add(part(&pid("holder"), "Lamp holder", id, "The holder joins the lamp to the wires.", (0.35, 0.35, 0.38),
                               Some(straight(m.entry(), m.exit(), HOLDER_R))),
                          fidelity(Level::F1, Level::F1, Level::F1, "proportions of a school lamp holder"));
                    b.add(part(&pid("cap"), "Lamp cap", id, "The metal screw cap of the lamp; it screws into the holder.", (0.72, 0.70, 0.62),
                               Some(straight(f.at(0.0, HOLDER_R * 0.5), f.at(0.0, CAP_TOP), CAP_R))),
                          fidelity(Level::F2, Level::F1, Level::F1, lamp_sheet));
                    let support = (0.6, 0.6, 0.62);
                    b.add(part(&pid("supports"), "Filament supports", id, "Two wires that carry the current up to the filament and back.", support, None),
                          fidelity(Level::F0, Level::F0, Level::F0, ""));
                    for (k, x) in [(1, -SUP_X), (2, SUP_X)] {
                        b.add(part(&pid(&format!("support-{k}")), "Filament support", &pid("supports"), "A wire that carries current to or from the filament.", support,
                                   Some(straight(f.at(x, HOLDER_R), f.at(x, FIL_Y), 0.03))),
                              fidelity(Level::F1, Level::F1, Level::F1, lamp_sheet));
                    }
                    let filament: Vec<[f64; 3]> = coil().into_iter().map(|(x, y)| f.at(x, y)).collect();
                    b.add(part(&pid("filament"), "Filament", id, "A very thin coiled metal wire. With a current it gets hot and glows.", FILAMENT_OFF, Some(tube(filament, 0.018))),
                          fidelity(Level::F1, Level::F1, Level::F1, lamp_sheet));
                    let mut glass = part(&pid("glass"), "Glass bulb", id, "The glass bulb round the filament.", GLASS_OFF,
                                         Some(Geometry::Ellipsoid { centre, radii: bulb_radii, rotate_deg: [0.0; 3], wall: None }));
                    glass.opacity = 0.25;
                    b.add(glass, fidelity(Level::F2, Level::F1, Level::F1, lamp_sheet));
                    let mut glow = part(&pid("glow"), "Glow", id, "The light given out by the hot filament.", (1.0, 0.85, 0.35), Some(ball(centre, glow_r)));
                    glow.opacity = 0.45;
                    b.add(glow, fidelity(Level::F0, Level::F1, Level::F0, "a translucent ball stands for the light"));
                    let top = f.at(0.0, FIL_Y + BULB_HALF);
                    b.name(id.to_owned(), Some(id), Some(centre), Some((id, "Lamp", Some(top))));
                    b.name(named("holder"), Some(&pid("holder")), None, Some((&pid("holder"), "Lamp holder", None)));
                    b.name(named("cap"), Some(&pid("cap")), None, Some((&pid("cap"), "Cap", None)));
                    b.name(named("supports"), Some(&pid("supports")), None, Some((&pid("supports"), "Filament supports", None)));
                    b.name(named("filament"), Some(&pid("filament")), Some(centre), Some((&pid("filament"), "Filament", Some(centre))));
                    b.name(named("glass"), Some(&pid("glass")), None, Some((&pid("glass"), "Glass bulb", None)));
                    lamps.push(id.to_owned());
                }
            }
        }

        // Wires from each member's exit to the next member's entry, round the corners between them.
        b.add(part("wire", "Wires", root, "Copper wires in a clear coat. The metal is full of electrons that are free to move.", (0.5, 0.5, 0.5), None),
              fidelity(Level::F0, Level::F0, Level::F0, ""));
        let mut loop_path: Vec<[f64; 3]> = Vec::new();
        let mut longest: (f64, [f64; 3]) = (0.0, [0.0; 3]);
        let push = |path: &mut Vec<[f64; 3]>, points: &[[f64; 3]]| {
            for &p in points {
                if path.last().is_none_or(|q: &[f64; 3]| (p[0] - q[0]).hypot(p[1] - q[1]) > 1e-9) {
                    path.push(p);
                }
            }
        };
        for (k, m) in placed.iter().enumerate() {
            let next = &placed[(k + 1) % placed.len()];
            let mut corners = vec![m.exit()];
            let mut side = m.side;
            while side != next.side {
                let (start, ..) = SIDES[(side + 1) % 4];
                corners.push([start[0], start[1], 0.0]);
                side = (side + 1) % 4;
            }
            corners.push(next.entry());
            let route = rounded_route(&corners, CORNER, SPACING)
                .map_err(|e| invalid(format!("the wire from '{}' to '{}' cannot be routed: {e}", m.id, next.id)))?;
            let length: f64 = route.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
            if length > longest.0 {
                let (mid, _) = point_along(&route, 0.5).expect("a route has two points");
                longest = (length, [mid[0], mid[1], WIRE_HW]);
            }
            push(&mut loop_path, &route);
            push(&mut loop_path, &next.inside());
            let wid = format!("wire-{}", k + 1);
            let mut w = part(&wid, &format!("Wire from the {} to the {}", noun(m.kind), noun(next.kind)), "wire", "A copper wire in a clear coat.", COPPER,
                             Some(Geometry::Tube { path: corners, half_widths: vec![(WIRE_HW, WIRE_HW)], up: [0.0, 0.0, 1.0], steps_per_span: 8, wall: None, arc_deg: None,
                                                   corner_radius: Some(CORNER), point_spacing: Some(SPACING) }));
            w.opacity = WIRE_OPACITY;
            b.add(w, fidelity(Level::F1, Level::F1, Level::F1, "routed with rounded corners; drawn about four times thicker than real"));
            b.name(format!("wire.{}", k + 1), Some(&wid), None, Some((&wid, "Wire", None)));
        }
        b.name("wire".to_owned(), Some("wire"), Some(longest.1), Some(("wire", "Copper wire", Some(longest.1))));

        b.out.notes = vec![
            "Electrons are drawn much faster than they really move: here 2 cm/s; the real drift speed is well under 1 mm/s.".to_owned(),
            "Electrons are drawn as balls far larger than real, about one per centimetre of wire.".to_owned(),
            "Wires are drawn 5 mm thick, about four times a real 7/0.2 hook-up wire.".to_owned(),
            "Cell and lamp sizes are nominal LR6 (AA) and MES (E10) values, to be checked against IEC 60086-2 and a lamp datasheet.".to_owned(),
            "The knife switch is schematic. The electron path runs inside the cell, hidden by its case.".to_owned(),
        ];
        b.out.camera = Some(ViewCamera { yaw_deg: 40.0, pitch_deg: -20.0, zoom: 1.3 });
        Ok((Self { switches, lamps, loop_path }, b.out))
    }

    /// How long a verb takes.
    #[must_use]
    pub fn verb_seconds(&self, _verb: &Verb) -> f64 {
        LEVER_SWING_S
    }

    /// The electrons' path once round the loop, starting and ending at the cell's negative terminal.
    #[must_use]
    pub fn loop_path(&self) -> &[[f64; 3]] {
        &self.loop_path
    }

    /// Apply a verb that starts at `at`: the lever's motion. The kit records when the switch counts as closed or open.
    pub fn apply(&mut self, verb: &Verb, at: f64) -> Result<Vec<Timed>, ItemError> {
        let id = verb.object();
        let track = self.switches.get_mut(id).ok_or_else(|| {
            ItemError::Invalid(format!("'{id}' is not a switch, so it cannot be opened or closed"))
        })?;
        let close = verb.circuit() == Some(CircuitVerb::Close);
        if track.closed == close {
            return Err(ItemError::Invalid(format!("'{id}' is already {}", if close { "closed" } else { "open" })));
        }
        if at + EPS < track.busy_until {
            return Err(ItemError::Invalid(format!("'{id}' is still moving at {at} s")));
        }
        track.closed = close;
        track.busy_until = at + LEVER_SWING_S;
        // Closed from the instant the lever touches the contact; open from the instant it starts to lift.
        track.events.push(if close { (at + LEVER_SWING_S, true) } else { (at, false) });
        let p = track.pivot;
        Ok(vec![Timed {
            at,
            action: ActionDocument::Turn {
                part: format!("{id}-lever"),
                axis: (0.0, 0.0, 1.0),
                deg: if close { track.close_deg } else { -track.close_deg },
                about: Some((p[0], p[1], p[2])),
                each: false,
                for_s: LEVER_SWING_S,
                at_s: 0.0,
                ease: Ease::Smooth,
            },
        }])
    }

    /// The intervals in `0..=end` when the loop is complete: every switch closed.
    #[must_use]
    pub fn complete_intervals(&self, end: f64) -> Vec<(f64, f64)> {
        let mut times: Vec<f64> = vec![0.0, end];
        times.extend(self.switches.values().flat_map(|s| s.events.iter().map(|e| e.0)));
        times.sort_by(f64::total_cmp);
        times.dedup_by(|a, b| (*a - *b).abs() < EPS);
        let closed_at = |t: f64| {
            self.switches.values().all(|s| {
                s.events.iter().filter(|e| e.0 <= t + EPS).last().map_or(s.initially_closed, |e| e.1)
            })
        };
        let mut out: Vec<(f64, f64)> = Vec::new();
        for w in times.windows(2) {
            if w[1] - w[0] > EPS && closed_at((w[0] + w[1]) / 2.0) {
                match out.last_mut() {
                    Some(last) if (last.1 - w[0]).abs() < EPS => last.1 = w[1],
                    _ => out.push((w[0], w[1])),
                }
            }
        }
        out
    }

    /// The consequences of the rule over the whole story: while the loop is complete, the electrons flow
    /// at one speed all round it and every lamp glows; the moment it breaks, both stop.
    pub fn consequences(&self, end: f64) -> Result<Vec<Timed>, ItemError> {
        let intervals = self.complete_intervals(end);
        let mut out = Vec::new();
        let length: f64 = self.loop_path.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let count = ((length / E_SPACING).round() as usize).max(1);
        if intervals.is_empty() {
            for lamp in &self.lamps {
                out.push(Timed { at: 0.0, action: ActionDocument::Hide { part: format!("{lamp}-glow"), at_s: 0.0 } });
            }
        }
        for (k, &(a, b)) in intervals.iter().enumerate() {
            out.push(Timed {
                at: a,
                action: ActionDocument::Flow {
                    id: format!("electrons-{}", k + 1),
                    count,
                    path: self.loop_path.iter().map(|p| (p[0], p[1], p[2])).collect(),
                    corner_radius: None,
                    point_spacing: None,
                    speed: E_SPEED,
                    lateral: E_LATERAL,
                    seed: 3,
                    particle_radius: E_RADIUS,
                    colour: E_COLOUR,
                    for_s: b - a,
                    at_s: 0.0,
                    key: Some("Electron".to_owned()),
                },
            });
            let warm = WARM_UP_S.min(b - a);
            for lamp in &self.lamps {
                let colour = |part: &str, to, at| Timed {
                    at,
                    action: ActionDocument::Colour { part: format!("{lamp}-{part}"), to, for_s: warm, at_s: 0.0, ease: Ease::Linear },
                };
                out.push(colour("filament", FILAMENT_ON, a));
                out.push(colour("glass", GLASS_ON, a));
                out.push(Timed { at: a, action: ActionDocument::Show { part: format!("{lamp}-glow"), at_s: 0.0 } });
                if b < end - EPS {
                    out.push(colour("filament", FILAMENT_OFF, b));
                    out.push(colour("glass", GLASS_OFF, b));
                    out.push(Timed { at: b, action: ActionDocument::Hide { part: format!("{lamp}-glow"), at_s: 0.0 } });
                }
            }
        }
        Ok(out)
    }
}

fn noun(kind: Kind) -> &'static str {
    match kind {
        Kind::Cell => "cell",
        Kind::Switch(_) => "switch",
        Kind::Lamp => "lamp",
    }
}
