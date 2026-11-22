# Animation in SVF — design note (proposal for user review)

Status: **approved by the user on 2026-11-22; slice A1 done (see section 13).** Written 2026-11-22. Decided with the user: first slice is
poses and steps **plus** particle verbs; the author's words mirror Prismal's timeline words.
Everything below is "proposed" until the user approves it. Context: `svf-prismal-boundary.md`
(animation is an SVF capability; the line between tools is whether the learner runs an experiment),
`flower-structure.md` (findings 1 to 3), `item-output-bar.md`.

## 1. What it is for

A **fixed account that moves**, written in a few steps with the system doing the rest: growth stages,
a flower's ovary swelling into a fruit, water up a stem, diffusion, atoms moving together. The learner
watches, pauses, steps and scrubs. The learner does **not** set variables and compare outcomes; that is
Prismal's job.

## 2. Principles

1. **Pure function of time.** The picture at presentation time `t` is `pose_at(t)`. No state carried from
   frame to frame. This gives pause, scrub, replay, deterministic tests, and a screenshot at a given time
   (`--at <seconds>`). Prismal states the same principle for its animations (guide chapter 8, "Animation").
2. **Authors write steps and verbs**, never vertices, keyframe tables or equations.
3. **Mirror Prismal's words** where the idea is the same: `timeline`, `scene`, `beat`, `narrate "text" for 3 s`,
   `wait`, `hide`, `highlight`, `camera ... for d`, `ease`. SVF acts on **parts** where Prismal acts on
   representations. Beats play one after another; actions inside a beat run together; a beat lasts as long as its
   longest action.
4. **Not an experiment.** No learner-set parameters; only time controls (play, pause, step a beat, scrub).
5. **Particle verbs are generators, not physics**: seeded, deterministic, bounded. They show a process; they do
   not compute one.

## 3. Proposed file form

An optional `timelines` list beside `views`. File version stays 1; files without it load unchanged, as
`views` did. (RON syntax, to be fixed during slice A1.)

```
timelines: [
  Timeline(id: "pollination", name: "After pollination", scenes: [
    Scene(id: "wither", beats: [
      Beat(id: "fall", actions: [
        Narrate("The petals and stamens wither and fall.", for_s: 3.0),
        Move(part: "petals", by: (0.0, -12.0, 0.0), for_s: 3.0),
        Hide(part: "stamens", after_s: 3.0),
      ]),
      Beat(id: "swell", actions: [
        Narrate("The ovary swells into a fruit around the seeds.", for_s: 4.0),
        Grow(part: "ovary", to: 1.6, for_s: 4.0, ease: Smooth),
      ]),
    ]),
  ]),
]
```

## 4. Actions (first set)

| Action | Effect |
|---|---|
| `Show`, `Hide` | a part appears or disappears (instant, or scale-based over `for_s`; see 6) |
| `Move(part, by, for_s)` | translate |
| `Grow(part, to, for_s, about)` | uniform scale about the part's centre or a named anchor |
| `Turn(part, about, deg, for_s)` | rotate about an axis |
| `Colour(part, to, for_s)` | change colour |
| `Camera(yaw, pitch, zoom, for_s)` | move the camera, like a view's setting |
| `Narrate(text, for_s)` | caption text |
| `Wait(for_s)`, `Highlight(part)` | pause the timeline; mark a part |

`Grow` removes flower finding 1: the ovary grows into the fruit, so stage parts need not be duplicated.
Parts that are not mentioned stay as authored.

## 5. Particle verbs (second half of the first slice)

Generators over a part, seeded and deterministic.

- `Flow(count, speed, along: <part with a Tube path>, seed)`: water up a stem, blood in a vessel.
- `Spread(count, from: <point or part>, inside: <closed part>, seed, for_s)`: diffusion. Particles wander
  and **reflect at the container's wall**, using the point-inside-closed-mesh test the cockroach fixture
  tests already use (move it into core).
- `Meet` (atoms moving together to form a molecule): later; needs the chemistry module.

A particle's position at time `t` must be a pure function of `(seed, index, t)`, using a small PRNG
written for the purpose and no hash-ordered iteration, so results match on every platform.

## 6. Limits from what exists today

- The renderer is opaque (`blend: None` in `crates/svf-render/src/lib.rs`): **no fade**. `Hide` is instant or
  scales a part to nothing. Opacity is a separate renderer decision.
- The Explorer redraws on demand (`ControlFlow::Wait` in `apps/explorer/src/main.rs`). Animation needs a clock
  (`WaitUntil` while playing).
