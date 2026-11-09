//! Dimensionless schematic views of VSEPR electron-domain arrangements.
//!
//! These coordinates describe a canonical presentation of an idealized
//! electron-domain arrangement. They are not molecular coordinates, world
//! coordinates, ångström measurements, or measured electron-density maxima.

use crate::vsepr::{
    ElectronDomainId, ElectronDomainKind, ElectronDomainArrangement, MolecularShape, VseprCentre,
};

/// A finite, dimensionless point in a local VSEPR schematic coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VseprSchematicPoint3 {
    coordinates: [f64; 3],
}

impl VseprSchematicPoint3 {
    /// Constructs a point from finite dimensionless schematic coordinates.
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Option<Self> {
        if x.is_finite() && y.is_finite() && z.is_finite() {
            Some(Self {
                coordinates: [x, y, z],
            })
        } else {
            None
        }
    }

    /// The local schematic origin, used for the central-atom anchor.
    #[must_use]
    pub const fn origin() -> Self {
        Self {
            coordinates: [0.0, 0.0, 0.0],
        }
    }

    /// Returns the dimensionless coordinates in local schematic space.
    #[must_use]
    pub const fn to_array(self) -> [f64; 3] {
        self.coordinates
    }
}

/// One schematic marker preserving the identity and source meaning of a
/// validated VSEPR electron domain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VseprDomainGlyph3 {
    domain_id: ElectronDomainId,
    kind: ElectronDomainKind,
    anchor: VseprSchematicPoint3,
}

impl VseprDomainGlyph3 {
    #[must_use]
    pub const fn domain_id(self) -> ElectronDomainId {
        self.domain_id
    }

    #[must_use]
    pub const fn kind(self) -> ElectronDomainKind {
        self.kind
    }

    /// Returns the dimensionless local schematic anchor.
    #[must_use]
    pub const fn anchor(self) -> VseprSchematicPoint3 {
        self.anchor
    }
}

/// A canonical schematic of a validated four-domain VSEPR arrangement.
///
/// The domain anchors show the idealized electron-domain arrangement only.
/// Molecular shape is retained as separate semantic metadata and is never
/// inferred from these schematic coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct VseprSchematicView3 {
    central_atom: crate::chemistry::AtomId,
    central_anchor: VseprSchematicPoint3,
    arrangement: ElectronDomainArrangement,
    molecular_shape: MolecularShape,
    domains: Vec<VseprDomainGlyph3>,
}

impl VseprSchematicView3 {
    /// Builds the canonical four-domain tetrahedral schematic for a validated
    /// VSEPR centre. Source domain order and provenance are preserved.
    #[must_use]
    pub fn from_centre(centre: &VseprCentre) -> Self {
        const INV_SQRT_3: f64 = 0.577_350_269_189_625_8;
        const DIRECTIONS: [[f64; 3]; 4] = [
            [INV_SQRT_3, INV_SQRT_3, INV_SQRT_3],
            [INV_SQRT_3, -INV_SQRT_3, -INV_SQRT_3],
            [-INV_SQRT_3, INV_SQRT_3, -INV_SQRT_3],
            [-INV_SQRT_3, -INV_SQRT_3, INV_SQRT_3],
        ];

        let domains = centre
            .domains()
            .iter()
            .zip(DIRECTIONS)
            .map(|(domain, direction)| VseprDomainGlyph3 {
                domain_id: domain.id(),
                kind: domain.kind(),
                anchor: VseprSchematicPoint3 {
                    coordinates: direction,
                },
            })
            .collect();

        Self {
            central_atom: centre.central_atom(),
            central_anchor: VseprSchematicPoint3::origin(),
            arrangement: centre.arrangement(),
            molecular_shape: centre.molecular_shape(),
            domains,
        }
    }

    #[must_use]
    pub const fn central_atom(&self) -> crate::chemistry::AtomId {
        self.central_atom
    }

