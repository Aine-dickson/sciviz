# VSEPR schematic view: design contract

Status: design contract for an SVF-only scientific visualization experiment.

Related:
- docs/vsepr-electron-domain-design.md
- docs/upper-secondary-chemistry-vsepr-learning-case.md

## 1. Purpose and evaluation boundary

The current SVF chemistry work can represent atoms, bonds, electron domains,
and validated VSEPR classifications, but it cannot yet show those concepts
as a visual arrangement. This increment defines the smallest graphics-
independent representation needed to inspect the four-domain tetrahedral
VSEPR model for methane and water.

Evaluate SVF on its own. The implementation and sample must use only SVF
crates and the existing Explorer host. No Mava or Prismal API, dependency,
runtime, data format, or integration assumption is in scope.

This is a bounded scientific visualization experiment, not a general VSEPR
visualization engine or a complete lesson player.

## 2. Scientific distinction the view must preserve

The view represents **electron-domain arrangement**, not molecular geometry.

- Both CH₄ and H₂O have four electron domains around the central atom.
- The four domain directions are shown in an idealized tetrahedral
  arrangement.
- CH₄ has four bonding-pair domains and a tetrahedral molecular shape.
- H₂O has two bonding-pair domains and two lone-pair domains, while its
  molecular shape is bent.
- The tetrahedral angle between idealized domain directions must not be
  labelled or presented as the H–O–H bond angle.
- A lone-pair glyph is a schematic classification marker, not a literal
  electron cloud boundary, electron trajectory, or measured density maximum.

The arrangement view must not draw bonding-domain spokes as molecular bonds
or place neighbouring atoms at the domain anchors. In particular, doing so
for water would visually imply a tetrahedral H–O–H angle and contradict the
bent molecular shape.

The molecular shape classification may be displayed as explicit descriptive
metadata (for example, "Molecular shape: bent"). It is not inferred from the
schematic domain anchor coordinates.

## 3. Coordinate and geometry contract

Introduce a dedicated schematic coordinate type or module-local value type.
Do not reuse MolecularPoint3, WorldPoint3, or ångström units.

- Coordinates are dimensionless, local to the VSEPR arrangement schematic.
- The central-atom anchor is the origin.
- Four deterministic, normalized tetrahedral direction vectors are used.
- The four vectors have equal length and pairwise dot product -1/3, within
  floating-point tolerance.
- Their orientation is a canonical presentation convention, not a measured
  molecular orientation.
- Each domain in the validated VseprCentre receives exactly one direction.
  Preserve the input domain order when assigning canonical direction slots.
- No source molecular coordinate is read or changed.
- No bond length, molecular bond angle, electron-density distribution, or
  physical domain size is derived from these coordinates.

The view must expose the coordinate-space meaning and must not call these
coordinates world positions or molecular coordinates.

## 4. Proposed graphics-independent API

The exact names may be adjusted to fit existing SVF conventions, but keep
these responsibilities:

- VseprSchematicPoint3: finite dimensionless local coordinates.
- VseprDomainGlyph3: source ElectronDomainId, source ElectronDomainKind
  (therefore preserving BondId and neighbouring AtomId for bonding domains),
  and a schematic anchor point.
- VseprSchematicView3: central AtomId, ElectronDomainArrangement,
  MolecularShape, and ordered domain glyphs.
- build_vsepr_schematic_view(&VseprCentre) -> Result<..., ...> or an
  equivalent infallible constructor if the fixed canonical mapping cannot
  fail after VseprCentre validation.

The adapter consumes only a validated VseprCentre. It does not need the
source Molecule, renderer types, material/style objects, or GPU resources.

Do not add a generic scene graph, generic glyph framework, arbitrary
orientation editor, style system, or configurable layout in this increment.

## 5. Presentation semantics

The view data must let a renderer distinguish:

- the central atom anchor;
- a bonding-pair domain;
- a lone-pair domain;
- the stable domain identity and source relationship;
- the tetrahedral electron-domain arrangement;
- the molecular-shape classification.

The renderer may use visibly different glyph forms for bonding-pair and
lone-pair domains, but glyph appearance is a visual convention. It must not
make lone pairs look like hydrogen atoms or ordinary chemical bonds.

A legend or equivalent annotation must identify the glyph classes. The
molecular-shape classification must remain visible independently of the
domain arrangement.

## 6. SVF-only sample and evaluation

After the core adapter is implemented, add a deliberately small Explorer
sample that constructs the methane and water models directly from SVF's
chemistry and VSEPR APIs and displays their schematic domain arrangements
side by side or through an explicit sample switch.

The sample should make it possible to verify:

1. Both examples show four tetrahedrally arranged domain markers.
2. Methane shows four bonding-pair domains and zero lone-pair domains.
3. Water shows two bonding-pair domains and two lone-pair domains.
4. Both examples retain their correct molecular-shape labels.
5. Selecting/inspecting a domain (if selection is included in the sample)
   resolves its exact ElectronDomainId and classification; a bonding
   domain also resolves its exact source BondId and neighbouring AtomId.
6. Camera movement changes only the view, not the source molecule or
   VSEPR semantics.

Do not add domain selection if the existing Explorer input/picking path
cannot support it without disproportionate infrastructure. A static,
clearly labelled sample is sufficient for the first visual acceptance.

The sample is a validation instrument for SVF, not a commitment to a general
educational UI or integration layer.

## 7. Required tests

Core tests:
- all four canonical vectors are finite, normalized, and pairwise tetrahedral;
- the central anchor is the origin;
- every source domain appears exactly once and in source order;
- domain IDs and complete bonding-domain provenance are preserved;
- methane maps to four bonding glyphs and tetrahedral shape metadata;
- water maps to two bonding glyphs and two lone-pair glyphs with bent shape
  metadata;
- output is deterministic;
- source VseprCentre remains unchanged;
- no molecular coordinates or ångström values leak into the schematic API.

Explorer tests or acceptance checks:
- both sample cases construct from SVF data;
- labels and domain classifications match their source models;
- the arrangement is visually distinguishable in the running application;
- existing Explorer modes remain unchanged.

## 8. Explicit non-goals

- Molecular atom-position or bond-angle reconstruction.
- A measured or computed H–O–H angle.
- General VSEPR domain counts, arrangements, or molecular shapes.
- Electron-density or quantum-chemistry visualization.
- Lewis-structure parsing or automatic VSEPR inference.
- Molecular simulation or animation.
- Arbitrary molecule authoring.
- Generic interaction, scene, or glyph frameworks.
- Mava or Prismal integration.
- A complete lesson player, assessment system, or teacher controls.

## 9. Acceptance and decision gate

This increment is successful only if the standalone SVF sample makes the
scientific distinction visible without implying that water's molecular shape
is tetrahedral.

After the sample is locally validated, assess what it actually demonstrates
and what remains missing from the selected upper-secondary learning case.
Do not automatically expand the feature set. In particular, a correct
electron-domain arrangement schematic does not by itself provide a validated
molecular geometry or bond-angle visualization.
