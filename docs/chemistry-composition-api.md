# Chemistry composition: initial core API

Status: design decisions resolved; ready for implementation planning. This document authorizes the bounded core-model implementation described below, not a renderer integration or generic component framework.

Related study: [Experimental chemistry composition](experimental-chemistry-composition.md).

## Goal

Define the smallest graphics-independent `svf-core` model that can represent reusable chemical element definitions, distinct atom instances, bond relationships, and molecule assemblies. Validate the model with water (H2O) and methane (CH4).

The API preserves scientific identity and connectivity independently of generated geometry. This is a structural representation experiment, not a chemistry simulator.

## Module and scope

Use the domain-specific module `svf_core::chemistry`. Do not introduce a generic component/assembly framework in this increment.

Implementation is limited to `svf-core`. Existing dataset, geometry, renderer, and Explorer behavior must remain unchanged.

## Resolved API decisions

### Element: immutable value type with domain-defined constants

Use an immutable `Element` struct with private fields for atomic number, symbol, and name. Expose predefined Hydrogen, Carbon, and Oxygen values as associated constants (for example, `Element::HYDROGEN`). Provide read-only accessors.

This is preferred over an enum: the type remains open to additional element definitions without changing its fundamental representation, while controlled construction prevents invalid definitions. Do not add a complete periodic table or speculative properties such as valence, electronegativity, or atomic radius.

### Atom identity and instances

Each atom occurrence has an `AtomId` distinct from its element definition. Multiple atoms may share the same `Element` value.

Use small ID newtypes and explicit IDs supplied when constructing atoms and bonds. The validated molecule constructor checks uniqueness. Caller-supplied IDs keep identity stable and make provenance predictable across adapters and serialized representations; do not add an ID-generating builder in this first API.

Conceptual shape:

```rust
pub struct AtomId(u32);

pub struct Atom {
    id: AtomId,
    element: Element,
    position: MolecularPoint3,
}
```

Keep fields private and expose read-only accessors.

### MolecularPoint3: dedicated ångström type

Define a dedicated `MolecularPoint3` with f64 coordinates in ångströms (Å), with a fallible constructor that rejects non-finite values. This makes units explicit at the API boundary and avoids confusing molecular coordinates with generic scene/world coordinates.

Conversion to visualization/world coordinates belongs in a later representation adapter, not in the chemistry model.

### BondOrder and Bond

A bond is a semantic relationship between two atom instances, not a line or cylinder.

```rust
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
```

Use a `BondId` newtype and explicit IDs. Keep fields private with read-only accessors. Aromatic, coordinate, partial, and other bond models are deferred.

### Molecule: validated constructor, no builder initially

A molecule owns its atom instances and bond relationships. Construct it through a validated `Molecule::new(atoms, bonds) -> Result<Molecule, ChemistryError>`.

The constructor validates the complete assembly atomically and returns no partial molecule on failure. It checks unique atom and bond IDs, endpoint existence, no self-bonds, and no duplicate undirected atom pairs. Preserve input atom and bond order deterministically.

A builder is deliberately deferred: explicit IDs plus a single validated constructor provide a smaller API and avoid introducing mutable intermediate state before examples demonstrate a need for it.

## Required invariants

1. Element definitions have valid, immutable identity.
2. Atom IDs are unique within a molecule.
3. Bond IDs are unique within a molecule.
4. Atom positions are finite f64 coordinates in ångströms.
5. Every bond endpoint resolves to an atom in the same molecule.
6. A bond cannot connect an atom to itself.
7. Duplicate undirected atom pairs are rejected; reversing endpoint order cannot bypass this check.
8. Atom and bond order is deterministic and preserved.
9. The model has no renderer/GPU types and does not depend on `svf-render`.
10. Geometry is derived from the molecule and cannot replace or mutate its scientific identity.

Do not validate valence or assert chemical stability in this increment. Bond order and connectivity alone are insufficient for general chemical validity.

## Initial tests

### Element definitions
- Hydrogen, Carbon, and Oxygen expose correct atomic numbers, symbols, and names.
- Definitions are immutable and reusable across atom instances and molecules.

### Atom instances
- Multiple hydrogen instances can reference the same element definition.
- Those instances have distinct IDs and independent positions.
- Atom order is preserved.
- Non-finite molecular coordinates are rejected.

### Bond relationships
- Single, double, and triple bond orders are preserved.
- Bond endpoints resolve correctly.
- Unknown endpoints, self-bonds, duplicate IDs, and duplicate undirected pairs are rejected.
- Reversing endpoint order does not evade duplicate-pair validation.

### Molecule composition
- Water contains one oxygen, two distinct hydrogens, and two single bonds.
- Methane contains one carbon, four distinct hydrogens, and four single bonds.
- The same element definitions are reused across both molecules.
- Accessors can identify bonds incident to an atom without losing source IDs.
- Invalid construction returns structured errors and no partial molecule.

### Domain/presentation separation
- The chemistry module compiles and tests without renderer dependencies.
- A small adapter-level test or example may derive simple ball-and-stick primitives while leaving the original molecule unchanged, if this can be done without expanding the core-model scope.
- No mesh, material, colour, GPU buffer, or camera data is stored as authoritative chemistry state.

## Representation boundary

The intended later flow is:

`Molecule -> molecular representation adapter -> prepared geometry with source AtomId/BondId provenance -> renderer`

The adapter should eventually support more than one representation without changing the molecule. Ball-and-stick is the first candidate. Space-filling and other representations are deferred.

Selection and inspection should ultimately resolve rendered parts back to atom or bond identity, but Explorer UI integration is not required to validate this initial data model.

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

## Acceptance criteria

The implementation adds the domain model and focused tests in `svf-core`, demonstrates water and methane composition, validates all invariants above, and introduces no renderer or Explorer changes. It must not claim general chemical validity beyond the explicitly implemented structural rules.
