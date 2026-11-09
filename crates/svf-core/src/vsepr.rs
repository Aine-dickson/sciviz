//! Graphics-independent VSEPR electron-domain semantics for chemistry lessons.
//!
//! This first model is deliberately limited to four-domain tetrahedral
//! arrangements for methane and water. Electron domains represent regions of
//! electron density in a VSEPR model, not individual electrons or measured
//! spatial boundaries. Molecular coordinates are not used to infer the model.

use std::{
    collections::HashSet,
    error::Error,
    fmt,
};

use crate::chemistry::{AtomId, BondId, Molecule};

/// Stable identity for one electron domain within a VSEPR centre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElectronDomainId(u32);

impl ElectronDomainId {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// The semantic classification and source relationship of an electron domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElectronDomainKind {
    /// One bonding-pair region associated with one central-atom bond.
    BondingPair {
        bond_id: BondId,
        neighboring_atom: AtomId,
    },
    /// A lone-pair region associated only with the central atom.
    LonePair,
}

/// One explicitly identified electron domain around a central atom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElectronDomain {
    id: ElectronDomainId,
    kind: ElectronDomainKind,
}

impl ElectronDomain {
    #[must_use]
    pub const fn bonding_pair(
        id: ElectronDomainId,
        bond_id: BondId,
        neighboring_atom: AtomId,
    ) -> Self {
        Self {
            id,
            kind: ElectronDomainKind::BondingPair {
                bond_id,
                neighboring_atom,
            },
        }
    }

    #[must_use]
    pub const fn lone_pair(id: ElectronDomainId) -> Self {
        Self {
            id,
            kind: ElectronDomainKind::LonePair,
        }
    }

    #[must_use]
    pub const fn id(self) -> ElectronDomainId {
        self.id
    }

    #[must_use]
    pub const fn kind(self) -> ElectronDomainKind {
        self.kind
    }

    #[must_use]
    pub const fn is_bonding_pair(self) -> bool {
        matches!(self.kind, ElectronDomainKind::BondingPair { .. })
    }

    #[must_use]
    pub const fn is_lone_pair(self) -> bool {
        matches!(self.kind, ElectronDomainKind::LonePair)
    }
}

/// Electron-domain arrangement supported by this initial VSEPR experiment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElectronDomainArrangement {
    /// Four electron domains arranged tetrahedrally.
    Tetrahedral,
}

/// Molecular shape supported by this initial VSEPR experiment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MolecularShape {
    Tetrahedral,
    Bent,
}

/// Errors produced while validating a VSEPR centre against a molecule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VseprError {
    UnknownCentralAtom { atom_id: AtomId },
    DuplicateDomainId { domain_id: ElectronDomainId },
    UnknownBond { bond_id: BondId },
    BondNotIncidentToCentralAtom { bond_id: BondId, central_atom: AtomId },
    IncorrectNeighbor {
        bond_id: BondId,
        expected: AtomId,
        actual: AtomId,
    },
    DuplicateBondDomain { bond_id: BondId },
    MissingIncidentBondDomain { bond_id: BondId },
    UnsupportedDomainPattern {
        bonding_pairs: usize,
        lone_pairs: usize,
    },
    InconsistentGeometryClassification {
        arrangement: ElectronDomainArrangement,
        shape: MolecularShape,
    },
}

impl fmt::Display for VseprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCentralAtom { atom_id } => {
                write!(f, "unknown central atom {}", atom_id.get())
            }
            Self::DuplicateDomainId { domain_id } => {
                write!(f, "duplicate electron-domain ID {}", domain_id.get())
            }
            Self::UnknownBond { bond_id } => {
                write!(f, "unknown source bond {}", bond_id.get())
            }
            Self::BondNotIncidentToCentralAtom { bond_id, central_atom } => write!(
                f,
                "bond {} is not incident to central atom {}",
                bond_id.get(),
                central_atom.get()
            ),
            Self::IncorrectNeighbor { bond_id, expected, actual } => write!(
                f,
                "bond {} has neighboring atom {}, not {}",
                bond_id.get(),
                actual.get(),
                expected.get()
            ),
            Self::DuplicateBondDomain { bond_id } => {
                write!(f, "bond {} is represented by multiple domains", bond_id.get())
            }
            Self::MissingIncidentBondDomain { bond_id } => {
                write!(f, "incident bond {} has no bonding domain", bond_id.get())
            }
            Self::UnsupportedDomainPattern { bonding_pairs, lone_pairs } => write!(
                f,
                "unsupported VSEPR domain pattern: {bonding_pairs} bonding pairs and {lone_pairs} lone pairs"
            ),
            Self::InconsistentGeometryClassification { arrangement, shape } => write!(
                f,
                "molecular shape {shape:?} is inconsistent with {arrangement:?} arrangement and domain pattern"
            ),
        }
    }
}

