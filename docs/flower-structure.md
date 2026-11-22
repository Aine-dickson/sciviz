# Flower — design note (draft for user review)

Status: **proposal; nothing implemented.** Written 2026-11-22. Part of the item
register in `item-output-bar.md`. Follows the structure conventions in
`explorable-structures.md` and `cockroach-structure.md`.

## Why this item

It comes from a real written lesson requirement (RQ-PRISMAL-030, lesson
L-S-PRP-04, syllabus area PRP), it is a different organism group and scale
again, and it tests one thing the earlier items did not: **an item that has
stages** (flower at pollination, then young fruit, then mature fruit and seed).

## Scope decision the user must confirm

RQ-PRISMAL-030 mixes two different needs:

1. **Structure**: a flower cut in half from top to bottom, showing stigma,
   style, ovary and ovules. This is what SVF already does well.
2. **Process**: the learner chooses how many pollen grains land, tubes grow to
   ovules over simulated days, seeds form only where a tube arrives, the flower
   withers with no pollen, and a maize cob shows gaps where silks got no pollen.
   This is a simulation with learner-set inputs and random outcomes. SVF has no
   time or state model, and the lesson team itself asked Prismal whether it
   could show it.

**Proposal:** build (1) in SVF now, with the stages as **named views** (the
existing `View` feature), so SVF shows what the flower *looks like* at each
stage. Leave (2) to Prismal, or revisit it once SVF has a justified time
model. Do not invent a time or state model for one lesson. This keeps the
test honest: does the structure convention cover a staged item?

## Lesson content (to be confirmed by the user)

Not yet confirmed, unlike the cockroach. A proposal from the primary syllabus
(PRP: "parts of a flower, pollination, fruits and seed dispersal"):

- Outer to inner: peduncle (stalk), receptacle, sepals, petals, stamens
  (filament and anther), carpel (stigma, style, ovary, ovules).
- One generic, radially symmetric flower. Not a named species, unless the user
  picks one. Pick one that learners know (maize needs a different structure and
  is out of scope here).
- Sections: lengthwise through the centre, so the style, ovary and ovules show.
- Views: whole flower; cut lengthwise; carpel only; **after pollination**
  (petals and stamens withered, ovary swelling); **young fruit** with
  developing seeds.

## Geometry and blocks (all already implemented)

| Part | Block |
|---|---|
| Peduncle, style, filament | `Tube` |
| Receptacle, ovary, anther, ovules, stigma | `Ellipsoid` |
| Petals, sepals | `Ellipsoid` flattened, with `repeat: Radial` and `rotate_deg` |
| Ovules inside the ovary | `Ellipsoid` with `repeat` (ovary needs `wall` so ovules are visible when cut) |

The mirror feature is not needed (radial symmetry uses `Radial` repeat).

## Risks

- **Stages as views.** A view can only hide, show and move the camera. It
  cannot change a part's size (an ovary swelling into a fruit). If stages need
  different geometry, each stage needs its own parts (for example `ovary` and
  `young-fruit`), and a view shows one or the other. That is workable but
  verbose, and it is exactly the authoring cost to record in the register.
- **Overlapping bodies.** Sepals, petals and stamens meet at the receptacle.
  The code does not guarantee that parts do not overlap (as for the cockroach).
- **Hollow ovary.** The ovary needs a wall so the cut shows ovules, as the
  heart cavities do; check that the section cap handles a wall plus inner parts.
- **Accuracy.** Authored and schematic. It must say so in the file and never be
  presented as measured. A citation for the arrangement is the user's to confirm.

## Acceptance (to be run once implemented)

- Fixture loads, every part closed and outward-wound; section along the axis
  works and stays closed across a sweep (the same regression as the heart).
- Peeling reveals sepals, petals, stamens and carpel in a sensible order.
- Each stage view shows a coherent flower or fruit.
- The user judges legibility by looking, and the register records the cost.

## Questions for the user

1. Is the structure-only scope right, with the process left to Prismal?
2. Which flower, if any, should it be based on, or generic?
3. Which stages matter for the lesson (the three above, or fewer)?