- A change of the visible set rebuilds the mesh (`replace_coloured_mesh` reallocates); a pose change with the
  same triangle counts can use `update_mesh` (a buffer write). So per frame: transform vertices per part range, then
  `update_mesh`. Visibility changes happen at step boundaries and may rebuild.
- Particles in the first cut are merged low-poly spheres (cap about 500, about 40k triangles) updated per frame.
  Renderer instancing only if an item needs more.
- **Performance is not measured.** Whether the heart's 80k triangles can be transformed and uploaded each frame
  comfortably is an estimate; measure it in slice A3.

## 7. Interaction with the existing features

- **Section, peel and isolate** apply to the **paused pose**. While playing they are disabled, because a cut
  per frame is too slow on dense meshes. (Open question 1.)
- **Picking** uses the posed mesh, so what is clicked is what is shown.
- **Views** stay as they are (camera and visibility presets). A timeline may start from a view.
- Tests sample `pose_at` at chosen times; no clock is needed to test.

## 8. Architecture (reusing what exists)

- `svf-core`: new module `structure_animation` (f64, no graphics, `forbid(unsafe_code)` as the crate):
  timeline types and validation against parts, layers and views; `Pose`; `pose_at`; particle generators;
  `apply_pose(&StructureGeometry, &Pose)`, which transforms vertices per `PartRange` and keeps triangle counts.
- `crates/svf-core/src/structure.rs`: the optional `timelines` field, validated like `views`.
- `crates/svf-render`: no change for the first slice (`update_mesh`, `replace_coloured_mesh`).
- `apps/explorer`: the clock, Space for play and pause, keys to step and scrub, the caption in the title, and `--at`.

## 9. Slices after approval

- **A1** core: timeline types, parser, validation, `pose_at`, tests sampling `t`.
- **A2** core: `apply_pose`; tests that moves keep volume, scaling multiplies it by k cubed, bodies stay closed.
- **A3** Explorer: clock, keys, caption, `--at`; measure the frame cost.
- **A4** flower: add the `pollination` timeline; compare with the stage-view build; record the saving in the register.
- **B1** core: `Flow` and `Spread`; tests for seeded determinism, particles staying inside the container, count conserved.
- **B2** items: diffusion in a beaker and water up the dicot stem (needs a longitudinal xylem tube).

Each slice: tests run by Claude, screenshots, register and handoff updated; the user judges legibility.

## 10. What this deliberately does not do

Physics or equations (Prismal), learner-set variables, graphs, sound or narration audio, video export,
vertex keyframes, opacity, instancing. Each can be reconsidered when an item needs it.

## 11. Decisions (user, 2026-11-22)

1. **Section, peel and isolate are disabled while playing and available when paused.**
2. **`Hide` and `Show` are instant for now** (they waited for the renderer). Scale-based hiding is **not** built.
   **Fade is built** (2026-11-22, `docs/opacity.md`): `Fade(part, to, for_s)` and per-part `opacity`.
3. **First particle item: diffusion in a beaker** (B2). Water up the stem follows.
4. **Captions only**, no sound.
5. **RON** for the file form; no separate text syntax.

## 12. Changes made while building A1 (so the note matches the code)

- Actions are RON struct variants with named fields, e.g. `Narrate(text: "...", for_s: 3.0)`; every timed
  action may take `at_s` (an offset inside its beat, default 0) and `ease` (`Linear` default, or `Smooth`).
- `Hide(part)` and `Show(part)` take effect at `at_s`; the first of them on a part decides whether the part
  starts shown (first is `Show`: hidden at the start; first is `Hide`: shown at the start).
- A `Camera` action needs the timeline's start camera (`camera: Some((yaw_deg, pitch_deg, zoom))`), since the
  core does not know the Explorer's fitted camera.
