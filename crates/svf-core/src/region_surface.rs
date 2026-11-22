//! Tag regions of a tetrahedral mesh as closed, consistently oriented surfaces.
//! Design: `docs/heart-structure.md` sections 5 to 7 and 15.
//!
//! A *labelled surface* holds every face of a conforming tetrahedral mesh whose
//! two sides carry different labels: a cell tag, [`VOID`] outside, or a cavity
//! label for each enclosed void. Each face is wound so that its normal points
//! from its `front` label to its `back` label, and `front < back`, so all faces
//! between the same pair of labels agree. The orientation of a face is decided
//! from geometry (away from the opposite vertex of its tetrahedron), never from
//! the order of the cell's points, which the source does not certify.
//!
//! * [`relabel_islands`] gives small stray components of a tag to the neighbouring
//!   tag they share the most face area with.
//! * [`LabelledSurface::build`] extracts the faces, finds the enclosed cavities
//!   and labels them.
//! * [`LabelledSurface::patches`] splits the surface into patches: faces with the
//!   same label pair, connected through edges shared by exactly two such faces.
//! * [`LabelledSurface::region_triangles`] returns the closed surface of any set of
//!   labels, wound outward.
//! * [`repair_pinches`] turns a surface whose parts touch along an edge or at a
//!   vertex into a closed 2-manifold by pairing the faces round each pinched
//!   edge and duplicating vertices.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    error::Error,
    fmt,
};

use crate::unstructured::{CellType3, UnstructuredGrid3};

/// A point in world coordinates.
pub type P3 = [f64; 3];

/// The label of the space outside every cell and every cavity.
pub const VOID: u32 = 0;
/// Cavity `k` has the label `CAVITY_BASE + k`. Cell tags must be below this.
pub const CAVITY_BASE: u32 = 1 << 20;

/// Why a region operation failed.
#[derive(Clone, Debug, PartialEq)]
pub enum RegionError {
    NotTetrahedra { cell: usize },
    TagCountMismatch { tags: usize, cells: usize },
    /// Tags start at 1; 0 is the void.
    ZeroTag { cell: usize },
    TagTooLarge { cell: usize },
    /// Faces used by more than two cells: the mesh is not conforming.
    NonConforming { faces: usize },
    /// An edge's uses cannot be paired (odd count, or unequal forward and backward).
    Unpairable { edges: usize },
    /// Two faces use an edge in the same direction: the surface is not consistently wound.
    Inconsistent { edges: usize },
    /// Repair finished but some edge is still not used by exactly two faces.
    Unresolved { edges: usize },
}

impl fmt::Display for RegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotTetrahedra { cell } => write!(f, "cell {cell} is not a tetrahedron"),
            Self::TagCountMismatch { tags, cells } => write!(f, "{tags} tags for {cells} cells"),
            Self::ZeroTag { cell } => write!(f, "cell {cell} has tag 0 (tags start at 1)"),
            Self::TagTooLarge { cell } => write!(f, "cell {cell} has a tag of {CAVITY_BASE} or more"),
            Self::NonConforming { faces } => write!(f, "{faces} faces are used by more than two cells"),
            Self::Unpairable { edges } => write!(f, "{edges} non-manifold edges cannot be paired"),
            Self::Inconsistent { edges } => write!(f, "{edges} edges are used in the same direction twice"),
            Self::Unresolved { edges } => write!(f, "{edges} edges are still non-manifold after repair"),
        }
    }
}

impl Error for RegionError {}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: P3, b: P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Signed volume of a triangle mesh by the divergence theorem; positive when wound outward.
#[must_use]
pub fn signed_volume(points: &[P3], triangles: &[[u32; 3]]) -> f64 {
    triangles
        .iter()
        .map(|t| dot(points[t[0] as usize], cross(points[t[1] as usize], points[t[2] as usize])) / 6.0)
        .sum()
}

/// Every directed edge is used once and its reverse once.
#[must_use]
pub fn is_closed_manifold(triangles: &[[u32; 3]]) -> bool {
    let mut uses: HashMap<(u32, u32), u32> = HashMap::new();
    for t in triangles {
        for k in 0..3 {
            *uses.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
        }
    }
    uses.iter().all(|(&(a, b), &n)| n == 1 && uses.get(&(b, a)) == Some(&1))
}

struct Sets(Vec<u32>);

impl Sets {
    fn new(n: usize) -> Self {
        Self((0..n as u32).collect())
    }
    fn find(&mut self, mut x: u32) -> u32 {
        while self.0[x as usize] != x {
            self.0[x as usize] = self.0[self.0[x as usize] as usize];
            x = self.0[x as usize];
        }
        x
    }
    fn union(&mut self, a: u32, b: u32) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            // The smaller index is the root, so components are named deterministically.
            let (low, high) = if a < b { (a, b) } else { (b, a) };
            self.0[high as usize] = low;
        }
    }
}

// ------------------------------------------------------------------ input

fn tetrahedra(grid: &UnstructuredGrid3) -> Result<Vec<[u32; 4]>, RegionError> {
    (0..grid.cell_count())
        .map(|cell| {
            if grid.cell_type(cell) != Some(CellType3::Tetrahedron4) {
                return Err(RegionError::NotTetrahedra { cell });
            }
            let ids = grid.cell_point_indices(cell).ok_or(RegionError::NotTetrahedra { cell })?;
            Ok([ids[0], ids[1], ids[2], ids[3]])
        })
        .collect()
}

