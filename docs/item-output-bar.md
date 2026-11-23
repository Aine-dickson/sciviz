# Valuable-output bar and item register

Last updated: 2026-11-23 (fidelity column and "Fidelity by part" added, C1). Status: working definition, proposed by Claude and agreed in direction by the user.
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

| Item | Subject | Method | Fidelity (F / B / A) | Correct | Legible | Explorable | Provenance / licence | Cost | Notes |
|---|---|---|---|---|---|---|---|---|---|
| Heart (Rodero case 01) | Biology | adapted source mesh | F3 / – / F0 | yes: worst deviation 0.396 mm vs 0.4 mm target, all 21 parts closed (user-reported) | partly: peeled view does not show all four cavities independently; user chose not to polish | pick, isolate, peel, section all exercised by the user | attribution embedded in the fixture; licence not re-checked for redistribution | high (several PRs) | `docs/evidence/b2-5e-heart-adaptation-validation.md` |
| Cockroach | Biology | authored, procedural | F0 / – / F0 | schematic, not measured | | views, peeling, relations implemented | `reference` citation is still the user's to confirm | medium | `docs/cockroach-structure.md` |
| Generic flower and fruit | Biology | authored, existing blocks | F0 / F0 / F0 | schematic, not measured; says so in the file | looks distinguishable in screenshots; the user has not judged | pick, isolate, peel, section, three views; `pollination` timeline (20 s, 5 beats, captions; petals and stamens fade as they fall) | citation to be confirmed by the user; no source data | low: one session, no core change | `docs/flower-structure.md`, `docs/evidence/flower-poc/`, `docs/evidence/animation-a4-flower/`; proof of concept, throw-away |
| Water moving up a dicot stem | Plant transport | authored, existing blocks; fades and particle flow added to the existing stem fixture | F0 / F0 / F0 | schematic, not measured; says so in the file; straight paths up each xylem bundle, wrapping from top to bottom, show direction not speed | red xylem tubes with water particles inside are clear in screenshots at 8 s and 12 s once the other tissues are faded out; the user has not judged | 24 s timeline, 3 beats, captions; section, peel, isolate and views still work on the paused stem | citation to be confirmed by the user (the stem's, not the flow's); no source data | low: one session, after opacity and particles | `fixtures/structures/dicot-stem.ron` (`water-up`), `docs/evidence/stem-water/`; proof of concept, throw-away |
| Breathing: rib cage, diaphragm, lungs, airways | Human body (BOD, TRN) | authored, existing blocks (tubes with repeat, ellipsoids with mirror) plus fades and particle flow | F0 / F0 / F0 | schematic, not measured; says so in the file; 7 ribs instead of 12 pairs; no heart, pleura or gas exchange; air particles show direction only | two lungs, bronchi, windpipe, ribs and diaphragm are distinguishable at the front view in screenshots at 3 s, 7 s and 12 s; the user has not judged | 29 s timeline (rest, breathe in, breathe out, two quick breaths) with captions; peel (4 layers), section and 3 views work | citation to be confirmed by the user; the breathing rate for children is a rounded textbook range | low: one session, no core change | `fixtures/structures/breathing.ron`, `crates/svf-core/tests/breathing_fixture.rs`, `docs/evidence/breathing/`; proof of concept, throw-away |
| Gas exchange in an air sac | Human body (TRN) | authored, existing blocks (hollow ellipsoid, tube) plus particle flows | F0 / F0 / F0 | schematic and NOT TO SCALE, says so in the file; walls drawn thick; one capillary touches one sac; gases drawn as short straight streams, blood as two groups of cells (dark before the sac, bright after) | sphere, capillary, dark and bright blood and the two gas streams are distinguishable in a screenshot at 28 s; the user has not judged; colours are named in the captions | 39 s timeline, 5 beats, captions; peel, section (cut-open view) and 2 views work | citation to be confirmed by the user; no source data | low: one session, no core change | `fixtures/structures/gas-exchange.ron`, `crates/svf-core/tests/gas_exchange_fixture.rs`, `docs/evidence/gas-exchange/`; proof of concept, throw-away |
| The digestive system | Human body (BOD) | authored, existing blocks (ellipsoids, tubes built from lines and arcs) plus particle flows and spreads | F0 / F0 / F0 | schematic and NOT TO SCALE, says so in the file; intestines drawn short; no pancreas, gall bladder, blood vessels, teeth or appendix | mouth, gullet, stomach, liver, coiled small intestine and framing large intestine are distinguishable in screenshots at 12, 28, 48 and 60 s; the user has not judged; particle colours are named in the captions only | 75 s timeline, 8 beats (one organ each), captions; peel (5 layers), section and 3 views work | citation to be confirmed by the user; no source data | medium: one session; the intestine paths needed arcs sampled at even spacing to pass the tube-bend check, and one small core change (`Spread` `stay`) | `fixtures/structures/digestive-system.ron`, `crates/svf-core/tests/digestive_fixture.rs`, `docs/evidence/digestive/`; proof of concept, throw-away |
| Hydrogen and oxygen make water (2H2 + O2 -> 2H2O) | Chemistry (chemical reactions) | authored, existing blocks (ball atoms, bond tubes) with Move, Fade and Show | F2 / F0 / F0 | schematic; bond lengths 0.74, 1.21, 0.96 angstrom and the 104.5 degree angle are the textbook values and are checked to 1e-9 by the test; not the real mechanism (stated in the file); atoms drawn as small balls | two H2 and one O2 before, two bent H2O after, are clear in screenshots at 1 s and 30 s; the user has not judged | 39 s timeline, 5 beats, captions; atom count conserved (6 atoms always shown); no atom overlaps at any of 391 sampled times | citation to be confirmed by the user; values are standard textbook figures | low: one session, no core change; the final positions needed trigonometry, so the file was generated and then kept as the source | `fixtures/structures/water-reaction.ron`, `crates/svf-core/tests/water_reaction_fixture.rs`, `docs/evidence/water-reaction/`; proof of concept, throw-away |
| The magnetic field of a bar magnet | Physics (magnets, magnetic fields) | authored magnet (two tubes) plus field lines and needles generated from a two-point-pole model and written into the file | F0–F2 / F2 / F0 | schematic; the model (north pole at +3.6, south at -3.6 cm) is stated in the file; directions are correct for it, line spacing shows strength only roughly (5 lines copied 6 times round the axis); the field inside the magnet is not drawn | a recognisable 3D field of lines leaving the red end and curving into the blue end, a compass that turns along one line, and 8 compasses around, in screenshots at 14, 28 and 56 s; the user has not judged; colours named in captions only | 56 s timeline, 5 beats, captions; section and peel are not set up for this item | citation to be confirmed by the user; field model is the textbook two-pole picture | medium: one session; the lines and needle positions needed a generator (field integration), and a first choice of lines (20 degrees) made the picture too big | `fixtures/structures/bar-magnet.ron`, `crates/svf-core/tests/bar_magnet_fixture.rs`, `docs/evidence/bar-magnet/`; proof of concept, throw-away |
| Salt: an ionic lattice that dissolves (NaCl) | Chemistry (ionic bonding, dissolving) | generated: a 4x4x4 rock-salt lattice of balls, water molecules as three balls; the generator is kept (`tools/structure-items/salt_dissolving.py`) | F2 / F0 / F0 | schematic; lattice constant 5.64 angstrom, ionic radii 1.02/1.81, O-H 0.96, H-O-H 104.5, ion to water-oxygen 2.4/3.2 angstrom, all checked by the test; six water molecules per ion and corner-only dissolving are simplifications stated in the file | the lattice with grid lines (14 s), the six neighbours of one ion (25 s), water at the corners (44 s) and two hydrated ions, oxygen ends facing Na+ and hydrogen ends facing Cl- (90 s), are clear in screenshots; the two first freed ions overlap in projection; the user has not judged | 90 s timeline, 9 beats, captions; 64 ions always shown; no two balls overlap at any of 901 sampled times (Na+ and Cl- touch at real size) | citation to be confirmed by the user; values are standard textbook and crystallographic figures | medium: one session, no core change; needed a generator (178 parts, 200 actions) and three camera tries | `fixtures/structures/salt-dissolving.ron`, `crates/svf-core/tests/salt_dissolving_fixture.rs`, `docs/evidence/salt-dissolving/`; proof of concept, throw-away |
| A simple circuit with a switch | Physics (electricity, circuits) | generated: cell, wires (rounded tubes from lines and arcs), switch lever, filament lamp, one closed-loop `Flow` of electrons; generator kept (`tools/structure-items/simple_circuit.py`) | F0 / F0 / F0 | schematic; electron direction (negative to positive) and the same current all round are right; speed is a picture (stated: real drift is under 1 mm/s); conventional current named in a caption only | open lever, lit lamp with electrons through switch and filament, and the lamp out again are clear in screenshots at 5, 21.8, 30 and 50 s; the user has not judged | 65.5 s timeline, 5 beats, captions; electrons and lamp on exactly while the lever is down (tested every 0.1 s) | citation to be confirmed by the user; standard textbook account | low: one session, no core change; a zigzag filament was refused (too tight a bend) and became a sine coil; a first draft lit the lamp 2.5 s after the switch closed, caught on a screenshot and fixed | `fixtures/structures/simple-circuit.ron`, `crates/svf-core/tests/simple_circuit_fixture.rs`, `docs/evidence/simple-circuit/`; proof of concept, throw-away |
| Diffusion of a dye in a beaker | Particle view | authored, existing blocks plus particle generator | F0 / F0 / F0 | schematic, not measured; says so in the file; particles are a picture of random motion from a fixed seed, not a simulation | tight drop spreading through the beaker looks clear in screenshots at 4 s and 12 s; the user has not judged | 28 s timeline with captions; translucent glass and water (opacity); no picking of particles | citation to be confirmed by the user; no source data | low: one session, after the particle slice | `fixtures/structures/diffusion-beaker.ron`, `docs/evidence/animation-b2-beaker/`; proof of concept, throw-away |
| Generic plant cell | Biology (CEL) | authored, existing blocks | F0 / – / F0 | schematic, not measured; says so in the file | the cut view looks like a textbook diagram (Claude's look); the user has not judged | pick, isolate, peel, section, three views | citation to be confirmed by the user; no source data | low: under one session, no core change | `docs/evidence/plant-cell-poc/`, `crates/svf-core/tests/plant_cell_fixture.rs`; proof of concept, throw-away |
| Dicot stem | Biology | authored, extrusion | F0 / – / F0 | schematic, not measured | | section, peel | `reference` citation is still the user's to confirm | low | `fixtures/structures/dicot-stem.ron` |
| VSEPR molecules | Chemistry | rule-driven | F1 / – / F0 | | | | | | `docs/upper-secondary-chemistry-vsepr-learning-case.md` (proposal) |
| Heat diffusion, fibres | Physics, biology | scalar and vector fields | – / F2 / F0 | validated against an analytical solution | | | | | analytic fixtures in `fixtures/analytical-heat-diffusion/` |
| Kidney + mass (KiTS-00005) | Biology | voxel segmentation | F3 / – / F0 | yes: surface equals the SEG mask exactly (`docs/MILESTONES.md`, 2026-11-22), but blocky and not a smoothed structure | no: staircase surface | none yet | **unresolved** (CC BY-NC-SA vs CC BY 3.0) | high | Capability probe only. Paused. |

## Fidelity by part

Scored on 2026-11-23 from what the repository records, against the levels in `docs/authoring-convention.md` section 4.
The register column gives form / behaviour / appearance (F / B / A). The range runs from the lowest part to the highest,
and "–" means the item has no such axis (a static structure has no behaviour). An item's fidelity is the list below,
never the column alone. "Stopped by" says why the part has not climbed further. It is the work list for C2 to C5.

**Appearance is F0 for every item.** The renderer shades each triangle flat (`mesh.wgsl`), and F1 needs smooth shading.
Labels, the key and translucency, which also belong to F1, already exist. So smooth shading (C2, first slice) lifts every
item's appearance to F1 at once. No item has material cues (F2).

| Item | Part (or group) | Form | Behaviour | Stopped by |
|---|---|---|---|---|
| Heart | 21 regions from Rodero case 01 | F3 (worst deviation 0.396 mm) | – (static; fibres are a separate item, B1.2) | nothing in form; behaviour would need motion data (cardiac cycle) |
| Kidney + mass | kidney, mass | F3 (equals the mask exactly) | – | staircase surface: needs smoothing before use; licence unresolved |
| Cockroach | body segments, legs, wings | F0 (tubes, ellipsoids) | – | no smooth organic shapes yet (C2 list, section 6); no open scan audited |
| Flower and fruit | petals, sepals, stamens, carpel, fruit | F0 | F0 (hand-timed falls and fades) | smooth organic shapes (lofts, lathes); MorphoSource not audited |
| Dicot stem | tissues (extrusions) | F0 | – | no open cell-level dataset found yet; F1 needs textbook proportions |
| Water up a dicot stem | xylem flow | F0 (straight paths) | F0 (direction only, not speed) | a transport kit with a stated flow rate (F2) |
| Plant cell | organelles | F0 | – | smooth organic shapes; no audited micrograph-derived source |
| Breathing | ribs, diaphragm, lungs, airways | F0 (7 ribs, not 12 pairs) | F0 (hand-timed; rate is a rounded textbook range) | hinge for ribs (C2); CT segmentation (TotalSegmentator, C5) |
| Gas exchange | air sac, capillary, gases, blood | F0, not to scale | F0 | a stated diffusion model (F2) |
| Digestive system | mouth to large intestine | F0, not to scale, intestines short | F0 (hand-placed path points) | CT organs plus centrelines (C5) |
| Water reaction | molecules (H2, O2, H2O) | F2 (textbook bond lengths 0.74, 1.21, 0.96 Å and 104.5°, tested) | F0 (not the mechanism; stated) | atoms drawn as small balls (stated); molecule bridge from `chemistry.rs` (C4); mechanism stays F0 by choice |
| Bar magnet | magnet body | F0 (two tubes) | – | real magnet dimensions (F2 form) |
| Bar magnet | field lines, compasses | – | F2 (computed from a stated two-point-pole model) | point poles are a crude model; the target is the analytic cuboid field (section 7); compass timing is hand-chosen (F0) |
| Salt dissolving | lattice ions | F2 (5.64 Å lattice, ionic radii 1.02 and 1.81 Å, tested) | F0 (one hand-timed `Grow` per ion) | a crystal kit (C4); dissolving is choreographed, not modelled |
| Salt dissolving | water and hydration shell | F2 (O–H 0.96 Å, 104.5°, ion to oxygen 2.4 and 3.2 Å) | F0 | six waters per ion and corner-only dissolving are simplifications (stated); published radial distribution data (C4) |
| Simple circuit | cell, wires, switch, lamp | F0 (no real dimensions) | F0 (lamp and current hand-synchronised to the switch, tested) | datasheet dimensions (F2 form); a circuit kit for F1 behaviour (C3) |
| Diffusion beaker | beaker, water, dye particles | F0 | F0 (a picture of random motion from a fixed seed, not a simulation) | a random walk with a real diffusion coefficient (F2) |
| VSEPR molecules | atoms, bonds, domains | F1 (idealised teaching geometry from the VSEPR rule) | – | sourced geometry (PubChem 3D) would give F2; the case asks for the geometry status to be explicit either way |
| Heat diffusion | scalar field on a test mesh | – (test domain, not an object) | F2 (validated against an analytical solution) | not a lesson item |

What the scoring shows:

- **Only real data reaches F3**, and only in form (heart, kidney). Every authored biology part is F0 in both form and
  behaviour.
- **F2 exists only where a number could be looked up** (bond lengths, lattice constants, a field model), and only on one axis
  at a time. No item is above F0 in behaviour except the magnet's computed field and the heat fixture.
- **Behaviour is the weakest axis.** Everything that moves is hand-timed. That is the case for kits (C3, C4): a kit is how
  behaviour first reaches F1.

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
