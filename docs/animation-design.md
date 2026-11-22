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
2. **`Hide` and `Show` are instant for now** (they wait for the renderer). Scale-based hiding and fade
   are **not** built. To do: fade needs opacity in the renderer (`blend: None` today); do it as a
   renderer decision when an item needs it.
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