fn check_tags(tags: &[u32], cells: usize) -> Result<(), RegionError> {
    if tags.len() != cells {
        return Err(RegionError::TagCountMismatch { tags: tags.len(), cells });
    }
    for (cell, &tag) in tags.iter().enumerate() {
        if tag == VOID {
            return Err(RegionError::ZeroTag { cell });
        }
        if tag >= CAVITY_BASE {
            return Err(RegionError::TagTooLarge { cell });
        }
    }
    Ok(())
}

struct FaceRecord {
    key: [u32; 3],
    cell: u32,
    opposite: u32,
}

/// Every (face, cell) pair, sorted by face then cell, so grouping is deterministic.
fn face_records(tets: &[[u32; 4]]) -> Vec<FaceRecord> {
    let mut records = Vec::with_capacity(tets.len() * 4);
    for (cell, t) in tets.iter().enumerate() {
        for skip in 0..4 {
            let mut key = [0u32; 3];
            let mut n = 0;
            for (k, &v) in t.iter().enumerate() {
                if k != skip {
                    key[n] = v;
                    n += 1;
                }
            }
            key.sort_unstable();
            records.push(FaceRecord { key, cell: cell as u32, opposite: t[skip] });
        }
    }
    records.sort_unstable_by_key(|r| (r.key, r.cell));
    records
}

fn runs(records: &[FaceRecord]) -> impl Iterator<Item = &[FaceRecord]> {
    records.chunk_by(|a, b| a.key == b.key)
}

/// The face wound so that its normal points away from `opposite`.
fn away_from(points: &[P3], key: [u32; 3], opposite: u32) -> [u32; 3] {
    let [a, b, c] = key;
    let (pa, pb, pc) = (points[a as usize], points[b as usize], points[c as usize]);
    let toward = dot(cross(sub(pb, pa), sub(pc, pa)), sub(points[opposite as usize], pa));
    if toward > 0.0 {
        [a, c, b]
    } else {
        [a, b, c]
    }
}

fn area(points: &[P3], t: [u32; 3]) -> f64 {
    let n = cross(sub(points[t[1] as usize], points[t[0] as usize]), sub(points[t[2] as usize], points[t[0] as usize]));
    0.5 * dot(n, n).sqrt()
}

// ---------------------------------------------------------------- islands

/// A stray component of a tag that was given to a neighbouring tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IslandFix {
    pub cells: Vec<usize>,
    pub from: u32,
    pub to: u32,
}

/// Cells of one tag are components when joined through shared faces. Every tag
/// keeps its largest component. Any other component with fewer than
/// `max_island_cells` cells is relabelled to the other tag it shares the most
/// face area with (ties: the lower tag). An island with no tagged neighbour is
/// left alone. Returns the new tags and the fixes made.
pub fn relabel_islands(
    grid: &UnstructuredGrid3,
    tags: &[u32],
    max_island_cells: usize,
) -> Result<(Vec<u32>, Vec<IslandFix>), RegionError> {
    let tets = tetrahedra(grid)?;
    check_tags(tags, tets.len())?;
    let points: Vec<P3> = grid.points().iter().map(|p| p.to_array()).collect();
    let records = face_records(&tets);

    let mut sets = Sets::new(tets.len());
    let mut contacts: Vec<(u32, u32, f64)> = Vec::new();
    let mut shared_by_many = 0;
    for run in runs(&records) {
        match run {
            [_] => {}
            [a, b] => {
                if tags[a.cell as usize] == tags[b.cell as usize] {
                    sets.union(a.cell, b.cell);
                } else {
                    contacts.push((a.cell, b.cell, area(&points, a.key)));
                }
            }
            _ => shared_by_many += 1,
        }
    }
    if shared_by_many > 0 {
        return Err(RegionError::NonConforming { faces: shared_by_many });
    }

    let mut size: HashMap<u32, usize> = HashMap::new();
    for cell in 0..tets.len() as u32 {
        *size.entry(sets.find(cell)).or_default() += 1;
    }
    // The largest component of each tag (ties: the lowest root).
    let mut largest: BTreeMap<u32, (usize, u32)> = BTreeMap::new();
    for (&root, &n) in &size {
        let entry = largest.entry(tags[root as usize]).or_insert((n, root));
        if n > entry.0 || (n == entry.0 && root < entry.1) {
            *entry = (n, root);
        }
    }
    let mut neighbours: HashMap<u32, BTreeMap<u32, f64>> = HashMap::new();
    for &(a, b, face_area) in &contacts {
        let (ra, rb) = (sets.find(a), sets.find(b));
        *neighbours.entry(ra).or_default().entry(tags[b as usize]).or_default() += face_area;
        *neighbours.entry(rb).or_default().entry(tags[a as usize]).or_default() += face_area;
    }

    let mut new_tags = tags.to_vec();
    let mut islands: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for cell in 0..tets.len() {
        let root = sets.find(cell as u32);
        if largest[&tags[cell]].1 != root && size[&root] < max_island_cells {
            islands.entry(root).or_default().push(cell);
        }
    }
    let mut fixes = Vec::new();
    for (root, cells) in islands {
        let Some(options) = neighbours.get(&root) else { continue };
        let Some((&to, _)) = options.iter().max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0))) else {
            continue;
        };
        let from = tags[root as usize];
        for &cell in &cells {
            new_tags[cell] = to;
        }
        fixes.push(IslandFix { cells, from, to });
    }
    Ok((new_tags, fixes))
}

// ---------------------------------------------------------------- surface

