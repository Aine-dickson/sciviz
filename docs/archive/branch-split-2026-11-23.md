# Split of `b2-5e-heart-tool` into a PR stack (2026-11-23)

`b2-5e-heart-tool` had grown to 110 commits on top of `main`: the heart work (open PRs #73 to #75), the kidney probe, the
proof-of-concept items, animation, labels and the authoring convention. At the user's request it was split into a stack of PRs.
Each PR is a contiguous range of the original linear history, so no commit was reordered or edited, with one exception
noted below.

| PR | Branch | Original range | Theme |
|---|---|---|---|
| #73 | `b2-5b-region-surfaces` | `4ff5e22` | region surfaces (existing PR) |
| #74 | `b2-5c-simplify` | `e2c2d07` | simplifier (existing PR) |
| #75 | `b2-5d-mesh-format` | `ec9a19d` | `.svfmesh`, `Geometry::Mesh` (existing PR) |
| #76 | `split/04-heart-adaptation` | `3bf0f2d`..`0560351` + fix | heart adaptation tool and validated fixture |
| #77 | `split/05-direction-and-kidney-audit` | `0f0270e`..`4b9b5ae` | educational direction, kidney data audit (docs and scripts) |
| #78 | `split/06-image-volumes-segmentation` | `be4eae8`..`fa9fc10` | image volumes, segmentation, voxel surfaces (KiTS probe) |
| #79 | `split/07-register-and-first-items` | `b84f50d`..`1902ed8` | register, Prismal boundary, flower, plant cell, animation design note |
| #80 | `split/08-animation` | `3855274`..`856ede3` (minus the fix) | timelines, particles, opacity, Explorer player |
| #81 | `split/09-poc-items` | `8f6c477`..`0667035` | proof-of-concept items with timelines |
| #82 | `split/10-labels` | `a09e770`..`67583fa` | labels, key, in-picture captions |
| #83 | `split/11-authoring-convention` | `1cd74ac`.. top | authoring convention, handoff housekeeping, this record |

**The one change of order.** Commit `3e62a38` (in #76) introduced a section-plane regression that `3855274` fixed later, inside
the animation work. Tested head by head, #76 to #79 each failed the same two section tests. The fix was cherry-picked onto
the top of #76, and the rest of the stack was rebased onto it, which dropped the original fix from #80 as already applied.
After the rebase:

- the top of the stack has exactly the same tree as `b2-5e-heart-tool` at `a0a3b84`;
- the #80, #81 and #82 heads have exactly the same trees as the original heads;
- the #76 to #79 heads differ from the originals only in `crates/svf-core/src/structure_section.rs` (the fix).

**Verified** (Claude, locally on Windows, release): `cargo test --workspace --release --no-fail-fast` gives 0 failures at every
head:

| PR | Head | Tests passed |
|---|---|---|
| #76 | `b7a31f5` | 473 |
| #77 | `d143017` | 473 |
| #78 | `39424a4` | 507 |
| #79 | `56457c6` | 518 |
| #80 | `71b7245` | 570 |
| #81 | `0e9f0db` | 609 |
| #82 | `e714026` | 632 |
| #83 | the code of `a93e5b8` (same tree) | 0 failures, including core 457, render 20 and explorer 87 |

The #80 to #82 results were measured at the original heads, which have identical trees.

**Merging** (needs the owner's explicit OK, one PR at a time, bottom-up):

1. Merge #73 into `main`.
2. Retarget the next PR's base to `main`.
3. Merge `origin/main` into its head.
4. Squash-merge.
5. Repeat for the next PR.

Never force-push. The original `b2-5e-heart-tool` branch is kept unchanged as a reference, and can be deleted once #83 is merged.
