# Valuable-output bar and item register

Last updated: 2026-11-22. Status: working definition, proposed by Claude and agreed in direction by the user.
The criteria below are a draft to be confirmed by the user.

## Why this exists

The user's goal for January 2027 is about 75% confidence that SVF produces
**valuable output** (the things that go into lessons), across several
subjects. The question is not yet whether a lesson can be built, how Mava
embeds SVF, or what the author-facing API looks like. Those come after the way
items are built has settled into a convention. Several different items, built
by different methods, are how that convention and the effort of authoring
become visible.

An item counts toward the 75% only when it meets this bar. Each gap is the work.

## The bar

An item is *valuable output* when all five hold, each with evidence recorded
in this register (not asserted):

1. **Correct for its purpose.** Real-data items state their measured error
   against the source. Schematic and authored items say so in the item itself
   and are never presented as patient-derived or measured evidence.
2. **Legible to the intended learner.** A learner can tell the parts apart and
   see the idea the lesson needs, at a sensible default view. The user judges
   this, by looking.
3. **Explorable.** The interactions the teaching idea needs work on the item
   (pick, isolate, peel, section, views, a scalar or vector field, and so on).
   Not every item needs every interaction.
4. **Provenance and licence clear.** Source, attribution, processing and
   limitations are recorded, and the licence allows educational redistribution.
5. **Cost recorded.** How long it took and which steps were manual, so that the
   convention and the author surface can be designed from real effort.

## Register

"Evidence" means something recorded in the repository or reported by the user.
Blank means not yet judged, not failed.

| Item | Subject | Method | Correct | Legible | Explorable | Provenance / licence | Cost | Notes |
|---|---|---|---|---|---|---|---|---|
| Heart (Rodero case 01) | Biology | adapted source mesh | yes: worst deviation 0.396 mm vs 0.4 mm target, all 21 parts closed (user-reported) | partly: peeled view does not show all four cavities independently; user chose not to polish | pick, isolate, peel, section all exercised by the user | attribution embedded in the fixture; licence not re-checked for redistribution | high (several PRs) | `docs/evidence/b2-5e-heart-adaptation-validation.md` |
| Cockroach | Biology | authored, procedural | schematic, not measured | | views, peeling, relations implemented | `reference` citation is still the user's to confirm | medium | `docs/cockroach-structure.md` |
| Generic flower and fruit | Biology | authored, existing blocks | schematic, not measured; says so in the file | looks distinguishable in screenshots; the user has not judged | pick, isolate, peel, section, three views; `pollination` timeline (20 s, 5 beats, captions) | citation to be confirmed by the user; no source data | low: one session, no core change | `docs/flower-structure.md`, `docs/evidence/flower-poc/`, `docs/evidence/animation-a4-flower/`; proof of concept, throw-away |
| Diffusion of a dye in a beaker | Particle view | authored, existing blocks plus particle generator | schematic, not measured; says so in the file; particles are a picture of random motion from a fixed seed, not a simulation | tight drop spreading through the beaker looks clear in screenshots at 4 s and 12 s; the user has not judged | 28 s timeline with captions; no picking of particles; water not drawn (opaque renderer) | citation to be confirmed by the user; no source data | low: one session, after the particle slice | `fixtures/structures/diffusion-beaker.ron`, `docs/evidence/animation-b2-beaker/`; proof of concept, throw-away |
| Generic plant cell | Biology (CEL) | authored, existing blocks | schematic, not measured; says so in the file | the cut view looks like a textbook diagram (Claude's look); the user has not judged | pick, isolate, peel, section, three views | citation to be confirmed by the user; no source data | low: under one session, no core change | `docs/evidence/plant-cell-poc/`, `crates/svf-core/tests/plant_cell_fixture.rs`; proof of concept, throw-away |
| Dicot stem | Biology | authored, extrusion | schematic, not measured | | section, peel | `reference` citation is still the user's to confirm | low | `fixtures/structures/dicot-stem.ron` |
| VSEPR molecules | Chemistry | rule-driven | | | | | | `docs/upper-secondary-chemistry-vsepr-learning-case.md` (proposal) |
| Heat diffusion, fibres | Physics, biology | scalar and vector fields | validated against an analytical solution | | | | | analytic fixtures in `fixtures/analytical-heat-diffusion/` |
| Kidney + mass (KiTS-00005) | Biology | voxel segmentation | yes: surface equals the SEG mask exactly (`docs/MILESTONES.md`, 2026-11-22), but blocky and not a smoothed structure | no: staircase surface | none yet | **unresolved** (CC BY-NC-SA vs CC BY 3.0) | high | Capability probe only. Paused. |

## Candidate next items

Drawn from the Bitcraft syllabus (`science-math/curriculum/syllabus.yaml`),
which is a better guide than the 172 written lessons: those were written
before SVF existed and are expected to change as SVF proves out. Anything not
mathematical, or that a model can show, may move to SVF depending on how good
SVF comes out. The user confirms the list.

| Candidate | Syllabus area | Why it tests something new |
|---|---|---|
| ~~Flower cut in half~~ | PRP | Built as a proof of concept (structure only); see the register. Its process half stays with Prismal. |
| Digestive system | BOD | Authored (no suitable open data), tube-like organs, systems |
| Breathing and circulation | TRN | Linked systems and flow |
| ~~Plant cell~~ | CEL | Built as a proof of concept; an animal cell would add little. Cytoplasm is not drawn (it fills the space), a limit to remember. |
| Water and particle view | PAR | A zoom between an everyday view and a particle view (RQ-PRISMAL-016) |
| Molecular and crystal structure | Bonding | Beyond VSEPR: giant structures |

## Decisions

- **2026-11-22 (user):** animation is an SVF capability too (diffusion is the example). The line between the
  tools is whether the learner runs an experiment, not whether things move. See `svf-prismal-boundary.md`.
- **2026-11-22 (user):** items built now are throw-away proofs of concept, not lesson material; a generic
  flower is enough.

- **2026-11-22 (user):** test SVF on many items and subjects first; integration
  questions come afterwards. Mava will embed SVF output; Prismal may never need
  SVF. Authors must never meet the engineering, so a high-level author surface
  is wanted, but only after the build convention has stabilised.
- **2026-11-22 (Claude, agreed by the user):** the kidney is a capability
  probe, not a lesson item. Pause it after the independent verification.
  Smoothing and conversion into a structure would be needed before it could be
  scored; do that only if a lesson requirement appears.
- Convention emerging so far: a semantic structure (named parts, parent and
  child, relations, views, sectioning and peeling) over any geometry source.
  Record what each new item forces to change in that convention.
