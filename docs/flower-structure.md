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

**Update 2026-11-22: the user confirmed the structure-only scope.** The process
(learner-chosen pollen, growth, random seeds) belongs to Prismal under the rule
in `svf-prismal-boundary.md`. The flower's stages double as the first evidence
for the open question there: what a fixed process over a structure needs.
Questions 2 and 3 below are still open.

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

## Result (2026-11-22): proof of concept built

Scope as agreed with the user: a **generic, throw-away proof of concept**, not lesson material. The user
said generic is enough and did not pick stages, so I chose two stages (flower, then fruit) with three views.

- Fixture: `fixtures/structures/flower.ron` (peduncle, receptacle, sepals, petals, filaments, anthers,
  stigma, style, hollow ovary with six ovules, and a fruit stage with a hollow fruit and six seeds). Only
  existing blocks were used: `Tube`, `Ellipsoid`, `Radial` repeat. No core code changed.
- Tests: `crates/svf-core/tests/flower_fixture.rs`. 5 passed, run by Claude in a release build on
  2026-11-22: the fixture parses, every body is closed, plane cuts on all three axes stay closed, the
  lengthwise cut shows ovary, ovules, style, stigma, fruit and seeds.
- Screenshots (Explorer, captured by Claude with PrintWindow): `docs/evidence/flower-poc/`.
  `carpel.png` predates a camera change to the carpel view.
- Legibility: from the screenshots the parts are distinguishable and the lengthwise cut shows the hollow
  ovary with ovules. **The user has not yet judged this by looking.**

### What building it taught (inputs to the convention and to the register's cost column)

1. **A stage that changes size needs separate parts.** The ovary became `fruit` and the ovules became
   `seeds`: four parts to show two stages. Views only show, hide and move the camera.
2. **No default view.** With no view chosen both stages show at once, the orange fruit hiding the ovary
   (`default.png`). Peeling one layer (`fruit-stage` is the outermost layer) removes it, a workaround.
3. **A view cannot frame its own parts.** The camera centres on the whole structure's bounding box, so a
   view of the carpel was off-centre while a long stalk was in the scene. Worked around by shortening
   the stalk to 10 mm. A view needs a camera target.
4. **A `Tube` along the Y axis needs an explicit `up`.** The default `up` is +Y, so a vertical tube is
   refused. Easy to hit; an author would not know why.
5. **View syntax is easy to get wrong** (`section: Some((axis: Z, offset: 0.0, flip: false))`). Two parse
   errors cost two rebuilds.
6. **Effort:** about one working session including tests and screenshots, for 14 parts with copies.
   Cheaper than the cockroach, since the blocks already existed; no core change was needed.
7. **Animation** (pollen tube growth, the ovary swelling) is not shown. Under the user's rule that is the
   SVF animation question in `svf-prismal-boundary.md`, still open.
