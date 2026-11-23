# Biology dataset audit for C5 (digestive system)

Date: 2026-11-23. Status: **C5 step 1 (audit) done; adaptation not started.** Part of `docs/authoring-convention.md` section 8, C5.
Decision D4 applies: any source may be used if its licence terms are honoured, including attribution, and share-alike for
derived geometry.

## 1. Candidates

| Source | What it holds | Licence (as found) | Size | Verdict |
|---|---|---|---|---|
| **BodyParts3D** (DBCLS), `partof_BP3D_4.0_obj_99.zip` | Organ meshes (OBJ, mm) of one adult male body, keyed by FMA ID; reduced by 99% | Conflict: the README (2025-02-27) says **CC BY 4.0**, but every OBJ file's header says **CC BY-SA 2.1 Japan** | 62 MB | **Chosen** for the digestive tract: small, per organ, and in one body frame, so the organs fit together. |
| HRA / HuBMAP CCF 3D reference objects | Reference organs (GLB), male and female, including the small intestine | CC BY 4.0 (from v1.3 for some organs; check per version) | per organ | **Fallback.** A stomach model is not confirmed. GLB needs a binary reader. |
| TotalSegmentator CT dataset (Zenodo, v2.0.1) | 1,228 CT scans with 117 labelled structures | CC BY 4.0 (from third-party pages; the Zenodo "Rights" field is not yet checked) | 23.6 GB | Later, for measured variation. Too large for one item, and needs the kidney's voxel pipeline. |
| Z-Anatomy | Whole-body Blender atlas | CC BY-SA 4.0 (not re-checked in this audit) | large | Not needed for the digestive tract. |

## 2. Licence decision for BodyParts3D

Honour both licences:

- **Attribute** with the README's required line: "BodyParts3D, © The Database Center for Life Science licensed under CC
  Attribution 4.0 International".
- **Also state the files' own terms.** Release the adapted geometry (`.svfmesh` and the structure file) under CC BY-SA, as
  the files' headers require.

This keeps the item legal whichever notice governs. **The user should confirm** that share-alike output is acceptable for Mava
lessons. D4 already allows it.

Source pages:

- README: https://dbarchive.biosciencedbc.jp/data/bodyparts3d/LATEST/README_e.html
- Download page: https://dbarchive.biosciencedbc.jp/en/bodyparts3d/download.html

The local copy is in `datasets/bodyparts3d/`, which git ignores: the zip, `partof_parts_list_e.txt` and
`partof_element_parts.txt`.

## 3. What the meshes are like (measured, 2026-11-23)

FMA concept IDs map to mesh elements (FJ files) through `partof_element_parts.txt`.

| Organ (FMA) | Elements | Faces | Open edges | Components | Size (mm) |
|---|---|---|---|---|---|
| Oesophagus (FMA7131) | FJ2563 | 578 | 72 | 4 | 32 × 58 × 274 |
| Stomach (FMA7148) | FJ2564 | 1,810 | 22 | 1 | 135 × 113 × 117 |
| Duodenum (FMA7206) | FJ2573 | 1,860 | 100 | 1 | 98 × 53 × 92 |
| Ascending colon (FMA14545) | FJ2566 | 3,310 | 370 | 4 | 63 × 55 × 186 |
| Descending colon (FMA14547) | FJ2567 | 5,588 | 596 | 5 | 135 × 92 × 239 |
| Rectum (FMA14544) | FJ2571 | 2,118 | 138 | 5 | 62 × 62 × 117 |
| Transverse colon (FMA14546) | FJ2572 | 3,472 | 452 | 2 | 185 × 90 × 71 |
| Large-intestine part FJ2569 | FJ2569 | 13,342 | 13,436 | 1,930 | 170 × 98 × 187 (fragments: leave out) |
| Small intestine (FMA7200) | about 30 elements, FJ2573 onwards | – | – | – | – |

Findings:

- **No mesh is closed.** The tube organs are open at their ends, and some have small holes.
- **Several organs come in pieces.**
- The coordinates are a standing body in mm, with z up (z from about 760 to 1390).
- The heavy reduction makes the shapes faceted. Smooth shading (C2) hides much of this.

## 4. Plan for the adaptation (C5 step 2, next session; Opus design, then Sonnet build)

1. **`svf-core::obj_import`**: read OBJ vertices and triangles, with tests. Keep the source file and element ID as
   provenance (`structure_mesh::Provenance`).
2. **Make each body closed**, with a measured, recorded change:
   - cap every boundary loop: fan from the loop's centroid, or ear-clip a planar loop;
   - keep components above a size threshold, and drop fragments, recording what was dropped;
   - check with the existing closed-mesh test.
   - **Design question for Opus:** capping a tube's ends closes the gut lumen. That is right for a solid "organ" body, and
     a flow inside it is still drawn along the centreline.
3. **Change frame:** mm to cm, centred, with the body's up as +Y to match the other items. Write one `.svfmesh` with one body
   per organ (the heart's container format, `docs/heart-structure.md` section 18).
4. **Centrelines (new):** for each tube organ, find the two end loops (the largest boundary loops before capping). Then:
   - march from one end to the other, slicing the mesh with planes normal to the current direction;
   - take each slice's centroid, and step on;
   - test against an analytic tube (a torus section), then on the oesophagus.
   - This is what C5's "flows along extracted centrelines" needs. It replaces the hand-placed path of the PoC digestive item.
5. **Kit and object:** `Organ(id, from: "bodyparts3d:FMA7148")` and `Flow(food, along: ["oesophagus", "stomach",
   "duodenum"])` in the item document, via a biology kit.
6. **Re-author the digestive item.** Score it F3 in form, with a measured change from the source (the volume added by
   the caps), and F1 for the flow. Record the audit with the item.

Owed by the user: confirm the share-alike output (section 2), and choose whether the small intestine (about 30 elements)
is in scope or shown as a schematic coil.
