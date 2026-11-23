# SVF handoff

Last updated: 2026-11-23 (late night, after C3, C4 and the C5 audit). Keep this file short. History lives in `docs/archive/HANDOFF-history-to-2026-11-23.md`
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
- **C3 and C4 are done** on local branch `c3-circuit-kit` (2 commits stacked on `c1-fidelity-register`, which was rebased
  onto `main` after #73-#83 were merged; no PRs, nothing pushed). Item documents (`fixtures/items/*.item.ron`) compile
  through kits (`svf_core::item`, the `Kit` trait, `svf_core::kits::{circuit, chemistry}`) to structure files. The Explorer
  opens them directly. Three items have been re-authored: the circuit, the water reaction and salt dissolving. Design,
  results, costs and limits: `docs/item-document.md`. Chemistry data has one source: `svf_core::chemistry_library`.
- **C5 step 1 (dataset audit) is done:** `docs/biology-dataset-audit.md`. BodyParts3D was chosen and downloaded to
  `datasets/bodyparts3d/`. Its meshes are open and come in pieces, and its licence notices conflict; both are recorded. The
  adaptation plan is in section 4 of that file.
- **C6 is prepared:** `docs/authoring-guide.md` is the page the user writes an item from.
- **C1 and C2 are done** on branch `c1-fidelity-register` (7 commits on `main`, local only, no PR yet): fidelity scores
  (`docs/item-output-bar.md`), smooth shading, `Turn` hinge (`about`), rounded routes (`corner_radius`), group `Grow`/`Turn`
  (`each`), and `Cone` plus `point_along` for direction markers. Specs: `docs/animation-design.md` sections 27 to 30. What was
  decided on the way, and what the user must look at: `docs/while-you-slept-2026-11-23.md`.
- The PR stack #73 to #83 is merged into `main`. New work stacks on `c3-circuit-kit`. **Nothing is merged without the user's
  explicit OK.** Never force-push.

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

1. **The user:**
   - Read `docs/while-you-slept-2026-11-23.md` (C1 to C4) and look at the evidence in `docs/evidence/c3-circuit-kit/` and
     `docs/evidence/c4-chemistry-kits/`.
   - Say whether to open PRs: C1+C2 (`c1-fidelity-register`), then C3+C4 (`c3-circuit-kit`, stacked on it).
   - Decided 2026-11-23: share-alike output for BodyParts3D geometry is accepted; the small intestine is in scope for C5.
2. **C6, the user:** write one item from `docs/authoring-guide.md` with no help, and note the cost (section 7 of the guide).
   It can run before C5: the circuit and chemistry kits are enough.
3. **C5 step 2 (Opus design first):** the BodyParts3D adaptation and centrelines, following
   `docs/biology-dataset-audit.md` section 4. Start with the capping design question there.

Model: C5's capping and centreline design is Opus; the OBJ reader and the kit wiring after that are Sonnet.

## Verified

- `cargo test --workspace --release --no-fail-fast`: 0 failures at `cad08f5` on `c3-circuit-kit` (core 476, render 23,
  explorer 87, every fixture suite, plus `simple_circuit_item` 9 and `chemistry_items` 6).
- 2026-11-23: the user hand-tested the three items live ("works"); only labels glitched. Fixed (uncommitted):
  `SceneRenderer::set_camera` now takes the surface aspect, so views and timeline cameras (fitted at 1280:800) no longer
  draw the scene at a different aspect from the label overlay after a resize. Workspace tests: 670 passed, 0 failed.
- Screenshots of each item, looked at by Claude, are in `docs/evidence/`. None have been judged live by the user.
