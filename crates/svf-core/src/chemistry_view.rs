//! Graphics-independent ball-and-stick views derived from chemistry structures.
//!
//! All positions, radii, and offsets are expressed in ångströms (Å). These
//! dimensions are presentation parameters, not measured atomic boundaries.

use crate::chemistry::{AtomId, BondId, BondOrder, Element, MolecularPoint3, Molecule};

use std::{error::Error, fmt};

/// Errors produced while configuring or deriving a molecular view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MolecularViewError {
    /// An atom display radius is not finite and strictly positive.
    InvalidAtomRadius { element: Element },
    /// The same element occurs more than once in a style.
    DuplicateElementStyle { element: Element },
    /// The style has no display radius for an element in the molecule.
    MissingElementStyle { element: Element },
    /// The bond display radius is not finite and strictly positive.
    InvalidBondRadius,
    /// Multiple-bond lane separation is not finite and strictly positive.
    InvalidBondSeparation,
    /// A bond joins atoms at the same molecular position.
    CoincidentBondEndpoints { bond_id: BondId },
    /// A derived molecular coordinate is not finite.
    NonFiniteDerivedCoordinate { bond_id: BondId },
}

impl fmt::Display for MolecularViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAtomRadius { element } => {
                write!(f, "display radius for {} must be finite and positive", element.symbol())
            }
            Self::DuplicateElementStyle { element } => {
                write!(f, "duplicate display style for {}", element.symbol())
            }
            Self::MissingElementStyle { element } => {
                write!(f, "no display radius configured for {}", element.symbol())
            }
            Self::InvalidBondRadius => write!(f, "bond display radius must be finite and positive"),
            Self::InvalidBondSeparation => {
                write!(f, "multiple-bond separation must be finite and positive")
            }
            Self::CoincidentBondEndpoints { bond_id } => {
                write!(f, "bond {} has coincident atom positions", bond_id.get())
            }
            Self::NonFiniteDerivedCoordinate { bond_id } => {
                write!(f, "bond {} produced a non-finite display coordinate", bond_id.get())
            }
        }
    }
}

impl Error for MolecularViewError {}

/// A complete, explicit ball-and-stick display style.
///
/// Atom radii are keyed by element definition. No fallback radius is inferred:
/// every element in the molecule must have an entry. All dimensions are in Å.
#[derive(Clone, Debug, PartialEq)]
pub struct BallAndStickStyle {
    atom_radii: Vec<(Element, f64)>,
    bond_radius: f64,
    multiple_bond_separation: f64,
}

impl BallAndStickStyle {
    /// Creates and validates a style.
    pub fn new(
        atom_radii: Vec<(Element, f64)>,
        bond_radius: f64,
        multiple_bond_separation: f64,
    ) -> Result<Self, MolecularViewError> {
        if !bond_radius.is_finite() || bond_radius <= 0.0 {
            return Err(MolecularViewError::InvalidBondRadius);
        }
        if !multiple_bond_separation.is_finite() || multiple_bond_separation <= 0.0 {
            return Err(MolecularViewError::InvalidBondSeparation);
        }

        for (index, (element, radius)) in atom_radii.iter().enumerate() {
            if !radius.is_finite() || *radius <= 0.0 {
                return Err(MolecularViewError::InvalidAtomRadius { element: *element });
            }
            if atom_radii[..index].iter().any(|(existing, _)| existing == element) {
                return Err(MolecularViewError::DuplicateElementStyle { element: *element });
            }
        }

        Ok(Self {
            atom_radii,
            bond_radius,
            multiple_bond_separation,
        })
    }

    /// A compact conventional style for the initial H/C/O examples.
    ///
    /// These are illustrative display radii, not empirical atomic radii.
    pub fn illustrative_h_c_o() -> Self {
        Self::new(
            vec![
                (Element::HYDROGEN, 0.25),
                (Element::CARBON, 0.38),
                (Element::OXYGEN, 0.36),
            ],
            0.08,
            0.18,
        )
        .expect("built-in ball-and-stick style is valid")
    }

