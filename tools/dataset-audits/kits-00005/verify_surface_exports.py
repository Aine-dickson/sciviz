"""Independent check of SVF KiTS surface exports against the DICOM SEG.

Does NOT use SVF's exported masks or metadata: reads the DICOM SEG directly
with pydicom, rebuilds each segment's voxel grid from per-frame positions,
then compares it with the exported VTK surface.

usage: python -I verify_surface.py <seg.dcm> <surface-dir>
"""
import sys, time
import numpy as np
import pydicom

seg_path, surf_dir = sys.argv[1], sys.argv[2]

t = time.time()
ds = pydicom.dcmread(seg_path)
px = ds.pixel_array  # (frames, rows, cols), 0/1
print("SEG frames", px.shape, "dtype", px.dtype, f"({time.time()-t:.1f}s)")
shared = ds.SharedFunctionalGroupsSequence[0]
pm = shared.PixelMeasuresSequence[0]
dy, dx = [float(v) for v in pm.PixelSpacing]  # row spacing, column spacing
iop = [float(v) for v in shared.PlaneOrientationSequence[0].ImageOrientationPatient]
print("PixelSpacing(row,col)", dy, dx, "IOP", iop)
assert iop == [1, 0, 0, 0, 1, 0], "test assumes identity orientation"

seg_labels = {int(s.SegmentNumber): s.SegmentLabel for s in ds.SegmentSequence}
print("segments", seg_labels)

frames = ds.PerFrameFunctionalGroupsSequence
seg_of = np.array([int(f.SegmentIdentificationSequence[0].ReferencedSegmentNumber) for f in frames])
pos = np.array([[float(v) for v in f.PlanePositionSequence[0].ImagePositionPatient] for f in frames])
assert np.ptp(pos[:, 0]) == 0 and np.ptp(pos[:, 1]) == 0, "x/y origin varies per frame"
x0, y0 = pos[0, 0], pos[0, 1]
zs = np.unique(np.round(pos[:, 2], 6))
dz = np.diff(zs)
print("slices", len(zs), "z range", zs[0], zs[-1], "spacing min/max", dz.min(), dz.max())
assert np.allclose(dz, dz[0], atol=1e-4)
z0, dzs = zs[0], float(dz[0])
kidx = np.round((pos[:, 2] - z0) / dzs).astype(int)
nk, nj, ni = len(zs), px.shape[1], px.shape[2]
print("grid ijk", ni, nj, nk, "voxel mm", dx, dy, dzs)
vox_vol = dx * dy * dzs


def read_vtk(path):
    with open(path, "r") as f:
        lines = f.read().split("\n")
    pi = next(i for i, l in enumerate(lines) if l.startswith("POINTS"))
    n = int(lines[pi].split()[1])
    pts = np.array(" ".join(lines[pi + 1:]).split(None, 3 * n)[:3 * n], dtype=float).reshape(n, 3)
    li = next(i for i, l in enumerate(lines) if l.startswith("POLYGONS"))
    m = int(lines[li].split()[1])
    tri = np.array([l.split() for l in lines[li + 1:li + 1 + m]], dtype=np.int64)
    assert (tri[:, 0] == 3).all()
    return pts, tri[:, 1:]


def check(segnum, fname):
    label = seg_labels[segnum]
    print(f"\n===== Segment {segnum} {label} vs {fname} =====")
    m = np.zeros((nk, nj, ni), dtype=bool)
    sel = np.where(seg_of == segnum)[0]
    m[kidx[sel]] = px[sel].astype(bool)
    count = int(m.sum())
    print("mask voxels", count)

    pts, tri = read_vtk(f"{surf_dir}/{fname}")
    pts = pts * np.array([-1, -1, 1])  # RAS -> LPS
    print("surface verts", len(pts), "tris", len(tri))

    # A. signed volume
    a, b, c = pts[tri[:, 0]], pts[tri[:, 1]], pts[tri[:, 2]]
    vol = np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0
    mvol = count * vox_vol
    print(f"A volume  surface {vol:.4f}  mask {mvol:.4f}  rel diff {(vol-mvol)/mvol:+.2e}")

    # B. bbox & centroid
    idx = np.argwhere(m)
    cen = np.array([x0 + idx[:, 2].mean() * dx, y0 + idx[:, 1].mean() * dy, z0 + idx[:, 0].mean() * dzs])
    lo = np.array([x0 + (idx[:, 2].min() - .5) * dx, y0 + (idx[:, 1].min() - .5) * dy, z0 + (idx[:, 0].min() - .5) * dzs])
    hi = np.array([x0 + (idx[:, 2].max() + .5) * dx, y0 + (idx[:, 1].max() + .5) * dy, z0 + (idx[:, 0].max() + .5) * dzs])
    print("B bbox surface-min", pts.min(0).round(4), "mask-min", lo.round(4), "maxabs", np.abs(pts.min(0) - lo).max())
    print("B bbox surface-max", pts.max(0).round(4), "mask-max", hi.round(4), "maxabs", np.abs(pts.max(0) - hi).max())
    # area-weighted surface centroid isn't the volume centroid; use volume centroid via tetra decomposition
    tc = (a + b + c) / 4.0
    tv = np.einsum("ij,ij->i", a, np.cross(b, c)) / 6.0
    scen = (tc * tv[:, None]).sum(0) / tv.sum()
    print("B centroid surface", scen.round(4), "mask", cen.round(4), "maxabs", np.abs(scen - cen).max())

    # C. every triangle separates an inside voxel (behind) from an outside voxel (front)
    nrm = np.cross(b - a, c - a)
    ax = np.abs(nrm).argmax(1)
    sgn = np.sign(nrm[np.arange(len(nrm)), ax])
    ctr = (a + b + c) / 3.0
    # offset a quarter voxel either side along the face normal axis
    step = np.array([dx, dy, dzs])[ax] * 0.25
    p_out = ctr.copy(); p_in = ctr.copy()
    p_out[np.arange(len(p_out)), ax] += sgn * step
    p_in[np.arange(len(p_in)), ax] -= sgn * step

    def inside(p):
        i = np.round((p[:, 0] - x0) / dx).astype(int)
        j = np.round((p[:, 1] - y0) / dy).astype(int)
        k = np.round((p[:, 2] - z0) / dzs).astype(int)
        ok = (i >= 0) & (i < ni) & (j >= 0) & (j < nj) & (k >= 0) & (k < nk)
        r = np.zeros(len(p), dtype=bool)
        r[ok] = m[k[ok], j[ok], i[ok]]
        return r

    # offset normals must be axis aligned
    off_axis = np.abs(nrm).max(1) / np.linalg.norm(nrm, axis=1)
    print("C min axis-alignment of face normals", off_axis.min().round(6))
    good = inside(p_in) & ~inside(p_out)
    print(f"C faces with inside-behind / outside-front: {good.sum()} / {len(tri)}  (bad {int((~good).sum())})")

    # D. independent exposed-face count from the mask
    pad = np.pad(m, 1)
    exposed = 0
    for axn in range(3):
        for s in (1, -1):
            sh = np.roll(pad, s, axis=axn)
            exposed += int((pad & ~sh)[1:-1, 1:-1, 1:-1].sum())
    print(f"D exposed voxel faces {exposed} -> triangles {2*exposed} vs surface {len(tri)}")
    return dict(vol_rel=(vol - mvol) / mvol, bad=int((~good).sum()), tri_match=(2 * exposed == len(tri)))


r1 = check(1, "kidney.vtk")
r2 = check(2, "mass.vtk")
print("\nSUMMARY", r1, r2)