/// One face between two labels, wound from `front` toward `back`; `front < back`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelledFace {
    pub triangle: [u32; 3],
    pub front: u32,
    pub back: u32,
}

/// An enclosed void.
#[derive(Clone, Debug, PartialEq)]
pub struct Cavity {
    pub label: u32,
    pub face_count: usize,
    /// Enclosed volume, positive.
    pub volume: f64,
    pub area: f64,
    /// The cell tags that bound it, ascending.
    pub bounding_tags: Vec<u32>,
}

/// Faces with the same label pair, connected through edges shared by exactly
/// two such faces. Face indices are into [`LabelledSurface::faces`], ascending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Patch {
    pub front: u32,
    pub back: u32,
    pub faces: Vec<usize>,
}

/// The labelled surface of a tagged tetrahedral mesh.
#[derive(Clone, Debug)]
pub struct LabelledSurface {
    points: Vec<P3>,
    faces: Vec<LabelledFace>,
    cavities: Vec<Cavity>,
}

fn normalised(triangle: [u32; 3], front: u32, back: u32) -> LabelledFace {
    if front < back {
        LabelledFace { triangle, front, back }
    } else {
        LabelledFace { triangle: [triangle[0], triangle[2], triangle[1]], front: back, back: front }
    }
}

impl LabelledSurface {
    /// Extracts the faces between different labels and labels the enclosed
    /// cavities. `tags[cell]` is the tag of each tetrahedron, from 1.
    pub fn build(grid: &UnstructuredGrid3, tags: &[u32]) -> Result<Self, RegionError> {
        let tets = tetrahedra(grid)?;
        check_tags(tags, tets.len())?;
        let points: Vec<P3> = grid.points().iter().map(|p| p.to_array()).collect();
        let records = face_records(&tets);

        // Void faces are wound with the void in front: the normal points into the cell.
        let mut faces: Vec<LabelledFace> = Vec::new();
        let mut many = 0;
        for run in runs(&records) {
            match run {
                [only] => {
                    let outward = away_from(&points, only.key, only.opposite);
                    faces.push(normalised(outward, tags[only.cell as usize], VOID));
                }
                [a, b] => {
                    let (ta, tb) = (tags[a.cell as usize], tags[b.cell as usize]);
                    if ta != tb {
                        faces.push(normalised(away_from(&points, a.key, a.opposite), ta, tb));
                    }
                }
                _ => many += 1,
            }
        }
        drop(records);
        if many > 0 {
            return Err(RegionError::NonConforming { faces: many });
        }

        let cavities = label_cavities(&points, &mut faces);
        Ok(Self { points, faces, cavities })
    }

    #[must_use]
    pub fn points(&self) -> &[P3] {
        &self.points
    }

    #[must_use]
    pub fn faces(&self) -> &[LabelledFace] {
        &self.faces
    }

    /// The enclosed voids, largest first by face count.
    #[must_use]
    pub fn cavities(&self) -> &[Cavity] {
        &self.cavities
    }

    /// The closed surface of the region made of `labels`, wound outward:
    /// every face with exactly one side in the set.
    #[must_use]
    pub fn region_triangles(&self, labels: &[u32]) -> Vec<[u32; 3]> {
        let set: BTreeSet<u32> = labels.iter().copied().collect();
        self.faces
            .iter()
            .filter_map(|f| match (set.contains(&f.front), set.contains(&f.back)) {
                (true, false) => Some(f.triangle),
                (false, true) => Some([f.triangle[0], f.triangle[2], f.triangle[1]]),
                _ => None,
            })
            .collect()
    }

    /// Patches, ordered by label pair then by their lowest face.
    #[must_use]
    pub fn patches(&self) -> Vec<Patch> {
        let mut entries: Vec<((u32, u32), u32)> = Vec::with_capacity(self.faces.len() * 3);
        for (i, f) in self.faces.iter().enumerate() {
            for k in 0..3 {
                let (a, b) = (f.triangle[k], f.triangle[(k + 1) % 3]);
                entries.push(((a.min(b), a.max(b)), i as u32));
            }
        }
        entries.sort_unstable();
        let mut sets = Sets::new(self.faces.len());
        for run in entries.chunk_by(|a, b| a.0 == b.0) {
            if let [(_, f), (_, g)] = run {
                let (a, b) = (&self.faces[*f as usize], &self.faces[*g as usize]);
                if (a.front, a.back) == (b.front, b.back) {
                    sets.union(*f, *g);
                }
            }
        }
        let mut grouped: BTreeMap<(u32, u32, u32), Vec<usize>> = BTreeMap::new();
        for (i, f) in self.faces.iter().enumerate() {
            let root = sets.find(i as u32);
            grouped.entry((f.front, f.back, root)).or_default().push(i);
        }
        grouped.into_iter().map(|((front, back, _), faces)| Patch { front, back, faces }).collect()
    }
}

