//! Reference chemistry data: the single source of the bond lengths, bond
//! angles, radii and colours that kits use (`docs/item-document.md`, C4).
//!
//! Values are the usual textbook (CRC Handbook) values, in ångströms and
//! degrees. They are stated here once; kits and tests read them from here.
//! The user still owes a citation check of these values.

use crate::chemistry::{Atom, AtomId, Bond, BondId, BondOrder, Element, Molecule, MolecularPoint3};

/// Bond lengths (Å).
pub const H_H: f64 = 0.74;
pub const O_O_DOUBLE: f64 = 1.21;
pub const N_N_TRIPLE: f64 = 1.10;
pub const O_H: f64 = 0.96;
pub const C_O_DOUBLE: f64 = 1.16;
pub const C_H: f64 = 1.09;
pub const N_H: f64 = 1.01;
/// Bond angles (degrees).
pub const H_O_H_DEG: f64 = 104.5;
pub const H_N_H_DEG: f64 = 107.8;
/// The tetrahedral angle of methane.
pub const H_C_H_DEG: f64 = 109.47;

/// Rock salt (NaCl) lattice constant (Å).
pub const NACL_LATTICE: f64 = 5.64;
/// Ion to water-oxygen distance in solution (Å), from diffraction studies.
pub const NA_O_HYDRATION: f64 = 2.4;
pub const CL_O_HYDRATION: f64 = 3.2;

/// A chemical species in the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Species {
    pub formula: &'static str,
    /// The formula with subscripts, for labels.
    pub display: &'static str,
    pub name: &'static str,
}

/// Every molecule the library can build.
pub const MOLECULES: [Species; 7] = [
    Species { formula: "H2", display: "H₂", name: "Hydrogen molecule" },
    Species { formula: "O2", display: "O₂", name: "Oxygen molecule" },
    Species { formula: "N2", display: "N₂", name: "Nitrogen molecule" },
    Species { formula: "H2O", display: "H₂O", name: "Water molecule" },
    Species { formula: "CO2", display: "CO₂", name: "Carbon dioxide molecule" },
    Species { formula: "CH4", display: "CH₄", name: "Methane molecule" },
    Species { formula: "NH3", display: "NH₃", name: "Ammonia molecule" },
];

/// The library entry for `formula`.
#[must_use]
pub fn species(formula: &str) -> Option<Species> {
    MOLECULES.iter().copied().find(|s| s.formula == formula)
}

/// CPK colour (convention rule 3).
#[must_use]
pub fn cpk_colour(e: Element) -> (f64, f64, f64) {
    match e.symbol() {
        "H" => (0.95, 0.95, 0.95),
        "C" => (0.25, 0.25, 0.27),
        "N" => (0.20, 0.35, 0.90),
        "O" => (0.85, 0.15, 0.15),
        "Na" => (0.60, 0.45, 0.90),
        "Cl" => (0.30, 0.80, 0.35),
        _ => (0.8, 0.5, 0.8),
    }
}

/// Bondi van der Waals radius (Å).
#[must_use]
pub fn van_der_waals_radius(e: Element) -> f64 {
    match e.symbol() {
        "H" => 1.20,
        "C" => 1.70,
        "N" => 1.55,
        "O" => 1.52,
        "Na" => 2.27,
        "Cl" => 1.75,
        _ => 1.8,
    }
}

/// Shannon ionic radius for six-fold coordination (Å): Na⁺ and Cl⁻.
#[must_use]
pub fn ionic_radius(e: Element) -> Option<f64> {
    match e.symbol() {
        "Na" => Some(1.02),
        "Cl" => Some(1.81),
        _ => None,
    }
}

fn point(p: [f64; 3]) -> MolecularPoint3 {
    MolecularPoint3::new(p[0], p[1], p[2]).expect("library coordinates are finite")
}

