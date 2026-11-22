# Authoring convention and the high-level author surface

Last updated: 2026-11-22. Status: **agreed by the user on 2026-11-23** (decisions in section 9). It is derived from the proof-of-concept items
(register in `docs/item-output-bar.md`; build notes in `docs/animation-design.md` sections 15 to 26, `docs/labels.md`).

## 1. Purpose

The items built so far were experiments, not lesson material. They had one job: show how SVF items are actually built, so
that a standard can be written for high-level authoring. The register records the rule this note follows: *authors must never
meet the engineering; a high-level author surface is wanted, but only after the build convention has stabilised.* After
twelve items in three subjects the convention is visible. This note:

- states it as rules every item follows (section 3);
- defines **fidelity**, so items can be polished towards reality as far as the sources allow, as the heart was (section 4);
- proposes the author surface (section 5);
- ranks the engine gaps (section 6) and the realism work (section 7);
- gives a plan with the decisions the user must make (section 8).

## 2. What the experiments showed

| Item | Method | What it forced, or showed |
|---|---|---|
| Heart (Rodero case 01) | adapted from a published mesh | Real data gives realism and a measurable error (0.396 mm); cost is high but paid once per source. The only item near "reality". |
| Kidney (KiTS) | voxel segmentation | Data alone is not enough: a staircase surface needs smoothing before it is usable; licence questions can block an item. |
| Cockroach, plant cell, dicot stem, flower | authored from blocks | A semantic tree (parts, layers, views, relations) over geometry works for structure. Shapes are recognisable but schematic. |
| Beaker, stem water, breathing, gas exchange, digestive | blocks plus particles | Processes are told by a timeline. Particles carry meaning, but colours had to be named in captions (now solved by the key). Long tubes need lines-and-arcs paths; a rib cannot hinge. |
| Water reaction | positions computed, then written into the file | The chemistry (bond lengths, angles) had to be recomputed by hand: the molecule model in `chemistry.rs` is not usable from structure files. |
| Bar magnet | field traced by a generator | The physics lives in a throw-away script; the file only holds its output. |
| Salt (NaCl) | generator kept | 178 parts and about 200 actions; one `Grow` per ion. Crystal geometry belongs to the crystal, not to the author. |
| Simple circuit | generator kept, then labelled | A real physics mistake was possible (lamp lit 2.5 s after the switch closed): the author choreographed the consequence by hand. A hinge was faked with a turn plus a move. |

Five lessons follow.

1. **The semantic layer is right and stable.** Every item, in every subject, fitted "named parts in a tree, with descriptions,
   layers, views, relations, a reference". Keep it.
2. **The geometry layer is too low for authors.** Four of the last four items needed a program to compute positions. Authors
   cannot write those programs, so the knowledge in them must move into SVF as reusable kits.
3. **Consequences must come from rules, not choreography.** The circuit's lamp, the salt's hydration shell and the compass
   needle are consequences of physics or chemistry. When the author has to animate a consequence by hand, it can be wrong.
   A kit that knows the rule cannot make that mistake.
4. **Realism comes from sources, not effort.** The heart is realistic because its geometry came from data. Authored items
   stay schematic however carefully they are drawn. Polishing means finding a better source, or a better generator, for
   each part.
5. **Presentation is now uniform.** Labels, the key, captions, camera and opacity work the same way for every item. They
   are part of the convention, not per-item code.

## 3. The convention: what every item is and must have

An item has five layers. Each layer has one owner and one set of rules.

| Layer | Holds | Owned by |
|---|---|---|
| **Meaning** | parts tree (ids, names, descriptions), relations, layers | author |
| **Form** | geometry per part, each with a **source** and a **fidelity** (section 4) | kits and datasets, chosen by the author |
| **Story** | timelines: beats, captions and actions on parts | author, in story verbs (section 5) |
| **Presentation** | views, camera, labels, key, colours | author, with defaults from the convention |
| **Record** | reference, licence, fidelity statement, tests, evidence images, cost | the build, checked |

Rules:

