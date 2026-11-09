# Chemistry composition: initial core API proposal

Status: proposed API/design contract for review. No implementation is authorized by this document alone.

Related study: [Experimental chemistry composition](experimental-chemistry-composition.md).

## Goal

Define the smallest graphics-independent svf-core model that can represent reusable chemical element definitions, distinct atom instances, bond relationships, and molecule assemblies. Validate the model with water (H2O) and methane (CH4).

The API should preserve scientific identity and connectivity independently of generated geometry. This is a structural representation experiment, not a chemistry simulator.

## Proposed module

svf_core::chemistry

The module is domain-specific by design. Do not introduce a generic component/assembly framework in this increment.

## Proposed model

### Element

A reusable, immutable domain definition. For the initial experiment, provide definitions for Hydrogen, Carbon, and Oxygen.

Candidate public type:

\`\`\`rust
pub struct Element {
    atomic_number: u8,
    symbol: &'static str,
    name: &'static str,
}
\`\`\`

The initial API may expose constants or a small enum-backed lookup. It should not require users to manually construct invalid element definitions. Do not add a complete periodic table or speculative properties such as valence, electronegativity, or atomic radius until a concrete use case requires them.

### AtomId and Atom

Each atom occurrence has an identity distinct from its element definition.

\`\`\`rust
pub struct AtomId(u32);

pub struct Atom {
    id: AtomId,
    element: Element,
    position: MolecularPoint3,
}
\`\`\`

The same Element definition may be used by many atoms. Atom IDs are unique within a molecule and stable for the lifetime of that immutable molecule value. IDs should be assigned deterministically by the molecule builder or explicitly supplied if the API has a strong reason to support caller-provided IDs.

### MolecularPoint3

Molecular coordinates need an explicit unit convention. Do not silently reuse generic world coordinates as though their units were physical molecular units.

Proposed initial convention: coordinates are expressed in ångströms (Å). A small domain-specific point type or clearly named coordinate wrapper should make that convention discoverable and prevent confusion with renderer world-space units. It should preserve f64 values and reject non-finite coordinates.

The exact conversion from molecular coordinates to visualization/world coordinates belongs in a later representation adapter, not in the chemistry model.

### BondOrder and Bond

A bond is a semantic relationship between two atom instances, not a line or cylinder.

\`\`\`rust
pub enum BondOrder {
    Single,
    Double,
    Triple,
}

pub struct Bond {
    id: BondId,
    atom_a: AtomId,
    atom_b: AtomId,
    order: BondOrder,
}
\`\`\`

BondId is unique within the molecule. Endpoints must refer to existing atoms. A bond must not connect an atom to itself. The initial contract should reject duplicate undirected atom pairs rather than silently creating parallel duplicate bonds. Aromatic, coordinate, partial, and other bond models are deferred.

### Molecule

A molecule owns its atom instances and bond relationships.

\`\`\`rust
pub struct Molecule {
    atoms: Vec<Atom>,
    bonds: Vec<Bond>,
}
\`\`\`

Construction should go through a validating constructor or builder. The resulting molecule should expose read-only accessors and deterministic iteration. Atom and bond IDs must resolve to their corresponding entries. The model should not infer chemical validity from connectivity alone: valence, charge, stereochemistry, and chemical stability are outside the initial validation contract.

## Construction API

Prefer a small builder if it materially improves safe reference creation and error reporting. For example:

\`\`\`rust
let mut builder = MoleculeBuilder::new();

let oxygen = builder.add_atom(Element::OXYGEN, point(0.0, 0.0, 0.0))?;
let hydrogen_a = builder.add_atom(Element::HYDROGEN, point(...))?;
let hydrogen_b = builder.add_atom(Element::HYDROGEN, point(...))?;

builder.add_bond(oxygen, hydrogen_a, BondOrder::Single)?;
builder.add_bond(oxygen, hydrogen_b, BondOrder::Single)?;

let water = builder.build()?;
\`\`\`

This is illustrative rather than a fixed signature. The implementation should avoid redundant validation at every layer and return structured errors. If a builder introduces unnecessary complexity, a direct validated Molecule::new(atoms, bonds) is preferable.

## Required invariants

1. Element definitions have valid, immutable identity.
2. Atom IDs are unique within a molecule.
3. Bond IDs are unique within a molecule.
4. Atom positions are finite f64 coordinates in ångströms.
5. Every bond endpoint resolves to an atom in the same molecule.
6. A bond cannot connect an atom to itself.
7. Duplicate undirected atom pairs are rejected in this initial model.
8. Atom and bond order is deterministic and preserved.
9. The model does not contain renderer/GPU types or depend on svf-render.
10. Geometry generation is derived from the molecule and cannot replace or mutate its scientific identity.

Do not validate valence or assert that a molecule is chemically stable in this increment. The selected examples are intentionally simple, but general chemical validity requires richer concepts than bond order alone.

## Initial tests

### Element definitions
- Hydrogen, Carbon, and Oxygen expose correct atomic numbers, symbols, and names.
- Definitions are reusable and immutable.

### Atom instances
- Multiple hydrogen atom instances can reference the same element definition.
- Those instances have distinct IDs and independent positions.
- Atom order is preserved.
- Non-finite coordinates are rejected.

### Bond relationships
- Single, double, and triple bond orders are preserved.
- Bond endpoints resolve correctly.
- Unknown endpoints, self-bonds, duplicate IDs, and duplicate undirected pairs are rejected.
- Reversing endpoint order does not evade duplicate-pair validation.

### Molecule composition
- Water contains one oxygen, two distinct hydrogens, and two single bonds.
- Methane contains one carbon, four distinct hydrogens, and four single bonds.
- The same element definitions are reused across both molecules.
- Connectivity queries or accessors can identify bonds incident to a given atom without losing source IDs.
- Invalid construction returns structured errors and no partial molecule.

### Domain/presentation separation
- The chemistry module compiles and tests without renderer dependencies.
- A test or small adapter-level example can derive simple ball-and-stick primitives from a molecule while the original molecule remains unchanged.
- No mesh, material, colour, GPU buffer, or camera data is stored as authoritative chemistry state.

## Representation boundary

Do not implement the renderer integration in the first core-model change unless it is required to prove the boundary. The intended later flow is:

Molecule -> molecular representation adapter -> prepared geometry with source AtomId/BondId provenance -> renderer

The adapter should eventually support more than one representation without changing the molecule. Ball-and-stick is the first candidate. Space-filling and other representations are deferred.

Selection/inspection should ultimately resolve rendered parts back to atom or bond identity, but full Explorer UI integration is not required to validate this initial data model.

## Explicit non-goals

- Full periodic table.
- Electron shells/orbitals or quantum chemistry.
- Chemical reactions, molecular dynamics, force fields, or energy calculations.
- Valence/charge/stereochemistry validation.
- Molecular file format import/export.
- Biological component types.
- Generic user-defined components or generic scene graph.
- Materials, lighting, rendering, or Explorer UI in the core model.
- Automatic molecule generation or chemical structure inference.

## Review questions

1. Is a small immutable Element definition appropriate, or should the initial API use an Element enum?
2. Should molecular coordinates use a dedicated ångström point type, or a unit-explicit wrapper around the existing spatial type?
3. Is a builder useful for generating stable atom/bond IDs and safe bond references, or is a validated constructor simpler?
4. Are duplicate undirected bonds and self-bonds the right initial rejection rules?
5. Are water and methane sufficient to validate this first model before adding a second molecular representation?

## Acceptance criteria

The API design is ready for implementation when the identity model, coordinate unit convention, construction approach, validation rules, and test scope are agreed. Implementation should remain limited to svf-core and should not alter existing dataset, geometry, renderer, or Explorer behavior.
