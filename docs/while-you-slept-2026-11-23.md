# Notes left for the morning (2026-11-23 night session)

The user went to sleep and asked Claude to keep working through the plan and leave behind anything they need to know.
Newest entries last. Nothing here is merged; nothing was pushed. Branch: `c1-fidelity-register` (off #83).

## C2.1 Smooth shading (done, committed)

- The renderer now gives meshes smooth vertex normals (`with_smooth_normals` in `crates/svf-render/src/lib.rs`, shader
  `mesh.wgsl`). A vertex is split into one copy per group of faces within a 40 degree crease angle, so balls and tubes are
  smooth while boxes, caps and steps keep sharp edges. Winding is ignored (the shader is two-sided). Colour and opacity
  are copied to every copy of a vertex.
- **Decision (mine, easy to change):** crease angle 40 degrees, constant `CREASE_ANGLE_DEGREES`.
- **Decision:** the plain `SceneRenderer::new(window, mesh)` path stays flat on purpose, because `update_mesh` (the
  parametric demo) needs a fixed vertex count. Structures, coloured meshes and scalar/isosurface scenes are smoothed
  (unshaded scalar surfaces are not touched).
- Evidence: `docs/evidence/c2-smooth-shading/` (salt, water, breathing, circuit); the old flat shots are in the other
  evidence folders for comparison. Looked at by Claude only.
- Tests: 3 new renderer tests (cube stays sharp, sphere smooth and unsplit, mixed winding / zero-area face / colour kept).
  Workspace tests: 0 failures.
- Cost on the heart: the whole Explorer start plus screenshot takes 0.68 s (`heart.png` in the evidence folder), so no cache is needed.

## C2.2 Hinge for Turn (done, committed)

- `Turn(..., about: Some((x, y, z)))` swings a part about that point (RON, optional; old files unchanged). Spec in
  `docs/animation-design.md` section 27. The hinge point travels with `Move`; `Grow` does not shift it.
- **Decision:** `PartPose` gained `turn_pivots` parallel to `turns` instead of changing the `turns` tuple, so existing
  code and tests are untouched.
- The circuit switch and the bar-magnet compass are NOT retrofitted; the circuit is re-authored in C3.

## C2.3 Rounded routes (done, committed)

- New module `structure_route` plus RON `corner_radius` / `point_spacing` on `Tube` and on the `Flow` action. Spec in
  `docs/animation-design.md` section 28.
- **Decision:** routing between *named* points is left to the C3 item-document compiler (it resolves names to coordinates);
  the engine only rounds corners. A tube with `corner_radius` takes exactly one `half_widths` entry (a varying width along a
  routed tube is not supported; say if you want it).
- **Decision:** a radius too big for its straight run is an error, never silently shrunk.

## C2.4 Group transforms (done, committed)

- `Grow` / `Turn` on a group scale or turn the whole group as one body about its combined centre; `each: true` does each
  part about its own centre. Spec: `docs/animation-design.md` section 29.
- **Behaviour change to know about:** `Grow`/`Turn` on a part that has children now takes the children too (before, only the
  part itself). No fixture was affected; one old unit test that asserted the old rule was rewritten.
- **Decision:** one part may not be grown/turned both inside a group and on its own in the same timeline (refused with a
  message), because one part has a single scale and a single centre.
- The salt item still has its 64 separate grows; it is re-authored in C4.
- **Limit:** a parent that owns geometry cannot be scaled without its children (split its own geometry into a child part).

## C2.5 Direction markers (done, committed)

- New `Cone(base, tip, radius)` geometry block and `structure_route::point_along(route, fraction)`. Spec:
  `docs/animation-design.md` section 30; picture: `docs/evidence/c2-markers/demo.png`.
- **Decision:** an arrowhead is a separate cone part, placed by a kit using `point_along`; there is no one-piece "arrow"
  primitive yet, and no moving arrowhead. Say if you want either.

## C2 is complete. What I did NOT do, and why

- **C3 (circuit kit and item-document compiler) is not started.** Your policy says kit and item-document design is Opus, and
  it begins with a design note whose choices (what a kit is in Rust, the item-document grammar, how names such as
  `cell.negative` resolve, how kits declare their rules) have long-term consequences. This needs an Opus session. It should
  start with `/model opus` then read `docs/authoring-convention.md` sections 5 and 9, then the notes above.
- Nothing is merged or pushed. The branch is `c1-fidelity-register` (6 commits on top of #83).

## Also checked

- **PR #41 (VSEPR schematic view) is superseded.** `origin/main` already has `crates/svf-core/src/vsepr_view.rs`, in a newer
  form than the PR head. I did not close it; that is yours to do (`gh pr close 41`).
- **Please look at, in this order:** the "Fidelity by part" scores (the magnet's field lines are F2 though the model is crude),
  then the pictures in `docs/evidence/c2-smooth-shading/` and `docs/evidence/c2-markers/`, then the behaviour change in C2.4.
- Nothing was merged, pushed or deleted. Stashes untouched. No `cargo fmt`.

## Second night session: C3, C4, the C5 audit and C6 preparation

The user asked Claude to carry on until all steps were done. This session ran on Opus, so the C3 design was done here.

**Branches.**

- `main` already had #73 to #83, so `c1-fidelity-register` was moved onto `main` (`git rebase --onto origin/main ae472fc`;
  the trees were checked to be identical first).
- New work is on `c3-circuit-kit`, stacked on it. Nothing was merged or pushed, and no PR was opened.

**C3 (commit "C3: item documents …").**

- **Decision:** an item document compiles to an ordinary structure document, so the engine keeps all of its checks. Kits
  return actions with absolute times, and the compiler puts them into beats. Design note: `docs/item-document.md`.
- **Engine addition:** `Beat.length_s`. With it a beat has a fixed length, and an action can run on into later beats (the
  current runs from Close to Open). The structure document types also now derive `Serialize`.
- **Decision:** the circuit's rule. The current flows from the instant the lever touches the contact. It stops from the
  instant the lever starts to lift. The lamp and the electron key follow the current. The author cannot write any of these.
- **Decision:** the circuit kit sets the layout (a 16 × 10 cm loop; cell at the bottom, then left, top, right). The cell and
  lamp are at AA and MES datasheet sizes, and the wires are drawn about four times thicker than real. The AA and MES
  values must be checked against their sources (your citation check).
- **Known, accepted:** a flow is drawn at its end instant (an engine convention from before), so for that one instant the
  electrons show while the glow is already off. The tests allow exactly that instant.
- **Cost:** the circuit is now 40 lines of RON with no numbers, against a 154-line generator plus hand timing.

**C4 (commit "C4: chemistry kits …").**

- `chemistry_library` is now the one place for bond lengths, angles, radii, colours and hydration distances.
- Reactions refuse unbalanced equations, and atoms only rearrange.
- The salt crystal has `OpenUp` and `Dissolve`. Water turns its oxygen end to Na⁺; for Cl⁻, one O–H bond points straight at
  the ion.
- **Decision:** one kit per item (a circuit and a reaction cannot be mixed), because the units differ.
- **Decision:** cameras are the kits' defaults:
  - circuit: yaw 40, pitch -20, zoom 1.3;
  - reaction: 45, -27, 1.2;
  - crystal: 30, -40, 1.4, chosen so the dissolved ions stay in the frame.
- Captions were carried over from the PoC items, with colour names removed because the key explains colours.
- Limits are listed in `docs/item-document.md` section 9. **Please judge the pictures** in `docs/evidence/c4-chemistry-kits/`.

**C5: audit only.**

- BodyParts3D was chosen and downloaded to `datasets/bodyparts3d/`, which is local and git-ignored.
- **Its licence notices conflict:** the README says CC BY 4.0; the files say CC BY-SA 2.1 JP. Plan: honour both. **Please
  confirm.**
- The meshes are open and some come in pieces, so the adaptation needs new capping and centreline tools.
- Claude stopped there and did not rush them. The plan is in `docs/biology-dataset-audit.md` section 4.

**C6:** this needs you. `docs/authoring-guide.md` is the page to write an item from, and it lists what to note.
