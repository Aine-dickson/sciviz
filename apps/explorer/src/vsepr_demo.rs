//! Standalone methane/water VSEPR schematic sample for the Explorer.
//!
//! The source molecules and VSEPR centres are built from svf-core. Rendered
//! markers are a deliberately simple display adapter: octahedral markers stand
//! for bonding-pair domains, tetrahedral markers for lone-pair domains, and
//! no lines are drawn between domains. These are not atom positions or bonds.

use svf_core::{
    chemistry::{Atom, AtomId, Bond, BondId, BondOrder, Element, Molecule, MolecularPoint3},
    geometry::TriangleMesh3,
    spatial::WorldPoint3,
    vsepr::{
        ElectronDomain, ElectronDomainArrangement, ElectronDomainId, MolecularShape, VseprCentre,
    },
    vsepr_view::VseprSchematicView3,
};

const DOMAIN_DISTANCE: f64 = 1.35;
const BONDING_MARKER_RADIUS: f64 = 0.25;
const LONE_PAIR_MARKER_RADIUS: f64 = 0.22;
const CENTRAL_MARKER_RADIUS: f64 = 0.34;

/// Validated scientific models and their schematic views for this sample.
pub struct VseprDemo {
    pub mesh: TriangleMesh3,
    pub methane: VseprSchematicView3,
    pub water: VseprSchematicView3,
}

impl VseprDemo {
    pub fn build() -> Result<Self, Box<dyn std::error::Error>> {
        let (methane_molecule, methane_centre) = methane_model()?;
        let (water_molecule, water_centre) = water_model()?;
        let methane = VseprSchematicView3::from_centre(&methane_centre);
        let water = VseprSchematicView3::from_centre(&water_centre);

        // Keep source molecules alive through construction to make provenance
        // explicit; the schematic adapter itself stores stable source IDs.
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        append_view(&mut vertices, &mut indices, &methane, -3.0)?;
        append_view(&mut vertices, &mut indices, &water, 3.0)?;
        let mesh = TriangleMesh3::new(vertices, indices)?;

        let _source_models = (methane_molecule, water_molecule);
        Ok(Self { mesh, methane, water })
    }
}

fn molecule_point() -> Result<MolecularPoint3, Box<dyn std::error::Error>> {
    Ok(MolecularPoint3::new(0.0, 0.0, 0.0)?)
}

fn methane_model() -> Result<(Molecule, VseprCentre), Box<dyn std::error::Error>> {
    let atoms = vec![
        Atom::new(AtomId::new(1), Element::CARBON, molecule_point()?),
        Atom::new(AtomId::new(2), Element::HYDROGEN, molecule_point()?),
        Atom::new(AtomId::new(3), Element::HYDROGEN, molecule_point()?),
        Atom::new(AtomId::new(4), Element::HYDROGEN, molecule_point()?),
        Atom::new(AtomId::new(5), Element::HYDROGEN, molecule_point()?),
    ];
    let bonds = vec![
        Bond::new(BondId::new(10), AtomId::new(1), AtomId::new(2), BondOrder::Single),
        Bond::new(BondId::new(11), AtomId::new(1), AtomId::new(3), BondOrder::Single),
        Bond::new(BondId::new(12), AtomId::new(1), AtomId::new(4), BondOrder::Single),
        Bond::new(BondId::new(13), AtomId::new(1), AtomId::new(5), BondOrder::Single),
    ];
    let molecule = Molecule::new(atoms, bonds)?;
    let centre = VseprCentre::new(
        &molecule,
        AtomId::new(1),
        vec![
            ElectronDomain::bonding_pair(ElectronDomainId::new(100), BondId::new(10), AtomId::new(2)),
            ElectronDomain::bonding_pair(ElectronDomainId::new(101), BondId::new(11), AtomId::new(3)),
            ElectronDomain::bonding_pair(ElectronDomainId::new(102), BondId::new(12), AtomId::new(4)),
            ElectronDomain::bonding_pair(ElectronDomainId::new(103), BondId::new(13), AtomId::new(5)),
        ],
        ElectronDomainArrangement::Tetrahedral,
        MolecularShape::Tetrahedral,
    )?;
    Ok((molecule, centre))
}

fn water_model() -> Result<(Molecule, VseprCentre), Box<dyn std::error::Error>> {
    let atoms = vec![
        Atom::new(AtomId::new(20), Element::OXYGEN, molecule_point()?),
        Atom::new(AtomId::new(21), Element::HYDROGEN, molecule_point()?),
        Atom::new(AtomId::new(22), Element::HYDROGEN, molecule_point()?),
    ];
    let bonds = vec![
        Bond::new(BondId::new(30), AtomId::new(20), AtomId::new(21), BondOrder::Single),
        Bond::new(BondId::new(31), AtomId::new(20), AtomId::new(22), BondOrder::Single),
    ];
    let molecule = Molecule::new(atoms, bonds)?;
    let centre = VseprCentre::new(
        &molecule,
        AtomId::new(20),
        vec![
            ElectronDomain::bonding_pair(ElectronDomainId::new(200), BondId::new(30), AtomId::new(21)),
            ElectronDomain::bonding_pair(ElectronDomainId::new(201), BondId::new(31), AtomId::new(22)),
            ElectronDomain::lone_pair(ElectronDomainId::new(202)),
            ElectronDomain::lone_pair(ElectronDomainId::new(203)),
        ],
        ElectronDomainArrangement::Tetrahedral,
        MolecularShape::Bent,
    )?;
    Ok((molecule, centre))
}