1. **Ids and names.** Ids are lowercase with hyphens and are stable. Names are what a learner reads; a label uses the name
   unless it says otherwise. Every part with geometry has a one- or two-sentence description in plain English.
2. **Units are real.** Use real units (mm, cm or angstrom) at real size. When something is drawn at another scale (atoms at
   0.4 of their size, a speed that is not the real drift), the file states it and so does a caption, once.
3. **Colour conventions per subject:**
   - chemistry uses CPK colours for atoms (O red, H white, C dark grey, N blue, Cl green, Na purple);
   - circulation uses oxygenated red and deoxygenated blue;
   - plant tissues follow the stem's palette;
   - colours that carry meaning are explained by the **key**, never by naming colours in a caption.
4. **Labels.** Every part a learner must name has a label in at least one view or beat. Anchors are chosen where parts are
   close together (`docs/labels.md`). Labels persist by default, as in a textbook diagram.
5. **Captions.** One idea per caption, at most about 45 words, present tense, plain words. A caption never names a colour
   that the key already explains.
6. **Timing.** About 2.5 words per second of caption. A beat lasts as long as its caption. A consequence starts at the same
   instant as its cause (lesson 3).
7. **Camera.** The default view shows every labelled part. Pull the camera back before things leave the frame.
8. **Honesty.** Every simplification is written in the file and, where a learner could be misled, in a caption (for example,
   "not the reaction mechanism", or "electrons shown much faster").
9. **Tests.** Every item has a fixture test. It checks:
   - that meshes are closed and every drawn frame is closed;
   - the item's **scientific invariants**: conservation of atoms or ions, stated distances and angles, a consequence at the
     moment of its cause, and directions that agree with the field;
   - that the labels and the key appear when the story needs them.
10. **Evidence.** Screenshots at chosen times, looked at, plus a register row covering correctness, legibility,
    explorability, provenance, cost and fidelity.

## 4. Fidelity: how close to reality, per part

Fidelity is stated **per part** and on **three axes**, because a part can have real geometry and schematic behaviour (a CT
stomach with drawn food), or schematic geometry and exact behaviour (field lines of a computed field).

| Level | Form (geometry) | Behaviour (motion and process) | Appearance |
|---|---|---|---|
| **F0 schematic** | blocks: tubes, balls, extrusions | hand-timed moves | flat colour, faceted |
| **F1 sculpted** | smooth organic shapes from generators (lofts, lathes, smooth unions), proportions from references | rule-driven by a kit (a closed loop has current; a needle follows a field) | smooth shading, translucency, labels |
| **F2 measured** | dimensions and positions from published measurements (crystal data, component datasheets, anatomical atlases) | computed from a stated model (analytic field of a real magnet shape, random walk with a real diffusion coefficient) | material cues: glass, metal, wet tissue |
| **F3 source-derived** | geometry adapted from real data (CT/MRI segmentation, scans), with a measured error | measured or simulated data (flow fields, motion capture) | as F2 |

The policy for **"as far as possible"**: for every part, climb until no better source exists that is legal to use. Record
the level reached and what stopped it ("no open dataset of a dicot stem at cell level; F1 from textbook proportions"). An item's
fidelity is the list of its parts' levels, never one word.

Today almost every part is F0. Only the heart (and the paused kidney probe) is F3 in form. Only the bar magnet, the salt
and the water reaction are F2 in places (a stated field model; crystallographic spacing and ionic radii; textbook bond
lengths and angles). The per-part scores are in `docs/item-output-bar.md`, "Fidelity by part" (C1, 2026-11-23).

## 5. The author surface

### 5.1 Three layers

1. **Engine** (exists): structure files with blocks, timelines, particles, labels. These are written by kits and by
   engineers, not by authors.
2. **Kits** (new, in `svf-core`, tested): domain objects that know their geometry and their rules. Each kit turns a few
   author words into engine parts and actions.
   - The Python generators written for the experiments are the first drafts of kits: circuit, crystal lattice, field lines,
     molecule and reaction.
   - Each kit has tests of its science (lesson 3).
3. **Item document** (new, author-facing): what an author writes.
   - It names objects from kits and datasets, lists labels, and tells the story in **story verbs**.
   - It compiles to an engine structure file.
   - The author never writes a coordinate unless they choose to place something.

