# Experimental study: compositional scientific structures

Status: proposed research/design note; not an implementation contract.

## Purpose

Explore how SVF can represent, inspect, and visualize scientific structures assembled from reusable, domain-defined components. Chemistry is the first experimental domain. The work is driven by the user's educational use case: learners should be able to see and meaningfully interact with scientific structures instead of having to imagine them from abstract descriptions.

This study is deliberately not a general-purpose scene editor or a commitment to a generic component framework. Domain-defined scientific examples should reveal which reusable mechanisms are actually needed.

## Why this experiment

Prismal primarily describes models, simulates their behaviour, and exposes observations through a chosen presentation layer. That is a useful fit for dynamic mathematical and physical models, but it does not by itself provide a natural way to author arbitrary structured scientific objects such as an atom, molecule, cell, or organelle assembly.

SVF already has graphics-independent geometry, scientific datasets, fields, topology, provenance, and inspection. It does not yet have a first-class model for semantic scientific components and assemblies. Reusing triangle meshes alone would lose important identity and relationship information.

## Experimental sequence

Use chemistry to discover and test the domain model before extracting generic construction abstractions.

1. Define reusable domain concepts for atoms and bonds.
2. Create distinct atom instances and compose them into a molecule.
3. Preserve identity, scientific properties, connectivity, and spatial arrangement independently of rendering.
4. Produce a first educational representation of the molecule and inspect its constituent atoms and bonds.
5. Evaluate the model against a second molecule with different composition and geometry.
6. Only after those examples work, identify common mechanisms that could support user-defined components and another domain such as biological cells.

The first representative examples should be water (H2O) and methane (CH4). They are small enough to inspect manually but exercise atom reuse, different composition, bond connectivity, and distinct molecular geometries. This is a representation experiment, not a claim to simulate chemical behaviour.

## Candidate domain model

These are concepts to validate, not committed Rust API names or final schemas.

### Element definition

A reusable definition for a chemical element, distinct from any particular atom occurrence. At minimum it should carry stable scientific identity (atomic number and symbol). Additional properties should be added only when a real representation or learning task needs them.

### Atom instance

A particular occurrence of an element within a structure. It should have a stable identity within its assembly, refer to its element definition, and have its own spatial position and instance-level properties where appropriate. Two hydrogen atoms in one molecule share an element definition but remain distinct instances.

### Bond relationship

A scientifically meaningful relationship between two atom instances. It should preserve endpoint identity and bond semantics (initially at least single, double, and triple order if required by the chosen examples). A bond is not merely a rendered cylinder or line: its visual form is derived from its semantic relationship.

### Molecule assembly

A composition containing atom instances and bond relationships. It should retain the distinction between membership/containment and chemical connectivity. The assembly should be inspectable as a whole and by its constituent parts.

### Presentation representation

A derived view of the scientific structure. The initial candidate is a ball-and-stick representation, with atom and bond geometry generated from the domain model. A later space-filling representation may use the same underlying structure. Presentation changes must not mutate element identity, atom identity, bond order, or molecular connectivity.

## Architectural principles to test

- **Scientific identity is not geometry.** A mesh or primitive is not the authoritative definition of an atom or bond.
- **Definitions are not instances.** Reusable element/component definitions are shared; each occurrence has its own identity and state.
- **Composition is not connectivity.** An assembly contains components; domain relationships connect selected components and have their own semantics.
- **Presentation is derived.** One scientific assembly may have multiple visual representations.
- **Inspection resolves to science.** Selecting a rendered atom or bond should identify the corresponding scientific instance or relationship and expose its meaningful properties.
- **Nested composition remains possible.** A molecule can be an assembly; the same compositional mechanism may later support organelles within cells, cells within tissues, and other hierarchical structures. Avoid designing all those domains before the chemistry experiment validates the need.
- **Core remains graphics-independent.** Scientific definitions, instances, relationships, and composition belong in the core model; renderer-specific buffers and materials do not.
- **Do not imply false physical models.** An educational atomic representation must label its visual conventions. In particular, avoid presenting electrons as literal planets following fixed classical orbits.

## Initial scope

In scope for the experiment:
- Domain-defined element and atom concepts.
- Distinct atom instances with stable identity.
- Bond relationships with validated endpoints.
- Molecule composition and connectivity validation.
- Spatial arrangement suitable for a basic 3D molecular representation.
- A derived, inspectable ball-and-stick visualization.
- Tests demonstrating that scientific structure remains intact independently of the visual representation.

Explicitly out of scope initially:
- Quantum-mechanical electron orbitals or electron-density calculations.
- Chemical reaction simulation, molecular dynamics, or force fields.
- A complete periodic-table database.
- A generic user-facing component editor.
- Arbitrary scripting or user-defined component types.
- Full chemistry standards/import formats.
- Biological structures and their specialized semantics.
- Photorealistic rendering, advanced materials, or GPU-side scientific state.

## Questions the experiment must answer

1. What is the minimum stable identity scheme for definitions, atom instances, and relationships?
2. Which properties belong to an element definition, an atom instance, a bond, or the molecule?
3. How should coordinates and units be represented without confusing display scale with scientific dimensions?
4. Which chemical constraints can be validated reliably at this level, and which require more advanced chemistry knowledge?
5. How should a visual pick resolve to an atom or bond and then to its scientific data?
6. What is the smallest shared composition mechanism that could later support a biological structure without making chemistry-specific assumptions?
7. Can a second representation be generated without changing the underlying molecular model?

## Exit criteria

The experiment is useful if:
- Water and methane can be represented using shared element definitions and distinct atom instances.
- Bond relationships connect the intended instances and invalid references are rejected.
- The molecule retains scientific identity and connectivity independently of generated geometry.
- At least one visual representation can be generated and inspected by scientific entity.
- A change in presentation does not alter the underlying molecular structure.
- The implementation reveals which abstractions are genuinely common and which should remain chemistry-specific.

## Workflow

This document is a discussion and design baseline. Review and revise it before implementation. Do not create a generic component framework or select subsequent milestones automatically. The next implementation increment should be narrowly scoped from the accepted experiment, and every later abstraction should be justified by the working chemistry and biological examples.
