//! The chemistry kit (`docs/item-document.md` section 9): reactions built from
//! the molecule library, and a rock-salt crystal that dissolves in water. All
//! geometry comes from `chemistry_library`, the single source of bond lengths,
//! angles and radii. The rules are the kit's: a reaction must balance and only
//! rearranges atoms; water turns its oxygen end to a cation and a hydrogen end
//! to an anion, at the measured hydration distances.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::chemistry::{BondOrder, Element, Molecule};
use crate::chemistry_library::{
    self as lib, cpk_colour, ionic_radius, species, van_der_waals_radius, CL_O_HYDRATION, H_O_H_DEG, NACL_LATTICE,
    NA_O_HYDRATION, O_H,
};
use crate::item::{Fidelity, ItemError, KitBuild, LabelDefault, Level, Named, Timed, Verb};
use crate::structure::{Geometry, PartDocument};
use crate::structure_animation::{ActionDocument, Ease};
use crate::structure_lesson::ViewCamera;

/// How long the atoms take to rearrange.
pub const REACT_S: f64 = 3.0;
/// How long the reactant bonds take to fade.
pub const BREAK_S: f64 = 1.0;
/// Ball-and-stick: atoms drawn at this fraction of their van der Waals radius.
pub const BALL_SCALE: f64 = 0.25;
/// Opened-up crystal and water: ions and atoms drawn at this fraction of their size.
pub const SHRINK: f64 = 0.4;
/// How long the crystal takes to open up.
pub const OPEN_UP_S: f64 = 2.0;
/// Dissolving: outer waters arrive, the ion leaves with them, inner waters arrive.
pub const APPROACH_S: f64 = 1.5;
pub const LEAVE_S: f64 = 2.0;
pub const DISSOLVE_S: f64 = APPROACH_S + LEAVE_S + APPROACH_S;
const APPROACH: f64 = 4.0;
const LEAVE: f64 = 7.0;
const COLUMN_X: f64 = 4.0;
const ROW_GAP: f64 = 3.0;
const BOND_R: f64 = 0.07;
const BOND_GAP: f64 = 0.1;
const BOND_COLOUR: (f64, f64, f64) = (0.62, 0.62, 0.66);
const EPS: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum CrystalKind {
    #[default]
    NaCl,
}

/// The chemistry objects the item compiler hands over.
#[derive(Clone, Debug, PartialEq)]
pub enum ChemistryObject {
    Reaction { id: String, equation: String },
    Crystal { id: String, kind: CrystalKind, edge: usize },
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn tuple(a: [f64; 3]) -> (f64, f64, f64) {
    (a[0], a[1], a[2])
}
fn norm(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
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

fn ball(centre: [f64; 3], r: f64) -> Geometry {
    Geometry::Ellipsoid { centre, radii: [r, r, r], rotate_deg: [0.0; 3], wall: None }
}

fn rod(a: [f64; 3], b: [f64; 3], r: f64) -> Geometry {
    let path = vec![a, mul(add(a, b), 0.5), b];
    Geometry::Tube {
        half_widths: vec![(r, r); path.len()],
        path,
        up: { let d = sub(b, a); if d[2].abs() > 0.9 * norm(d) { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] } },
        steps_per_span: 2,
        wall: None,
        arc_deg: None,
        corner_radius: None,
        point_spacing: None,
    }
}

/// The rods of one bond: one, two or three, side by side.
fn bond_rods(a: [f64; 3], b: [f64; 3], order: BondOrder) -> Vec<Geometry> {
    let d = sub(b, a);
    let mut side = cross(d, [0.0, 0.0, 1.0]);
    if norm(side) < 1e-6 {
        side = cross(d, [1.0, 0.0, 0.0]);
    }
    let side = mul(side, 1.0 / norm(side));
    let offsets: &[f64] = match order {
        BondOrder::Single => &[0.0],
        BondOrder::Double => &[-BOND_GAP, BOND_GAP],
        BondOrder::Triple => &[-1.4 * BOND_GAP, 0.0, 1.4 * BOND_GAP],
    };
    offsets.iter().map(|&o| rod(add(a, mul(side, o)), add(b, mul(side, o)), BOND_R * if offsets.len() > 1 { 0.75 } else { 1.0 })).collect()
}

fn key_for(e: Element, ion: bool) -> (String, (f64, f64, f64), String) {
    let text = if ion {
        match e.symbol() {
            "Na" => "Sodium ion (Na⁺)".to_owned(),
            _ => "Chloride ion (Cl⁻)".to_owned(),
        }
    } else {
        format!("{} atom ({})", e.name(), e.symbol())
    };
    (format!("key-{}", e.symbol().to_lowercase()), cpk_colour(e), text)
}

fn fidelity(form: Level, behaviour: Level, appearance: Level, source: &str) -> Fidelity {
    Fidelity { form, behaviour, appearance, source: source.to_owned() }
}

/// One equation term: how many, and which species.
fn parse_side(side: &str) -> Result<Vec<(usize, &'static str)>, ItemError> {
    side.split('+')
        .map(|term| {
            let term = term.trim();
            let digits: String = term.chars().take_while(char::is_ascii_digit).collect();
            let formula = term[digits.len()..].trim();
            let count = if digits.is_empty() { 1 } else { digits.parse().unwrap_or(0) };
            let s = species(formula).ok_or_else(|| {
                let known: Vec<&str> = lib::MOLECULES.iter().map(|s| s.formula).collect();
                ItemError::Invalid(format!("'{formula}' is not in the molecule library ({})", known.join(", ")))
            })?;
            if count == 0 || count > 6 {
                return Err(ItemError::Invalid(format!("'{term}': a coefficient must be 1 to 6")));
            }
            Ok((count, s.formula))
        })
        .collect()
}

fn element_counts(side: &[(usize, &str)]) -> BTreeMap<&'static str, usize> {
    let mut out = BTreeMap::new();
    for &(n, f) in side {
        for a in lib::molecule(f).expect("parsed formulas are in the library").atoms() {
            *out.entry(a.element().symbol()).or_insert(0) += n;
        }
    }
    out
}