impl Error for VseprError {}

/// A validated VSEPR model centred on one atom in a source molecule.
///
/// Domain input order is preserved. This model records idealized VSEPR
/// classifications; it does not derive them from molecular coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VseprCentre {
    central_atom: AtomId,
    domains: Vec<ElectronDomain>,
    arrangement: ElectronDomainArrangement,
    molecular_shape: MolecularShape,
}

impl VseprCentre {
    /// Constructs a VSEPR centre after validating domain identity and provenance.
    ///
    /// Every bond incident to `central_atom` must be represented by exactly
    /// one bonding-pair domain. A double or triple bond still contributes one
    /// domain, as required by VSEPR. Only the methane and water four-domain
    /// patterns are currently supported.
    pub fn new(
        molecule: &Molecule,
        central_atom: AtomId,
        domains: Vec<ElectronDomain>,
        arrangement: ElectronDomainArrangement,
        molecular_shape: MolecularShape,
    ) -> Result<Self, VseprError> {
        if molecule.atom(central_atom).is_none() {
            return Err(VseprError::UnknownCentralAtom {
                atom_id: central_atom,
            });
        }

        let mut domain_ids = HashSet::with_capacity(domains.len());
        let mut represented_bonds = HashSet::with_capacity(domains.len());
        let mut bonding_pairs = 0;
        let mut lone_pairs = 0;

        for domain in &domains {
            if !domain_ids.insert(domain.id) {
                return Err(VseprError::DuplicateDomainId {
                    domain_id: domain.id,
                });
            }

            match domain.kind {
                ElectronDomainKind::BondingPair {
                    bond_id,
                    neighboring_atom,
                } => {
                    let bond = molecule
                        .bond(bond_id)
                        .ok_or(VseprError::UnknownBond { bond_id })?;

                    let expected_neighbor = if bond.atom_a() == central_atom {
                        bond.atom_b()
                    } else if bond.atom_b() == central_atom {
                        bond.atom_a()
                    } else {
                        return Err(VseprError::BondNotIncidentToCentralAtom {
                            bond_id,
                            central_atom,
                        });
                    };

                    if neighboring_atom != expected_neighbor {
                        return Err(VseprError::IncorrectNeighbor {
                            bond_id,
                            expected: expected_neighbor,
                            actual: neighboring_atom,
                        });
                    }

                    if !represented_bonds.insert(bond_id) {
                        return Err(VseprError::DuplicateBondDomain { bond_id });
                    }
                    bonding_pairs += 1;
                }
                ElectronDomainKind::LonePair => lone_pairs += 1,
            }
        }

        for bond in molecule.bonds_for_atom(central_atom) {
            if !represented_bonds.contains(&bond.id()) {
                return Err(VseprError::MissingIncidentBondDomain {
                    bond_id: bond.id(),
                });
            }
        }

        let supported_shape = match (bonding_pairs, lone_pairs) {
            (4, 0) => Some(MolecularShape::Tetrahedral),
            (2, 2) => Some(MolecularShape::Bent),
            _ => None,
        };

        let Some(expected_shape) = supported_shape else {
            return Err(VseprError::UnsupportedDomainPattern {
                bonding_pairs,
                lone_pairs,
            });
        };

        if arrangement != ElectronDomainArrangement::Tetrahedral
            || molecular_shape != expected_shape
        {
            return Err(VseprError::InconsistentGeometryClassification {
                arrangement,
                shape: molecular_shape,
            });
        }

        Ok(Self {
            central_atom,
            domains,
            arrangement,
            molecular_shape,
        })
    }

    #[must_use]
    pub const fn central_atom(&self) -> AtomId {
        self.central_atom
    }

    #[must_use]
    pub fn domains(&self) -> &[ElectronDomain] {
        &self.domains
    }

    #[must_use]
    pub const fn arrangement(&self) -> ElectronDomainArrangement {
        self.arrangement
    }

    #[must_use]
    pub const fn molecular_shape(&self) -> MolecularShape {
        self.molecular_shape
    }

    #[must_use]
    pub fn bonding_pair_count(&self) -> usize {
        self.domains.iter().filter(|domain| domain.is_bonding_pair()).count()
    }

    #[must_use]
    pub fn lone_pair_count(&self) -> usize {
        self.domains.iter().filter(|domain| domain.is_lone_pair()).count()
    }

