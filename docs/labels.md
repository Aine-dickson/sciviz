# Labels, keys and captions in the picture

Last updated: 2026-11-22. Status: **design contract, proposed by Claude**. Dependency decided by the user on 2026-11-22: `fontdue` with DejaVu Sans (section 6).

## 1. Why now

Ten items have timelines, and every one that uses colour to mean something names those colours in a caption
("electrons (blue)", "sodium ions (purple)"): beaker, stem, breathing, gas exchange, digestive, water reaction,
bar magnet, salt, circuit. Nothing in a picture says what a part is unless the learner clicks it, and the
captions themselves go to the window title, so they are missing from every screenshot. `explorable-structures.md`
already promised "simple labels attached to parts" and views that carry labels; neither was built. A textbook
diagram is mostly its labels, so this is the largest gap between the items and a usable lesson picture.

## 2. Three things, one overlay

| Thing | What it is | Example |
|---|---|---|
| **Label** | text pointing at one part, with a leader line | "Filament", "Switch", "Chloride ion" |
| **Key entry** | a colour swatch and text, pointing at nothing | blue dot = "Electron"; purple = "Sodium ion, Na⁺" |
| **Caption** | the narration already in the pose, drawn in the picture | the timeline captions |

All three are computed in `svf-core` as one host-neutral **overlay** for a frame (texts, pixel positions, leader
segments, swatch colours). The Explorer draws the overlay; a web host (Mava) could draw the same overlay with HTML.
Core stays graphics-free: it never measures or rasterises text. The host passes text sizes in.

## 3. Authoring

- **Views** get `labels: ["part-id", ...]` (labels shown in that view) and `key: [...]` (entries, below).
- **Timeline actions**, which persist until they are removed, like `Show`/`Hide`:
  - `Label(part: "filament", text: Some("Filament (very thin wire)"), at: Some((x, y, z)), side: Some(Left), at_s: 0.0)`.
    `text` defaults to the part's name, `at` (anchor) defaults to section 4's rule, and `side` defaults to the rule in section 5.
  - `Unlabel(part: "filament", at_s: 0.0)`.
  - `Key(id: "na", colour: (0.60, 0.45, 0.90), text: "Sodium ion, Na⁺", at_s: 0.0)` and `Unkey(id: "na", at_s: 0.0)`.
- **Particles** (`Spread`, `Flow`) get an optional `key: Some("Electron")`. While the particles are shown, a key
  entry with their colour is shown too. Most "named in the caption" cases are particles, so this removes them
  with one field.
- A label on a group part (no geometry) points at its descendants (anchor rule below). Unknown part ids are
  refused when the file loads, as for every other action.
- `Pose` gains `labels: Vec<PartLabel>` (part, text, posed anchor, side) and `key: Vec<KeyEntry>` (colour, text),
  both in authored order and pure functions of time, like the rest of the pose.

## 4. Anchors

- **Default anchor:** the part's mesh vertex nearest to its bounding-box centre. It is on the surface (unlike the
  centre itself, which for a tube or a shell is in empty space), it is stable, and it is computed once. For a
  group part, use the nearest such vertex over its descendants.
- **Authored anchor** `at`: a point in structure units, for when the default is a poor spot (a long tube,
  an inner chamber).
- **Moving parts:** the anchor follows the part's pose (scale, turns, offset), using the same transform as
  `apply_pose`. A label on the moving compass or a leaving ion stays on it.
- **When a label is not drawn:**
  - its part is hidden or faded below 0.2 opacity;
  - its anchor is behind the camera or outside the viewport;
  - its anchor is hidden behind another opaque part. This is one ray test from the camera to the anchor,
    reusing the Explorer's picking. Translucent parts do not hide anchors, so labels inside the beaker, the
    lamp glass or a faded lattice still show.

## 5. Layout: the textbook callout columns (core, pure, tested)

Input: the viewport size in pixels; for each label, its projected anchor and its measured text box; and a
reserved band at the bottom for the caption and a box at the top-right for the key.
Output: each label's box position and its leader segment (from the anchor to the nearer edge of the box).

1. **Side:** a label goes in the left column if its anchor is left of the picture's centre, otherwise in the
   right column. An authored `side` overrides this.
2. Within a column, sort by the anchor's y, put each box at its anchor's y, then push boxes apart to a minimum
   gap and clamp them inside the viewport, keeping the order.
3. Column x is a fixed margin from the viewport edge. The leader runs from the anchor to the box's inner edge.

Guarantees, all tested:
- boxes never overlap;
- boxes stay inside the viewport and outside the caption band and the key box;
- within a column the vertical order of boxes equals the order of their anchors, so leaders in one column
  do not cross;
- the same input always gives the same output.

Known limitation: when an anchor crosses the centre during a move, its label jumps sides. The `side` override is
the fix when it matters; no hysteresis, because the layout is a pure function of one frame.

## 6. Drawing in the Explorer (`svf-render`)

- **A screen-space overlay pass, drawn last with no depth test:**
  - text as textured quads from a glyph atlas (one R8 texture, built as glyphs are first needed);
  - a dark half-opaque panel behind each text, for legibility over any colour;
  - leaders as screen-space lines;
  - key swatches as small quads.
- Screenshots go through the same render, so they will show labels and captions. This also fixes the captions
  being missing from the evidence images.
- **Captions:** a bottom band, wrapped to the viewport width. The window title keeps the time only.
- **Proposed dependency:** `fontdue` 0.9.5 (MIT, Apache-2.0 or Zlib; a small pure-Rust TrueType rasteriser),
  plus one embedded font with a licence that allows bundling. Candidates are DejaVu Sans (Bitstream Vera
  licence) and Noto Sans (SIL OFL 1.1). The font must have the subscript and superscript characters the
  captions already use (₂ ⁺ ⁻). L3 checks every string in every fixture against the font.