/// Finds the components of the void-facing faces; each one that encloses a void
/// (negative volume as seen from the cells) gets a cavity label.
fn label_cavities(points: &[P3], faces: &mut [LabelledFace]) -> Vec<Cavity> {
    let exterior: Vec<usize> = (0..faces.len()).filter(|&i| faces[i].front == VOID).collect();
    let mut entries: Vec<((u32, u32), u32)> = Vec::with_capacity(exterior.len() * 3);
    for (slot, &i) in exterior.iter().enumerate() {
        let t = faces[i].triangle;
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            entries.push(((a.min(b), a.max(b)), slot as u32));
        }
    }
    entries.sort_unstable();
    let mut sets = Sets::new(exterior.len());
    for run in entries.chunk_by(|a, b| a.0 == b.0) {
        for pair in run.windows(2) {
            sets.union(pair[0].1, pair[1].1);
        }
    }
    struct Comp {
        faces: Vec<usize>,
        volume: f64,
        area: f64,
    }
    let mut comps: BTreeMap<u32, Comp> = BTreeMap::new();
    for (slot, &i) in exterior.iter().enumerate() {
        let root = sets.find(slot as u32);
        let t = faces[i].triangle;
        // Void-front faces point into the cell; reversed they point out of it.
        let outward = [t[0], t[2], t[1]];
        let c = comps.entry(root).or_insert(Comp { faces: Vec::new(), volume: 0.0, area: 0.0 });
        c.faces.push(i);
        c.volume += dot(points[outward[0] as usize], cross(points[outward[1] as usize], points[outward[2] as usize])) / 6.0;
        c.area += area(points, t);
    }
    let mut inner: Vec<Comp> = comps.into_values().filter(|c| c.volume < 0.0).collect();
    // Largest first; the lowest face index breaks ties, so labels are deterministic.
    inner.sort_by(|a, b| b.faces.len().cmp(&a.faces.len()).then(a.faces[0].cmp(&b.faces[0])));
    let mut cavities = Vec::with_capacity(inner.len());
    for (k, c) in inner.into_iter().enumerate() {
        let label = CAVITY_BASE + k as u32;
        let mut bounding: BTreeSet<u32> = BTreeSet::new();
        for &i in &c.faces {
            let f = faces[i];
            bounding.insert(f.back);
            faces[i] = normalised(f.triangle, label, f.back);
        }
        cavities.push(Cavity {
            label,
            face_count: c.faces.len(),
            volume: -c.volume,
            area: c.area,
            bounding_tags: bounding.into_iter().collect(),
        });
    }
    cavities
}

// ----------------------------------------------------------------- repair

/// A closed 2-manifold with its own vertex list (vertices may be duplicated in place).
#[derive(Clone, Debug, PartialEq)]
pub struct ManifoldMesh {
    pub points: Vec<P3>,
    pub triangles: Vec<[u32; 3]>,
}

/// What [`repair_pinches`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RepairReport {
    pub nonmanifold_edges: usize,
    /// Edges whose faces did not alternate round the edge as a solid's would, and
    /// were paired in angular order instead.
    pub irregular_pairings: usize,
    /// Edges that needed a pairing other than the solid wedges (a handle through the pinch).
    pub alternate_pairings: usize,
    /// Vertex copies made, over and above one per used vertex.
    pub split_vertices: usize,
}

struct EdgeUse {
    key: (u32, u32),
    face: u32,
    corner: u8,
    /// The face winds `key.0` to `key.1`.
    forward: bool,
}

/// Makes a consistently wound surface a closed 2-manifold.
///
/// Round an edge used by more than two faces, the faces are sorted by angle. A
/// solid wedge lies between a face that winds the edge backward and the next
/// forward one, so each such pair becomes a manifold sheet. Then every vertex is
/// duplicated once per connected fan of faces round it, with fans connected
/// through manifold edges and paired faces. The vertex list is compacted to
/// the used vertices, in order of first use.
pub fn repair_pinches(
    points: &[P3],
    triangles: &[[u32; 3]],
) -> Result<(ManifoldMesh, RepairReport), RegionError> {
    let mut uses: Vec<EdgeUse> = Vec::with_capacity(triangles.len() * 3);
    for (face, t) in triangles.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            uses.push(EdgeUse { key: (a.min(b), a.max(b)), face: face as u32, corner: k as u8, forward: a < b });
        }
    }
    uses.sort_unstable_by_key(|u| (u.key, u.face, u.corner));
    let runs: Vec<&[EdgeUse]> = uses.chunk_by(|a, b| a.key == b.key).collect();

    let (mut unpairable, mut inconsistent, mut nonmanifold) = (0, 0, 0);
    for run in &runs {
        match run.len() {
            1 => {}
            2 => {
                if run[0].forward == run[1].forward {
                    inconsistent += 1;
                }
            }
            n => {
                nonmanifold += 1;
                if run.iter().filter(|u| u.forward).count() * 2 != n {
                    unpairable += 1;
                }
            }
        }
    }
    if unpairable > 0 {
        return Err(RegionError::Unpairable { edges: unpairable });
    }
    if inconsistent > 0 {
        return Err(RegionError::Inconsistent { edges: inconsistent });
    }

    // Which of the possible pairings each non-manifold edge uses. Pairing 0 follows
    // the solid wedges; if fans round both ends of an edge are connected anyway
    // (a handle through the pinch), the edge stays non-manifold, and the next
    // pairing, which crosses between the wedges, separates them.
    let mut variant: HashMap<(u32, u32), usize> = HashMap::new();
    for attempt in 0..=MAX_PAIRINGS {
        let built = assemble(points, triangles, &runs, &variant);
        if built.unresolved.is_empty() {
            let report = RepairReport {
                nonmanifold_edges: nonmanifold,
                irregular_pairings: built.irregular,
                alternate_pairings: variant.values().filter(|&&v| v > 0).count(),
                split_vertices: built.split_vertices,
            };
            return Ok((built.mesh, report));
        }
        if attempt == MAX_PAIRINGS {
            return Err(RegionError::Unresolved { edges: built.unresolved.len() });
        }
        for key in &built.unresolved {
            match runs.binary_search_by(|run| run[0].key.cmp(key)) {
                Ok(i) if runs[i].len() > 2 => *variant.entry(*key).or_default() += 1,
                // An open edge, or an edge that is manifold in the source: not fixable here.
                _ => return Err(RegionError::Unresolved { edges: built.unresolved.len() }),
            }
        }
    }
    unreachable!("the loop returns on its last attempt")
}

