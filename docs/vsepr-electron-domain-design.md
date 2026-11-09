# VSEPR electron-domain model — experimental design contract

**Status:** proposed design for review; no implementation authorized by this document.  
**Prerequisites:** E1 chemistry composition core, E2 molecular views, and E3 upper-secondary VSEPR learning case.  
**Target lesson:** Senior Five, CH₄ versus H₂O.

## 1. Purpose

Introduce the smallest graphics-independent scientific model needed to explain why methane is tetrahedral while water is bent.

The central missing concept is the electron domain. E1 models atoms and bonds; E2 derives ball-and-stick glyphs from them. Neither can represent a lone pair, distinguish electron-domain geometry from molecular shape, or explain the water bond-angle compression.

This experiment must add electron-domain semantics without treating electron domains as atoms, bonds, literal particles, or arbitrary decorations.

## 2. Recommended architecture

Add a narrowly scoped chemistry VSEPR model in a dedicated module, provisionally `svf_core::chemistry::vsepr` (or a sibling module if the existing module layout makes that cleaner).

Keep three layers distinct:

1. **Molecular composition** — existing `Molecule`, `AtomId`, `BondId`, elements, bonds and molecular coordinates.
2. **VSEPR scientific model** — electron-domain identities and classifications around a specified central atom, plus the explicitly supported arrangement/shape classification.
3. **Derived educational representation** — graphics-independent schematic glyphs for explaining the model. These must retain provenance to the VSEPR domain and, for bonding domains, to the source bond and atoms.

Do not put VSEPR fields directly onto `Atom` or `Bond`. A molecule can be inspected structurally without having a VSEPR teaching model attached. Do not introduce a generic component, annotation, scene-graph, or plugin framework.

## 3. Core semantic concepts

### 3.1 Electron-domain identity

Introduce a small stable `ElectronDomainId` newtype. IDs are supplied explicitly, following E1's `AtomId` and `BondId` policy. IDs are unique within one central-atom VSEPR model.

An electron domain represents one region of electron density around a central atom for the purpose of the selected VSEPR model. It does not represent an individual electron or a measured spatial boundary.

### 3.2 Domain classification

Use a closed, explicit classification for this first case:

- **Bonding pair domain:** associated with exactly one bond incident to the central atom. It retains the source `BondId` and the other endpoint's `AtomId`.
- **Lone pair domain:** associated with the central atom and has no bond or second atom endpoint.

A domain must not be both types. A lone pair must never be encoded as a fake atom or bond.

For the initial CH₄/H₂O scope, each bond from the central atom contributes one bonding domain. A double or triple bond is still one VSEPR domain for the bonded neighbour; bond order does not multiply the domain count. The initial examples use only single bonds, so multiple-bond behaviour is an invariant to document, not a reason to broaden the first lesson implementation.

### 3.3 Central-atom model

A VSEPR centre/model identifies:

- one source central `AtomId`;
- an ordered list of uniquely identified electron domains;
- the supported electron-domain arrangement;
- the molecular-shape classification for the attached-atom arrangement.

For this experiment, support only the four-domain tetrahedral electron-domain arrangement and the two molecular shapes required by the examples:

- four bonding domains, zero lone pairs → tetrahedral molecular shape (CH₄);
- two bonding domains, two lone pairs → bent molecular shape (H₂O).

Do not add trigonal planar, trigonal pyramidal, linear, trigonal bipyramidal, octahedral, expanded-octet, ionic, or general molecular/ionic geometry support in this increment. Expand only when a selected teaching case requires it.

The arrangement and shape are explicit VSEPR model semantics, not inferred from arbitrary molecular coordinates. The constructor validates the two supported domain patterns so contradictory combinations cannot be assembled.

## 4. Validation contract

Construct a VSEPR centre against a source `Molecule`, not in isolation. The validated constructor must reject:

