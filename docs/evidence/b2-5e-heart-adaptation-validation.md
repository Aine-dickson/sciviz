# B2 step 5e — Rodero heart adaptation validation

Date: 2026-11-18

## Result

The Rodero 2021 case 01 heart has been adapted into the committed structure
fixture `fixtures/structures/heart-case01.ron` and
`fixtures/structures/heart-case01.svfmesh`.

User-reported Windows validation on branch `b2-5e-heart-tool`:

- Adaptation command: `cargo run -p svf-core --release --example heart_adapt -- ../Downloads/rodero2021/01-svf-order.vtk --out-dir target/heart-adapt-diagnostic-10 --deviation 0.4 --thin-deviation 0.4`.
- Repeated diagnostic runs 9 and 10 produced identical results.
- Adapted surface: 369,806 to 51,456 triangles; final parts: 80,786 triangles.
- Worst measured deviation: 0.396 mm, within the 0.4 mm target.
- No duplicate labelled face keys after assembly.
- All 21 parts are closed.
- Right atrium wall volume difference: +0.05%; right atrium cavity: -0.17%.
- All other parts passed their configured volume tolerances; thin parts use the 10% tolerance.
- Output container size: 1,455,110 bytes.

## Sectioning correction

The heart fixture exposed a section-plane coincidence edge case on dense geometry.
`crates/svf-core/src/structure_section.rs` was updated to find a clear gap in
vertex projections rather than relying on a fixed set of tiny plane shifts.

User-reported regression result:
- `plane_cuts_on_all_three_axes_succeed_and_stay_closed`: passed.
- 98 cuts across all three axes succeeded.
- Slowest cut: 18.6696 ms.
- Every resulting section passed the closed-mesh assertion.

## Fixture integration validation

User-reported command:

```bash
cargo test -p svf-core --release --test heart_fixture
```

Result: 9 passed, 0 failed, 0 ignored (2.78 seconds).

Coverage includes fixture structure and provenance, heart frame orientation,
relations and views, layer peeling, named chamber cuts, all-axis section sweeps,
closed outward parts with source-volume checks, and non-overlap with an empty
allow-list.

The build reports one existing warning: `VectorSource3::new` is unused in
`vector_visualization.rs`. It does not affect the test result.

## Commits

- `3e62a38` — Find a clear section plane gap on dense meshes.
- `5229a66` — Add validated adapted heart fixture (user-created local commit).

The user confirmed `git status --short` was empty after commit `5229a66`.
The fixture commit is currently local until pushed.

## Remaining work

- Push the user's local commit to `origin/b2-5e-heart-tool`.
- Perform a live Explorer check of the heart structure: loading, framing,
  parts/layers, chamber views, picking, peeling, and sectioning.
- Capture and inspect screenshots for the heart fixture.
- Run broader workspace validation after any Explorer changes.
- Update the main `HANDOFF.md` with the final 5e state and next milestone.