### 5.2 What an author would write (illustration, not final syntax)

```ron
Item(
    title: "A simple circuit with a switch",
    subject: Physics, idea: "A current flows only round a complete circuit.",
    objects: [
        Cell(id: "cell", kind: AA),                       // kit: real AA dimensions, terminals named
        Switch(id: "switch", kind: Knife, starts: Open),  // kit: lever with a hinge
        Lamp(id: "lamp", kind: FilamentMES),              // kit: holder, glass, filament
        Loop(["cell.negative", "switch", "lamp", "cell.positive"]),  // kit: wires routed with lines and arcs
    ],
    labels: ["cell", "cell.positive", "cell.negative", "switch", "lamp", "lamp.filament", "wire"],
    story: [
        Beat(say: "A simple circuit. The cell pushes charge round ...", show: All),
        Beat(say: "The switch is open ...", point_at: "switch"),
        Beat(say: "Close the switch ...", do: Close("switch")),   // the kit starts the current and lights the lamp at that instant
        Beat(say: "Open the switch again ...", do: Open("switch")),
    ],
)
```

The same shape covers the other subjects:

- **Chemistry:**
  - `Reaction("2 H2 + O2 -> 2 H2O")` builds the molecules from `chemistry.rs` (with the bond lengths and angles that
    module already holds), checks the atom balance, and choreographs the rearrangement.
  - `Crystal(NaCl, cells: 2)` builds a lattice from crystallographic data.
  - `Dissolve(crystal, in: Water, take: Corners(4))` builds the hydrated ions.
- **Physics:** `BarMagnet(length: 8 cm)`, `FieldLines(of: "magnet", count: 30)` and `Compass(follow: FieldLine(3))` are
  all computed from one field model.
- **Biology:**
  - `Organ(from: "<dataset>:stomach")` adapts real geometry, as for the heart.
  - `Flow(food, along: ["oesophagus", "stomach", "small-intestine"])` uses centrelines extracted from the organs,
    instead of hand-placed path points.

### 5.3 Story verbs

- **Presentation** (exist as actions): `say`, `show`, `hide`, `point_at` (highlight plus label), `fade`, `camera`,
  `label`, `key`.
- **Motion on parts** (exist): `move`, `turn` (gains a hinge), `grow` (gains groups), `colour`.
- **Kit verbs** (new): each kit adds its own, such as `Close`/`Open`, `React`, `Dissolve`, `Pump`, `Breathe`, `Flow along`
  and `Diffuse`. These are where the rules live.

### 5.4 Compile, don't interpret

The item document compiles to today's structure file. So:

- every engine feature, test and Explorer key keeps working;
- the compiled file can be inspected;
- a kit's output can be tested like any fixture.

Authors edit only the item document.

## 6. Engine gaps, ranked

Ranked by how many items hit the gap, and whether it blocks kits.

| Gap | Hit by | Note |
|---|---|---|
| Smooth shading (vertex normals) | every item | The renderer shades each triangle flat (`mesh.wgsl` takes the normal from screen-space derivatives), so balls and tubes look faceted. Cheapest, most visible realism step. |
| Hinge: `Turn` about a pivot | circuit switch, ribs (breathing), compass, flower opening | Removes the turn-plus-move workaround. |
| Paths from lines and arcs, and routing between named points | digestive, circuit, breathing airways, stem water | Wires, guts and airways. |
| Transforms on groups (`Grow`/`Turn` each child about its own centre, or the group as one body) | salt (64 grows), heart chambers, cell organelles | |
| Direction markers (arrowheads on tubes and paths) | bar magnet, circuit (conventional current), circulation | |
| Molecule bridge (`chemistry.rs` to structure parts) | water reaction, salt's water molecules, VSEPR | Single source of truth for bond lengths and angles. |
| Smooth organic shapes (lofts with varying sections, lathes, smooth union of blocks meshed with the existing marching tetrahedra) | organs, petals, cells, glassware | The F1 form step without data. |
| Centrelines from adapted meshes | flows through real organs | Needed once biology goes F3. |
| Label anchor cache | heart-sized meshes | From `docs/labels.md` known issues. |

