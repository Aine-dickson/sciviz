//! Graphics-independent chemical structure and composition primitives.
//!
//! This module models element definitions, atom instances, bond relationships,
//! and molecule composition. It deliberately does not model chemical reactions,
//! molecular dynamics, or rendered geometry. Molecular coordinates are in
//! ångströms (Å), not SVF world-coordinate units.

use std::{
    collections::HashSet,
    error::Error,
    fmt,
};

/// Errors produced when constructing chemical structure values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChemistryError {
    /// Atomic numbers must be positive.
    InvalidAtomicNumber,
    /// Element symbols must not be empty.
    EmptyElementSymbol,
    /// Element symbols must contain one or two ASCII letters.
    InvalidElementSymbol,
    /// Element names must not be empty.
    EmptyElementName,
    /// A molecular coordinate contains a non-finite value.
    NonFiniteCoordinate { axis: MolecularAxis },
    /// An atom identifier occurs more than once in a molecule.
    DuplicateAtomId { id: AtomId },
    /// A bond identifier occurs more than once in a molecule.
    DuplicateBondId { id: BondId },
    /// A bond endpoint does not identify an atom in the molecule.
    UnknownBondAtom { bond_id: BondId, atom_id: AtomId },
    /// A bond cannot connect an atom to itself.
    SelfBond { bond_id: BondId, atom_id: AtomId },
    /// An undirected pair of atoms can have only one bond in this model.
    DuplicateBondPair { first: AtomId, second: AtomId },
}

impl fmt::Display for ChemistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAtomicNumber => write!(f, "atomic number must be positive"),
            Self::EmptyElementSymbol => write!(f, "element symbol must not be empty"),
            Self::InvalidElementSymbol => {
                write!(f, "element symbol must contain one or two ASCII letters")
            }
            Self::EmptyElementName => write!(f, "element name must not be empty"),
            Self::NonFiniteCoordinate { axis } => {
                write!(f, "molecular {axis:?} coordinate must be finite")
            }
            Self::DuplicateAtomId { id } => write!(f, "duplicate atom ID {}", id.get()),
            Self::DuplicateBondId { id } => write!(f, "duplicate bond ID {}", id.get()),
            Self::UnknownBondAtom { bond_id, atom_id } => write!(
                f,
                "bond {} references unknown atom {}",
                bond_id.get(),
                atom_id.get()
            ),
            Self::SelfBond { bond_id, atom_id } => write!(
                f,
                "bond {} connects atom {} to itself",
                bond_id.get(),
                atom_id.get()
            ),
            Self::DuplicateBondPair { first, second } => write!(
                f,
                "atoms {} and {} already have a bond",
                first.get(),
                second.get()
            ),
        }
    }
}

impl Error for ChemistryError {}

/// A Cartesian axis in molecular coordinate space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MolecularAxis {
    X,
    Y,
    Z,
}

/// An immutable definition of a chemical element.
///
/// Definitions are values shared by atom instances. The predefined constants
/// cover the initial chemistry experiment; `new` permits additional valid
/// element definitions without changing this type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Element {
    atomic_number: u8,
    symbol: &'static str,
    name: &'static str,
}

impl Element {
    /// Hydrogen (atomic number 1).
    pub const HYDROGEN: Self = Self {
        atomic_number: 1,
        symbol: "H",
        name: "Hydrogen",
    };

    /// Carbon (atomic number 6).
    pub const CARBON: Self = Self {
        atomic_number: 6,
        symbol: "C",
        name: "Carbon",
    };

    /// Oxygen (atomic number 8).
    pub const OXYGEN: Self = Self {
        atomic_number: 8,
        symbol: "O",
        name: "Oxygen",
    };

    /// Creates an immutable element definition.
    ///
    /// The symbol must be one or two ASCII letters. This validates the shape
    /// of a definition, not its membership in an authoritative periodic table.
    pub fn new(
        atomic_number: u8,
        symbol: &'static str,
        name: &'static str,
    ) -> Result<Self, ChemistryError> {
        if atomic_number == 0 {
            return Err(ChemistryError::InvalidAtomicNumber);
        }
        if symbol.is_empty() {
            return Err(ChemistryError::EmptyElementSymbol);
        }
        if !(1..=2).contains(&symbol.len()) || !symbol.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(ChemistryError::InvalidElementSymbol);
        }
        if name.trim().is_empty() {
            return Err(ChemistryError::EmptyElementName);
        }