    #[must_use]
    pub fn atom_radius(&self, element: Element) -> Option<f64> {
        self.atom_radii
            .iter()
            .find_map(|(candidate, radius)| (*candidate == element).then_some(*radius))
    }

    #[must_use]
    pub const fn bond_radius(&self) -> f64 {
        self.bond_radius
    }

    #[must_use]
    pub const fn multiple_bond_separation(&self) -> f64 {
        self.multiple_bond_separation
    }
}

/// An atom sphere in a derived molecular view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtomGlyph3 {
    atom_id: AtomId,
    element: Element,
    center: MolecularPoint3,
    display_radius: f64,
}

impl AtomGlyph3 {
    #[must_use]
    pub const fn atom_id(self) -> AtomId {
        self.atom_id
    }

    #[must_use]
    pub const fn element(self) -> Element {
        self.element
    }

    #[must_use]
    pub const fn center(self) -> MolecularPoint3 {
        self.center
    }

    #[must_use]
    pub const fn display_radius(self) -> f64 {
        self.display_radius
    }
}

/// One axis primitive for displaying a semantic bond.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BondAxis3 {
    start: MolecularPoint3,
    end: MolecularPoint3,
    display_radius: f64,
}

impl BondAxis3 {
    #[must_use]
    pub const fn start(self) -> MolecularPoint3 {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> MolecularPoint3 {
        self.end
    }

    #[must_use]
    pub const fn display_radius(self) -> f64 {
        self.display_radius
    }
}

/// The display geometry derived from one semantic bond.
///
/// A double or triple bond has multiple parallel axes, but all axes retain the
/// identity of this one source bond.
#[derive(Clone, Debug, PartialEq)]
pub struct BondGlyph3 {
    bond_id: BondId,
    endpoints: [AtomId; 2],
    order: BondOrder,
    axes: Vec<BondAxis3>,
}

impl BondGlyph3 {
    #[must_use]
    pub const fn bond_id(&self) -> BondId {
        self.bond_id
    }

    #[must_use]
    pub const fn endpoints(&self) -> [AtomId; 2] {
        self.endpoints
    }

    #[must_use]
    pub const fn order(&self) -> BondOrder {
        self.order
    }

    #[must_use]
    pub fn axes(&self) -> &[BondAxis3] {
        &self.axes
    }
}

/// A graphics-independent ball-and-stick representation in ångströms.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MolecularView3 {
    atoms: Vec<AtomGlyph3>,
    bonds: Vec<BondGlyph3>,
}

impl MolecularView3 {
    #[must_use]
    pub fn atoms(&self) -> &[AtomGlyph3] {
        &self.atoms
    }

    #[must_use]
    pub fn bonds(&self) -> &[BondGlyph3] {
        &self.bonds
    }

    #[must_use]
    pub const fn length_unit(&self) -> MolecularLengthUnit {
        MolecularLengthUnit::Angstrom
    }
}

/// Length unit used by the molecular composition and its derived view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MolecularLengthUnit {
    Angstrom,
}

