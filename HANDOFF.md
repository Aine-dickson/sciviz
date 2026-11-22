# SVF handoff

Last updated: 2026-11-23. Keep this file short. History lives in `docs/archive/HANDOFF-history-to-2026-11-23.md`
(everything up to this date) and `docs/MILESTONES.md`. Design contracts live in `docs/<feature>.md`.

## Objective

SVF (Scientific Visualization Framework) is a Rust workspace that shows *what something is and how its parts relate*, for
learners. It is the visualisation layer for Mava lessons, built standalone. Target: begin building lessons in January 2027,
with about 75% confidence that SVF produces valuable output across several subjects (`docs/EDUCATIONAL-DIRECTION.md`,
`docs/item-output-bar.md`, `docs/svf-prismal-boundary.md`).

## State

- Workspace: `svf-core` (f64, graphics-free), `svf-render` (wgpu 30, overlay text with fontdue and DejaVu Sans),
  `apps/explorer` (winit host).
- Built:
  - structures (RON) with blocks, sections, peeling, views and relations;
  - the adapted heart (Rodero case 01, F3 form);
  - the KiTS kidney capability probe (paused);
  - timelines (actions, particles, camera, opacity);
  - labels, colour key and in-picture captions;
  - twelve proof-of-concept items in biology, chemistry and physics.
- The proof-of-concept items have done their job: the **authoring convention is agreed** (`docs/authoring-convention.md`,
  decisions in section 9). Items are now polished towards reality through kits, one fidelity level at a time.
- Branch `b2-5e-heart-tool` was split on 2026-11-23 into the PR stack #73 to #83 (`docs/archive/branch-split-2026-11-23.md`).
  The top is `split/11-authoring-convention` (#83). **Start new work on a new branch off that top**, not off `b2-5e-heart-tool`, which is
  kept unchanged as a reference only. **Nothing is merged without the user's explicit OK.** Merge bottom-up: retarget the next PR's base
  to `main`, merge `origin/main` into its head, then squash. Never force-push.

## Decisions not to revisit

- The five item layers, the rules of the convention, and fidelity F0 to F3 per part on form, behaviour and appearance.
- Kits own domain rules; authors do not animate consequences by hand. The item document is RON and compiles to structure files.
- Datasets: any source is allowed when its copyright and licence terms are honoured (attribution; share-alike for derived
  geometry). Each source is audited before use, and the audit is recorded with the item.
- Labels: core computes the overlay and its layout, and the host draws it. The Explorer uses fontdue and DejaVu Sans.
- Animation is an SVF capability. The SVF/Prismal line is whether the learner runs an experiment.
- Claude leads technical direction; merges need the user's OK.

## Standing rules

- Never claim a build, test or runtime result without evidence. Keep "implemented" separate from "user-validated".
- Do not run `cargo fmt` or make formatting-only commits.
- Do not pop or drop the two stashes (`stash@{0}` is M6.1 formatting, `stash@{1}` is the pre-picking backup).
- `datasets/` is local only and gitignored: never commit patient or imaging data.
- Write multi-line or apostrophe-containing shell scripts to a file before running them. `curl` needs `--ssl-no-revoke` here.

## Known issues

- Known warnings (not blocking): unused `VectorSource3::new`, `prepare_imported_scene_with_options/_with_vectors`, and the
  `VseprDemo` fields.
- Labels: the anchor cost on heart-sized meshes has not been measured; the reference axis shows faintly through the caption
  panel; key swatches look lighter than the lit particles.
- The renderer shades each triangle flat, so curved surfaces look faceted (first item of C2).
- KiTS licence unresolved (CC BY-NC-SA vs CC BY 3.0); the kidney stays a paused probe.
- Still owed by the user: a live look at the ten timelines, caption wording and pacing, and citations for the authored items.
- PR #41 (VSEPR schematic view, 2026-11-09) is still open; check whether it is superseded before acting on it.

## Exact next action

1. **C1:** add a fidelity column (form / behaviour / appearance, F0 to F3) to the register in `docs/item-output-bar.md`, and
   score every item from what is recorded.
2. **C2:** the engine gaps, one slice each with tests and screenshots, in this order:
   1. smooth shading (vertex normals instead of the per-triangle normal in `mesh.wgsl`);
   2. a hinge pivot for `Turn`;
   3. lines-and-arcs paths and routing between named points;
   4. group transforms;
   5. direction markers (arrowheads).
3. **C3:** the circuit kit and the item-document compiler (Opus; short design note first). Re-author the circuit with them and
   compare the cost with `tools/structure-items/simple_circuit.py`.

Model: C1 and C2 are routine (Sonnet). Kit and item-document design is Opus.

## Verified

- `cargo test --workspace --release --no-fail-fast`: 0 failures at `a93e5b8` (core 457, render 20, explorer 87, and every
  fixture suite). The result for each split PR head is in `docs/archive/branch-split-2026-11-23.md`.
- Screenshots of each item, looked at by Claude, are in `docs/evidence/`. None have been judged live by the user.