fn append_view(
    vertices: &mut Vec<WorldPoint3>,
    indices: &mut Vec<u32>,
    view: &VseprSchematicView3,
    x_offset: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    append_octahedron(vertices, indices, [x_offset, 0.0, 0.0], CENTRAL_MARKER_RADIUS)?;
    for domain in view.domains() {
        let [x, y, z] = domain.anchor().to_array();
        let center = [
            x_offset + x * DOMAIN_DISTANCE,
            y * DOMAIN_DISTANCE,
            z * DOMAIN_DISTANCE,
        ];
        match domain.kind() {
            svf_core::vsepr::ElectronDomainKind::BondingPair { .. } => {
                append_octahedron(vertices, indices, center, BONDING_MARKER_RADIUS)?;
            }
            svf_core::vsepr::ElectronDomainKind::LonePair => {
                append_tetrahedron(vertices, indices, center, LONE_PAIR_MARKER_RADIUS)?;
            }
        }
    }
    Ok(())
}

fn push_point(vertices: &mut Vec<WorldPoint3>, center: [f64; 3], offset: [f64; 3]) {
    vertices.push(WorldPoint3::new(
        center[0] + offset[0],
        center[1] + offset[1],
        center[2] + offset[2],
    ));
}

fn append_octahedron(
    vertices: &mut Vec<WorldPoint3>,
    indices: &mut Vec<u32>,
    center: [f64; 3],
    radius: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let base = u32::try_from(vertices.len())?;
    for offset in [
        [radius, 0.0, 0.0], [-radius, 0.0, 0.0],
        [0.0, radius, 0.0], [0.0, -radius, 0.0],
        [0.0, 0.0, radius], [0.0, 0.0, -radius],
    ] {
        push_point(vertices, center, offset);
    }
    for triangle in [
        [0, 2, 4], [2, 1, 4], [1, 3, 4], [3, 0, 4],
        [2, 0, 5], [1, 2, 5], [3, 1, 5], [0, 3, 5],
    ] {
        indices.extend(triangle.map(|index| base + index));
    }
    Ok(())
}

fn append_tetrahedron(
    vertices: &mut Vec<WorldPoint3>,
    indices: &mut Vec<u32>,
    center: [f64; 3],
    radius: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let base = u32::try_from(vertices.len())?;
    let s = radius / 3.0_f64.sqrt();
    for offset in [
        [s, s, s], [s, -s, -s], [-s, s, -s], [-s, -s, s],
    ] {
        push_point(vertices, center, offset);
    }
    for triangle in [[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]] {
        indices.extend(triangle.map(|index| base + index));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use svf_core::vsepr::{ElectronDomainKind, MolecularShape};

    #[test]
    fn builds_independent_methane_and_water_models() {
        let demo = VseprDemo::build().unwrap();
        assert_eq!(demo.methane.domains().len(), 4);
        assert_eq!(demo.water.domains().len(), 4);
        assert_eq!(demo.methane.molecular_shape(), MolecularShape::Tetrahedral);
        assert_eq!(demo.water.molecular_shape(), MolecularShape::Bent);
        assert!(demo.methane.domains().iter().all(|domain| matches!(
            domain.kind(), ElectronDomainKind::BondingPair { .. }
        )));
        assert_eq!(
            demo.water.domains().iter().filter(|domain| matches!(
                domain.kind(), ElectronDomainKind::BondingPair { .. }
            )).count(),
            2
        );
        assert_eq!(
            demo.water.domains().iter().filter(|domain| {
                domain.kind() == ElectronDomainKind::LonePair
            }).count(),
            2
        );
    }

    #[test]
    fn sample_mesh_contains_separate_marker_geometry_without_domain_spokes() {
        let demo = VseprDemo::build().unwrap();
        // Each molecule: one central octahedron, four bonding/lone markers.
        // Methane has five octahedra; water has three octahedra and two
        // tetrahedra. Marker geometry is unconnected between domain centres.
        assert_eq!(demo.mesh.vertex_count(), 5 * 6 + 3 * 6 + 2 * 4);
        assert_eq!(demo.mesh.triangle_count(), 5 * 8 + 3 * 8 + 2 * 4);
        assert_eq!(demo.mesh.indices().len(), demo.mesh.triangle_count() * 3);
        assert!(demo.mesh.vertices().iter().all(|point| point.to_array().iter().all(|v| v.is_finite())));
    }
}