        Ok(Self {
            atomic_number,
            symbol,
            name,
        })
    }

    #[must_use]
    pub const fn atomic_number(self) -> u8 {
        self.atomic_number
    }

    #[must_use]
    pub const fn symbol(self) -> &'static str {
        self.symbol
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }
}

/// Stable identity for one atom occurrence within a molecule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtomId(u32);

impl AtomId {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Stable identity for one bond relationship within a molecule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BondId(u32);

impl BondId {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A point in molecular coordinate space, expressed in ångströms (Å).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MolecularPoint3 {
    x: f64,
    y: f64,
    z: f64,
}

impl MolecularPoint3 {
    /// Creates a molecular point after rejecting non-finite coordinates.
    pub fn new(x: f64, y: f64, z: f64) -> Result<Self, ChemistryError> {
        for (axis, value) in [
            (MolecularAxis::X, x),
            (MolecularAxis::Y, y),
            (MolecularAxis::Z, z),
        ] {
            if !value.is_finite() {
                return Err(ChemistryError::NonFiniteCoordinate { axis });
            }
        }
        Ok(Self { x, y, z })
    }

    #[must_use]
    pub const fn x(self) -> f64 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> f64 {
        self.y
    }

    #[must_use]
    pub const fn z(self) -> f64 {
        self.z
    }

    #[must_use]
    pub const fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

/// One occurrence of an element at a molecular position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Atom {
    id: AtomId,
    element: Element,
    position: MolecularPoint3,
}

impl Atom {
    #[must_use]
    pub const fn new(id: AtomId, element: Element, position: MolecularPoint3) -> Self {
        Self {
            id,
            element,
            position,
        }
    }

    #[must_use]
    pub const fn id(self) -> AtomId {
        self.id
    }

    #[must_use]
    pub const fn element(self) -> Element {
        self.element
    }

    #[must_use]
    pub const fn position(self) -> MolecularPoint3 {
        self.position
    }
}

/// The supported semantic bond orders in the initial chemistry model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BondOrder {
    Single,
    Double,
    Triple,
}

/// A semantic relationship between two atom instances.
///
/// A bond is not a geometric line or cylinder; any visual representation is
/// derived from this relationship.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bond {
    id: BondId,
    atom_a: AtomId,
    atom_b: AtomId,
    order: BondOrder,
}

impl Bond {
    #[must_use]
    pub const fn new(id: BondId, atom_a: AtomId, atom_b: AtomId, order: BondOrder) -> Self {
        Self {
            id,
            atom_a,
            atom_b,
            order,
        }
    }

    #[must_use]
    pub const fn id(self) -> BondId {
        self.id
    }

    #[must_use]
    pub const fn atom_a(self) -> AtomId {
        self.atom_a
    }

    #[must_use]
    pub const fn atom_b(self) -> AtomId {
        self.atom_b
    }

    #[must_use]
    pub const fn order(self) -> BondOrder {
        self.order
    }

    /// Returns the endpoint IDs in their stored order.
    #[must_use]
    pub const fn endpoints(self) -> [AtomId; 2] {
        [self.atom_a, self.atom_b]
    }
}

/// A validated molecular assembly of atom instances and bond relationships.
///
/// Atom and bond input order is preserved. This initial structural model does
/// not validate valence, charge, stereochemistry, or chemical stability.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Molecule {
    atoms: Vec<Atom>,
    bonds: Vec<Bond>,
}