/// Build `formula` with its central atom at the origin, flat molecules in the XY plane.
#[must_use]
pub fn molecule(formula: &str) -> Option<Molecule> {
    use BondOrder::{Double, Single, Triple};
    let (h, c, n, o) = (Element::HYDROGEN, Element::CARBON, Element::NITROGEN, Element::OXYGEN);
    let half = |d: f64| d / 2.0;
    let (atoms, bonds): (Vec<(Element, [f64; 3])>, Vec<(u32, u32, BondOrder)>) = match formula {
        "H2" => (vec![(h, [-half(H_H), 0.0, 0.0]), (h, [half(H_H), 0.0, 0.0])], vec![(0, 1, Single)]),
        "O2" => (vec![(o, [-half(O_O_DOUBLE), 0.0, 0.0]), (o, [half(O_O_DOUBLE), 0.0, 0.0])], vec![(0, 1, Double)]),
        "N2" => (vec![(n, [-half(N_N_TRIPLE), 0.0, 0.0]), (n, [half(N_N_TRIPLE), 0.0, 0.0])], vec![(0, 1, Triple)]),
        "H2O" => {
            let a = (H_O_H_DEG / 2.0).to_radians();
            (
                vec![(o, [0.0; 3]), (h, [-O_H * a.sin(), -O_H * a.cos(), 0.0]), (h, [O_H * a.sin(), -O_H * a.cos(), 0.0])],
                vec![(0, 1, Single), (0, 2, Single)],
            )
        }
        "CO2" => (
            vec![(c, [0.0; 3]), (o, [-C_O_DOUBLE, 0.0, 0.0]), (o, [C_O_DOUBLE, 0.0, 0.0])],
            vec![(0, 1, Double), (0, 2, Double)],
        ),
        "CH4" => {
            let k = C_H / 3f64.sqrt();
            (
                vec![(c, [0.0; 3]), (h, [k, k, k]), (h, [-k, -k, k]), (h, [-k, k, -k]), (h, [k, -k, -k])],
                vec![(0, 1, Single), (0, 2, Single), (0, 3, Single), (0, 4, Single)],
            )
        }
        "NH3" => {
            // Three N-H bonds about -Y, each at angle t from it, 120 degrees apart round it.
            let cos_t = ((H_N_H_DEG.to_radians().cos() + 0.5) / 1.5).sqrt();
            let sin_t = (1.0 - cos_t * cos_t).sqrt();
            let mut atoms = vec![(n, [0.0; 3])];
            for k in 0..3 {
                let phi = f64::from(k) * 120f64.to_radians();
                atoms.push((h, [N_H * sin_t * phi.cos(), -N_H * cos_t, N_H * sin_t * phi.sin()]));
            }
            (atoms, vec![(0, 1, Single), (0, 2, Single), (0, 3, Single)])
        }
        _ => return None,
    };
    let atoms = atoms.into_iter().enumerate().map(|(i, (e, p))| Atom::new(AtomId::new(i as u32), e, point(p))).collect();
    let bonds = bonds
        .into_iter()
        .enumerate()
        .map(|(i, (a, b, order))| Bond::new(BondId::new(i as u32), AtomId::new(a), AtomId::new(b), order))
        .collect();
    Some(Molecule::new(atoms, bonds).expect("library molecules are valid"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    fn angle(m: &Molecule, a: usize, centre: usize, b: usize) -> f64 {
        let p = |i: usize| m.atoms()[i].position().to_array();
        let (u, v) = (
            [p(a)[0] - p(centre)[0], p(a)[1] - p(centre)[1], p(a)[2] - p(centre)[2]],
            [p(b)[0] - p(centre)[0], p(b)[1] - p(centre)[1], p(b)[2] - p(centre)[2]],
        );
        let dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
        (dot / (dist(p(a), p(centre)) * dist(p(b), p(centre)))).acos().to_degrees()
    }

    #[test]
    fn every_library_molecule_has_its_stated_bond_lengths_and_angles() {
        for s in MOLECULES {
            let m = molecule(s.formula).unwrap();
            for b in m.bonds() {
                let (a, c) = (m.atom(b.atom_a()).unwrap(), m.atom(b.atom_b()).unwrap());
                let pair = [a.element().symbol(), c.element().symbol()];
                let want = match (pair, b.order()) {
                    (["H", "H"], _) => H_H,
                    (["O", "O"], _) => O_O_DOUBLE,
                    (["N", "N"], _) => N_N_TRIPLE,
                    (["O", "H"], _) => O_H,
                    (["C", "O"], _) => C_O_DOUBLE,
                    (["C", "H"], _) => C_H,
                    (["N", "H"], _) => N_H,
                    other => panic!("unexpected bond {other:?}"),
                };
                let d = dist(a.position().to_array(), c.position().to_array());
                assert!((d - want).abs() < 1e-9, "{}: {pair:?} is {d}", s.formula);
            }
        }
        assert!((angle(&molecule("H2O").unwrap(), 1, 0, 2) - H_O_H_DEG).abs() < 1e-9);
        assert!((angle(&molecule("NH3").unwrap(), 1, 0, 2) - H_N_H_DEG).abs() < 1e-9);
        assert!((angle(&molecule("CH4").unwrap(), 1, 0, 2) - H_C_H_DEG).abs() < 0.01);
        assert!((angle(&molecule("CO2").unwrap(), 1, 0, 2) - 180.0).abs() < 1e-9);
        assert!(molecule("XY").is_none());
    }
}