    #[must_use]
    pub const fn central_anchor(&self) -> VseprSchematicPoint3 {
        self.central_anchor
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
    pub fn domains(&self) -> &[VseprDomainGlyph3] {
        &self.domains
    }

    #[must_use]
    pub fn domain(&self, id: ElectronDomainId) -> Option<&VseprDomainGlyph3> {
        self.domains.iter().find(|domain| domain.domain_id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chemistry::{Atom, AtomId, Bond, BondId, BondOrder, Element, MolecularPoint3};
    use crate::vsepr::{ElectronDomain, ElectronDomainId};

    fn molecule_water() -> crate::chemistry::Molecule {
        let point = |x| MolecularPoint3::new(x, 0.0, 0.0).unwrap();
        crate::chemistry::Molecule::new(
            vec![
                Atom::new(AtomId::new(10), Element::OXYGEN, point(0.0)),
                Atom::new(AtomId::new(11), Element::HYDROGEN, point(1.0)),
                Atom::new(AtomId::new(12), Element::HYDROGEN, point(-1.0)),
            ],
            vec![
                Bond::new(BondId::new(20), AtomId::new(10), AtomId::new(11), BondOrder::Single),
                Bond::new(BondId::new(21), AtomId::new(10), AtomId::new(12), BondOrder::Single),
            ],
        )
        .unwrap()
    }

    fn water_centre() -> VseprCentre {
        VseprCentre::new(
            &molecule_water(),
            AtomId::new(10),
            vec![
                ElectronDomain::bonding_pair(ElectronDomainId::new(200), BondId::new(20), AtomId::new(11)),
                ElectronDomain::bonding_pair(ElectronDomainId::new(201), BondId::new(21), AtomId::new(12)),
                ElectronDomain::lone_pair(ElectronDomainId::new(202)),
                ElectronDomain::lone_pair(ElectronDomainId::new(203)),
            ],
            ElectronDomainArrangement::Tetrahedral,
            MolecularShape::Bent,
        )
        .unwrap()
    }

    #[test]
    fn canonical_directions_are_finite_unit_and_pairwise_tetrahedral() {
        let view = VseprSchematicView3::from_centre(&water_centre());
        let directions = view.domains().iter().map(|glyph| glyph.anchor().to_array()).collect::<Vec<_>>();
        for direction in &directions {
            let length = direction.iter().map(|value| value * value).sum::<f64>().sqrt();
            assert!((length - 1.0).abs() < 1.0e-12);
            assert!(direction.iter().all(|value| value.is_finite()));
        }
        for first in 0..directions.len() {
            for second in (first + 1)..directions.len() {
                let dot = (0..3).map(|axis| directions[first][axis] * directions[second][axis]).sum::<f64>();
                assert!((dot + 1.0 / 3.0).abs() < 1.0e-12);
            }
        }
    }

    #[test]
    fn central_anchor_is_local_origin() {
        let view = VseprSchematicView3::from_centre(&water_centre());
        assert_eq!(view.central_anchor().to_array(), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn water_preserves_domain_order_identity_and_provenance() {
        let centre = water_centre();
        let view = VseprSchematicView3::from_centre(&centre);
        assert_eq!(view.central_atom(), AtomId::new(10));
        assert_eq!(view.arrangement(), ElectronDomainArrangement::Tetrahedral);
        assert_eq!(view.molecular_shape(), MolecularShape::Bent);
        assert_eq!(
            view.domains().iter().map(|glyph| glyph.domain_id().get()).collect::<Vec<_>>(),
            [200, 201, 202, 203]
        );
        assert_eq!(
            view.domain(ElectronDomainId::new(200)).unwrap().kind(),
            ElectronDomainKind::BondingPair {
                bond_id: BondId::new(20),
                neighboring_atom: AtomId::new(11),
            }
        );
        assert!(view.domain(ElectronDomainId::new(202)).unwrap().kind() == ElectronDomainKind::LonePair);
        assert_eq!(centre, water_centre());
    }

    #[test]
    fn schematic_mapping_is_deterministic() {
        let centre = water_centre();
        assert_eq!(
            VseprSchematicView3::from_centre(&centre),
            VseprSchematicView3::from_centre(&centre)
        );
    }

    #[test]
    fn schematic_point_rejects_non_finite_coordinates() {
        assert!(VseprSchematicPoint3::new(f64::NAN, 0.0, 0.0).is_none());
        assert!(VseprSchematicPoint3::new(0.0, f64::INFINITY, 0.0).is_none());
        assert_eq!(VseprSchematicPoint3::new(1.0, 2.0, 3.0).unwrap().to_array(), [1.0, 2.0, 3.0]);
    }
}
