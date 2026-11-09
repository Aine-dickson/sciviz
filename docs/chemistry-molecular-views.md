# Chemistry molecular views — experimental design contract

**Status:** proposed design for review  
**Branch:** `e2-chemistry-molecular-views-design`  
**Prerequisite:** E1 chemistry composition core (PR #33, merged)

## 1. Purpose

Turn the domain-defined chemistry structures introduced in E1 into scientifically meaningful, inspectable visual representations. The first experiment is a graphics-independent molecular representation adapter for water (H₂O) and methane (CH₄).

This is not yet a renderer integration, molecule editor, general scene graph, or simulation system. It tests whether semantic atoms and bonds can produce useful visual primitives while preserving scientific identity and source provenance.

## 2. Educational use case

The representation must support:

- **Teacher-led demonstration:** present a molecule and identify its constituent atoms and bonds.
- **Guided exploration:** allow learners to select an atom or bond and inspect its identity, element, bond order, and relationships.
- **Independent investigation (later):** compare structures and representations without changing the underlying scientific model.

A visual should help explain composition and connectivity. It must not imply that a ball-and-stick drawing is a literal picture of molecular scale or electron behaviour.

## 3. Core design principles

1. **Scientific identity remains authoritative.** The source `Molecule`, `AtomId`, `BondId`, element definitions, and bond orders are never replaced by display geometry.
2. **Representation is derived.** The same molecule may later produce ball-and-stick, space-filling, wireframe, or other views without duplicating or mutating chemistry data.
3. **Units remain explicit.** Source molecular coordinates and all initial representation dimensions remain in ångströms (Å). Conversion to SVF world units is a later, explicit presentation transform.
4. **Core remains graphics-independent.** No renderer, GPU, windowing, mesh tessellation, or material types enter `svf-core`.
5. **Provenance is mandatory.** Every atom glyph refers to exactly one `AtomId`; every bond glyph refers to exactly one `BondId`. Derived lanes or segments must retain that bond identity.
6. **No invented chemistry.** Display radii and bond spacing are visual conventions, not claims about exact atomic boundaries or electron distributions. Their policy and units must be documented.
7. **Domain examples lead.** Implement only what water and methane require. Extract reusable mechanisms only after additional scientific structures demonstrate the need.

## 4. Initial representation: ball-and-stick

The first representation is a conventional ball-and-stick view because it exposes both composition and connectivity clearly.

### 4.1 Atom glyph

A graphics-independent atom glyph contains:

- source `AtomId`;
- element identity (or enough immutable element data to resolve it from the source molecule);
- molecular-space center (`MolecularPoint3`);
- positive finite display radius in Å.

The display radius comes from an explicit representation style, not from mutating the element or atom. Initial styles may provide domain-defined values for Hydrogen, Carbon, and Oxygen. Values are visual parameters and must not be labelled as measured atomic radii.

### 4.2 Bond glyph

A bond glyph contains:

- source `BondId`;
- source endpoint atom IDs;
- semantic `BondOrder`;
- one or more molecular-space cylinder/axis primitives with finite endpoints and positive finite display radius.

Single bonds produce one axis. Double and triple bonds produce two or three parallel axes respectively, with a documented separation policy. Parallel offsets are derived deterministically from the bond direction and a stable perpendicular basis. A bond's axes are display geometry only; they do not create additional chemical bonds.

The initial adapter may keep axes between atom centers. If trimmed-to-surface endpoints are needed for visual quality, that should be a separately specified geometry policy with tests. It must not modify atom positions.

### 4.3 Output container

A `MolecularView3` (or equivalent) owns ordered atom and bond glyph collections and representation metadata. It must preserve source order and deterministic output. It must not own or mutate the source molecule.

The API should expose read-only slices and enough metadata to identify the representation style and units.

## 5. API boundary proposal

Add a graphics-independent module under `svf_core::chemistry` or a narrowly scoped sibling module.

Conceptual pipeline:

```text
Molecule
  + MolecularRepresentationStyle
  -> build_molecular_view(...)
  -> MolecularView3
       atom glyphs  -> AtomId provenance
       bond glyphs  -> BondId + endpoint AtomId provenance
```

The style is explicit input. It defines atom display radii, bond display radius, and multiple-bond separation. The adapter validates all style values before producing output.

Do not add a generic plugin/registry framework for representations. A small domain-defined style type is sufficient for the experiment.

## 6. Validation and error handling

Reject invalid representation input rather than emitting corrupt or ambiguous primitives:

- missing style for an element present in the molecule;
- non-finite, zero, or negative display radii/separation;
- non-finite derived coordinates;
- coincident endpoints for a bonded pair (no defined bond direction);
- overflow while deriving offsets or endpoints.

Empty molecules and isolated atoms should remain representable. An empty molecule produces an empty view. Isolated atoms produce atom glyphs without bond glyphs.

All output ordering must be deterministic. Repeated conversion of the same molecule and style must produce equal output.

## 7. Tests

### Water (H₂O)

- 3 atom glyphs and 2 bond glyphs;
- each atom glyph retains the corresponding source `AtomId`;
- each bond glyph retains its `BondId`, endpoint IDs, and single-bond order;
- positions and display radii remain in Å;
- the bent molecular geometry is preserved.

### Methane (CH₄)

- 5 atom glyphs and 4 bond glyphs;
- central carbon and four hydrogen identities are preserved;
- tetrahedral arrangement is preserved;
- all four single bonds map to the correct endpoint IDs.

### Multiple bond primitive policy

Use a small synthetic molecule with double and triple bonds to verify lane count, deterministic offsets, finite coordinates, and common source `BondId` provenance. This is a representation test, not a claim that the initial element set covers a complete chemistry model.

### Invalid input and determinism

Test missing element style, invalid style values, coincident bonded atom positions, finite-output guarantees, empty molecule, isolated atom, stable ordering, and repeated conversion.

## 8. Explicit non-goals

- GPU or renderer integration;
- Explorer controls, camera presets, labels, legends, or selection UI;
- file import/export and chemical file formats;
- periodic table expansion;
- valence, formal charge, aromaticity, stereochemistry, partial charges, or electron orbitals;
- molecular dynamics, reactions, or energy calculations;
- space-filling or electron-cloud representations;
- generic user-defined component authoring;
- biological structures.

## 9. Exit criteria

The experiment is complete when:

1. A graphics-independent ball-and-stick adapter produces validated views for water and methane.
2. Atom and bond provenance is retained exactly.
3. Multiple bond orders produce deterministic display primitives without changing semantic connectivity.
4. Invalid styles and degenerate bonded coordinates fail explicitly.
5. Focused tests, the complete workspace tests, and `cargo check --workspace` pass on Windows.
6. The resulting API is reviewed before any renderer or Explorer integration is proposed.

After this experiment, assess the actual generated representations and decide whether a renderer adapter is useful for the user's teaching workflow. Do not automatically expand into more representation styles or unrelated chemistry features.
