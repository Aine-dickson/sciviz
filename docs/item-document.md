# Item documents and kits (C3 design note)

Status: **decided by Claude (lead mode), 2026-11-23**. This is the C3 design check in `docs/authoring-convention.md` section 8.
The user has not reviewed it yet. It covers four questions: what a kit is in Rust; the item-document grammar in RON; how names
such as `cell.negative` resolve; and how a kit declares and tests its rules. The first kit is the circuit (section 6).

## 1. The pipeline

```
item document (RON, author)  --parse-->  ItemDocument
        --kits build objects-->  parts + names + rule state
        --story is timed, verbs go to their kit-->  absolute-time actions
        --kits emit consequences from their rules-->  more absolute-time actions
        --actions are put into beats-->  StructureDocument  -->  Structure (engine)  /  RON text (inspection)
```

The output is an ordinary `StructureDocument`, so it gets the engine's whole validation, every engine test and every Explorer
key (section 5.4 of the convention: compile, don't interpret). The structure document types now derive `Serialize` so the
compiled file can be written out and read. Code: `svf_core::item` (document, names, timing, compiler) and
`svf_core::kits::circuit`.

## 2. What a kit is in Rust

A kit is a module under `svf_core::kits` that owns one domain. A kit does not implement a shared trait yet. With one kit,
a trait would only guess at what C4 needs. The compiler calls each kit through three steps, and C4 will show whether a trait
is worth having.

1. **Build.** The kit takes its objects from the item and returns:
   - its engine parts (with names and descriptions);
   - its **names** (section 4);
   - a **fidelity** per part (form, behaviour and appearance, F0 to F3, with the source);
   - its **notes**: simplifications for the file header (convention rule 8);
   - an optional preferred camera.
2. **Verbs.** The compiler passes each kit verb (`Close("switch")`) with its absolute start time. The kit checks it against
   its state (you cannot close a closed switch). It returns the motion actions (the lever's hinge turn) and records *when the
   cause happens* (the lever touches the contact 1.5 s later).
3. **Consequences.** After the whole story, the compiler asks the kit for the actions that follow from its rules over the
   whole timeline. The circuit's rule is: current flows if and only if the loop is complete. The kit works out the intervals
   when every switch in the loop is closed, and for each interval emits the electron flow, the lamp's glow and colour, and
   the electron key. Each starts at the instant the interval starts and stops at the instant it ends. **The author cannot
   emit these.** No story verb lights a lamp. This is decision D2 made mechanical.

Actions from steps 2 and 3 carry absolute times. The compiler puts each one into the beat it starts in, with `at_s` relative
to that beat. Every compiled beat gets an explicit `length_s`, a new engine field, so a current that starts in one beat can
run on until a later beat opens the switch. Without it, a beat would stretch to its longest action. The field is covered by
a unit test in `structure_animation.rs`.

## 3. The item-document grammar (RON)

```ron
Item(
    id: "simple-circuit",                 // lowercase, hyphens: the root part's id
    title: "A simple circuit with a switch",
    subject: Physics,                     // Physics | Chemistry | Biology
    idea: "A current flows only round a complete circuit.",
    reference: "…",                       // the account the item follows (the record layer)
    objects: [ Cell(id: "cell"), Switch(id: "switch"), Lamp(id: "lamp"), Loop(["cell", "switch", "lamp"]) ],
    labels: ["cell"],                     // labelled from the start (they persist)
    camera: None,                         // default: the kit's preferred camera
    notes: [],                            // extra honesty notes for the file header
    story: [
        Beat(say: "…", point_at: ["cell", "wire", "switch", "lamp"]),
        Beat(say: "…", do: [Close("switch")]),
        Beat(say: "…", label: ["lamp.filament"], hold_s: Some(8.0)),
    ],
)
```

Fields of a beat (all but `say` are optional):

| Field | Meaning |
|---|---|
| `say` | The caption. It runs for the whole beat. |
| `point_at` | Names to highlight and label. They are spread evenly through the beat, so four names in a 12 s beat come at 0, 3, 6 and 9 s. A label stays once it is shown. |
| `label` | Names to label at the start of the beat, with no highlight. |
| `unlabel` | Names whose label is removed at the start of the beat. |
| `do` | Kit verbs, all starting at the beat's start. |
| `camera` | `Some((yaw_deg, pitch_deg, zoom))`: the camera moves there over the first 1.5 s. |
| `hold_s` | The beat's length, overriding the timing rule. |

**Timing (convention rule 6).** A beat lasts `max(3 s, words / 2.5 per second, the longest verb + 1 s)`, rounded up to 0.5 s.
`hold_s` replaces this. The compiler, not the author, writes every `for_s` and `at_s`.

**Objects.** `objects` holds one enum, `Object`, with a variant for each kit object: `Cell`, `Switch`, `Lamp` and `Loop` today.
C4 adds chemistry variants to the same enum. An object's `id` follows rule 1 (lowercase, hyphens, no dots).

## 4. Names

Every object adds names to one table. A name is either an object id (`cell`) or `<object>.<piece>` (`cell.negative`,
`lamp.filament`). A name can stand for any of:

- a **part**: what `point_at` highlights. Its engine id is the name with the dot turned into a hyphen: `lamp.filament`
  becomes part `lamp-filament`;
- a **point**: where a label is anchored, and where a route can start or end;
- a **label**: the default text and side. `cell.negative` is labelled "Negative terminal (−)" at the flat end, on the left.

Unknown names are an error, and the error lists every name the item has. This makes a misspelt label a compile error, not a
missing label. The circuit's names are listed in section 6.

Routing between named points is done by the kit that owns the route: the loop resolves its members' terminal points and
routes the wires between them. A generic `Route(from: "a.x", to: "b.y")` waits for a second user, the digestive flow in C5.

## 5. How a kit states and tests its rules

- The rule is code in the kit: `CircuitKit::consequences` derives every consequence from the switch states over time.
- A kit test checks the rule on the compiled output, not on the kit's internals. For every 0.1 s, the electrons flow and the
  lamp is lit **if and only if** every lever in the loop is at its contact. A second item, with two switches in series,
  proves the rule is general and not tuned to one story. The current flows only while both are closed.
- Item fixture tests (convention rule 9) check closed meshes in every drawn frame, labels and key, and the real dimensions
  the kit claims.
- Misuse is an error with a reason: closing a closed switch, a loop without exactly one cell, an object in two loops, an
  unknown name.

## 6. The circuit kit

**Objects.**

- `Cell(id, kind: AA)`. The size is set from the LR6 (AA) limits in IEC 60086-2: 14.5 mm across and 50.5 mm long, with a
  5.5 mm positive contact. **The user still has to check these values against the standard** (owed citation).
- `Switch(id, starts: Open | Closed)`. A knife switch with a 3 cm lever on a hinge.
- `Lamp(id, kind: Mes)`. An MES (E10) filament lamp with an 11 mm glass bulb, a holder and two filament supports.
- `Loop([cell, …])`. The series loop. It starts at its one cell. The other members go round a rounded rectangle in
  the electrons' direction, and the kit routes the wires between them. This is what places the objects, so the author
  writes no coordinates.

**Layout.** The loop is a 16 cm × 10 cm rounded rectangle in the XY plane, with corners of radius 1.5 cm.

- The cell sits on the bottom side, with its negative terminal on the left.
- The other members go on the left, top and right sides, in order: one member goes on top; two go left then top; three
  go left, top and right. With more than three, the extra members go on the top.
- Wires are drawn 5 mm thick, about four times a real 7/0.2 hook-up wire. That is stated in the notes.

**Names.**

| Object | Names |
|---|---|
| Cell | `cell`, `cell.body`, `cell.top`, `cell.positive`, `cell.negative` (a point and a label on `cell.body`) |
| Switch | `switch`, `switch.lever`, `switch.pivot`, `switch.contact` |
| Lamp | `lamp`, `lamp.holder`, `lamp.supports`, `lamp.filament`, `lamp.glass` (and the hidden `lamp.glow`) |
| Loop | `wire`, and `wire.1` to `wire.n` in the electrons' order from the cell |

**Verbs.** `Close(switch)` and `Open(switch)` swing the lever 35° about its pivot in 1.5 s, using the `Turn` hinge from C2.

**Rule.** A current flows round the loop if and only if every switch in it is closed. A switch counts as closed from the
instant its lever arrives at the contact. It counts as open from the instant the lever starts to lift. While a current flows:

- the electrons move as one `Flow` along the whole loop at one speed, because the current is the same all round a series
  loop. They leave the negative terminal and come back through the cell;
- every lamp in the loop glows;
- the key shows "Electron".

**Fidelity.**

| Parts | Form | Behaviour | Appearance |
|---|---|---|---|
| Cell, lamp | F2 (datasheet sizes, still to be checked) | F1 (the kit's rule) | F1 (smooth shading, glass) |
| Switch | F0 | F1 | F0 |
| Wires | F1 (rounded routes) | F1 | F1 |

## 7. Result and cost comparison (2026-11-23)

Built: `svf_core::item`, `svf_core::kits::circuit`, the item document `fixtures/items/simple-circuit.item.ron`, its tests
`crates/svf-core/tests/simple_circuit_item.rs` (9 tests), and the engine field `Beat.length_s`. The Explorer opens
`*.item.ron` directly (`--structure fixtures/items/simple-circuit.item.ron --timeline story`). To read the compiled file, run
`cargo run --release -p svf-core --example compile_item -- <item.ron> <out.ron>`. Evidence: `docs/evidence/c3-circuit-kit/`
(open at 20 s, closed at 32 s, after opening at 75 s).

| | Throw-away build (`simple_circuit.py`) | Item document |
|---|---|---|
| What the author writes | 154 lines of Python that compute coordinates, plus hand timing | 40 lines of RON: no numbers at all |
| Coordinates written by the author | every one | none |
| Timing written by the author | every `at_s` and `for_s`; the 2.5 s lamp mistake happened here | none: the captions set the pace, and the rule sets the consequences |
| Lamp and current | choreographed by hand | emitted by the kit's rule; the author cannot write them |
| A second circuit (two switches) | a new script | a different `objects` list (tested) |
| Labels | anchors computed by hand | names, with anchors and text from the kit |
| Fidelity record | written by hand in the register | emitted per part in the compiled file's header |

The engineering cost is about 1,160 lines of Rust (518 for the compiler, 642 for the kit), plus 282 lines of tests. It is
paid once per kit. The compiler is shared by every kit.

What this does not show yet: whether an author who is not an engineer can write the document. That is C6.

Known limits:

- one loop per item;
- cells, switches and filament lamps only;
- the layout is fixed (a 16 × 10 cm rectangle);
- a flow is drawn at its end instant (an engine convention), so for one instant electrons show while the glow has just gone;
- the AA and MES sizes still need the user's citation check.

## 8. Two kits: the `Kit` trait (C4)

With a second kit, the three steps of section 2 became a trait, `svf_core::item::Kit`, with three methods: `verb_seconds`,
`apply` and `consequences`. The compiler picks the kit from the item's objects. An item uses one kit, because kits differ in
units (cm or ångström) and layout. Mixing kits is an error with a reason.

## 9. The chemistry kit (C4)

**Single source of chemistry data.** `svf_core::chemistry_library` holds every number the kit uses:

- bond lengths: H–H 0.74, O=O 1.21, N≡N 1.10, O–H 0.96, C=O 1.16, C–H 1.09 and N–H 1.01 Å;
- angles: H–O–H 104.5°, H–N–H 107.8°, and tetrahedral for methane;
- the NaCl lattice constant, 5.64 Å;
- Shannon ionic radii: Na⁺ 1.02 and Cl⁻ 1.81 Å;
- Bondi van der Waals radii and CPK colours;
- hydration distances: Na–O 2.4 and Cl–O 3.2 Å.

From these it builds `chemistry::Molecule` values for H₂, O₂, N₂, H₂O, CO₂, CH₄ and NH₃. This is the molecule bridge: the
`chemistry` module's types, turned into parts by the kit. **The user still has to check these values against the CRC
Handbook and the published hydration studies** (owed citation).

**Objects and verbs.**

| Object | Verbs | What the kit does |
|---|---|---|
| `Reaction(id, equation: "2 H2 + O2 -> 2 H2O")` | `React(id)` | Parses the equation and refuses it unless every element balances. It lays the reactants out in a column on the left and the products on the right, and pairs the atoms of each element top to bottom. `React`: the old bonds fade over 1 s, every atom moves to its place in a product over 3 s, and the new bonds appear when the atoms arrive. Atoms are never shown, hidden or faded. |
| `Crystal(id, kind: NaCl, edge: 4)` | `OpenUp(id)`, `Dissolve(crystal, ions)` | Builds the lattice at the Shannon radii. `OpenUp` shrinks every ion to 0.4 in place and shows the grid lines. `Dissolve` takes corner ions in a fixed order (Na⁺, Cl⁻, Na⁺, Cl⁻ on top, then the bottom four). Three waters arrive at the outer faces, the ion leaves along its corner diagonal with them, and three more arrive at the faces it uncovered. Each water is turned by the kit: oxygen towards Na⁺ at 2.4 Å; for Cl⁻, one O–H bond points straight at the ion, with O at 3.2 Å. |

**Names.**

- A reaction: `<id>.h2-1`, `<id>.o2-1`, …, for the reactant molecules, and `<id>.h2o-1`, … for the new molecules' bonds,
  plus `<id>.reactants` and `<id>.products`.
- A crystal: `<id>`, `<id>.sodium`, `<id>.chloride`, `<id>.lines` and `<id>.water`; `<id>.a-sodium-ion` and
  `<id>.a-chloride-ion` (the ions nearest the middle, with six neighbours); and `<id>.corner-1` to `<id>.corner-8` in
  dissolving order.

**Keys.** The kit emits the key for every element shown, so captions never name colours (convention rule 5). Water's oxygen
and hydrogen join the key when the first water arrives.

**Tests** (`crates/svf-core/tests/chemistry_items.rs`, 6 tests; and a library unit test):

- atoms are conserved, and never hidden or faded;
- after `React`, every oxygen has two hydrogens at 0.96 Å and 104.5°;
- a second reaction from the same kit (methane burning) ends with a straight CO₂;
- unbalanced equations, unknown formulas and a second `React` are refused;
- the lattice has 32 + 32 ions at their radii, with nearest neighbours of the other kind at 2.82 Å;
- every dissolved ion has six waters at the hydration distance, turned the right way;
- no two drawn balls overlap at the end;
- every frame is closed, and the key is right.

**Result.**

- `fixtures/items/water-reaction.item.ron`: 27 lines, against a hand-computed structure file.
- `fixtures/items/salt-dissolving.item.ron`: 48 lines, against the 218-line `salt_dissolving.py` and about 200
  hand-placed actions.
- Evidence is in `docs/evidence/c4-chemistry-kits/`.

**Known limits.**

- One reaction or one crystal per item.
- Only corner ions dissolve.
- Inner waters arrive along the leaving diagonal, so they pass close to the ion.
- The dissolved sodium's label is hidden inside its water shell (the anchor is on the ion).
- In the reaction the products sit to the right of the reactants, so the left of the picture is empty afterwards.
