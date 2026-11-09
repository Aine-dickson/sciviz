# B1.1 — Preliminary BodyParts3D source audit

## Status

**Online metadata audit complete; local archive audit pending.**

The official BodyParts3D 4.0 PART-OF README, concept table, element mapping table, inclusion table, licence, coordinate diagram, and release note were inspected. The 62 MB OBJ archive has not been downloaded or inspected in the current environment. Therefore, archive membership and mesh-quality findings below are provisional until the included audit script is run against the actual archive.

## Official source records

- Dataset: BodyParts3D 4.0, PART-OF tree.
- OBJ archive: `partof_BP3D_4.0_obj_99.zip` (official page lists 62 MB).
- Concept table: `partof_parts_list_e.txt`.
- Compound-element mapping: `partof_element_parts.txt`.
- Inclusion relationships: `partof_inclusion_relation_list.txt`.
- Official source directory: https://dbarchive.biosciencedbc.jp/data/bodyparts3d/LATEST/
- Official download page: https://dbarchive.biosciencedbc.jp/en/bodyparts3d/download.html
- Official licence: CC BY 4.0.
- Required attribution: **BodyParts3D, © The Database Center for Life Science licensed under CC Attribution 4.0 International**.
- Release 4.0 date: 2013-05-16; official README and licence page updated 2025-02-27.

The archive page identifies the mesh files as Wavefront OBJ files, each associated with a unique ELEMENT file ID. The concept table maps FMA concepts to BP representation IDs; the element table maps COMPOUND concepts to one or more FJ element file IDs. BP IDs are not OBJ filenames.

## Corrected concept mapping

The official PART-OF concept table confirms these representation IDs:

| Structure | FMA | BP representation |
|---|---|---|
| Heart | FMA7088 | BP9305 |
| Right atrium | FMA7096 | BP9831 |
| Left atrium | FMA7097 | BP9454 |
| Right ventricle | FMA7098 | BP9598 |
| Left ventricle | FMA7101 | BP9448 |
| Aorta | FMA3734 | BP10374 |
| Superior vena cava | FMA4720 | BP10386 |
| Inferior vena cava | FMA10951 | BP10397 |
| Pulmonary trunk | FMA8612 | **BP9912** |
| Right pulmonary artery | FMA50872 | BP9427 |
| Left pulmonary artery | FMA50873 | BP9414 |
| Right superior pulmonary vein | FMA49914 | BP9365 |
| Right inferior pulmonary vein | FMA49911 | BP9407 |
| Left superior pulmonary vein | FMA49916 | BP9499 |
| Left inferior pulmonary vein | FMA49913 | BP9712 |
| Tricuspid valve | FMA7234 | BP9830 |
| Pulmonary valve | FMA7246 | BP9868 |
| Mitral valve | FMA7235 | BP9455 |
| Aortic valve | FMA7236 | BP9447 |

**Correction:** an earlier design draft listed pulmonary trunk as BP9911. The official table shows BP9911 is the right conus artery; the pulmonary trunk is BP9912.

## Element mapping findings

### Four chambers

| Concept | FJ element IDs in official mapping table |
|---|---|
| Right atrium | FJ2421, FJ2424, FJ2433, FJ2436, FJ2439 |
| Left atrium | FJ2420, FJ2425, FJ2426, FJ2431, FJ2432, FJ2438 |
| Right ventricle | FJ2417, FJ2419, FJ2421, FJ2423, FJ2427, FJ2430, FJ2433, FJ2434, FJ2436, FJ2437 |
| Left ventricle | FJ2418, FJ2420, FJ2422, FJ2426, FJ2429, FJ2431, FJ2432, FJ2435 |

The right atrium and right ventricle share FJ2421, FJ2433, and FJ2436. The left atrium and left ventricle share FJ2420, FJ2426, FJ2431, and FJ2432.

### Four valves

| Concept | FJ element IDs in official mapping table |
|---|---|
| Tricuspid valve | FJ2421, FJ2433, FJ2436 |
| Pulmonary valve | FJ2417, FJ2427, FJ2434 |
| Mitral valve | FJ2420, FJ2426, FJ2431, FJ2432 |
| Aortic valve | FJ2426, FJ2431, FJ2435 |

