# Animation in SVF — design note (proposal for user review)

Status: **proposal; nothing is implemented.** Written 2026-11-22. Decided with the user: first slice is
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

## 11. Open questions for the user

1. Section while playing: disabled (proposed), or allowed with a slow-frame warning?
2. Is scale-based hide acceptable until the renderer supports opacity?
3. Which first particle item: diffusion in a beaker, or water up the stem?
4. Narration: text captions only (proposed), with sound deferred?
5. Should the file form stay RON (proposed, consistent with `structure.ron`), or should timelines get their own
   Prismal-like text syntax? RON keeps one parser and one validator; a text syntax would match Prismal more closely
   but costs a second parser.