- A grouping part (no geometry) given to `Show`, `Hide`, `Move`, `Colour` or `Highlight` stands for its
  descendants, as in views. `Grow` and `Turn` need a part with geometry (they act about the part's own centre);
  a named anchor is deferred.
- Two `Grow`, two `Colour` or two `Camera` actions that overlap in time on the same target are refused.

## 13. Slice A1 result (2026-11-22)

Built in `crates/svf-core/src/structure_animation.rs`, wired into `structure.rs` (`timelines` field,
`Structure::timelines()`, four new `StructureError` variants) and `lib.rs`. No renderer or Explorer change.

- Actions: Narrate, Wait, Show, Hide, Highlight, Move, Grow, Turn, Colour, Camera (section 12 describes the form).
- `Timeline::pose_at(t)` is pure; `t` is clamped to the timeline; `beats()` gives start and end times for stepping.
- 13 unit tests, run by Claude in a release build, all passed: pure and repeatable poses, easing, groups versus
  `Grow`, chained `Grow`, first `Show`/`Hide` deciding the initial state, ordered turns, colour from the authored
  colour, camera from the start camera, captions, highlights, beat order, and refusal of bad input.
- Not done and not claimed: `apply_pose` (A2), the Explorer clock and keys (A3), any fixture timeline (A4), particle verbs (B1).
- Pivot note: `Grow` and `Turn` act about each part's own centre; the centre is chosen when the pose is applied (A2).

## 14. Slice A2 and A3 result (2026-11-22)

- **A2:** `StructureGeometry::apply_pose` (core). Scale about the bounding-box centre, then turns in order about the same
  centre, then offset; a pose colour replaces the part colour; a highlighted part is lightened 45% towards white (the renderer
  has no outline). Visibility is applied by building only the visible parts, not by `apply_pose`.
- **A3:** Explorer clock (`apps/explorer`). `--timeline <id>` loads a timeline paused at 0; `--at <s>` shows that moment (first
  timeline if none named) and, with `--screenshot`, writes it. Keys: Space play/pause (loads the first timeline if none),
  Left/Right scrub 0.5 s, `,` and `.` previous/next beat start, Home rewind, `T` next timeline. The caption and
  `paused|playing t/T s` are in the window title. The timeline camera is applied while a timeline moves (a mouse drag lasts until
  the next frame while playing); explicit `--camera-*` flags or `--view` win at start-up.
- Section, peel, isolate and view keys are refused while playing, and work on the paused pose (the section cuts the posed mesh).
- Part geometry is rebuilt only when the set of visible parts changes (a Show/Hide instant); every other frame is `apply_pose`
  on the cached geometry plus the upload.
- **Measured (scratch probe, release, not committed):** the cockroach, 82,016 triangles, `set_time` (pose and clone, no section)
  costs about 5.5 ms per call. The upload and the drawing were not timed by that probe; playing prints
  `pose + upload` per frame when you pause or the timeline ends. The heart (denser) has not been timed.
- Known limit: if a pose hides every visible part, the frame is refused ("no part with geometry would be visible") and the
  previous frame stays.
- Evidence: `docs/evidence/animation-a3-stem-at-5s.png` (stem, skin hidden, pith half-way up, from a scratch timeline).
  Live playback with a real window has **not** been run by Claude; the user has not tried the keys.

## 15. Slice A4 result: the flower's `pollination` timeline (2026-11-22)

- Added to `fixtures/structures/flower.ron`: 5 beats of 4 s (20 s): flower (anthers highlighted), pollen lands (stigma highlighted,
  camera moves in), pollen tube (style and ovules highlighted), petals and stamens fall while the ovary and ovules swell, the fruit
  stage replaces the carpel (Hide/Show, camera to the fruit). One new test, `the_pollination_timeline_swaps_the_flower_for_the_fruit`
  (poses at start, middle and end; all 41 poses built at 0.5 s steps are closed meshes). Run by Claude, release build, passing.
- Screenshots: `docs/evidence/animation-a4-flower/at-{1,6,10,15,20}.png` (via `--timeline pollination --at <s>`). I looked at 1, 15 and 20.
- **Saving versus stage views:** the three views of the flower still exist, but the story needed no extra parts and no new fixture:
  about 45 lines of RON on top of the existing parts. The "stages need separate parts" limit remains: the fruit is a second set of parts.
  The ovary swell hides the swap only roughly (ovary 1.7x against the fruit's 1.6x/1.8x radii), so the swap is a small jump.
- Camera values in a timeline are offsets from the fitted camera (as in views), so `pitch_deg: 0` is the default elevation, not level.
- Not judged by the user: legibility, wording of captions, and live playback.

## 16. Slice B1 result: particle generators in core (2026-11-22)

New `crates/svf-core/src/structure_particles.rs`. **Core only: no timeline action, no Explorer drawing, no fixture yet** (that is B2).

- `Spread` (diffusion): particles start in a small ball and take fixed-length steps in random directions. A step that would leave the
  container is tried in the mirror direction, then skipped, so nothing leaves and the count never changes. The walk is replayed once
  on a fixed time step when the spread is made; `position(i, t)` then reads and interpolates, so it is a pure function of the
  specification and `t` (clamped to the walk).
- `Flow` (water in a stem, blood in a vessel): particles travel along a polyline at a given speed, wrap from the end to the start, and sit
  at fixed offsets within a lateral radius of the centre line. Evaluated directly from `t`; not clamped.
- Containers: `Cylinder` (axis +Y), `Box3`, `Ball`, and `MeshContainer` (ray parity over a closed mesh; cost is one pass over the
  triangles per question, so low-polygon containers only).
- Determinism: SplitMix64 written here (first output for seed 0 is pinned in a test), only `+ - * /` and `sqrt` on floats, no hash-ordered
  iteration. Not yet checked on a second platform; the argument is by construction.
- Limits: 2000 particles, 20 million stored positions. Reflection is "try the mirror step, else stay", not a physical bounce; fine for a
  picture, and stated here so it is not mistaken for physics.
- 12 unit tests, run by Claude (release): known generator value, determinism and seed dependence, inside-container at every step, count
  fixed, start region, spreading then levelling off, clamping and interpolation, mesh container agreeing with the cube it encloses,
  bad input refused, flow near its path / at its speed / repeating after one lap / evenly spread.
- Design changes from section 5: none to the verbs. `Spread` takes a start ball and a container shape instead of "a point or part" and "a closed
  part"; connecting it to parts (a container taken from a part's own geometry, a path taken from a `Tube` part) is B2's job.
- Decision for B2 (not needing Opus): particles are drawn as merged low-poly spheres in one extra mesh per frame, as section 6 says.

## 17. Slice B2 result: particles in timelines, and the diffusion beaker (2026-11-22)

- **Two new actions** in `structure_animation.rs`: `Spread(id, count, from, radius, inside, step, seed, particle_radius, colour, for_s, at_s)`
  and `Flow(id, count, path, speed, lateral, seed, particle_radius, colour, for_s, at_s)`. `inside` is an explicit shape written in the action
  (`Cylinder`, `Box` or `Ball`), as proposed in the handoff: no part geometry is read, so the author states the water region. `Pose.particles` lists
  the groups running at time `t`. A `Spread` appears when it starts and rests where it ended; a `Flow` is shown only while it runs. Ids must be unique
  within a timeline; bad input is refused with a reason (count, start outside the container, colours, radii, path, negative times).
- **Drawing:** `StructureGeometry::with_particles` adds each particle as a closed 20-triangle sphere after the parts (and after any section cut, so a
  cut does not slice them). Particle triangles belong to no part, so they are not pickable. The Explorer shows them with no other change.
- **Fixture:** `fixtures/structures/diffusion-beaker.ron`: base and back wall of a beaker (front half left out; water is not drawn because the
  renderer is opaque), a 28 s timeline `spreading` (3 s intro, then 150 dye particles for 25 s with three captions). Schematic; says so in the file.
- **Tests (run by Claude, release):** core particles 14, timeline particle actions 3 new, `tests/diffusion_fixture.rs` 2 (inside the water at 281 times, tight
  drop then spread, every drawn frame closed), Explorer 1 (particles drawn, not parts, gone after scrubbing back).
- **Cost:** about 0.15 ms per `set_time` for the beaker with 150 particles (scratch probe, release; no section, no upload).
- Screenshots: `docs/evidence/animation-b2-beaker/at-{0,4,12,28}.png`; I looked at 4 and 12. A first attempt put the half wall on the camera side
  (angles run from +x towards -z in `Sector`); the fixture uses `first_angle_deg: 225.0` for the back.
- **Not done:** water up the dicot stem (needs a longitudinal xylem path in the stem fixture; the `Flow` action exists and is tested only on a plain path),
  atoms combining (`Meet`), fade or opacity (water and glass are both opaque; the front half is simply absent). The user has not seen any of it live.

## 18. Opacity built (2026-11-22)

See `docs/opacity.md`. New action `Fade(part, to, for_s, at_s, ease)`; `PartPose.opacity`; a part faded to nothing is left out of the scene.
The beaker now has a translucent glass wall and water (no more cut-away); the flower's petals and stamens fade as they fall.
The to-do "fade and opacity in the renderer" is done. Still not built: scale-based hide, `Meet`, water up the stem.

## 19. Water up the dicot stem (2026-11-22)

`fixtures/structures/dicot-stem.ron` now has the timeline `water-up` (24 s, 3 beats): the whole stem; the tissues in the way fade
(epidermis to 0.1, pith to 0.12, cortex, phloem, cambium and medullary rays to nothing, xylem to 0.5) while the camera drops a little;
then eight `Flow` actions, one up the middle of each xylem bundle (14 particles each, 1.2 mm/s along a 12 mm path, so a lap is 10 s), with two captions.
- **Finding:** first try, fading everything to about 0.12, was unreadable (the translucent layers merged into one yellow-green mass).
  Taking the layers that are in the way out completely and keeping the xylem at 0.5 with bright cyan water worked. Lesson for later items:
  translucency is for context, not for stacking five layers; fade the ones in the way to nothing.
- **Limits:** straight paths and a wrap from top to bottom (direction, not a real circuit or speed); no roots or leaves are drawn, so "from the
  roots to the leaves" is carried by the caption; the paths were computed from the `Sector` angle convention (+x towards -z).
- **Tests (run by Claude, release):** `tests/dicot_stem_fixture.rs` (water inside the xylem radius, height and bundle angle at 301 times; fades; every
  drawn frame closed). Three Explorer tests that appended their own timeline to the stem text now use `plain_stem()`, which cuts the file at its
  `// Timelines.` marker; keep that marker when editing the fixture.
- Screenshots: `docs/evidence/stem-water/at-{1,8,12,20}.png` (8 and 12 retaken after the fix; I looked at 12). Not seen live by the user.

## 20. Breathing item (2026-11-22)

`fixtures/structures/breathing.ron`: rib cage (7 rib rings by a `Linear` repeat, plus a sternum), diaphragm (flattened ellipsoid), two lungs
(one part, `mirror: X`, translucent), windpipe and bronchi (translucent tubes). Timeline `breathing`, 29 s: at rest (5 s), breathing in (7 s),
breathing out (7 s), then two quicker breaths (10 s). In: the diaphragm moves down 1.8 cm, the ribs and sternum move up and out and the ribs grow
8%, the lungs grow 15% and sink 0.5 cm, and air particles flow down both airways. Out: the reverse, with the flow paths reversed so the air goes up.
- **This item needed no new core code**; it uses `Move`, `Grow`, `Fade`-capable parts, `Flow` and part opacity as built. The only generated part is the rib path
  (computed once from an ellipse and written into the file; the numbers are not meant to be edited by hand).
- Camera values: `yaw_deg: 45` is the front (+Z) view from the fitted camera, which starts at the +X/+Z corner; my first guess of -40 gave a side view.
- **Limits:** a rib cannot hinge (a `Turn` is about the part's own centre), so "ribs up and out" is a lift plus a 8% growth, not a swing; one `Grow`
  scales the whole part uniformly, so the diaphragm only moves, it does not flatten; the particles wrap from the end of a path to its start, so they
  show direction, not a count of air; the lungs sink slightly so they stay on the diaphragm's dome, which the real lungs do through the pleura (not drawn).
- **Not built:** gas exchange in the air sacs (a zoomed second timeline with oxygen and carbon dioxide), the heart and blood (TRN), and the 12 rib pairs.
- Tests (`tests/breathing_fixture.rs`, run by Claude, release): parts closed; beats start at 0, 5, 12, 19 and everything returns after each breath;
  air stays within 0.2 cm of the airway paths and goes down on breathing in and up on breathing out; every drawn frame is a closed mesh.
- Evidence: `docs/evidence/breathing/at-{3,7,12}.png` (looked at). Not seen live by the user.

## 21. Gas exchange in an air sac (2026-11-22)

`fixtures/structures/gas-exchange.ron`: a hollow translucent sphere (the air sac, wall 0.8 um, opacity 0.3) and a translucent tube (the capillary, opacity 0.35)
touching it. Timeline `gas-exchange`, 39 s, 5 beats: the sac; blood arriving (dark cells up the lower half of the capillary, bright cells up the upper half);
oxygen (blue) crossing from the sac into the blood; carbon dioxide (yellow) crossing the other way; the blood leaving.
- **No new core code.** Two-way gas movement is two sets of short opposite `Flow` streams across the wall, not a new verb. A real gas moves by random motion;
  the streams show direction only (stated in the file). Because a `Flow` wraps from the end of its path to the start, each gas stream loops, which reads as continuous crossing.
- Blood colour does not change as cells pass; two fixed groups stand for "before" and "after" (stated in the file). A proper version would need a particle colour that
  depends on position, which is not built.
- A section along Z shows the sac cut open (a view, `inside`); the gases lie on the Z=0 plane so the cut passes through the streams (not looked at).
- Captions name the colours (blue dots, yellow dots) because there is no legend. A legend or label mechanism does not exist yet; the same gap exists in the beaker and stem items.
- Tests (`tests/gas_exchange_fixture.rs`, run by Claude, release): parts closed and translucent; beat starts 0, 7, 15, 23, 31; blood stays within 1.1 um of the capillary axis and in its half;
  oxygen goes towards the capillary and carbon dioxide towards the sac; every drawn frame closed.
- Evidence: `docs/evidence/gas-exchange/at-{3,12,20,28,36}.png` (28 looked at, before and after enlarging the gas dots). Not seen live by the user.

## 22. The digestive system (2026-11-22)

`fixtures/structures/digestive-system.ron`: mouth, oesophagus, stomach, liver, small intestine (a coil of 5 rows with half-circle turns) and large intestine
(ascending, transverse, descending, sigmoid, rectum) in front of it. All hollow-looking organs are translucent (0.35 to 0.4); the liver is 0.75. Timeline `digestion`,
75 s, 8 beats: introduction; mouth (chewing: a `Spread` in a ball); oesophagus (a `Flow` of swallowed food); stomach (a `Spread` churning in a ball); liver (highlight and caption only);
small intestine (a `Flow` along the whole coil, plus four short streams of nutrients leaving backwards through the wall); large intestine (a slower `Flow` of leftovers, plus four
streams of water leaving sideways); the end (waste leaves).
- **One core change, small: `Spread` gets `stay` (default true).** With `stay: false` the particles disappear when their walk ends instead of resting where it ended. Without it the chewed food
  stayed frozen in the mouth and stomach for the rest of the film. The beaker still uses the default. One new test in `structure_animation.rs`.
- **Authoring finding:** a long winding tube cannot be written as a few corner points; the tube-bend check (curvature radius must exceed the widest half-width) rejects Catmull-Rom curves through
  uneven points. The fixture's paths were generated as exact lines and arcs sampled at an even spacing of 0.9 cm (turn radius 1.1 for the small intestine, 2.0 for the large) and written out.
  The generator is not kept in the repository; the numbers in the file are the source. A "path from lines and arcs" helper in the structure format would remove this chore (candidate for later,
  needs design).
- **Limits:** lengths and widths are not to scale; the particles do not change colour as food is digested (the stomach and small intestine use a darker tan than the mouth); the loop wraps from the end of
  a path to its start; nutrients and water just leave through the wall, there are no blood vessels or a colon wall to cross; a person's stomach is on the viewer's right (the person's left), as drawn.
- Tests (`tests/digestive_fixture.rs`, run by Claude, release): parts closed and translucent; beat starts 0, 8, 16, 24, 34, 42, 56, 68 and duration 75; chewing inside its ball and gone after its beat;
  churning inside its ball and gone; swallowed food within 0.3 cm of the gullet's line; small-intestine food inside its region; nutrients and water move outwards; leftovers inside the large intestine's region;
  every drawn frame closed.
- Evidence: `docs/evidence/digestive/at-{12,28,48,60}.png` (48 and 60 looked at, 48 again after the `stay` change). Not seen live by the user.

## 23. Hydrogen and oxygen make water: the first chemistry item (2026-11-22)

`fixtures/structures/water-reaction.ron`: six atom parts (4 H white, 2 O red, as balls of radius 0.28 and 0.45 angstrom, not real atomic sizes), three starting bonds
(two H-H singles, one O=O double as a two-copy `Linear` repeat) and four O-H bonds for the water molecules. Timeline `reaction`, 39 s, 5 beats: the starting molecules (9 s);
the old bonds fade out (`Fade`) while the oxygen atoms move apart (5 s); the hydrogen atoms move and join them (6 s); the new bonds appear (`Show`, drawn at the atoms' final positions)
(9 s); the atom count and the equation (10 s).
- **No new core code.** The atoms keep one position per instant (parts moved by `Move`). Bonds do not follow atoms: the old bonds fade before the atoms move, and the new bonds, authored at the
  atoms' final positions, appear after they arrive. A general "bond that follows its two atoms" would need a new action; not built, and not needed for a fixed account.
- **Chemistry checked by the test:** H-H 0.74, O=O 1.21, O-H 0.96 angstrom and H-O-H 104.5 degrees to 1e-9; each oxygen ends with exactly two bonded hydrogens and is more than 2.5 angstrom from
  the other water's hydrogens; six atoms are always shown and opaque; no two atoms overlap at any of 391 sampled times (so the choreography is collision-free).
- **Not the mechanism.** In reality hydrogen burns through several steps with short-lived particles. The file says so. The learning idea is conservation: the same atoms, joined differently.
- **Two worlds.** The existing molecule model (`chemistry.rs`, `vsepr.rs`, Explorer `--vsepr`) is not used here, because timelines work on structure files. A bridge (build a structure from a
  `Molecule`) would avoid hand-authoring and keep one source of truth for bond lengths and angles; it is a design decision (Opus), not started.
- Offsets in the file are written to 9 decimals; with 4 the bond-length check failed at 1e-4. Keep the precision if the file is regenerated.
- Evidence: `docs/evidence/water-reaction/at-{1,16,30}.png` (looked at). Not seen live by the user.

## 24. The magnetic field of a bar magnet (2026-11-22)

`fixtures/structures/bar-magnet.ron`: a bar magnet (two tube parts, north red at +x and south blue at -x), five field-line parts (each one line in the xy plane, copied six times round the x axis
with a `Radial` repeat), a compass needle (a tapered tube) and eight compass needles placed round the magnet. Timeline `field`, 56 s, 5 beats: the magnet and its poles (lines and compasses hidden);
a compass is moved from the north pole to the south pole and turns to stay along the field (14 `Move` and 14 `Turn` actions, one second each); the lines appear (`Show`); a caption about
strength; the eight compasses appear. No new core code.
- **The field is a model, stated in the file:** the magnet is two point poles (+1 at x = +3.6, -1 at x = -3.6 cm). Each line was traced by a fourth-order Runge-Kutta integration of the unit field direction
  from the north pole until it re-entered the magnet; needles and the compass's turns come from the same function. The numbers in the file are the source (the generator is not kept). It is the textbook
  picture, not a computed field of a real magnet, and the field inside the magnet is not drawn.
- **Authoring finding (a use for `Turn`):** a needle that pivots about its own centre and is carried along by `Move` follows a field line well, because the compass needle is a single part
  that is symmetric about its centre. This works for any "pointer that follows a path" (a compass, a wind vane). A needle with two coloured halves would not, because each part turns about its own centre.
- **Camera lesson:** the field lines of a magnet reach far from it (a line leaving 20 degrees off the axis goes 40 cm out for a 8 cm magnet). The first set of start angles made the magnet a speck;
  the file uses 62, 78, 95, 112 and 130 degrees, which keep the field to about 13 cm.
- **Not built:** arrows on the lines (there is no arrow-head verb, so direction is shown by the compass and the pointed needles); two magnets attracting or repelling (a second timeline would need the
  field of two magnets); iron filings; a legend (colours are named in the captions only).
- Tests (`tests/bar_magnet_fixture.rs`, run by Claude, release): parts closed; each of the five lines leaves near the north pole, ends at the south pole, and its direction of travel agrees with the stated field
  (every step cos > 0.9, mean > 0.99); the lines reach farther the shallower they start; the eight needles point along the field (cos > 0.9999); the moving compass is within 0.05 degrees of the field direction at every whole second
  and within 12 degrees between, and stays outside the magnet; lines and compasses appear in the stated order; every drawn frame closed.
- Evidence: `docs/evidence/bar-magnet/at-{3,14,28,56}.png` (looked at). Not seen live by the user.

## 25. Salt: an ionic lattice that dissolves in water (2026-11-22)

`fixtures/structures/salt-dissolving.ron`, generated by `tools/structure-items/salt_dissolving.py` (kept this time: 178 parts and about 200 actions are not hand-editable). A 4 x 4 x 4 piece of the
rock-salt lattice (32 Na+ purple, 32 Cl- green), 12 lattice-line parts (four tubes per direction, each copied four times by `Linear`), and 24 water molecules (each a group of three overlapping balls).
Timeline `dissolving`, 90 s, 9 beats: the crystal at real relative size (ions touching); every ion shrinks to 0.4 (`Grow`) and the grid lines appear; one sodium ion and its six chloride neighbours are marked
while the rest fade to 0.1; the rest come back and the count (32 and 32, so NaCl); the grid fades and twelve water molecules move in to the four top corners, oxygen ends to Na+, hydrogen ends to Cl-; a sodium ion leaves
along its corner diagonal with its three water molecules while three more join on its inner side; a chloride ion the same; the other two corners together while the camera pulls back (`Camera`); the closing caption.
- **No new core code.** A second `Fade` or `Grow` on a part already starts from where the previous one ended (the chaining pass in `structure_animation.rs`); a unit test for fade, grow and colour chaining was added
  while checking this (`a_later_fade_grow_or_colour_starts_where_the_previous_one_ended`).
- **Sizes:** ions are authored at Shannon ionic radii (Na+ 1.02, Cl- 1.81 angstrom, 6-coordinate) on a 2.82 angstrom spacing, so at the start they touch, as in the real crystal (1.02 + 1.81 = 2.83, the test allows 0.011).
  After the shrink everything, water included (0.4 of the van der Waals radii O 1.52, H 1.20), is drawn at the same 0.4.
- **Simplifications, stated in the file:** six water molecules on the three axes round each freed ion (really about 5 to 6 for Na+, 6 to 7 for Cl-, always moving); for Cl- both hydrogens face the ion equally (really one O-H points at it);
  only four corner ions leave, in turn, along straight diagonals.
- **Grow is per part.** `Grow` does not pass from a group to its children (each body scales about its own centre), so the shrink is one `Grow` per ion: 64 actions. A group-wide "each child about its own centre"
  grow would help lattice items; not built. Add it to the authoring-chores list.
- **Camera:** the timeline pitch is an offset from the fitted camera: -25 looked almost side-on, -55 lost the cube, -40 with zoom 1.8 works; the Explorer's `--camera-pitch` replaces the timeline camera instead of adding to it.
  The first two freed ions end on the same side and overlap in projection at the end; another yaw or other leaving corners would separate them.
- Tests (`tests/salt_dissolving_fixture.rs`, 5, run by Claude, release): rock salt (32/32, radii, every nearest neighbour at 2.82 is of the other kind, inner ions have six); beat order, shrink, highlight and fades; ions always
  shown; each freed ion ends with one water molecule on each of the six axes at 2.4 (Na+) or 3.2 (Cl-) angstrom, O-H 0.96, H-O-H 104.5, turned the right way, and more than two spacings from the crystal; no two balls overlap at any
  0.1 s step; every drawn frame (every 2 s) is a closed mesh.
- Evidence: `docs/evidence/salt-dissolving/at-{2,14,25,44,56,90}.png` (looked at: 14, 25, 44, 56, 90). Not seen live by the user.

## 26. A simple circuit with a switch (2026-11-22)

`fixtures/structures/simple-circuit.ron`, generated by `tools/structure-items/simple_circuit.py`. In the XY plane, in centimetres: a cell on the bottom wire (negative end left, positive nub right), a switch on the left wire
(two contact balls and a lever), a filament lamp on the top wire (holder, two supports, a coiled filament, translucent glass, a glow ball shown only when lit), and three translucent copper wires built from lines and
quarter-circle arcs. Timeline `circuit`, 65.5 s, 5 beats: the parts; the open switch (no current); close, current and energy (one beat: the lever closes in 1.5 s, and the electrons and the lamp start the moment it arrives and stop the moment it lifts);
the lamp goes out; summary with conventional current.
- **No new core code.** Electrons are one `Flow` on a closed path (first point = last point), so they wrap with no jump. The path runs through the switch, up one support, along the filament, down the other and through the inside of the cell,
  where the opaque case hides them. 66 electrons at about one per centimetre, 2 cm/s.
- **A lever without a hinge:** `Turn` about the lever's own centre plus a `Move` of that centre from the open to the closed midpoint, same duration and ease. The ends follow a chord, not an arc, so the pivot end wanders a little mid-turn;
  the test bounds it (within 0.25 cm of the pivot at every 0.1 s). Good enough for a switch; a real pivot for `Turn` is still on the chores list.
- **Caught on a screenshot, not by a test:** the first draft had a separate "close" beat, so the lever was down for 2.5 s before the current started. The physics was wrong (a closed circuit has a current at once). Fixed by putting the closing
  in the current beat, and the test now ties the flow and the lamp to the lever's closed window.
- **Tube limits:** a zigzag filament was refused ("bends more tightly than its width allows"); a sine coil (two waves over 1.4 cm, amplitude 0.15, half-width 0.05) passes.
- **Camera:** yaw 40, pitch -20, zoom 1.3 gives a near-front view. Yaw -35 turned the circuit edge-on and pitch +5 looked down from high above: positive yaw turns towards the front for a scene in the XY plane facing +Z.
- **Not built:** a circuit diagram (symbols) next to the real circuit, which textbooks pair with this picture; an ammeter; conventional-current arrows (no arrow-head verb). The colours are named only in captions.
- Tests (`tests/simple_circuit_fixture.rs`, 5, run by Claude, release): parts closed, beat starts; at every 0.1 s the electrons flow and the glow shows exactly while the lever is down (the flow also draws at its end instant, when the lever is only starting to lift),
  and the filament is fully bright or fully dark outside its 0.3 s ramps; electrons lie in the circuit and pass the switch and the filament within a lap; the open lever is more than 1 cm from the contact, the closed lever lies along
  the gap and touches it, and the pivot end stays put while turning; every drawn frame (every 0.5 s) is closed.
- Evidence: `docs/evidence/simple-circuit/at-{5,21.8,30,50}.png` (all looked at). Not seen live by the user.