Every valve mapping overlaps chamber mappings:
- Tricuspid valve elements are shared with right atrium and right ventricle.
- Pulmonary valve elements are shared with right ventricle.
- Mitral valve elements are shared with left atrium and left ventricle.
- Aortic valve elements overlap left ventricle.

**Implication:** the published PART-OF element mapping does not provide four separate valve meshes independently selectable from the chamber meshes. Do not duplicate shared meshes and relabel copies as independent valves. This is a source segmentation/mapping limitation, not evidence of an SVF rendering limitation.

### Major vessels

- Aorta: FJ1931, FJ1932, FJ3411, FJ3413, FJ3427.
- Superior vena cava: FJ3645.
- Inferior vena cava: FJ3441, FJ3659.
- Pulmonary trunk: FJ2966.
- Right/left pulmonary arteries and the four pulmonary veins each map to multiple FJ element IDs; the audit script will enumerate their complete lists and verify archive membership.

These are metadata mappings only. The local archive run must confirm the corresponding OBJ members actually exist and contain valid geometry.

## Coordinate system

The official BodyParts3D coordinate diagram specifies:
- length unit: millimetres;
- body centreline approximately along the Z axis;
- superior locations have greater Z values;
- anatomical left: X > 0; anatomical right: X < 0;
- posterior: Y > 0; anterior: Y < 0;
- all body-part Z values are positive.

Source: https://dbarchive.biosciencedbc.jp/archive/bodyparts3d/images/coordinate_system.png

Do not center individual structures independently. Preserve the common whole-body coordinate frame. If imported into SVF's +Y-up world convention, apply one documented transform consistently to all structures and preserve the source coordinates in the manifest.

## Dataset limitations and caveats

The official 4.0 release note states that coordinates were changed to accommodate large internal organs and many existing parts were shifted. It explicitly cautions that the dataset may still contain errors if used as a canonical model of common human anatomy. The official README describes the reference as an adult human male model.

Sources:
- https://dbarchive.biosciencedbc.jp/data/bodyparts3d/20130619/release_4.0_e.html
- https://dbarchive.biosciencedbc.jp/data/bodyparts3d/LATEST/README_e.html

These caveats require an independent anatomical reference review. BodyParts3D must not be described as a universal or clinical gold standard.

## Secondary reference

The HuBMAP CCF 3D Reference Object Library lists female and male whole-heart GLB objects. Its documentation says reference organs are developed by a specialist in 3D medical illustration and approved by organ experts, initially using Visible Human datasets. This is a useful whole-organ visual comparison, but the public library page does not establish separate downloadable geometry for all four chambers and valves.

Source: https://hubmapconsortium.github.io/ccf/pages/ccf-3d-reference-library.html

## Reproducible local archive audit

Run from the repository root on a machine with internet access and Python 3.10+:

```bash
python tools/biology-heart-anatomy-audit/audit_bodyparts3d.py
```

The script:
- downloads the official metadata tables and 62 MB archive into `.cache/bodyparts3d-4.0/`;
- checks each required FMA concept and expected BP representation ID;
- resolves each concept to its FJ element IDs;
- checks the archive contains each required OBJ member;
- identifies FJ elements shared by multiple required concepts;
- counts OBJ vertices, faces, and fan-triangulated face estimates;
- checks finite vertex coordinates, face index validity, and zero-area fan triangles;
- records per-mesh bounds and an archive SHA-256;
- writes `bodyparts3d-heart-audit.json`.

The script does not extract, repair, simplify, transform, or redistribute any mesh. It does not infer coordinate units or axis meaning from OBJ syntax. The official coordinate diagram supplies the coordinate convention.

Expected first-run download is approximately 62 MB plus small metadata files. If the official server is unavailable, the script stops rather than substituting a mirror or unverified archive.

## B1.1 decision still pending

The official mapping shows that BodyParts3D PART-OF is not sufficient by itself for the original requirement that all four valves and all four chambers be independently selectable geometry. After the local audit:
1. decide whether to revise B1 scope to the distinct structures this source actually supports;
2. or select a second, independently licensed detailed teaching-heart mesh source for the internal valves/chambers;
3. preserve BodyParts3D as a whole-body coordinate/reference dataset if useful.

Do not create SVF fixtures or modify SVF production code until the source choice and final structure inventory are accepted.