/// How many pairings of one edge's faces are tried before giving up.
const MAX_PAIRINGS: usize = 4;

struct Assembled {
    mesh: ManifoldMesh,
    /// Source edges (by source vertex ids) still not used by exactly two faces.
    unresolved: Vec<(u32, u32)>,
    irregular: usize,
    split_vertices: usize,
}

fn assemble(
    points: &[P3],
    triangles: &[[u32; 3]],
    runs: &[&[EdgeUse]],
    variant: &HashMap<(u32, u32), usize>,
) -> Assembled {
    // Pairs of (face, corner) that are adjacent across an edge.
    let mut adjacent: Vec<((u32, u8), (u32, u8))> = Vec::new();
    let mut irregular = 0;
    for run in runs {
        match run.len() {
            1 => {}
            2 => adjacent.push(((run[0].face, run[0].corner), (run[1].face, run[1].corner))),
            _ => {
                let choice = variant.get(&run[0].key).copied().unwrap_or(0);
                if !pair_round_edge(points, triangles, run, choice, &mut adjacent) {
                    irregular += 1;
                }
            }
        }
    }

    // Corners (3 per face) adjacent across an edge share a vertex copy at both ends.
    let mut corners = Sets::new(triangles.len() * 3);
    let corner_of = |face: u32, vertex: u32| -> u32 {
        let t = triangles[face as usize];
        let k = t.iter().position(|&v| v == vertex).expect("vertex of the face");
        face * 3 + k as u32
    };
    for &((f, kf), (g, _)) in &adjacent {
        let tf = triangles[f as usize];
        for vertex in [tf[kf as usize], tf[(kf as usize + 1) % 3]] {
            corners.union(corner_of(f, vertex), corner_of(g, vertex));
        }
    }

    let mut new_points: Vec<P3> = Vec::new();
    let mut origin: Vec<u32> = Vec::new();
    let mut copy_of: HashMap<(u32, u32), u32> = HashMap::new();
    let mut original_used: BTreeSet<u32> = BTreeSet::new();
    let mut new_triangles = Vec::with_capacity(triangles.len());
    for (face, t) in triangles.iter().enumerate() {
        let mut out = [0u32; 3];
        for k in 0..3 {
            let root = corners.find(face as u32 * 3 + k as u32);
            original_used.insert(t[k]);
            out[k] = *copy_of.entry((t[k], root)).or_insert_with(|| {
                new_points.push(points[t[k] as usize]);
                origin.push(t[k]);
                (new_points.len() - 1) as u32
            });
        }
        new_triangles.push(out);
    }
    let split_vertices = new_points.len() - original_used.len();

    let mut edge_uses: HashMap<(u32, u32), u32> = HashMap::new();
    for t in &new_triangles {
        for k in 0..3 {
            *edge_uses.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
        }
    }
    let mut unresolved: BTreeSet<(u32, u32)> = BTreeSet::new();
    for (&(a, b), &n) in &edge_uses {
        if n != 1 || edge_uses.get(&(b, a)) != Some(&1) {
            let (oa, ob) = (origin[a as usize], origin[b as usize]);
            unresolved.insert((oa.min(ob), oa.max(ob)));
        }
    }
    Assembled {
        mesh: ManifoldMesh { points: new_points, triangles: new_triangles },
        unresolved: unresolved.into_iter().collect(),
        irregular,
        split_vertices,
    }
}