impl Molecule {
    /// Constructs a molecule atomically, validating identities and bonds.
    ///
    /// Empty molecules and isolated atoms are permitted. Every bond endpoint
    /// must resolve within this molecule; self-bonds and duplicate undirected
    /// endpoint pairs are rejected.
    pub fn new(atoms: Vec<Atom>, bonds: Vec<Bond>) -> Result<Self, ChemistryError> {
        let mut atom_ids = HashSet::with_capacity(atoms.len());
        for atom in &atoms {
            if !atom_ids.insert(atom.id) {
                return Err(ChemistryError::DuplicateAtomId { id: atom.id });
            }
        }

        let mut bond_ids = HashSet::with_capacity(bonds.len());
        let mut connected_pairs = HashSet::with_capacity(bonds.len());
        for bond in &bonds {
            if !bond_ids.insert(bond.id) {
                return Err(ChemistryError::DuplicateBondId { id: bond.id });
            }

            if !atom_ids.contains(&bond.atom_a) {
                return Err(ChemistryError::UnknownBondAtom {
                    bond_id: bond.id,
                    atom_id: bond.atom_a,
                });
            }
            if !atom_ids.contains(&bond.atom_b) {
                return Err(ChemistryError::UnknownBondAtom {
                    bond_id: bond.id,
                    atom_id: bond.atom_b,
                });
            }
            if bond.atom_a == bond.atom_b {
                return Err(ChemistryError::SelfBond {
                    bond_id: bond.id,
                    atom_id: bond.atom_a,
                });
            }

            let pair = if bond.atom_a < bond.atom_b {
                (bond.atom_a, bond.atom_b)
            } else {
                (bond.atom_b, bond.atom_a)
            };
            if !connected_pairs.insert(pair) {
                return Err(ChemistryError::DuplicateBondPair {
                    first: pair.0,
                    second: pair.1,
                });
            }
        }

        Ok(Self { atoms, bonds })
    }

    #[must_use]
    pub fn atoms(&self) -> &[Atom] {
        &self.atoms
    }

    #[must_use]
    pub fn bonds(&self) -> &[Bond] {
        &self.bonds
    }

    #[must_use]
    pub fn atom(&self, id: AtomId) -> Option<&Atom> {
        self.atoms.iter().find(|atom| atom.id == id)
    }

    #[must_use]
    pub fn bond(&self, id: BondId) -> Option<&Bond> {
        self.bonds.iter().find(|bond| bond.id == id)
    }

    /// Iterates over bonds incident to the specified atom, in molecule order.
    pub fn bonds_for_atom(&self, id: AtomId) -> impl Iterator<Item = &Bond> {
        self.bonds
            .iter()
            .filter(move |bond| bond.atom_a == id || bond.atom_b == id)
    }

    #[must_use]
    pub fn atom_count(&self) -> usize {
        self.atoms.len()
    }

    #[must_use]
    pub fn bond_count(&self) -> usize {
        self.bonds.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.atoms.is_empty() && self.bonds.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64, z: f64) -> MolecularPoint3 {
        MolecularPoint3::new(x, y, z).unwrap()
    }

    fn atom(id: u32, element: Element, position: MolecularPoint3) -> Atom {
        Atom::new(AtomId::new(id), element, position)
    }

    fn bond(id: u32, a: u32, b: u32, order: BondOrder) -> Bond {
        Bond::new(BondId::new(id), AtomId::new(a), AtomId::new(b), order)
    }

    fn water() -> Molecule {
        Molecule::new(
            vec![
                atom(10, Element::OXYGEN, point(0.0, 0.0, 0.0)),
                atom(11, Element::HYDROGEN, point(0.9572, 0.0, 0.0)),
                atom(12, Element::HYDROGEN, point(-0.239, 0.927, 0.0)),
            ],
            vec![
                bond(20, 10, 11, BondOrder::Single),
                bond(21, 10, 12, BondOrder::Single),
            ],
        )
        .unwrap()
    }

    fn methane() -> Molecule {
        Molecule::new(
            vec![
                atom(1, Element::CARBON, point(0.0, 0.0, 0.0)),
                atom(2, Element::HYDROGEN, point(0.629, 0.629, 0.629)),
                atom(3, Element::HYDROGEN, point(-0.629, -0.629, 0.629)),
                atom(4, Element::HYDROGEN, point(-0.629, 0.629, -0.629)),
                atom(5, Element::HYDROGEN, point(0.629, -0.629, -0.629)),
            ],
            vec![
                bond(10, 1, 2, BondOrder::Single),
                bond(11, 1, 3, BondOrder::Single),
                bond(12, 1, 4, BondOrder::Single),
                bond(13, 1, 5, BondOrder::Single),
            ],
        )
        .unwrap()
    }

    #[test]
    fn predefined_elements_have_expected_scientific_identity() {
        assert_eq!(
            (Element::HYDROGEN.atomic_number(), Element::HYDROGEN.symbol(), Element::HYDROGEN.name()),
            (1, "H", "Hydrogen")
        );
        assert_eq!(
            (Element::CARBON.atomic_number(), Element::CARBON.symbol(), Element::CARBON.name()),
            (6, "C", "Carbon")
        );
        assert_eq!(
            (Element::OXYGEN.atomic_number(), Element::OXYGEN.symbol(), Element::OXYGEN.name()),
            (8, "O", "Oxygen")
        );
    }

