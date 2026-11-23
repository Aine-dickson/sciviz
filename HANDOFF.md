# SVF handoff

Last updated: 2026-11-23 (night, after C1 and C2). Keep this file short. History lives in `docs/archive/HANDOFF-history-to-2026-11-23.md`
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
- **C1 and C2 are done** on branch `c1-fidelity-register` (6 commits off #83, local only, no PR yet): fidelity scores
  (`docs/item-output-bar.md`), smooth shading, `Turn` hinge (`about`), rounded routes (`corner_radius`), group `Grow`/`Turn`
  (`each`), and `Cone` plus `point_along` for direction markers. Specs: `docs/animation-design.md` sections 27 to 30. What was
  decided on the way, and what the user must look at: `docs/while-you-slept-2026-11-23.md`.
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
- Labels: the reference axis shows faintly through the caption panel; key swatches look lighter than the lit particles.
- KiTS licence unresolved (CC BY-NC-SA vs CC BY 3.0); the kidney stays a paused probe.
- Still owed by the user: a live look at the ten timelines, caption wording and pacing, and citations for the authored items.
- PR #41 (VSEPR schematic view) is **superseded**: `main` already has `vsepr_view.rs` in a newer form (checked 2026-11-23).
  Closing it is the user's call.
- A `Grow`/`Turn` on a part that has children now takes the children with it (group transforms); a parent that owns geometry
  cannot be scaled alone. Smooth shading is not applied to the plain `SceneRenderer::new` path (the parametric demo).

## Exact next action

1. **The user:** look at the scores in `docs/item-output-bar.md` ("Fidelity by part"), the evidence pictures in
   `docs/evidence/c2-smooth-shading/` and `docs/evidence/c2-markers/`, and the decisions in
   `docs/while-you-slept-2026-11-23.md`. Say whether to open a PR for the C1+C2 branch.
2. **C3 (needs Opus, `/model opus`):** the circuit kit and the item-document compiler. Start with a short design note (what
   a kit is in Rust; the item-document grammar in RON; how names such as `cell.negative` resolve to coordinates for
   `rounded_route` and `point_along`; how a kit declares and tests its rules). Then re-author the circuit with it and compare
   the cost with `tools/structure-items/simple_circuit.py`. The engine pieces it needs now exist (hinge, rounded routes,
   markers, group transforms, smooth shading).
3. C4 chemistry kits, C5 biology realism, C6 author-surface review (`docs/authoring-convention.md` section 8).

Model: C3 design is Opus; building a kit once its note is agreed is Sonnet.

## Verified

- `cargo test --workspace --release --no-fail-fast`: 0 failures at `9ac18ef` on `c1-fidelity-register` (core 474, render 23,
  explorer 87, and every fixture suite). The result for each split PR head is in `docs/archive/branch-split-2026-11-23.md`.
- Screenshots of each item, looked at by Claude, are in `docs/evidence/`. None have been judged live by the user.