    #[must_use]
    pub fn domain(&self, id: ElectronDomainId) -> Option<&ElectronDomain> {
        self.domains.iter().find(|domain| domain.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chemistry::{
        Atom, Bond, BondOrder, Element, MolecularPoint3,
    };

    fn point(x: f64, y: f64, z: f64) -> MolecularPoint3 {
        MolecularPoint3::new(x, y, z).unwrap()
    }

    fn atom(id: u32, element: Element) -> Atom {
        Atom::new(AtomId::new(id), element, point(id as f64, 0.0, 0.0))
    }

    fn bond(id: u32, a: u32, b: u32) -> Bond {
        Bond::new(BondId::new(id), AtomId::new(a), AtomId::new(b), BondOrder::Single)
    }

    fn water() -> Molecule {
        Molecule::new(
            vec![
                atom(10, Element::OXYGEN),
                atom(11, Element::HYDROGEN),
                atom(12, Element::HYDROGEN),
            ],
            vec![bond(20, 10, 11), bond(21, 10, 12)],
        )
        .unwrap()
    }

    fn methane() -> Molecule {
        Molecule::new(
            vec![
                atom(1, Element::CARBON),
                atom(2, Element::HYDROGEN),
                atom(3, Element::HYDROGEN),
                atom(4, Element::HYDROGEN),
                atom(5, Element::HYDROGEN),
            ],
            vec![bond(10, 1, 2), bond(11, 1, 3), bond(12, 1, 4), bond(13, 1, 5)],
        )
        .unwrap()
    }

    fn bonding_domain(id: u32, bond_id: u32, neighbor: u32) -> ElectronDomain {
        ElectronDomain::bonding_pair(
            ElectronDomainId::new(id),
            BondId::new(bond_id),
            AtomId::new(neighbor),
        )
    }

    fn lone_pair(id: u32) -> ElectronDomain {
        ElectronDomain::lone_pair(ElectronDomainId::new(id))
    }

    fn tetrahedral() -> ElectronDomainArrangement {
        ElectronDomainArrangement::Tetrahedral
    }

    #[test]
    fn methane_has_four_bonding_domains_and_tetrahedral_shape() {
        let centre = VseprCentre::new(
            &methane(),
            AtomId::new(1),
            vec![
                bonding_domain(100, 10, 2),
                bonding_domain(101, 11, 3),
                bonding_domain(102, 12, 4),
                bonding_domain(103, 13, 5),
            ],
            tetrahedral(),
            MolecularShape::Tetrahedral,
        )
        .unwrap();

        assert_eq!(centre.central_atom(), AtomId::new(1));
        assert_eq!(centre.bonding_pair_count(), 4);
        assert_eq!(centre.lone_pair_count(), 0);
        assert_eq!(centre.arrangement(), tetrahedral());
        assert_eq!(centre.molecular_shape(), MolecularShape::Tetrahedral);
    }

    #[test]
    fn water_has_two_bonding_and_two_lone_pair_domains_and_bent_shape() {
        let centre = VseprCentre::new(
            &water(),
            AtomId::new(10),
            vec![
                bonding_domain(200, 20, 11),
                bonding_domain(201, 21, 12),
                lone_pair(202),
                lone_pair(203),
            ],
            tetrahedral(),
            MolecularShape::Bent,
        )
        .unwrap();

        assert_eq!(centre.bonding_pair_count(), 2);
        assert_eq!(centre.lone_pair_count(), 2);
        assert_eq!(centre.arrangement(), tetrahedral());
        assert_eq!(centre.molecular_shape(), MolecularShape::Bent);
        assert!(centre.domain(ElectronDomainId::new(202)).unwrap().is_lone_pair());
    }

    #[test]
    fn domain_input_order_and_identity_are_preserved() {
        let centre = VseprCentre::new(
            &water(),
            AtomId::new(10),
            vec![
                lone_pair(203),
                bonding_domain(201, 21, 12),
                lone_pair(202),
                bonding_domain(200, 20, 11),
            ],
            tetrahedral(),
            MolecularShape::Bent,
        )
        .unwrap();

        assert_eq!(
            centre.domains().iter().map(|domain| domain.id().get()).collect::<Vec<_>>(),
            [203, 201, 202, 200]
        );
        assert_eq!(centre.domain(ElectronDomainId::new(201)).unwrap().kind(),
            ElectronDomainKind::BondingPair {
                bond_id: BondId::new(21),
                neighboring_atom: AtomId::new(12),
            });
    }

    #[test]
    fn missing_central_atom_is_rejected() {
        assert_eq!(
            VseprCentre::new(
                &water(),
                AtomId::new(999),
                vec![],
                tetrahedral(),
                MolecularShape::Bent,
            ),
            Err(VseprError::UnknownCentralAtom { atom_id: AtomId::new(999) })
        );
    }

    #[test]
    fn duplicate_domain_ids_are_rejected() {
        assert_eq!(
            VseprCentre::new(
                &water(),
                AtomId::new(10),
                vec![lone_pair(1), lone_pair(1)],
                tetrahedral(),
                MolecularShape::Bent,
            ),
            Err(VseprError::DuplicateDomainId { domain_id: ElectronDomainId::new(1) })
        );
    }

    #[test]
    fn unknown_source_bond_is_rejected() {
        assert_eq!(
            VseprCentre::new(
                &water(),
                AtomId::new(10),
                vec![bonding_domain(1, 999, 11)],
                tetrahedral(),
                MolecularShape::Bent,
            ),
            Err(VseprError::UnknownBond { bond_id: BondId::new(999) })
        );
    }

    #[test]
    fn non_incident_source_bond_is_rejected() {
        let molecule = Molecule::new(
            vec![
                atom(1, Element::OXYGEN),
                atom(2, Element::HYDROGEN),
                atom(3, Element::HYDROGEN),
                atom(4, Element::HYDROGEN),
            ],
            vec![bond(10, 1, 2), bond(11, 3, 4)],
        )
        .unwrap();

        assert_eq!(
            VseprCentre::new(
                &molecule,
                AtomId::new(1),
                vec![
                    bonding_domain(1, 10, 2),
                    bonding_domain(2, 11, 4),
                    lone_pair(3),
                    lone_pair(4),
                ],
                tetrahedral(),
                MolecularShape::Bent,
            ),
            Err(VseprError::BondNotIncidentToCentralAtom {
                bond_id: BondId::new(11),
                central_atom: AtomId::new(1),
            })
        );
    }

    #[test]
    fn incorrect_neighbor_is_rejected() {
        assert_eq!(
            VseprCentre::new(
                &water(),
                AtomId::new(10),
                vec![
                    bonding_domain(1, 20, 12),
                    bonding_domain(2, 21, 11),
                    lone_pair(3),
                    lone_pair(4),
                ],
                tetrahedral(),
                MolecularShape::Bent,
            ),
            Err(VseprError::IncorrectNeighbor {
                bond_id: BondId::new(20),
                expected: AtomId::new(11),
                actual: AtomId::new(12),
            })
        );
    }

    #[test]
    fn duplicate_bond_domain_is_rejected() {
        assert_eq!(
            VseprCentre::new(
                &water(),
                AtomId::new(10),
                vec![
                    bonding_domain(1, 20, 11),
                    bonding_domain(2, 20, 11),
                    lone_pair(3),
                    lone_pair(4),
                ],
                tetrahedral(),
                MolecularShape::Bent,
            ),
            Err(VseprError::DuplicateBondDomain { bond_id: BondId::new(20) })
        );
    }

    #[test]
    fn omitted_incident_bond_domain_is_rejected() {
        assert_eq!(
            VseprCentre::new(
                &water(),
                AtomId::new(10),
                vec![
                    bonding_domain(1, 20, 11),
                    lone_pair(2),
                    lone_pair(3),
                ],
                tetrahedral(),
                MolecularShape::Bent,
            ),
            Err(VseprError::MissingIncidentBondDomain { bond_id: BondId::new(21) })
        );
    }

    #[test]
    fn unsupported_domain_patterns_are_rejected() {
        assert_eq!(
            VseprCentre::new(
                &water(),
                AtomId::new(10),
                vec![
                    bonding_domain(1, 20, 11),
                    bonding_domain(2, 21, 12),
                    lone_pair(3),
                ],
                tetrahedral(),
                MolecularShape::Bent,
            ),
            Err(VseprError::UnsupportedDomainPattern {
                bonding_pairs: 2,
                lone_pairs: 1,
            })
        );
    }

    #[test]
    fn contradictory_molecular_shape_is_rejected() {
        assert_eq!(
            VseprCentre::new(
                &water(),
                AtomId::new(10),
                vec![
                    bonding_domain(1, 20, 11),
                    bonding_domain(2, 21, 12),
                    lone_pair(3),
                    lone_pair(4),
                ],
                tetrahedral(),
                MolecularShape::Tetrahedral,
            ),
            Err(VseprError::InconsistentGeometryClassification {
                arrangement: tetrahedral(),
                shape: MolecularShape::Tetrahedral,
            })
        );
    }

    #[test]
    fn multiple_bond_order_still_maps_to_one_domain() {
        let molecule = Molecule::new(
            vec![
                atom(1, Element::OXYGEN),
                atom(2, Element::CARBON),
                atom(3, Element::HYDROGEN),
            ],
            vec![
                Bond::new(BondId::new(10), AtomId::new(1), AtomId::new(2), BondOrder::Double),
                bond(11, 1, 3),
            ],
        )
        .unwrap();

        let centre = VseprCentre::new(
            &molecule,
            AtomId::new(1),
            vec![
                bonding_domain(1, 10, 2),
                bonding_domain(2, 11, 3),
                lone_pair(3),
                lone_pair(4),
            ],
            tetrahedral(),
            MolecularShape::Bent,
        )
        .unwrap();

        assert_eq!(centre.bonding_pair_count(), 2);
        assert_eq!(centre.lone_pair_count(), 2);
    }
}