/// Derives a deterministic ball-and-stick view without modifying the molecule.
pub fn build_ball_and_stick_view(
    molecule: &Molecule,
    style: &BallAndStickStyle,
) -> Result<MolecularView3, MolecularViewError> {
    let mut atoms = Vec::with_capacity(molecule.atom_count());
    for atom in molecule.atoms() {
        let radius = style.atom_radius(atom.element()).ok_or(
            MolecularViewError::MissingElementStyle {
                element: atom.element(),
            },
        )?;
        atoms.push(AtomGlyph3 {
            atom_id: atom.id(),
            element: atom.element(),
            center: atom.position(),
            display_radius: radius,
        });
    }

    let mut bonds = Vec::with_capacity(molecule.bond_count());
    for bond in molecule.bonds() {
        let atom_a = molecule
            .atom(bond.atom_a())
            .expect("Molecule validates all bond endpoints");
        let atom_b = molecule
            .atom(bond.atom_b())
            .expect("Molecule validates all bond endpoints");
        let start = atom_a.position().to_array();
        let end = atom_b.position().to_array();
        let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
        if !delta.iter().all(|value| value.is_finite()) {
            return Err(MolecularViewError::NonFiniteDerivedCoordinate {
                bond_id: bond.id(),
            });
        }

        let length = delta[0].hypot(delta[1]).hypot(delta[2]);
        if !length.is_finite() {
            return Err(MolecularViewError::NonFiniteDerivedCoordinate {
                bond_id: bond.id(),
            });
        }
        if length == 0.0 {
            return Err(MolecularViewError::CoincidentBondEndpoints {
                bond_id: bond.id(),
            });
        }

        let direction = [delta[0] / length, delta[1] / length, delta[2] / length];
        let axes_count = match bond.order() {
            BondOrder::Single => 1,
            BondOrder::Double => 2,
            BondOrder::Triple => 3,
        };
        let offsets = lane_offsets(
            direction,
            axes_count,
            style.multiple_bond_separation,
        )
        .ok_or(MolecularViewError::NonFiniteDerivedCoordinate {
            bond_id: bond.id(),
        })?;

        let mut axes = Vec::with_capacity(axes_count);
        for offset in offsets {
            let mut axis_start = [0.0; 3];
            let mut axis_end = [0.0; 3];
            for axis in 0..3 {
                axis_start[axis] = start[axis] + offset[axis];
                axis_end[axis] = end[axis] + offset[axis];
            }
            if !axis_start.iter().chain(axis_end.iter()).all(|value| value.is_finite()) {
                return Err(MolecularViewError::NonFiniteDerivedCoordinate {
                    bond_id: bond.id(),
                });
            }
            axes.push(BondAxis3 {
                start: MolecularPoint3::new(axis_start[0], axis_start[1], axis_start[2])
                    .expect("derived coordinates were checked finite"),
                end: MolecularPoint3::new(axis_end[0], axis_end[1], axis_end[2])
                    .expect("derived coordinates were checked finite"),
                display_radius: style.bond_radius,
            });
        }

        bonds.push(BondGlyph3 {
            bond_id: bond.id(),
            endpoints: bond.endpoints(),
            order: bond.order(),
            axes,
        });
    }

    Ok(MolecularView3 { atoms, bonds })
}