/// A molecule placed in the scene.
struct Placed {
    formula: &'static str,
    molecule: Molecule,
    /// Atom centres, in molecule atom order.
    centres: Vec<[f64; 3]>,
    centre: [f64; 3],
}

/// Lay copies of each term out in a column at `x`, top to bottom in equation order.
fn column(side: &[(usize, &'static str)], x: f64) -> Vec<Placed> {
    let list: Vec<&'static str> = side.iter().flat_map(|&(n, f)| std::iter::repeat_n(f, n)).collect();
    #[allow(clippy::cast_precision_loss)]
    let top = (list.len() as f64 - 1.0) / 2.0 * ROW_GAP;
    list.into_iter()
        .enumerate()
        .map(|(i, formula)| {
            let molecule = lib::molecule(formula).expect("library");
            #[allow(clippy::cast_precision_loss)]
            let n = molecule.atoms().len() as f64;
            let mut c = [0.0; 3];
            for a in molecule.atoms() {
                c = add(c, mul(a.position().to_array(), 1.0 / n));
            }
            #[allow(clippy::cast_precision_loss)]
            let centre = [x, top - ROW_GAP * i as f64, 0.0];
            let centres = molecule.atoms().iter().map(|a| add(sub(a.position().to_array(), c), centre)).collect();
            Placed { formula, molecule, centres, centre }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
struct ReactionState {
    id: String,
    /// Atom part ids and their moves on reacting.
    moves: Vec<(String, [f64; 3])>,
    reactant_bonds: Vec<String>,
    product_groups: Vec<String>,
    elements: Vec<Element>,
    reacted: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct CrystalState {
    id: String,
    sodium: String,
    chloride: String,
    lines: Vec<String>,
    /// Corner ions in dissolving order, each with its outer and inner waters.
    corners: Vec<Corner>,
    dissolved: usize,
    opened: bool,
    first_dissolve: Option<f64>,
    busy_until: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Corner {
    ion: String,
    leave: [f64; 3],
    /// Outer waters (part id, approach move) arrive first and leave with the ion.
    outer: Vec<(String, [f64; 3])>,
    inner: Vec<(String, [f64; 3])>,
}

/// The built chemistry and its state through the story.
#[derive(Clone, Debug, PartialEq)]
pub struct ChemistryKit {
    reaction: Option<ReactionState>,
    crystal: Option<CrystalState>,
}

struct Out {
    b: KitBuild,
}

impl Out {
    fn add(&mut self, p: PartDocument, f: Fidelity) {
        if p.geometry.is_some() {
            self.b.fidelity.push((p.id.clone(), f));
        }
        self.b.parts.push(p);
    }
    fn name(&mut self, name: String, part: &str, point: Option<[f64; 3]>, text: Option<&str>) {
        // Labels anchor on the part's surface: a centre can lie inside an atom, where it is hidden.
        self.name_at(name, part, point, text, None);
    }
    fn name_at(&mut self, name: String, part: &str, point: Option<[f64; 3]>, text: Option<&str>, at: Option<[f64; 3]>) {
        let label = text.map(|t| LabelDefault { part: part.to_owned(), text: t.to_owned(), at, side: None });
        self.b.names.push((name, Named { part: Some(part.to_owned()), point, label }));
    }
}

/// A label anchor a front camera can see: the front of the atom nearest the molecule's centre.
fn front_of(p: &Placed) -> [f64; 3] {
    let (k, _) = p
        .centres
        .iter()
        .enumerate()
        .map(|(k, c)| (k, norm(sub(*c, p.centre))))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a molecule has atoms");
    let r = van_der_waals_radius(p.molecule.atoms()[k].element()) * BALL_SCALE;
    add(p.centres[k], [0.0, 0.0, r])
}

/// The waters of a hydrated ion: (oxygen, hydrogen, hydrogen) centres beside an ion at `ion`, along `u`.
/// Oxygen ends face a cation; one O-H bond points at an anion.
#[must_use]
pub fn hydration_water(ion: [f64; 3], u: [f64; 3], cation: bool) -> [[f64; 3]; 3] {
    // The plane of the molecule holds u and `side`: x -> y, y -> z, z -> x.
    let side = [u[2].abs(), u[0].abs(), u[1].abs()];
    let half = (H_O_H_DEG / 2.0).to_radians();
    if cation {
        let o = add(ion, mul(u, NA_O_HYDRATION));
        let h1 = add(o, add(mul(u, O_H * half.cos()), mul(side, O_H * half.sin())));
        let h2 = add(o, add(mul(u, O_H * half.cos()), mul(side, -O_H * half.sin())));
        [o, h1, h2]
    } else {
        let o = add(ion, mul(u, CL_O_HYDRATION));
        let h1 = sub(o, mul(u, O_H));
        let a = H_O_H_DEG.to_radians();
        // The second hydrogen at the H-O-H angle from the first (which points along -u), in the plane of u and side.
        let d2 = add(mul(u, -a.cos()), mul(side, a.sin()));
        let h2 = add(o, mul(d2, O_H));
        [o, h1, h2]
    }
}

impl ChemistryKit {
    pub fn build(objects: &[ChemistryObject], root: &str) -> Result<(Self, KitBuild), ItemError> {
        let invalid = |s: String| ItemError::Invalid(s);
        if objects.len() != 1 {
            return Err(invalid("a chemistry item holds one Reaction or one Crystal".to_owned()));
        }
        let mut out = Out { b: KitBuild { units: "angstrom", ..KitBuild::default() } };
        let mut kit = Self { reaction: None, crystal: None };
        match &objects[0] {
            ChemistryObject::Reaction { id, equation } => {
                kit.reaction = Some(build_reaction(&mut out, id, equation, root)?);
                out.b.camera = Some(ViewCamera { yaw_deg: 45.0, pitch_deg: -27.0, zoom: 1.2 });
                out.b.notes = vec![
                    "Atoms are drawn as small balls (a quarter of their van der Waals radius) joined by sticks: ball and stick, not their real size.".to_owned(),
                    "This is not the reaction mechanism: real reactions go through several steps, and atoms do not move in straight lines. Only the atoms before and after are true.".to_owned(),
                    "Bond lengths and angles are textbook values from svf_core::chemistry_library (to be checked against the CRC Handbook).".to_owned(),
                ];
            }
            ChemistryObject::Crystal { id, kind: CrystalKind::NaCl, edge } => {
                kit.crystal = Some(build_crystal(&mut out, id, *edge, root)?);
                out.b.camera = Some(ViewCamera { yaw_deg: 30.0, pitch_deg: -40.0, zoom: 1.4 });
                out.b.notes = vec![
                    "Ions start at their Shannon ionic radii (Na+ 1.02, Cl- 1.81 angstrom); opened up, ions and water atoms are drawn at 0.4 of their size.".to_owned(),
                    "Lattice constant 5.64 angstrom; hydration distances Na-O 2.4 and Cl-O 3.2 angstrom (to be checked against published diffraction studies).".to_owned(),
                    "Only corner ions dissolve, six water molecules each; real dissolving is a random, crowded process in liquid water.".to_owned(),
                ];
            }
        }
        Ok((kit, out.b))
    }

    #[must_use]
    pub fn verb_seconds(&self, verb: &Verb) -> f64 {
        match verb {
            Verb::React(_) => REACT_S,
            Verb::OpenUp(_) => OPEN_UP_S,
            Verb::Dissolve { .. } => DISSOLVE_S,
            _ => 0.0,
        }
    }

    pub fn apply(&mut self, verb: &Verb, at: f64) -> Result<Vec<Timed>, ItemError> {
        let invalid = |s: String| ItemError::Invalid(s);
        let mut out = Vec::new();
        let t = |at: f64, action: ActionDocument| Timed { at, action };
        match verb {
            Verb::React(id) => {
                let r = self.reaction.as_mut().filter(|r| &r.id == id).ok_or_else(|| invalid(format!("'{id}' is not a reaction")))?;
                if r.reacted {
                    return Err(invalid(format!("'{id}' has already reacted")));
                }
                r.reacted = true;
                for bond in &r.reactant_bonds {
                    out.push(t(at, ActionDocument::Fade { part: bond.clone(), to: 0.0, for_s: BREAK_S, at_s: 0.0, ease: Ease::Linear }));
                }
                for (atom, by) in &r.moves {
                    out.push(t(at, ActionDocument::Move { part: atom.clone(), by: tuple(*by), for_s: REACT_S, at_s: 0.0, ease: Ease::Smooth }));
                }
                for g in &r.product_groups {
                    out.push(t(at + REACT_S, ActionDocument::Show { part: g.clone(), at_s: 0.0 }));
                }
            }
            Verb::OpenUp(id) => {
                let c = self.crystal.as_mut().filter(|c| &c.id == id).ok_or_else(|| invalid(format!("'{id}' is not a crystal")))?;
                if c.opened {
                    return Err(invalid(format!("'{id}' is already opened up")));
                }
                c.opened = true;
                c.busy_until = at + OPEN_UP_S;
                for g in [&c.sodium, &c.chloride] {
                    out.push(t(at, ActionDocument::Grow { part: g.clone(), to: SHRINK, each: true, for_s: OPEN_UP_S, at_s: 0.0, ease: Ease::Smooth }));
                }
                for l in &c.lines {
                    out.push(t(at + OPEN_UP_S, ActionDocument::Show { part: l.clone(), at_s: 0.0 }));
                }
            }
            Verb::Dissolve { crystal, ions } => {
                let c = self.crystal.as_mut().filter(|c| &c.id == crystal).ok_or_else(|| invalid(format!("'{crystal}' is not a crystal")))?;
                if !c.opened {
                    return Err(invalid(format!("open up '{crystal}' first: at full size the ions leave no room for water")));
                }
                if at + EPS < c.busy_until {
                    return Err(invalid(format!("'{crystal}' is still changing at {at} s")));
                }
                if *ions == 0 || c.dissolved + ions > c.corners.len() {
                    return Err(invalid(format!(
                        "'{crystal}' has {} corner ions left to dissolve; this kit dissolves corner ions only",
                        c.corners.len() - c.dissolved
                    )));
                }
                c.first_dissolve.get_or_insert(at);
                c.busy_until = at + DISSOLVE_S;
                for corner in &c.corners[c.dissolved..c.dissolved + ions] {
                    for (w, by) in &corner.outer {
                        out.push(t(at, ActionDocument::Show { part: w.clone(), at_s: 0.0 }));
                        out.push(t(at, ActionDocument::Move { part: w.clone(), by: tuple(*by), for_s: APPROACH_S, at_s: 0.0, ease: Ease::Smooth }));
                        out.push(t(at + APPROACH_S, ActionDocument::Move { part: w.clone(), by: tuple(corner.leave), for_s: LEAVE_S, at_s: 0.0, ease: Ease::Smooth }));
                    }
                    out.push(t(at + APPROACH_S, ActionDocument::Move { part: corner.ion.clone(), by: tuple(corner.leave), for_s: LEAVE_S, at_s: 0.0, ease: Ease::Smooth }));
                    let late = at + APPROACH_S + LEAVE_S;
                    for (w, by) in &corner.inner {
                        out.push(t(late, ActionDocument::Show { part: w.clone(), at_s: 0.0 }));
                        out.push(t(late, ActionDocument::Move { part: w.clone(), by: tuple(*by), for_s: APPROACH_S, at_s: 0.0, ease: Ease::Smooth }));
                    }
                }
                c.dissolved += ions;
            }
            Verb::Close(id) | Verb::Open(id) => return Err(invalid(format!("'{id}' is not a switch"))),
        }
        Ok(out)
    }

    /// Start state and keys: product bonds hidden until the reaction; unused waters and the grid hidden.
    pub fn consequences(&self, _end: f64) -> Result<Vec<Timed>, ItemError> {
        let mut out = Vec::new();
        let at0 = |action| Timed { at: 0.0, action };
        if let Some(r) = &self.reaction {
            for e in &r.elements {
                let (id, colour, text) = key_for(*e, false);
                out.push(at0(ActionDocument::Key { id, colour, text, at_s: 0.0 }));
            }
            if !r.reacted {
                for g in &r.product_groups {
                    out.push(at0(ActionDocument::Hide { part: g.clone(), at_s: 0.0 }));
                }
            }
        }
        if let Some(c) = &self.crystal {
            for e in [Element::SODIUM, Element::CHLORINE] {
                let (id, colour, text) = key_for(e, true);
                out.push(at0(ActionDocument::Key { id, colour, text, at_s: 0.0 }));
            }
            if let Some(first) = c.first_dissolve {
                for e in [Element::OXYGEN, Element::HYDROGEN] {
                    let (id, colour, text) = key_for(e, false);
                    out.push(Timed { at: first, action: ActionDocument::Key { id, colour, text, at_s: 0.0 } });
                }
            }
            if !c.opened {
                for l in &c.lines {
                    out.push(at0(ActionDocument::Hide { part: l.clone(), at_s: 0.0 }));
                }
            }
            for corner in &c.corners[c.dissolved..] {
                for (w, _) in corner.outer.iter().chain(&corner.inner) {
                    out.push(at0(ActionDocument::Hide { part: w.clone(), at_s: 0.0 }));
                }
            }
        }
        Ok(out)
    }
}

fn build_reaction(out: &mut Out, id: &str, equation: &str, root: &str) -> Result<ReactionState, ItemError> {
    let invalid = |s: String| ItemError::Invalid(s);
    let normalised = equation.replace('→', "->");
    let Some((left, right)) = normalised.split_once("->") else {
        return Err(invalid(format!("the equation '{equation}' needs an arrow, '->'")));
    };
    let (reactants, products) = (parse_side(left)?, parse_side(right)?);
    let (before, after) = (element_counts(&reactants), element_counts(&products));
    if before != after {
        return Err(invalid(format!(
            "'{equation}' does not balance: {before:?} before, {after:?} after. Atoms are neither made nor lost in a reaction"
        )));
    }
    let left = column(&reactants, -COLUMN_X);
    let right = column(&products, COLUMN_X);

    out.add(part(id, "Reaction", root, &format!("The reaction {equation}: the atoms are rearranged into new molecules."), (0.5, 0.5, 0.5), None),
            fidelity(Level::F0, Level::F0, Level::F0, ""));
    out.add(part(&format!("{id}-reactants"), "Reactants", id, "The molecules before the reaction.", (0.5, 0.5, 0.5), None),
            fidelity(Level::F0, Level::F0, Level::F0, ""));
    out.add(part(&format!("{id}-products"), "Products", id, "The new bonds after the reaction.", (0.5, 0.5, 0.5), None),
            fidelity(Level::F0, Level::F0, Level::F0, ""));
    out.name(id.to_owned(), id, None, None);
    out.name(format!("{id}.reactants"), &format!("{id}-reactants"), None, None);
    out.name(format!("{id}.products"), &format!("{id}-products"), None, None);
    let source = "textbook bond lengths and angles (chemistry_library)";
    let mut atom_ids: Vec<(Element, String, [f64; 3])> = Vec::new();
    let mut reactant_bonds = Vec::new();
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for p in &left {
        let s = species(p.formula).expect("library");
        let n = seen.entry(p.formula).or_insert(0);
        *n += 1;
        let mid = format!("{id}-{}-{n}", p.formula.to_lowercase());
        out.add(part(&mid, s.name, &format!("{id}-reactants"), &format!("A {} ({}).", s.name.to_lowercase(), s.display), (0.5, 0.5, 0.5), None),
                fidelity(Level::F0, Level::F0, Level::F0, ""));
        out.name_at(format!("{id}.{}-{n}", p.formula.to_lowercase()), &mid, Some(p.centre), Some(&format!("{} ({})", s.name, s.display)), Some(front_of(p)));
        for (k, a) in p.molecule.atoms().iter().enumerate() {
            let e = a.element();
            let aid = format!("{mid}-{}{}", e.symbol().to_lowercase(), k + 1);
            out.add(part(&aid, &format!("{} atom", e.name()), &mid, &format!("A {} atom. Atoms are drawn as balls; real atoms have no sharp edge.", e.name().to_lowercase()),
                         cpk_colour(e), Some(ball(p.centres[k], van_der_waals_radius(e) * BALL_SCALE))),
                    fidelity(Level::F2, Level::F1, Level::F1, source));
            atom_ids.push((e, aid, p.centres[k]));
        }
        let bonds = format!("{mid}-bonds");
        out.add(part(&bonds, "Bonds", &mid, "The bonds holding the molecule together.", BOND_COLOUR, None), fidelity(Level::F0, Level::F0, Level::F0, ""));
        add_bonds(out, &bonds, p, source);
        reactant_bonds.push(bonds);
    }
    let mut product_groups = Vec::new();
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut targets: Vec<(Element, [f64; 3])> = Vec::new();
    for p in &right {
        let s = species(p.formula).expect("library");
        let n = seen.entry(p.formula).or_insert(0);
        *n += 1;
        let gid = format!("{id}-new-{}-{n}", p.formula.to_lowercase());
        out.add(part(&gid, &format!("New {}", s.name.to_lowercase()), &format!("{id}-products"), &format!("The bonds of a new {} ({}).", s.name.to_lowercase(), s.display), BOND_COLOUR, None),
                fidelity(Level::F0, Level::F0, Level::F0, ""));
        out.name_at(format!("{id}.{}-{n}", p.formula.to_lowercase()), &gid, Some(p.centre), Some(&format!("{} ({})", s.name, s.display)), Some(front_of(p)));
        add_bonds(out, &gid, p, source);
        product_groups.push(gid);
        for (k, a) in p.molecule.atoms().iter().enumerate() {
            targets.push((a.element(), p.centres[k]));
        }
    }
    // Pair atoms of each element top to bottom, so no two paths cross needlessly.
    let mut moves = Vec::new();
    let mut elements: Vec<Element> = Vec::new();
    for (e, ..) in &atom_ids {
        if !elements.contains(e) {
            elements.push(*e);
        }
    }
    let order = |a: &[f64; 3], b: &[f64; 3]| b[1].total_cmp(&a[1]).then(a[0].total_cmp(&b[0]));
    for e in &elements {
        let mut from: Vec<&(Element, String, [f64; 3])> = atom_ids.iter().filter(|a| a.0 == *e).collect();
        let mut to: Vec<[f64; 3]> = targets.iter().filter(|t| t.0 == *e).map(|t| t.1).collect();
        from.sort_by(|a, b| order(&a.2, &b.2));
        to.sort_by(order);
        for (a, b) in from.iter().zip(&to) {
            moves.push((a.1.clone(), sub(*b, a.2)));
        }
    }
    Ok(ReactionState { id: id.to_owned(), moves, reactant_bonds, product_groups, elements, reacted: false })
}

fn add_bonds(out: &mut Out, parent: &str, p: &Placed, source: &str) {
    for (k, b) in p.molecule.bonds().iter().enumerate() {
        let i = |id: crate::chemistry::AtomId| p.molecule.atoms().iter().position(|a| a.id() == id).expect("bond atom");
        let (a, c) = (p.centres[i(b.atom_a())], p.centres[i(b.atom_b())]);
        for (r, rod) in bond_rods(a, c, b.order()).into_iter().enumerate() {
            out.add(part(&format!("{parent}-{}-{}", k + 1, r + 1), "Bond", parent, "A covalent bond: a shared pair of electrons.", BOND_COLOUR, Some(rod)),
                    fidelity(Level::F2, Level::F1, Level::F0, source));
        }
    }
}

fn build_crystal(out: &mut Out, id: &str, edge: usize, root: &str) -> Result<CrystalState, ItemError> {
    if !(2..=6).contains(&edge) {
        return Err(ItemError::Invalid(format!("a crystal is 2 to 6 ions along each edge, not {edge}")));
    }
    let d = NACL_LATTICE / 2.0;
    #[allow(clippy::cast_precision_loss)]
    let pos = |i: usize, j: usize, k: usize| {
        let h = (edge as f64 - 1.0) / 2.0;
        [(i as f64 - h) * d, (j as f64 - h) * d, (k as f64 - h) * d]
    };
    let is_na = |i: usize, j: usize, k: usize| (i + j + k) % 2 == 0;
    let (sodium, chloride, lines) = (format!("{id}-sodium"), format!("{id}-chloride"), format!("{id}-lines"));
    let source = "Shannon ionic radii; NaCl lattice constant 5.64 angstrom";
    out.add(part(id, "Sodium chloride crystal", root, "Sodium ions and chloride ions in a cubic lattice, alternating in every direction.", (0.5, 0.5, 0.5), None),
            fidelity(Level::F0, Level::F0, Level::F0, ""));
    out.add(part(&sodium, "Sodium ions", id, "Sodium ions, Na+. A sodium atom has lost one electron, so the ion has one positive charge.", (0.5, 0.5, 0.5), None),
            fidelity(Level::F0, Level::F0, Level::F0, ""));
    out.add(part(&chloride, "Chloride ions", id, "Chloride ions, Cl-. A chlorine atom has gained one electron, so the ion has one negative charge.", (0.5, 0.5, 0.5), None),
            fidelity(Level::F0, Level::F0, Level::F0, ""));
    // For each kind, the ion nearest the middle: it has all six neighbours.
    let mut first: [Option<(f64, String)>; 2] = [None, None];
    for i in 0..edge {
        for j in 0..edge {
            for k in 0..edge {
                let na = is_na(i, j, k);
                let e = if na { Element::SODIUM } else { Element::CHLORINE };
                let ion = format!("{id}-{}-{i}{j}{k}", if na { "na" } else { "cl" });
                let (name, desc) = if na {
                    ("Sodium ion", "A sodium ion, Na+. Ionic radius 1.02 angstrom.")
                } else {
                    ("Chloride ion", "A chloride ion, Cl-. Ionic radius 1.81 angstrom.")
                };
                out.add(part(&ion, name, if na { &sodium } else { &chloride }, desc, cpk_colour(e), Some(ball(pos(i, j, k), ionic_radius(e).expect("an ion")))),
                        fidelity(Level::F2, Level::F1, Level::F1, source));
                let r = norm(pos(i, j, k));
                let slot = &mut first[usize::from(!na)];
                if slot.as_ref().is_none_or(|(best, _)| r < *best - EPS) {
                    *slot = Some((r, ion));
                }
            }
        }
    }
    out.add(part(&lines, "Lattice lines", id, "Lines that show the cubic grid the ions sit on. They are a guide for the eye, not bonds.", (0.5, 0.5, 0.5), None),
            fidelity(Level::F0, Level::F0, Level::F0, ""));
    let mut line_ids = Vec::new();
    #[allow(clippy::cast_precision_loss)]
    let h = (edge as f64 - 1.0) / 2.0 * d;
    for axis in 0..3 {
        for a in 0..edge {
            for b in 0..edge {
                // A line along `axis` through the row of ions at indices (a, b) on the other two axes.
                let row = pos(a, b, 0);
                let (mut from, mut to) = ([0.0; 3], [0.0; 3]);
                let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                from[u] = row[0];
                from[v] = row[1];
                to[u] = row[0];
                to[v] = row[1];
                from[axis] = -h;
                to[axis] = h;
                let lid = format!("{lines}-{axis}-{a}{b}");
                out.add(part(&lid, "Lattice line", &lines, "A guide line along a row of ions.", (0.62, 0.62, 0.68), Some(rod(from, to, 0.04))),
                        fidelity(Level::F2, Level::F1, Level::F0, source));
                line_ids.push(lid);
            }
        }
    }
    let water_group = format!("{id}-water");
    out.add(part(&water_group, "Water molecules", id, "Water molecules, H2O: a slightly negative oxygen end and a slightly positive hydrogen end.", (0.5, 0.5, 0.5), None),
            fidelity(Level::F0, Level::F0, Level::F0, ""));

    // Corners in dissolving order: the top four first (alternating sodium, chloride), then the bottom four.
    let top = edge - 1;
    let order = [(top, top, 0), (top, top, top), (0, top, top), (0, top, 0), (top, 0, 0), (top, 0, top), (0, 0, top), (0, 0, 0)];
    let mut corners = Vec::new();
    let mut water_n = 0;
    let wsource = "textbook water geometry; hydration distances from diffraction studies";
    for (n, &(i, j, k)) in order.iter().enumerate() {
        let na = is_na(i, j, k);
        let p = pos(i, j, k);
        let sign = |c: f64| if c > 0.0 { 1.0 } else { -1.0 };
        let s = [sign(p[0]), sign(p[1]), sign(p[2])];
        let leave = mul(s, LEAVE / 3f64.sqrt());
        let ion = format!("{id}-{}-{i}{j}{k}", if na { "na" } else { "cl" });
        let mut outer = Vec::new();
        let mut inner = Vec::new();
        for axis in 0..3 {
            for dir in [1.0, -1.0] {
                let mut u = [0.0; 3];
                u[axis] = dir * s[axis];
                let outward = dir > 0.0;
                // Outer waters sit at the lattice corner; inner ones at the ion's place after it has left.
                let at_ion = if outward { p } else { add(p, leave) };
                let atoms = hydration_water(at_ion, u, na);
                // Each arrives along its own axis from outside (outer) or along the leaving diagonal (inner).
                let approach = if outward { mul(u, APPROACH) } else { mul(s, APPROACH / 3f64.sqrt() * 1.5) };
                water_n += 1;
                let wid = format!("{id}-w{water_n}");
                let start: Vec<[f64; 3]> = atoms.iter().map(|a| add(*a, approach)).collect();
                add_water(out, &water_group, &wid, &start, wsource);
                let by = mul(approach, -1.0);
                if outward { outer.push((wid, by)) } else { inner.push((wid, by)) }
            }
        }
        let text = if na { "Sodium ion (Na⁺)" } else { "Chloride ion (Cl⁻)" };
        out.name(format!("{id}.corner-{}", n + 1), &ion, None, Some(text));
        corners.push(Corner { ion, leave, outer, inner });
    }
    out.name(id.to_owned(), id, None, Some("Sodium chloride crystal"));
    out.name(format!("{id}.sodium"), &sodium, None, Some("Sodium ion (Na⁺)"));
    out.name(format!("{id}.chloride"), &chloride, None, Some("Chloride ion (Cl⁻)"));
    out.name(format!("{id}.lines"), &lines, None, Some("Lattice lines"));
    out.name(format!("{id}.water"), &water_group, None, Some("Water molecule"));
    if let [Some((_, na)), Some((_, cl))] = &first {
        out.name(format!("{id}.a-sodium-ion"), na, None, Some("Sodium ion (Na⁺)"));
        out.name(format!("{id}.a-chloride-ion"), cl, None, Some("Chloride ion (Cl⁻)"));
    }
    Ok(CrystalState { id: id.to_owned(), sodium, chloride, lines: line_ids, corners, dissolved: 0, opened: false, first_dissolve: None, busy_until: 0.0 })
}

fn add_water(out: &mut Out, parent: &str, wid: &str, atoms: &[[f64; 3]], source: &str) {
    out.add(part(wid, "Water molecule", parent, "A water molecule, H2O: one oxygen atom bonded to two hydrogen atoms, bent at 104.5 degrees.", (0.5, 0.5, 0.5), None),
            fidelity(Level::F0, Level::F0, Level::F0, ""));
    let (o, h) = (Element::OXYGEN, Element::HYDROGEN);
    out.add(part(&format!("{wid}-o"), "Oxygen atom", wid, "The oxygen atom of a water molecule: its slightly negative end.", cpk_colour(o), Some(ball(atoms[0], van_der_waals_radius(o) * SHRINK))),
            fidelity(Level::F2, Level::F1, Level::F1, source));
    for k in 1..3 {
        out.add(part(&format!("{wid}-h{k}"), "Hydrogen atom", wid, "A hydrogen atom of a water molecule: its slightly positive end.", cpk_colour(h), Some(ball(atoms[k], van_der_waals_radius(h) * SHRINK))),
                fidelity(Level::F2, Level::F1, Level::F1, source));
    }
}