    #[test]
    fn additional_element_definitions_are_validated_values() {
        let nitrogen = Element::new(7, "N", "Nitrogen").unwrap();
        assert_eq!(nitrogen.atomic_number(), 7);
        assert_eq!(nitrogen.symbol(), "N");
        assert_eq!(nitrogen.name(), "Nitrogen");
        assert_eq!(Element::new(0, "X", "Example"), Err(ChemistryError::InvalidAtomicNumber));
        assert_eq!(Element::new(1, "", "Hydrogen"), Err(ChemistryError::EmptyElementSymbol));
        assert_eq!(Element::new(1, "Hyd", "Hydrogen"), Err(ChemistryError::InvalidElementSymbol));
        assert_eq!(Element::new(1, "H", " "), Err(ChemistryError::EmptyElementName));
    }

    #[test]
    fn molecular_points_preserve_f64_angstrom_coordinates() {
        let position = point(1.25, -2.5, 3.75);
        assert_eq!(position.to_array(), [1.25, -2.5, 3.75]);
    }

    #[test]
    fn molecular_points_reject_non_finite_coordinates_by_axis() {
        assert_eq!(
            MolecularPoint3::new(f64::NAN, 0.0, 0.0),
            Err(ChemistryError::NonFiniteCoordinate { axis: MolecularAxis::X })
        );
        assert_eq!(
            MolecularPoint3::new(0.0, f64::INFINITY, 0.0),
            Err(ChemistryError::NonFiniteCoordinate { axis: MolecularAxis::Y })
        );
        assert_eq!(
            MolecularPoint3::new(0.0, 0.0, f64::NEG_INFINITY),
            Err(ChemistryError::NonFiniteCoordinate { axis: MolecularAxis::Z })
        );
    }

    #[test]
    fn element_definitions_are_reused_by_distinct_atom_instances() {
        let molecule = water();
        let hydrogens: Vec<_> = molecule
            .atoms()
            .iter()
            .filter(|atom| atom.element() == Element::HYDROGEN)
            .collect();
        assert_eq!(hydrogens.len(), 2);
        assert_ne!(hydrogens[0].id(), hydrogens[1].id());
        assert_ne!(hydrogens[0].position(), hydrogens[1].position());
        assert_eq!(hydrogens[0].element(), hydrogens[1].element());
    }

    #[test]
    fn water_has_expected_composition_and_connectivity() {
        let molecule = water();
        assert_eq!(molecule.atom_count(), 3);
        assert_eq!(molecule.bond_count(), 2);
        assert_eq!(molecule.atoms()[0].element(), Element::OXYGEN);
        assert_eq!(
            molecule.atoms().iter().filter(|a| a.element() == Element::HYDROGEN).count(),
            2
        );
        assert_eq!(molecule.bonds_for_atom(AtomId::new(10)).count(), 2);
        assert_eq!(molecule.bonds_for_atom(AtomId::new(11)).count(), 1);
        assert_eq!(molecule.atom(AtomId::new(12)).unwrap().id(), AtomId::new(12));
        assert_eq!(molecule.bond(BondId::new(21)).unwrap().order(), BondOrder::Single);
    }

    #[test]
    fn methane_has_expected_composition_and_connectivity() {
        let molecule = methane();
        assert_eq!(molecule.atom_count(), 5);
        assert_eq!(molecule.bond_count(), 4);
        assert_eq!(molecule.atoms()[0].element(), Element::CARBON);
        assert_eq!(
            molecule.atoms().iter().filter(|a| a.element() == Element::HYDROGEN).count(),
            4
        );
        assert_eq!(molecule.bonds_for_atom(AtomId::new(1)).count(), 4);
        for id in 2..=5 {
            assert_eq!(molecule.bonds_for_atom(AtomId::new(id)).count(), 1);
        }
    }

    #[test]
    fn molecule_preserves_atom_and_bond_order() {
        let molecule = water();
        assert_eq!(molecule.atoms().iter().map(|a| a.id().get()).collect::<Vec<_>>(), [10, 11, 12]);
        assert_eq!(molecule.bonds().iter().map(|b| b.id().get()).collect::<Vec<_>>(), [20, 21]);
    }