fn lane_offsets(direction: [f64; 3], count: usize, separation: f64) -> Option<Vec<[f64; 3]>> {
    // Choose the world axis least aligned with the bond to avoid a near-zero
    // cross product, then derive a stable perpendicular direction.
    let reference = if direction[0].abs() <= direction[1].abs()
        && direction[0].abs() <= direction[2].abs()
    {
        [1.0, 0.0, 0.0]
    } else if direction[1].abs() <= direction[2].abs() {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };

    let perpendicular = [
        direction[1] * reference[2] - direction[2] * reference[1],
        direction[2] * reference[0] - direction[0] * reference[2],
        direction[0] * reference[1] - direction[1] * reference[0],
    ];
    let norm = perpendicular[0]
        .hypot(perpendicular[1])
        .hypot(perpendicular[2]);
    if !norm.is_finite() || norm == 0.0 {
        return None;
    }
    let perpendicular = [
        perpendicular[0] / norm,
        perpendicular[1] / norm,
        perpendicular[2] / norm,
    ];

    let mut offsets = Vec::with_capacity(count);
    for lane in 0..count {
        // One lane at zero; two at ±half separation; three at
        // -separation, zero, +separation.
        let multiplier = lane as f64 - (count as f64 - 1.0) / 2.0;
        let distance = multiplier * separation;
        let offset = [
            perpendicular[0] * distance,
            perpendicular[1] * distance,
            perpendicular[2] * distance,
        ];
        if !offset.iter().all(|value| value.is_finite()) {
            return None;
        }
        offsets.push(offset);
    }
    Some(offsets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chemistry::{Atom, AtomId, Bond, BondId, MolecularPoint3};

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

    fn style() -> BallAndStickStyle {
        BallAndStickStyle::illustrative_h_c_o()
    }

    #[test]
    fn water_view_preserves_atom_and_bond_identity_and_bent_geometry() {
        let molecule = water();
        let view = build_ball_and_stick_view(&molecule, &style()).unwrap();
        assert_eq!(view.atoms().len(), 3);
        assert_eq!(view.bonds().len(), 2);
        assert_eq!(view.length_unit(), MolecularLengthUnit::Angstrom);
        assert_eq!(view.atoms().iter().map(|a| a.atom_id().get()).collect::<Vec<_>>(), [10, 11, 12]);
        assert_eq!(view.bonds().iter().map(|b| b.bond_id().get()).collect::<Vec<_>>(), [20, 21]);
        assert_eq!(view.bonds()[0].endpoints(), [AtomId::new(10), AtomId::new(11)]);
        assert_eq!(view.bonds()[1].endpoints(), [AtomId::new(10), AtomId::new(12)]);
        assert_eq!(view.atoms()[1].center(), point(0.9572, 0.0, 0.0));
        assert_eq!(view.atoms()[2].center(), point(-0.239, 0.927, 0.0));
        assert_eq!(view.atoms()[0].display_radius(), 0.36);
        assert!(view.bonds().iter().all(|bond| bond.axes().len() == 1));
    }

    #[test]
    fn methane_view_preserves_tetrahedral_atom_and_bond_mapping() {
        let view = build_ball_and_stick_view(&methane(), &style()).unwrap();
        assert_eq!(view.atoms().len(), 5);
        assert_eq!(view.bonds().len(), 4);
        assert_eq!(view.atoms()[0].element(), Element::CARBON);
        assert_eq!(view.atoms()[0].display_radius(), 0.38);
        assert_eq!(
            view.bonds().iter().map(|b| b.endpoints()[1].get()).collect::<Vec<_>>(),
            [2, 3, 4, 5]
        );
    }

    #[test]
    fn multiple_bond_orders_generate_centered_parallel_axes_with_shared_provenance() {
        let molecule = Molecule::new(
            vec![
                atom(1, Element::CARBON, point(0.0, 0.0, 0.0)),
                atom(2, Element::CARBON, point(1.0, 0.0, 0.0)),
                atom(3, Element::CARBON, point(2.0, 0.0, 0.0)),
                atom(4, Element::CARBON, point(3.0, 0.0, 0.0)),
            ],
            vec![
                bond(10, 1, 2, BondOrder::Single),
                bond(11, 2, 3, BondOrder::Double),
                bond(12, 3, 4, BondOrder::Triple),
            ],
        )
        .unwrap();
        let view = build_ball_and_stick_view(&molecule, &style()).unwrap();
        assert_eq!(view.bonds()[0].axes().len(), 1);
        assert_eq!(view.bonds()[1].axes().len(), 2);
        assert_eq!(view.bonds()[2].axes().len(), 3);
        assert_eq!(view.bonds()[1].bond_id(), BondId::new(11));
        assert_eq!(view.bonds()[2].bond_id(), BondId::new(12));

        let double_axes = view.bonds()[1].axes();
        assert_eq!(double_axes[0].start().x(), 1.0);
        assert_eq!(double_axes[1].start().x(), 1.0);
        assert!((double_axes[0].start().y() + double_axes[1].start().y()).abs() < 1e-12);
        assert!((double_axes[0].start().z() + double_axes[1].start().z()).abs() < 1e-12);
        for axis in view.bonds().iter().flat_map(|bond| bond.axes()) {
            let start = axis.start().to_array();
            let end = axis.end().to_array();
            assert!(start.iter().chain(end.iter()).all(|value| value.is_finite()));
        }
    }

    #[test]
    fn multiple_bond_offsets_are_deterministic() {
        let molecule = Molecule::new(
            vec![
                atom(1, Element::CARBON, point(0.0, 0.0, 0.0)),
                atom(2, Element::CARBON, point(0.0, 2.0, 1.0)),
            ],
            vec![bond(1, 1, 2, BondOrder::Triple)],
        )
        .unwrap();
        let first = build_ball_and_stick_view(&molecule, &style()).unwrap();
        let second = build_ball_and_stick_view(&molecule, &style()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn empty_and_isolated_molecules_produce_valid_views() {
        let empty = Molecule::new(vec![], vec![]).unwrap();
        let empty_view = build_ball_and_stick_view(&empty, &style()).unwrap();
        assert!(empty_view.atoms().is_empty());
        assert!(empty_view.bonds().is_empty());

        let isolated = Molecule::new(
            vec![atom(7, Element::OXYGEN, point(1.0, 2.0, 3.0))],
            vec![],
        )
        .unwrap();
        let isolated_view = build_ball_and_stick_view(&isolated, &style()).unwrap();
        assert_eq!(isolated_view.atoms().len(), 1);
        assert!(isolated_view.bonds().is_empty());
    }

    #[test]
    fn missing_element_style_is_reported() {
        let nitrogen = Element::new(7, "N", "Nitrogen").unwrap();
        let molecule = Molecule::new(
            vec![atom(1, nitrogen, point(0.0, 0.0, 0.0))],
            vec![],
        )
        .unwrap();
        assert_eq!(
            build_ball_and_stick_view(&molecule, &style()),
            Err(MolecularViewError::MissingElementStyle { element: nitrogen })
        );
    }

    #[test]
    fn style_rejects_non_finite_non_positive_and_duplicate_values() {
        assert_eq!(
            BallAndStickStyle::new(vec![(Element::HYDROGEN, 0.0)], 0.1, 0.2),
            Err(MolecularViewError::InvalidAtomRadius { element: Element::HYDROGEN })
        );
        assert_eq!(
            BallAndStickStyle::new(vec![(Element::HYDROGEN, f64::NAN)], 0.1, 0.2),
            Err(MolecularViewError::InvalidAtomRadius { element: Element::HYDROGEN })
        );
        assert_eq!(
            BallAndStickStyle::new(vec![], 0.0, 0.2),
            Err(MolecularViewError::InvalidBondRadius)
        );
        assert_eq!(
            BallAndStickStyle::new(vec![], 0.1, f64::INFINITY),
            Err(MolecularViewError::InvalidBondSeparation)
        );
        assert_eq!(
            BallAndStickStyle::new(
                vec![(Element::HYDROGEN, 0.2), (Element::HYDROGEN, 0.3)],
                0.1,
                0.2
            ),
            Err(MolecularViewError::DuplicateElementStyle { element: Element::HYDROGEN })
        );
    }

    #[test]
    fn coincident_bonded_atom_positions_are_rejected() {
        let molecule = Molecule::new(
            vec![
                atom(1, Element::HYDROGEN, point(1.0, 1.0, 1.0)),
                atom(2, Element::HYDROGEN, point(1.0, 1.0, 1.0)),
            ],
            vec![bond(9, 1, 2, BondOrder::Single)],
        )
        .unwrap();
        assert_eq!(
            build_ball_and_stick_view(&molecule, &style()),
            Err(MolecularViewError::CoincidentBondEndpoints { bond_id: BondId::new(9) })
        );
    }

    #[test]
    fn derived_coordinate_overflow_is_rejected() {
        let molecule = Molecule::new(
            vec![
                atom(1, Element::CARBON, point(f64::MAX, 0.0, 0.0)),
                atom(2, Element::CARBON, point(-f64::MAX, 0.0, 0.0)),
            ],
            vec![bond(9, 1, 2, BondOrder::Single)],
        )
        .unwrap();
        assert_eq!(
            build_ball_and_stick_view(&molecule, &style()),
            Err(MolecularViewError::NonFiniteDerivedCoordinate { bond_id: BondId::new(9) })
        );
    }

    #[test]
    fn building_a_view_does_not_mutate_the_source_molecule() {
        let molecule = water();
        let original = molecule.clone();
        let _view = build_ball_and_stick_view(&molecule, &style()).unwrap();
        assert_eq!(molecule, original);
    }
}
