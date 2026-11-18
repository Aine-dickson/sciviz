# Educational direction and near-term roadmap

Last updated: 2026-11-18

## Product direction

SVF is primarily a scientific visualization framework for teaching and
learning, with an immediate emphasis on interactive 3D anatomy. It should
represent anatomical structures and scientific phenomena accurately, then let
learners inspect and investigate them through meaningful interactions.

Working definition:

> SVF is a scientific visualization framework for creating and exploring
> interactive 3D representations of anatomical structures and scientific
> phenomena for teaching and learning.

The framework should remain capable of handling arbitrary scientific datasets,
but that capability serves educational use. Do not redirect the project toward
a general-purpose scientific visualization product, CAD system, game engine,
medical diagnosis system, or generic 3D editor merely for breadth or feature
parity. If wider adoption later creates a justified need, revisit scope using
actual evidence.

## Product priorities

1. **Scientific representation:** preserve meaningful geometry, coordinates,
   topology, units, labels, fields, spatial relationships, and provenance.
2. **Educational exploration:** enable learners and teachers to inspect,
   isolate, compare, section, peel, measure, and investigate structures or
   phenomena where those interactions support a learning objective.
3. **Scientific correctness:** every visual operation and explanation must
   preserve the meaning and limitations of its source data. A visually
   attractive but misleading representation is unacceptable.
4. **Lesson usefulness:** capabilities should be selected and validated against
   concrete teaching and learning tasks, not an abstract feature checklist.

The educational progression to support is teacher-guided demonstration,
guided learner exploration, and independent investigation. Not every model
needs every interaction; the learning objective determines which interactions
are meaningful.

## Architectural boundaries

- SVF remains independently usable. Do not make its architecture depend on
  Mava, Prismal, or a particular lesson-authoring application.
- Keep scientific data, semantic identity, geometry, topology, measurements,
  and provenance authoritative in the CPU-side core.
- Keep rendering as a replaceable presentation layer over prepared scientific
  representations.
- Build reusable capabilities when they serve multiple concrete educational
  cases; avoid organ-specific code when a general representation is justified.
- Do not build speculative general-purpose abstractions without a demonstrated
  educational or scientific use case.
- Distinguish source observations, derived geometry, schematic illustrations,
  and synthetic test data. Document assumptions and limitations.

## Dataset strategy

Use three complementary dataset sources:

### 1. Public, source-derived datasets

Prefer trustworthy, legally reusable anatomy and scientific datasets with
clear provenance, licence, units, coordinate conventions, segmentation
semantics, and reference documentation. Audit a specific case before
committing to an implementation. Do not assume that a free download permits
every use; record attribution and any non-commercial or share-alike terms.

### 2. SVF-authored educational reference models

Create original datasets when suitable open datasets do not exist, are too
large, lack the required semantic labels, or do not support the intended
learning interaction. These may be authored manually or generated
procedurally. Validate anatomical models against authoritative references
and label them honestly as schematic or illustrative when they are not
patient-derived or anatomically validated.

### 3. Synthetic engineering fixtures

Generate controlled geometry and fields with known ground truth for algorithm
tests. Include expected dimensions, topology, volumes, field values, and
relationships. Synthetic fixtures are for reproducible validation and must not
be presented as anatomical evidence.

Every committed dataset should document, as applicable: source and citation,
licence and attribution, units, coordinate frame and orientation, semantic
labels, processing steps, known omissions, validation method, and whether it
is source-derived, manually authored, procedural, or synthetic.

## Near-term delivery target

The immediate planning target is to begin building lessons in **January 2027**.
This is a practical delivery target, not a promise that SVF will be feature
complete by then.

Prioritize a small number of scientifically sound, genuinely explorable
models that can support real lessons over a large collection of superficial
models. The December objective is to remove blockers to lesson construction,
not to finish every possible framework capability.

Proposed planning checkpoints (adjust to evidence and actual progress):

- **Remainder of November:** consolidate the heart milestone, inspect existing
  volumetric and segmentation capabilities, select and audit a kidney dataset,
  and define the smallest useful next increment.
- **December:** implement and validate reusable capabilities required by the
  selected anatomy/lesson cases; prepare a small set of lesson-ready models;
  test the interactions against intended learner tasks.
- **January 2027:** begin authoring and trying lessons with the available
  validated models. Feed observed teaching/learning needs back into SVF.

These are priorities, not permission to bypass design review, source audits,
scientific validation, or user-reported local tests.

## Next investigation: kidney anatomy and volumetric data

The kidney is the next candidate anatomy, not yet an approved implementation
specification. Use it to investigate capabilities that differ from the heart
mesh workflow:

- volumetric image input and voxel-grid semantics;
- segmentation labels and their mapping to anatomical structures;
- reconstruction of surfaces from labelled volumes;
- preservation of source-volume identity and spatial calibration;
- comparison of reconstructed surfaces with source slices;
- cross-sections that remain consistent with the source volume;
- anatomical relationships and educationally meaningful inspection.

Do not begin by writing kidney-specific production code. First inspect
existing SVF support for VTK/unstructured data, fields, isosurfaces, imported
surfaces, structure semantics, and sectioning. Then audit one manageable
public kidney case and identify the actual gaps.

A kidney dataset may represent only the kidney, tumour, or selected structures;
do not describe it as a complete urinary system unless its contents establish
that. Choose the dataset according to the learning objective, not its name.

## Definition of a useful educational model

A model is not lesson-ready merely because it loads and renders. Evaluate it
against a specific learning task:

- Are the represented structures and their labels scientifically supported?
- Can the learner distinguish the structures relevant to the objective?
- Do picking, isolation, peeling, sectioning, views, and measurements behave
  predictably where provided?
- Do interactions reveal meaningful relationships rather than just change
  appearance?
- Are coordinate orientation, units, colours, and schematic conventions clear?
- Are omissions and limitations visible to the teacher or learner?
- Can the model be reproduced from documented source data and processing?

Use only the interactions that serve the case. A technically valid mesh may
still need better educational framing, but presentation concerns should not
trigger geometry changes without evidence of a geometry defect.

## Relationship to existing milestones

The B2 heart work establishes an important baseline: labelled tetrahedral
source data, extracted and repaired region surfaces, measured patch
simplification, closed mesh containers, provenance, semantic structure parts,
layers, picking, isolation, sectioning, and peeling. The heart's source and
scope limitations remain documented in `docs/heart-structure.md`.

The next milestone should be designed from the kidney dataset audit and the
specific educational objective. It may be called B3, but its scope and
acceptance criteria must be written and reviewed before implementation.