    #[test]
    fn all_supported_bond_orders_are_preserved() {
        let molecule = Molecule::new(
            vec![
                atom(1, Element::CARBON, point(0.0, 0.0, 0.0)),
                atom(2, Element::CARBON, point(1.0, 0.0, 0.0)),
                atom(3, Element::CARBON, point(2.0, 0.0, 0.0)),
                atom(4, Element::CARBON, point(3.0, 0.0, 0.0)),
            ],
            vec![
                bond(1, 1, 2, BondOrder::Single),
                bond(2, 2, 3, BondOrder::Double),
                bond(3, 3, 4, BondOrder::Triple),
            ],
        )
        .unwrap();
        assert_eq!(
            molecule.bonds().iter().map(|b| b.order()).collect::<Vec<_>>(),
            [BondOrder::Single, BondOrder::Double, BondOrder::Triple]
        );
    }

    #[test]
    fn duplicate_atom_ids_are_rejected() {
        let result = Molecule::new(
            vec![
                atom(1, Element::HYDROGEN, point(0.0, 0.0, 0.0)),
                atom(1, Element::OXYGEN, point(1.0, 0.0, 0.0)),
            ],
            vec![],
        );
        assert_eq!(result, Err(ChemistryError::DuplicateAtomId { id: AtomId::new(1) }));
    }

    #[test]
    fn duplicate_bond_ids_are_rejected() {
        let result = Molecule::new(
            vec![
                atom(1, Element::CARBON, point(0.0, 0.0, 0.0)),
                atom(2, Element::CARBON, point(1.0, 0.0, 0.0)),
                atom(3, Element::CARBON, point(2.0, 0.0, 0.0)),
            ],
            vec![bond(7, 1, 2, BondOrder::Single), bond(7, 2, 3, BondOrder::Single)],
        );
        assert_eq!(result, Err(ChemistryError::DuplicateBondId { id: BondId::new(7) }));
    }

    #[test]
    fn unknown_bond_endpoints_are_rejected() {
        let result = Molecule::new(
            vec![atom(1, Element::HYDROGEN, point(0.0, 0.0, 0.0))],
            vec![bond(3, 1, 2, BondOrder::Single)],
        );
        assert_eq!(
            result,
            Err(ChemistryError::UnknownBondAtom {
                bond_id: BondId::new(3),
                atom_id: AtomId::new(2)
            })
        );
    }

    #[test]
    fn self_bonds_are_rejected() {
        let result = Molecule::new(
            vec![atom(1, Element::HYDROGEN, point(0.0, 0.0, 0.0))],
            vec![bond(3, 1, 1, BondOrder::Single)],
        );
        assert_eq!(
            result,
            Err(ChemistryError::SelfBond {
                bond_id: BondId::new(3),
                atom_id: AtomId::new(1)
            })
        );
    }

    #[test]
    fn duplicate_undirected_bond_pairs_are_rejected() {
        let result = Molecule::new(
            vec![
                atom(1, Element::OXYGEN, point(0.0, 0.0, 0.0)),
                atom(2, Element::HYDROGEN, point(1.0, 0.0, 0.0)),
            ],
            vec![
                bond(10, 1, 2, BondOrder::Single),
                bond(11, 2, 1, BondOrder::Double),
            ],
        );
        assert_eq!(
            result,
            Err(ChemistryError::DuplicateBondPair {
                first: AtomId::new(1),
                second: AtomId::new(2)
            })
        );
    }

    #[test]
    fn isolated_atoms_and_empty_molecules_are_valid_structures() {
        let empty = Molecule::new(vec![], vec![]).unwrap();
        assert!(empty.is_empty());
        let isolated = Molecule::new(
            vec![atom(4, Element::OXYGEN, point(0.0, 0.0, 0.0))],
            vec![],
        )
        .unwrap();
        assert_eq!(isolated.atom_count(), 1);
        assert_eq!(isolated.bond_count(), 0);
    }

    #[test]
    fn molecule_accessors_do_not_expose_mutable_storage() {
        let molecule = water();
        assert_eq!(molecule.atoms().len(), 3);
        assert_eq!(molecule.bonds().len(), 2);
        assert!(molecule.atom(AtomId::new(999)).is_none());
        assert!(molecule.bond(BondId::new(999)).is_none());
        assert_eq!(molecule.bonds_for_atom(AtomId::new(999)).count(), 0);
    }
}