## 7. Realism exemplars: one per subject, as far as possible

Polish one item per subject to the highest fidelity the sources allow. This proves the ladder and the kits on real cases,
just as the heart did for adapted data. The sources below are candidates. **Every licence must be audited before use**,
as the kidney showed. Share-alike licences in particular need the user's decision.

| Subject | Exemplar | Target | Candidate sources (to audit) |
|---|---|---|---|
| Physics | simple circuit | F2 form (real AA cell, MES lamp and wire gauge dimensions), F1 behaviour from a circuit kit | component datasheets |
| Physics | bar magnet | F2 behaviour: the analytic field of a uniformly magnetised cuboid instead of two point poles | textbook magnetostatics (closed-form cuboid field) |
| Chemistry | salt dissolving | F2 form from crystallographic data; F2 hydration geometry from published radial distribution data; molecule bridge for water | Crystallography Open Database (NaCl entry), published Na⁺/Cl⁻ hydration studies |
| Chemistry | water reaction | F2 molecules from the molecule bridge; honest F0 mechanism (stated) | `chemistry.rs`, PubChem 3D conformers |
| Biology | digestive system or breathing | F3 organ shapes from a whole-body CT segmentation dataset; flows along extracted centrelines | TotalSegmentator dataset, BodyParts3D, Z-Anatomy (licences differ: CC BY vs CC BY-SA) |
| Biology | flower or plant cell | F1 sculpted shapes from a published botanical reference; F3 only if an open micro-CT scan with a usable licence exists | MorphoSource (licences per specimen) |

## 8. Plan and decisions

Slices, each with a design check, tests and evidence as before:

1. **C1 Convention adopted.** The user agrees sections 3 and 4; the item register gains a fidelity column; existing items
   are scored F0/F1/F2/F3 per axis.
2. **C2 Engine gaps that every kit needs:** smooth shading, hinge, lines-and-arcs paths, group transforms, direction
   markers. Small and testable; routine work once designed.
3. **C3 First kit and item document end to end: the circuit.** It is the smallest domain, and it holds a rule (current
   starts with closure) that can be tested. Re-author the circuit as an item document. Compare its cost and its output with
   the throw-away build.
4. **C4 Chemistry kits:** the molecule bridge, reactions and crystals. Re-author the water reaction and the salt.
5. **C5 Biology realism:** a dataset audit, then one F3 organ system adapted (reusing the heart's region-surface and
   simplification pipeline), with centreline flows.
6. **C6 Author-surface review:** the user writes one new item in the item document without engineering help. Measure the
   cost. This is the test of the whole surface.

Decisions for the user:

- **D1:** agree the five layers and the rules (section 3), or amend them.
- **D2:** agree that kits own the domain rules, and that authors cannot animate a consequence by hand.
- **D3:** the exemplars and their order (section 7). Recommended: circuit (C3), then chemistry (C4), then biology realism
  (C5), because each needs the previous one's machinery less than the reverse.
- **D4:** whether share-alike (CC BY-SA) anatomy is acceptable for Mava lessons. This decides which biology sources are
  usable.
- **D5:** the form of the item document. It could be RON like today, a YAML-like text, or later a form-based editor that
  writes it. Recommended: RON first, because it compiles with the code that exists; the editor comes after C6.

## 9. Decisions (user, 2026-11-23)

- **D1 agreed:** the five layers and the rules of section 3 stand.
- **D2 agreed:** kits own the domain rules. Authors do not animate a consequence by hand.
- **D3:** Claude chooses the order. It is the order of section 8: C1, C2, then the circuit kit (C3), chemistry (C4) and
  biology realism (C5).
- **D4:** any dataset may be used as long as its copyright and licence terms are honoured. This covers attribution, and for
  share-alike sources, releasing derived geometry under the same terms. Find a way to blend sources in rather than rule a
  source out. Each dataset is still audited before use, and the audit is recorded with the item.
- **D5:** the item document is RON (already approved).