- **Rejected:**
  - `glyphon`: heavier (it brings a full shaping stack), and each release is tied to one wgpu version, which
    would have to match our pinned wgpu 30. Not verified which wgpu 0.12 targets.
  - A built-in stroke font with no dependency: ASCII only, so no ₂ or ⁺; one-pixel lines that read poorly in
    screenshots.

## 7. Slices

| Slice | Content | Tests |
|---|---|---|
| L1 | Core data: `Label`/`Unlabel`/`Key`/`Unkey` actions, particle `key`, view `labels`/`key`, default and posed anchors, `Pose.labels`/`Pose.key` | parse and refuse; persistence over time; group anchors; anchor follows Move/Turn/Grow; particle key appears and disappears with the particles |
| L2 | Core layout (`structure_labels.rs`): columns, ordering, gaps, clamping, leaders; occlusion and viewport filtering take a caller-supplied ray test | the guarantees above, plus property-style tests over many random anchor sets |
| L3 | `svf-render` overlay: fontdue atlas, overlay pipeline, panels, leaders, swatches | atlas packing; every fixture string has all its glyphs in the font; one rendered test frame has non-background pixels where a label must be |
| L4 | Explorer: caption band, labels, key; `--screenshot` includes them | screenshots of the circuit, salt and flower, looked at |
| L5 | Retrofit: keys on the particle items, labels in the circuit, salt and one heart view; captions drop "(blue)"-style colour naming where a key now does the job | the fixture tests still pass; new screenshots |

L1 and L2 need no new dependency. L3 adds `fontdue` and DejaVu Sans (user's decision, 2026-11-22).

## 8. Not in scope

- 3D text in the scene and billboards.
- Choosing labels automatically.
- Translation.
- Rich text.
- Clicking a label.
- Labels that avoid each other's leaders across columns.
- Smarter occlusion than the one anchor ray.
- Colour bars for scalar fields. That is a separate legend kind; it would reuse the overlay but not this layout.

## 9. Progress

- **L1 done (2026-11-22).** `crates/svf-core/src/structure_labels.rs` holds `PartLabel`, `KeyEntry`, `LabelSide`, `KeyDocument` and `Structure::label_anchor`.
  Timelines gain the `Label`, `Unlabel`, `Key` and `Unkey` actions and an optional `key` on `Spread`/`Flow`; `Pose` gains `labels` and `key`.
  Views gain `labels` and `key`. `apply_pose` and the anchors share `pose_point` and `range_vertices_and_centre`, so they cannot drift apart.
  7 unit tests: persistence, replacement, particle keys without repeats, refusals, views, default and group anchors, and anchors under Move/Turn/Grow.
  The Explorer does not use them yet (that is L4).
- **L2 done (2026-11-22).** `crates/svf-core/src/structure_label_layout.rs`: `layout_labels(&LabelFrame, &[LabelRequest]) -> Vec<PlacedLabel>`, plus
  `Camera3::project_to_viewport` (the inverse of `ray_from_viewport`, tested as a round trip). When a column overflows, the labels set last are dropped.
  Hiding by another part (the anchor ray test) is left to the host, which has picking. 7 layout tests: placement, side override, push-apart order,
  bands, rejects, overflow, and 500 random scenes checked against every guarantee and for determinism.
- **L3 done (2026-11-22).** `svf-render`: `overlay.rs` (CPU only: `TextKit` with fontdue and a 1024 x 1024 glyph atlas packed in shelves,
  text measuring and word wrapping, and `compose_overlay`, which lays out the caption band, the key box and the labels through the core layout) and
  `overlay_gpu.rs` + `overlay.wgsl` (one alpha-blended pipeline of screen-space quads drawn last in `encode_scene` with depth compare Always, so
  screenshots include it). `SceneRenderer::set_overlay` and `surface_size` added. Sizes scale with the target height (1200 px reference): labels 22 px,
  captions 26 px. 5 tests, including one that checks every character of every fixture file has a glyph in DejaVu Sans (no missing ₂ ⁺ ⁻ →).
- **L4 done (2026-11-22).** Explorer: `StructureScene::overlay_items` (labels from the pose, or from the current view when there is no timeline;
  anchors projected with the target's camera; labels on hidden parts, or on parts faded below 0.2, are dropped, and so is a label whose anchor is
  hidden by another opaque part, using one ray test). The overlay is rebuilt every redraw and at the capture size before `--screenshot`. The caption has
  left the window title. 2 Explorer tests: the circuit overlay, and occlusion by an opaque part but not a translucent one.
- **L5 started:** the simple circuit has 7 labels and an `Electron` key on its flow; "(blue)" was dropped from its caption. Evidence:
  `docs/evidence/labels/circuit-at-{5,30}.png` and `salt-at-44-caption.png` (looked at). First authoring lesson: two leaders crossed until the
  "Cell" label was anchored on top of the cell; anchors are worth choosing, not leaving to the default, when parts are close together.
- **Known issues:**
  - `label_anchor` copies the vertex list on every call; on the 80k-vertex heart this cost has not been measured.
  - The reference axis line shows faintly through the caption panel (panel alpha 0.8).
  - The key swatch is drawn unlit while the particles are lit, so the swatch looks a little lighter than the particles.
- **Next:** L5 for the other particle items (beaker, stem, breathing, gas exchange, digestive) and the salt item, dropping colour names from
  their captions where a key now says it; then labels in one heart view.