/// Pairs the faces round one non-manifold edge and records them as adjacent.
/// Pairing `choice` matches the j-th backward face (counter-clockwise from the
/// first) with the forward face `choice` places after the one that closes its
/// solid wedge. Returns false when the faces did not alternate round the edge
/// and plain list order was used.
fn pair_round_edge(
    points: &[P3],
    triangles: &[[u32; 3]],
    run: &[EdgeUse],
    choice: usize,
    adjacent: &mut Vec<((u32, u8), (u32, u8))>,
) -> bool {
    let (u, v) = run[0].key;
    let (pu, axis) = (points[u as usize], sub(points[v as usize], points[u as usize]));
    let axis_length = dot(axis, axis).sqrt();
    let e = [axis[0] / axis_length, axis[1] / axis_length, axis[2] / axis_length];
    let third = |entry: &EdgeUse| -> P3 {
        let t = triangles[entry.face as usize];
        let w = t[(entry.corner as usize + 2) % 3];
        let d = sub(points[w as usize], pu);
        let along = dot(d, e);
        [d[0] - e[0] * along, d[1] - e[1] * along, d[2] - e[2] * along]
    };
    let first = third(&run[0]);
    let length = dot(first, first).sqrt();
    let b1 = [first[0] / length, first[1] / length, first[2] / length];
    let b2 = cross(e, b1);

    let mut order: Vec<(f64, usize)> = run
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let d = third(entry);
            (dot(d, b2).atan2(dot(d, b1)), i)
        })
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));

    let n = order.len();
    let regular = (0..n).all(|i| run[order[i].1].forward != run[order[(i + 1) % n].1].forward);
    let mut push = |a: &EdgeUse, b: &EdgeUse| adjacent.push(((a.face, a.corner), (b.face, b.corner)));
    let m = n / 2;
    if regular {
        // Rotate so the list starts at a backward face; then the forward face that
        // follows backward face j is the one after it in counter-clockwise order.
        let start = (0..n).find(|&i| !run[order[i].1].forward).expect("a backward face");
        let backward: Vec<&EdgeUse> = (0..m).map(|j| &run[order[(start + 2 * j) % n].1]).collect();
        let forward: Vec<&EdgeUse> = (0..m).map(|j| &run[order[(start + 2 * j + 1) % n].1]).collect();
        for (j, a) in backward.into_iter().enumerate() {
            push(a, forward[(j + choice) % m]);
        }
    } else {
        let backward: Vec<&EdgeUse> = order.iter().map(|&(_, i)| &run[i]).filter(|u| !u.forward).collect();
        let forward: Vec<&EdgeUse> = order.iter().map(|&(_, i)| &run[i]).filter(|u| u.forward).collect();
        for (j, a) in backward.into_iter().enumerate() {
            push(a, forward[(j + choice) % m]);
        }
    }
    regular
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{spatial::WorldPoint3, unstructured::UnstructuredGrid3};

    fn grid(points: &[P3], tets: &[[u32; 4]]) -> UnstructuredGrid3 {
        UnstructuredGrid3::new(
            points.iter().map(|p| WorldPoint3::new(p[0], p[1], p[2])).collect(),
            vec![CellType3::Tetrahedron4; tets.len()],
            (0..=tets.len()).map(|i| i * 4).collect(),
            tets.iter().flatten().copied().collect(),
        )
        .unwrap()
    }

    fn tet_volume(points: &[P3], t: [u32; 4]) -> f64 {
        let p = t.map(|i| points[i as usize]);
        dot(sub(p[1], p[0]), cross(sub(p[2], p[0]), sub(p[3], p[0]))).abs() / 6.0
    }

    /// A cubic block of `n`^3 unit cubes, each cut into six tetrahedra round a
    /// diagonal (conforming across cubes). Every other tetrahedron has its point
    /// order flipped, so orientation must not matter. Returns points, tets and
    /// the cube index of each tet.
    fn block(n: usize) -> (Vec<P3>, Vec<[u32; 4]>, Vec<(usize, usize, usize)>) {
        let id = |i: usize, j: usize, k: usize| ((i * (n + 1) + j) * (n + 1) + k) as u32;
        let mut points = Vec::new();
        for i in 0..=n {
            for j in 0..=n {
                for k in 0..=n {
                    points.push([i as f64, j as f64, k as f64]);
                }
            }
        }
        let perms = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
        let (mut tets, mut cubes) = (Vec::new(), Vec::new());
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    for perm in perms {
                        let mut at = [i, j, k];
                        let mut t = [id(at[0], at[1], at[2]); 4];
                        for (s, &axis) in perm.iter().enumerate() {
                            at[axis] += 1;
                            t[s + 1] = id(at[0], at[1], at[2]);
                        }
                        if tets.len() % 2 == 1 {
                            t.swap(2, 3);
                        }
                        tets.push(t);
                        cubes.push((i, j, k));
                    }
                }
            }
        }
        (points, tets, cubes)
    }

    #[test]
    fn one_tetrahedron_has_four_outward_faces_whatever_its_point_order() {
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        for tet in [[0, 1, 2, 3], [0, 2, 1, 3]] {
            let g = grid(&points, &[tet]);
            let s = LabelledSurface::build(&g, &[7]).unwrap();
            assert_eq!(s.faces().len(), 4);
            assert!(s.cavities().is_empty());
            let tris = s.region_triangles(&[7]);
            assert!(is_closed_manifold(&tris));
            assert!((signed_volume(s.points(), &tris) - 1.0 / 6.0).abs() < 1e-12);
            // The void's region is the same surface wound the other way.
            let outside = s.region_triangles(&[VOID]);
            assert!((signed_volume(s.points(), &outside) + 1.0 / 6.0).abs() < 1e-12);
            assert!(s.faces().iter().all(|f| f.front == VOID && f.back == 7));
        }
    }

    #[test]
    fn two_tags_share_an_interface_and_each_region_is_closed() {
        let points =
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 1.0]];
        let g = grid(&points, &[[0, 1, 2, 3], [1, 2, 3, 4]]);
        let s = LabelledSurface::build(&g, &[1, 2]).unwrap();
        assert_eq!(s.faces().len(), 7, "3 + 3 exterior and 1 interface");
        let (r1, r2) = (s.region_triangles(&[1]), s.region_triangles(&[2]));
        assert!(is_closed_manifold(&r1) && is_closed_manifold(&r2));
        assert!((signed_volume(s.points(), &r1) - tet_volume(&points, [0, 1, 2, 3])).abs() < 1e-12);
        assert!((signed_volume(s.points(), &r2) - tet_volume(&points, [1, 2, 3, 4])).abs() < 1e-12);
        let both = s.region_triangles(&[1, 2]);
        assert_eq!(both.len(), 6);
        assert!(is_closed_manifold(&both));
        let pairs: Vec<_> = s.patches().iter().map(|p| (p.front, p.back, p.faces.len())).collect();
        assert_eq!(pairs, [(0, 1, 3), (0, 2, 3), (1, 2, 1)]);
    }

    #[test]
    fn an_enclosed_void_becomes_a_labelled_cavity() {
        let (points, tets, cubes) = block(3);
        let keep: Vec<usize> = (0..tets.len()).filter(|&t| cubes[t] != (1, 1, 1)).collect();
        let kept: Vec<[u32; 4]> = keep.iter().map(|&t| tets[t]).collect();
        let g = grid(&points, &kept);
        let tags = vec![1; kept.len()];
        let s = LabelledSurface::build(&g, &tags).unwrap();
        assert_eq!(s.cavities().len(), 1);
        let cavity = &s.cavities()[0];
        assert_eq!(cavity.label, CAVITY_BASE);
        assert_eq!(cavity.face_count, 12, "six square faces of two triangles");
        assert!((cavity.volume - 1.0).abs() < 1e-12);
        assert!((cavity.area - 6.0).abs() < 1e-12);
        assert_eq!(cavity.bounding_tags, [1]);

        let cast = s.region_triangles(&[cavity.label]);
        assert!(is_closed_manifold(&cast));
        assert!((signed_volume(s.points(), &cast) - 1.0).abs() < 1e-12, "outward from the cavity");
        let block_region = s.region_triangles(&[1]);
        assert!((signed_volume(s.points(), &block_region) - 26.0).abs() < 1e-9, "outer minus the cavity");
        // Wall plus cavity is the whole cube: the cavity surface is interior to it.
        let whole = s.region_triangles(&[1, cavity.label]);
        assert!((signed_volume(s.points(), &whole) - 27.0).abs() < 1e-9);
        assert_eq!(s.region_triangles(&[VOID]).len(), whole.len());
    }

    #[test]
    fn a_stray_island_goes_to_the_neighbour_with_the_largest_shared_area() {
        // A: tag 1. E: one tetrahedron of tag 2 glued to A. C and D: two tag-2
        // tetrahedra far away, so tag 2 has a big component and a one-cell island.
        let mut points = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
        ];
        let far: Vec<P3> = points.iter().map(|p| [p[0] + 10.0, p[1], p[2]]).collect();
        points.extend(far);
        let tets = [[0, 1, 2, 3], [1, 2, 3, 4], [5, 6, 7, 8], [6, 7, 8, 9]];
        let g = grid(&points, &tets);
        let tags = [1, 2, 2, 2];
        let (fixed, fixes) = relabel_islands(&g, &tags, 2).unwrap();
        assert_eq!(fixed, [1, 1, 2, 2]);
        assert_eq!(fixes, [IslandFix { cells: vec![1], from: 2, to: 1 }]);
        // A tag's only component is never an island, however small.
        let (same, none) = relabel_islands(&g, &[1, 2, 3, 3], 5).unwrap();
        assert_eq!(same, [1, 2, 3, 3]);
        assert!(none.is_empty());
    }

    #[test]
    fn input_errors_are_named() {
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let g = grid(&points, &[[0, 1, 2, 3]]);
        assert_eq!(
            LabelledSurface::build(&g, &[1, 1]).unwrap_err(),
            RegionError::TagCountMismatch { tags: 2, cells: 1 }
        );
        assert_eq!(LabelledSurface::build(&g, &[0]).unwrap_err(), RegionError::ZeroTag { cell: 0 });
        assert_eq!(
            LabelledSurface::build(&g, &[CAVITY_BASE]).unwrap_err(),
            RegionError::TagTooLarge { cell: 0 }
        );
        let triangle = UnstructuredGrid3::new(
            points.iter().map(|p| WorldPoint3::new(p[0], p[1], p[2])).collect(),
            vec![CellType3::Triangle3],
            vec![0, 3],
            vec![0, 1, 2],
        )
        .unwrap();
        assert_eq!(LabelledSurface::build(&triangle, &[1]).unwrap_err(), RegionError::NotTetrahedra { cell: 0 });
    }

    #[test]
    fn the_surface_is_deterministic() {
        let (points, tets, cubes) = block(2);
        let g = grid(&points, &tets);
        let tags: Vec<u32> = cubes.iter().map(|&(_, _, k)| 1 + (k % 2) as u32).collect();
        let (a, b) = (LabelledSurface::build(&g, &tags).unwrap(), LabelledSurface::build(&g, &tags).unwrap());
        assert_eq!(a.faces(), b.faces());
        assert_eq!(a.patches(), b.patches());
        for label in 1..=2 {
            assert!(is_closed_manifold(&a.region_triangles(&[label])));
        }
    }

    #[test]
    fn patches_split_a_pair_at_pinches_and_group_connected_faces() {
        // Two separate blocks of the same tag touching nothing: two patches of (0, 1).
        let (mut points, mut tets, _) = block(1);
        let (more, more_tets, _) = block(1);
        let offset = points.len() as u32;
        points.extend(more.iter().map(|p| [p[0] + 5.0, p[1], p[2]]));
        tets.extend(more_tets.iter().map(|t| t.map(|v| v + offset)));
        let g = grid(&points, &tets);
        let s = LabelledSurface::build(&g, &vec![1; tets.len()]).unwrap();
        let patches = s.patches();
        assert_eq!(patches.len(), 2);
        assert!(patches.iter().all(|p| (p.front, p.back) == (0, 1) && p.faces.len() == 12));
    }

    // ---------------------------------------------------------- repair

    fn volume_of(points: &[P3], tris: &[[u32; 3]]) -> f64 {
        signed_volume(points, tris)
    }

    #[test]
    fn two_tetrahedra_touching_along_an_edge_become_two_closed_bodies() {
        // Both contain the edge (0,0,0)-(1,0,0) and nothing else in common.
        let points = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, -1.0],
        ];
        let g = grid(&points, &[[0, 1, 2, 3], [0, 1, 4, 5]]);
        let s = LabelledSurface::build(&g, &[1, 1]).unwrap();
        let tris = s.region_triangles(&[1]);
        assert_eq!(tris.len(), 8);
        assert!(!is_closed_manifold(&tris), "the shared edge is used four times");

        let (mesh, report) = repair_pinches(s.points(), &tris).unwrap();
        assert!(is_closed_manifold(&mesh.triangles));
        assert_eq!(mesh.triangles.len(), 8);
        assert_eq!(report.nonmanifold_edges, 1);
        assert_eq!(report.irregular_pairings, 0);
        assert_eq!(report.split_vertices, 2, "both ends of the edge are duplicated");
        assert_eq!(mesh.points.len(), 8, "two separate tetrahedra");
        assert!((volume_of(&mesh.points, &mesh.triangles) - 2.0 / 6.0).abs() < 1e-12);
    }

    #[test]
    fn tetrahedra_touching_at_a_vertex_are_separated() {
        let points = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, -1.0],
        ];
        let g = grid(&points, &[[0, 1, 2, 3], [0, 4, 5, 6]]);
        let s = LabelledSurface::build(&g, &[1, 1]).unwrap();
        let tris = s.region_triangles(&[1]);
        let (mesh, report) = repair_pinches(s.points(), &tris).unwrap();
        assert!(is_closed_manifold(&mesh.triangles));
        assert_eq!(report.nonmanifold_edges, 0);
        assert_eq!(report.split_vertices, 1);
        assert_eq!(mesh.points.len(), 8);
        assert!((volume_of(&mesh.points, &mesh.triangles) - 2.0 / 6.0).abs() < 1e-12);
    }

    #[test]
    fn a_manifold_surface_is_left_as_it_is() {
        let (points, tets, _) = block(2);
        let g = grid(&points, &tets);
        let s = LabelledSurface::build(&g, &vec![1; tets.len()]).unwrap();
        let tris = s.region_triangles(&[1]);
        let (mesh, report) = repair_pinches(s.points(), &tris).unwrap();
        assert_eq!(report, RepairReport::default());
        assert_eq!(mesh.triangles.len(), tris.len());
        assert!((volume_of(&mesh.points, &mesh.triangles) - 8.0).abs() < 1e-9);
        assert!(is_closed_manifold(&mesh.triangles));
    }

    #[test]
    fn pinches_inside_a_larger_region_are_repaired_with_the_volume_kept() {
        // A 2x2x2 block with the two cubes along a space diagonal removed: the
        // remaining cubes (0,0,1),(0,1,0)... touch along edges and at vertices.
        let (points, tets, cubes) = block(2);
        let kept: Vec<usize> = (0..tets.len())
            .filter(|&t| matches!(cubes[t], (0, 0, 0) | (1, 1, 0) | (1, 0, 1) | (0, 1, 1)))
            .collect();
        let sub_tets: Vec<[u32; 4]> = kept.iter().map(|&t| tets[t]).collect();
        let g = grid(&points, &sub_tets);
        let s = LabelledSurface::build(&g, &vec![1; sub_tets.len()]).unwrap();
        let tris = s.region_triangles(&[1]);
        assert!(!is_closed_manifold(&tris));
        let (mesh, report) = repair_pinches(s.points(), &tris).unwrap();
        assert!(is_closed_manifold(&mesh.triangles));
        assert!(report.split_vertices > 0);
        assert!((volume_of(&mesh.points, &mesh.triangles) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn a_handle_through_a_pinched_edge_needs_the_crossing_pairing() {
        // Cubes (0,0,1) and (1,1,1) touch only along one vertical edge, and are
        // also joined by a path of cubes above and another below, so the fans
        // round both ends of that edge are connected. Pairing the faces along the
        // solid wedges leaves the edge non-manifold; crossing them separates it.
        let (points, tets, cubes) = block(3);
        let wanted = [
            (0, 0, 1),
            (1, 1, 1),
            (0, 0, 2),
            (0, 1, 2),
            (1, 1, 2),
            (0, 0, 0),
            (0, 1, 0),
            (1, 1, 0),
        ];
        let kept: Vec<[u32; 4]> = (0..tets.len()).filter(|&t| wanted.contains(&cubes[t])).map(|t| tets[t]).collect();
        let g = grid(&points, &kept);
        let s = LabelledSurface::build(&g, &vec![1; kept.len()]).unwrap();
        let tris = s.region_triangles(&[1]);
        assert!(!is_closed_manifold(&tris));
        let (mesh, report) = repair_pinches(s.points(), &tris).unwrap();
        assert!(is_closed_manifold(&mesh.triangles));
        assert_eq!(report.nonmanifold_edges, 1);
        assert_eq!(report.alternate_pairings, 1, "the wedge pairing alone cannot separate it");
        assert!((volume_of(&mesh.points, &mesh.triangles) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn repair_reports_surfaces_it_cannot_handle() {
        // An open triangle fan: one open edge is left, so the result is unresolved.
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        assert_eq!(repair_pinches(&points, &[[0, 1, 2]]).unwrap_err(), RegionError::Unresolved { edges: 3 });
        // Two faces winding an edge the same way.
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0]];
        assert!(matches!(
            repair_pinches(&points, &[[0, 1, 2], [0, 1, 3]]).unwrap_err(),
            RegionError::Inconsistent { .. }
        ));
    }
}