- a central atom ID absent from the molecule;
- a domain ID duplicated within the centre;
- a bonding domain whose bond does not exist;
- a referenced bond that is not incident to the selected central atom;
- a claimed neighbouring atom that is not the other endpoint of that bond;
- multiple domains assigned to the same incident bond;
- an incident central-atom bond omitted from the domain list;
- an invalid domain count or unsupported bonding/lone-pair pattern;
- an unsupported or contradictory electron-domain arrangement / molecular-shape combination.

For the bounded first case, the domain list must account for every bond incident to the selected central atom exactly once. Do not require every bond elsewhere in the molecule to be part of this centre.

Preserve caller order for deterministic output. Return structured errors and no partially valid model.

Do not attempt to validate general Lewis structures, formal charges, resonance, octet rules, valence, chemical stability, or whether a proposed molecule is chemically possible. Those require chemistry semantics not present in E1.

## 5. Two geometries, two meanings

A critical design constraint is to avoid conflating the geometry of the electron-domain teaching model with the molecular atom coordinates.

### 5.1 Molecular geometry

The existing molecule coordinates remain the source for the atom/bond representation. For the water lesson, the molecular-shape view should show a bent H–O–H arrangement. For methane, it should show the tetrahedral arrangement.

The current E1 coordinates do not carry an explicit statement about whether they are idealized teaching coordinates, experimentally sourced geometry, or arbitrary input. Therefore this increment must not silently claim that an angle computed from arbitrary coordinates is an experimental or canonical value.

### 5.2 Electron-domain arrangement

The VSEPR model records the four-domain tetrahedral arrangement. A later adapter may derive a clearly labelled schematic domain diagram from this classification.

For the initial educational representation, use a separate abstract arrangement view rather than forcing lone-pair glyphs into the molecule's existing coordinate system. Its four domain positions may use an ideal tetrahedral schematic layout, with bonding/lone-pair classifications and source links. Label this view as an **idealized electron-domain arrangement**.

Do not imply that the schematic lone-pair glyph positions are measured electron-density maxima or experimentally determined lone-pair directions. Do not present the schematic's ideal 109.5° domain separation as the measured H–O–H bond angle.

The molecule view and electron-domain view must be independently identifiable and switchable. Their different geometric meanings must be visible in labels and explanatory text, not left for learners to infer.

### 5.3 Bond-angle information

The lesson may state the approximate reference angles (CH₄ ≈ 109.5°, H₂O ≈ 104.5°) as curriculum-level reference information. This first VSEPR semantic model should not manufacture a measured angle field.

A future measured/derived angle API must specify:
- which three atoms define the angle;
- the coordinate source and geometry status;
- units (degrees or radians, explicit);
- whether the value is computed from molecular coordinates or supplied as a sourced reference;
- how rounding and approximation are communicated.

For the first implementation, either compute a displayed angle from explicitly documented teaching coordinates, or keep the approximate angle in the lesson/reference layer. Do not mix these sources or label a schematic domain angle as the molecular bond angle.

## 6. Derived educational representation

A graphics-independent adapter may derive a small VSEPR domain view from a validated centre.

Each domain glyph must retain:
- source `ElectronDomainId`;
- source central `AtomId`;
- domain classification;
- for bonding domains, source `BondId` and neighbouring `AtomId`;
- its position in the explicitly labelled schematic arrangement.

The output must preserve domain order and deterministic identity. The representation style may change glyph appearance, but cannot change domain classification or chemistry.

Do not put mesh, material, camera, renderer, GPU, or windowing types into the core model. Do not add a general-purpose rendering or annotation framework.

A domain glyph is a schematic visual encoding of a model concept. It must not look like a second atom or like a physical electron ball. Bonding and lone-pair domains should be visually distinguishable and have textual/inspection labels so colour is never the only cue.

## 7. Required educational interactions

The model and derived representation must make these interactions possible later, without implementing Explorer UI in this increment:

1. Selecting a domain resolves to its exact `ElectronDomainId` and classification.
2. Selecting a bonding domain also resolves to its source `BondId`, central atom and neighbouring atom.
3. Selecting a lone-pair domain resolves to the central atom and has no bond/other-atom identity.
4. Selecting an atom or bond in the molecular view continues to resolve through existing `AtomId`/`BondId`.
5. Switching between molecular geometry and electron-domain arrangement changes presentation only; it does not mutate the molecule or VSEPR model.
6. Hiding/fading lone-pair glyphs is a presentation change only; the lone-pair domains remain in the model and remain inspectable.
7. Comparison of methane and water preserves independent model identities and does not merge their atoms/domains.

The first core increment should prove semantic resolution and deterministic representation data, not build an interactive UI.

## 8. Required examples and tests

### Methane (CH₄)

- one central carbon;
- four incident C–H bonds;
- four unique bonding domains, each linked to exactly one source bond and hydrogen;
- zero lone-pair domains;
- tetrahedral electron-domain arrangement;
- tetrahedral molecular-shape classification;
- deterministic domain order and provenance.

### Water (H₂O)

- one central oxygen;
- two incident O–H bonds;
- two unique bonding domains linked to their exact source bonds/hydrogens;
- two unique lone-pair domains linked only to the central oxygen;
- tetrahedral electron-domain arrangement;
- bent molecular-shape classification;
- deterministic domain order and provenance.

### Validation and representation

Test missing central atom, missing/foreign/non-incident bond, wrong neighbouring atom, duplicate domain IDs, duplicate bond-domain mapping, omitted incident bond, invalid domain pattern, contradictory arrangement/shape, and stable output ordering.

If a schematic adapter is included in the same increment, test that every output glyph preserves domain provenance and that the idealized tetrahedral arrangement is clearly distinct from molecule coordinates. Test that switching/filtering presentation never deletes or mutates semantic domains.

## 9. Scope and non-goals

This experiment is limited to:
- the four-domain tetrahedral arrangement;
- methane's four bonding domains;
- water's two bonding and two lone-pair domains;
- semantic identity, validation, and a graphics-independent educational representation if justified.

Explicitly defer:
- a general-purpose VSEPR prediction engine;
- automatic Lewis-structure interpretation or domain inference;
- arbitrary molecule authoring;
- additional domain arrangements or molecular shapes;
- ionic geometry;
- dative covalency, polarity, hybridization, resonance, formal charge, molecular orbitals;
- measured electron-density fields or quantum-chemical calculations;
- simulation or Prismal integration;
- renderer/Explorer controls, lesson authoring or Mava integration;
- generic scene graph, generic component framework, or plugin registry.

## 10. Proposed implementation sequence

Keep the next code work small and reviewable:

1. Implement and test the domain identities, domain classifications, and validated VSEPR centre in `svf-core`.
2. Validate methane and water against the exact patterns above, including all source provenance and rejection cases.
3. Only after reviewing the semantic model, add a graphics-independent schematic domain-view adapter if the model can support it without inventing physical coordinates.
4. Evaluate the resulting data against the lesson: can a learner distinguish the four-domain arrangement from the atom-only molecular shape and explain why water is bent?
5. Do not integrate renderer, Explorer, Prismal, or Mava until the core model has been reviewed and the user confirms the next educational need.

## 11. Exit criteria

The design is ready for implementation when the user accepts:
- a separate VSEPR model layer over existing chemistry composition;
- explicit electron-domain identities and bonding/lone-pair classifications;
- strict provenance to source bonds/atoms;
- only the methane/water four-domain patterns initially;
- explicit separation between molecular coordinates and an idealized electron-domain schematic;
- no hardcoded measured bond-angle field in this first semantic model;
- no automatic general VSEPR engine or UI integration.

The implementation must remain driven by this learning case, not by feature parity with chemistry software.
