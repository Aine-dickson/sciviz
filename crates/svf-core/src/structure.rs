//! Explorable biological structures: a validated parts tree with recipe
//! geometry, authored as a small RON file. See `docs/structure-recipes.md`.
//!
//! Geometry is generated here in f64 and is graphics-free. Every recipe body
//! is a closed triangle mesh with outward winding.

use std::{collections::HashMap, error::Error, f64::consts::TAU, fmt};

use serde::Deserialize;

use crate::{
    geometry::{GeometryError, TriangleMesh3},
    spatial::WorldPoint3,
};

/// The only file version this reader accepts.
pub const SUPPORTED_VERSION: u32 = 1;
const MIN_CIRCLE_SEGMENTS: usize = 8;

fn default_one() -> usize {
    1
}

fn default_zero() -> f64 {
    0.0
}

/// File representation of a structure. Unknown fields are rejected so that
/// features deferred to later steps cannot be silently dropped.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Structure")]
pub struct StructureDocument {
    pub version: u32,
    pub name: String,
    pub units: String,
    pub reference: String,
    pub circle_segments: usize,
    pub parts: Vec<PartDocument>,
    /// Ordered layers, outermost first. Optional; see `docs/sections-and-peeling.md`.
    #[serde(default)]
    pub layers: Vec<LayerDocument>,
}

/// File representation of one layer: a named group of parts for peeling.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Layer")]
pub struct LayerDocument {
    pub id: String,
    pub name: String,
    pub parts: Vec<String>,
}

/// File representation of one part.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Part")]
pub struct PartDocument {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub description: String,
    pub colour: (f64, f64, f64),
    pub geometry: Option<Geometry>,
}

/// Recipe geometry for one part.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Geometry {
    /// A 2D profile swept along +Y from 0 to `axis_length`, about the Y axis.
    Extrusion { axis_length: f64, profile: Profile },
}

/// The 2D cross-section of an extrusion, in the XZ plane about the Y axis.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Profile {
    /// A solid disc.
    Disc { radius: f64 },
    /// A full ring.
    Annulus { inner: f64, outer: f64 },
    /// `count` ring sectors, each `width_deg` wide, evenly spaced about the
    /// axis; the first is centred on `first_angle_deg`.
    Sector {
        inner: f64,
        outer: f64,
        width_deg: f64,
        #[serde(default = "default_zero")]
        first_angle_deg: f64,
        #[serde(default = "default_one")]
        count: usize,
    },
}

/// Why a structure could not be read, validated or built.
#[derive(Clone, Debug, PartialEq)]
pub enum StructureError {
    Parse(String),
    UnsupportedVersion(u32),
    TooFewCircleSegments(usize),
    NoParts,
    EmptyId { index: usize },
    DuplicateId(String),
    UnknownParent { part: String, parent: String },
    ParentCycle(String),
    InvalidColour(String),
    InvalidDimension { part: String, what: &'static str },
    InnerNotLessThanOuter(String),
    InvalidSector { part: String, reason: &'static str },
    UnknownPartIndex(usize),
    EmptyLayerId { index: usize },
    DuplicateLayerId(String),
    EmptyLayer(String),
    UnknownLayerPart { layer: String, part: String },
    PartInTwoLayers { part: String },
    Geometry(GeometryError),
}

impl fmt::Display for StructureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "structure file could not be parsed: {message}"),
            Self::UnsupportedVersion(v) => {
                write!(f, "structure version {v} is not supported (expected {SUPPORTED_VERSION})")
            }
            Self::TooFewCircleSegments(n) => {
                write!(f, "circle_segments {n} is below the minimum {MIN_CIRCLE_SEGMENTS}")
            }
            Self::NoParts => write!(f, "structure has no parts"),
            Self::EmptyId { index } => write!(f, "part {index} has an empty id"),
            Self::DuplicateId(id) => write!(f, "duplicate part id '{id}'"),
            Self::UnknownParent { part, parent } => {
                write!(f, "part '{part}' names unknown parent '{parent}'")
            }
            Self::ParentCycle(id) => write!(f, "part '{id}' is part of a parent cycle"),
            Self::InvalidColour(id) => {
                write!(f, "part '{id}' colour channels must be finite and within [0, 1]")
            }
            Self::InvalidDimension { part, what } => {
                write!(f, "part '{part}': {what} must be finite and positive")
            }
            Self::InnerNotLessThanOuter(id) => write!(f, "part '{id}': inner must be less than outer"),
            Self::InvalidSector { part, reason } => write!(f, "part '{part}': {reason}"),
            Self::UnknownPartIndex(i) => write!(f, "part index {i} is out of range"),
            Self::EmptyLayerId { index } => write!(f, "layer {index} has an empty id"),
            Self::DuplicateLayerId(id) => write!(f, "duplicate layer id '{id}'"),
            Self::EmptyLayer(id) => write!(f, "layer '{id}' lists no parts"),
            Self::UnknownLayerPart { layer, part } => {
                write!(f, "layer '{layer}' names unknown part '{part}'")
            }
            Self::PartInTwoLayers { part } => write!(f, "part '{part}' is in more than one layer"),
            Self::Geometry(error) => write!(f, "generated geometry is invalid: {error}"),
        }
    }
}

impl Error for StructureError {}

impl From<GeometryError> for StructureError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

/// One validated part.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    id: String,
    name: String,
    parent: Option<usize>,
    description: String,
    colour: [f64; 3],
    geometry: Option<Geometry>,
}

impl Part {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Index of the parent part in [`Structure::parts`], if any.
    #[must_use]
    pub const fn parent(&self) -> Option<usize> {
        self.parent
    }
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }
    #[must_use]
    pub const fn colour(&self) -> [f64; 3] {
        self.colour
    }
    /// `None` for a grouping node.
    #[must_use]
    pub const fn geometry(&self) -> Option<&Geometry> {
        self.geometry.as_ref()
    }
}

/// One validated layer. A grouping node in the file expands to itself and all
/// its descendants, so `parts` holds every affected part index, ascending.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    id: String,
    name: String,
    parts: Vec<usize>,
}

impl Layer {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn parts(&self) -> &[usize] {
        &self.parts
    }
}

/// A validated structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Structure {
    name: String,
    units: String,
    reference: String,
    circle_segments: usize,
    parts: Vec<Part>,
    layers: Vec<Layer>,
}

/// The triangles of one part inside a [`StructureGeometry`] mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartRange {
    pub part_index: usize,
    pub first_triangle: usize,
    pub triangle_count: usize,
}

/// Merged recipe geometry for a set of parts.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureGeometry {
    mesh: TriangleMesh3,
    colours: Vec<[f64; 3]>,
    ranges: Vec<PartRange>,
}

impl StructureGeometry {
    #[must_use]
    pub const fn mesh(&self) -> &TriangleMesh3 {
        &self.mesh
    }
    /// One RGB colour per mesh vertex, from the owning part.
    #[must_use]
    pub fn colours(&self) -> &[[f64; 3]] {
        &self.colours
    }
    /// Contiguous triangle ranges in ascending order, one per built part.
    #[must_use]
    pub fn ranges(&self) -> &[PartRange] {
        &self.ranges
    }
    /// The part that owns a triangle of [`Self::mesh`].
    #[must_use]
    pub fn part_of_triangle(&self, triangle: usize) -> Option<usize> {
        self.ranges
            .iter()
            .find(|r| triangle >= r.first_triangle && triangle < r.first_triangle + r.triangle_count)
            .map(|r| r.part_index)
    }
}

impl Structure {
    /// Parses and validates a RON structure file.
    pub fn from_ron_str(text: &str) -> Result<Self, StructureError> {
        let document: StructureDocument =
            ron::de::from_str(text).map_err(|error| StructureError::Parse(error.to_string()))?;
        Self::from_document(document)
    }

    /// Validates a parsed document.
    pub fn from_document(document: StructureDocument) -> Result<Self, StructureError> {
        if document.version != SUPPORTED_VERSION {
            return Err(StructureError::UnsupportedVersion(document.version));
        }
        if document.circle_segments < MIN_CIRCLE_SEGMENTS {
            return Err(StructureError::TooFewCircleSegments(document.circle_segments));
        }
        if document.parts.is_empty() {
            return Err(StructureError::NoParts);
        }

        let mut by_id: HashMap<&str, usize> = HashMap::new();
        for (index, part) in document.parts.iter().enumerate() {
            if part.id.is_empty() {
                return Err(StructureError::EmptyId { index });
            }
            if by_id.insert(part.id.as_str(), index).is_some() {
                return Err(StructureError::DuplicateId(part.id.clone()));
            }
        }

        let mut parts = Vec::with_capacity(document.parts.len());
        for part in &document.parts {
            let parent = match &part.parent {
                None => None,
                Some(parent_id) => Some(*by_id.get(parent_id.as_str()).ok_or_else(|| {
                    StructureError::UnknownParent { part: part.id.clone(), parent: parent_id.clone() }
                })?),
            };
            let (r, g, b) = part.colour;
            if ![r, g, b].iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)) {
                return Err(StructureError::InvalidColour(part.id.clone()));
            }
            if let Some(geometry) = &part.geometry {
                validate_geometry(&part.id, geometry)?;
            }
            parts.push(Part {
                id: part.id.clone(),
                name: part.name.clone(),
                parent,
                description: part.description.clone(),
                colour: [r, g, b],
                geometry: part.geometry,
            });
        }

        for start in 0..parts.len() {
            let mut current = parts[start].parent;
            let mut steps = 0;
            while let Some(index) = current {
                steps += 1;
                if index == start || steps > parts.len() {
                    return Err(StructureError::ParentCycle(parts[start].id.clone()));
                }
                current = parts[index].parent;
            }
        }

        let layers = validate_layers(&document.layers, &parts, &by_id)?;

        Ok(Self {
            name: document.name,
            units: document.units,
            reference: document.reference,
            circle_segments: document.circle_segments,
            parts,
            layers,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn units(&self) -> &str {
        &self.units
    }
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }
    #[must_use]
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }

    /// Layers, outermost first (the order of removal when peeling).
    #[must_use]
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Parts to show when the `peeled` outermost layers are removed (clamped
    /// to the layer count) and, optionally, only `isolated` is wanted.
    /// Parts in no layer are never peeled. Ascending part order.
    #[must_use]
    pub fn visible_parts(&self, isolated: Option<usize>, peeled: usize) -> Vec<usize> {
        let mut hidden = vec![false; self.parts.len()];
        for layer in self.layers.iter().take(peeled) {
            for &part in &layer.parts {
                hidden[part] = true;
            }
        }
        (0..self.parts.len())
            .filter(|&i| !hidden[i] && isolated.is_none_or(|only| only == i))
            .collect()
    }

    /// Index of the part with this id.
    #[must_use]
    pub fn part_index(&self, id: &str) -> Option<usize> {
        self.parts.iter().position(|p| p.id == id)
    }

    /// Geometry of every part that has any.
    pub fn build_geometry(&self) -> Result<StructureGeometry, StructureError> {
        let all: Vec<usize> = (0..self.parts.len()).collect();
        self.build_geometry_for(&all)
    }

    /// Geometry of the listed parts only (grouping nodes contribute nothing),
    /// in ascending part order whatever the order of `visible`.
    pub fn build_geometry_for(&self, visible: &[usize]) -> Result<StructureGeometry, StructureError> {
        if let Some(&bad) = visible.iter().find(|&&i| i >= self.parts.len()) {
            return Err(StructureError::UnknownPartIndex(bad));
        }
        let mut wanted = vec![false; self.parts.len()];
        for &i in visible {
            wanted[i] = true;
        }

        let mut points: Vec<WorldPoint3> = Vec::new();
        let mut colours: Vec<[f64; 3]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut ranges = Vec::new();

        for (part_index, part) in self.parts.iter().enumerate() {
            let (true, Some(Geometry::Extrusion { axis_length, profile })) =
                (wanted[part_index], part.geometry)
            else {
                continue;
            };
            let mut builder = MeshBuilder::default();
            build_extrusion(&mut builder, profile, axis_length, self.circle_segments);

            let offset = u32::try_from(points.len()).map_err(|_| {
                StructureError::InvalidSector { part: part.id.clone(), reason: "mesh too large" }
            })?;
            let first_triangle = indices.len() / 3;
            colours.extend(std::iter::repeat_n(part.colour, builder.positions.len()));
            points.extend(builder.positions.iter().map(|p| WorldPoint3::new(p[0], p[1], p[2])));
            indices.extend(builder.indices.iter().map(|i| i + offset));
            ranges.push(PartRange {
                part_index,
                first_triangle,
                triangle_count: builder.indices.len() / 3,
            });
        }

        Ok(StructureGeometry { mesh: TriangleMesh3::new(points, indices)?, colours, ranges })
    }
}

fn validate_layers(
    documents: &[LayerDocument],
    parts: &[Part],
    by_id: &HashMap<&str, usize>,
) -> Result<Vec<Layer>, StructureError> {
    let mut seen: HashMap<&str, ()> = HashMap::new();
    let mut owner: Vec<Option<usize>> = vec![None; parts.len()];
    let mut layers = Vec::with_capacity(documents.len());
    for (layer_index, layer) in documents.iter().enumerate() {
        if layer.id.is_empty() {
            return Err(StructureError::EmptyLayerId { index: layer_index });
        }
        if seen.insert(layer.id.as_str(), ()).is_some() {
            return Err(StructureError::DuplicateLayerId(layer.id.clone()));
        }
        if layer.parts.is_empty() {
            return Err(StructureError::EmptyLayer(layer.id.clone()));
        }
        let mut members = vec![false; parts.len()];
        for part_id in &layer.parts {
            let root = *by_id.get(part_id.as_str()).ok_or_else(|| StructureError::UnknownLayerPart {
                layer: layer.id.clone(),
                part: part_id.clone(),
            })?;
            // The part and every descendant (a parent chain reaches `root`).
            for index in 0..parts.len() {
                let mut current = Some(index);
                while let Some(c) = current {
                    if c == root {
                        members[index] = true;
                        break;
                    }
                    current = parts[c].parent;
                }
            }
        }
        let mut indices = Vec::new();
        for (index, &member) in members.iter().enumerate() {
            if !member {
                continue;
            }
            if owner[index].is_some() {
                return Err(StructureError::PartInTwoLayers { part: parts[index].id.clone() });
            }
            owner[index] = Some(layer_index);
            indices.push(index);
        }
        layers.push(Layer { id: layer.id.clone(), name: layer.name.clone(), parts: indices });
    }
    Ok(layers)
}

impl StructureGeometry {
    /// Assemble a geometry from parts built elsewhere in the crate.
    pub(crate) fn from_parts(
        mesh: TriangleMesh3,
        colours: Vec<[f64; 3]>,
        ranges: Vec<PartRange>,
    ) -> Self {
        Self { mesh, colours, ranges }
    }
}

fn positive(part: &str, value: f64, what: &'static str) -> Result<(), StructureError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(StructureError::InvalidDimension { part: part.to_owned(), what })
    }
}

fn validate_geometry(part: &str, geometry: &Geometry) -> Result<(), StructureError> {
    let Geometry::Extrusion { axis_length, profile } = geometry;
    positive(part, *axis_length, "axis_length")?;
    match *profile {
        Profile::Disc { radius } => positive(part, radius, "radius"),
        Profile::Annulus { inner, outer } => {
            positive(part, inner, "inner")?;
            positive(part, outer, "outer")?;
            if inner >= outer {
                return Err(StructureError::InnerNotLessThanOuter(part.to_owned()));
            }
            Ok(())
        }
        Profile::Sector { inner, outer, width_deg, first_angle_deg, count } => {
            positive(part, inner, "inner")?;
            positive(part, outer, "outer")?;
            if inner >= outer {
                return Err(StructureError::InnerNotLessThanOuter(part.to_owned()));
            }
            positive(part, width_deg, "width_deg")?;
            if !first_angle_deg.is_finite() {
                return Err(StructureError::InvalidDimension {
                    part: part.to_owned(),
                    what: "first_angle_deg",
                });
            }
            if count == 0 {
                return Err(StructureError::InvalidSector {
                    part: part.to_owned(),
                    reason: "count must be at least 1",
                });
            }
            if width_deg >= 360.0 / count as f64 {
                return Err(StructureError::InvalidSector {
                    part: part.to_owned(),
                    reason: "width_deg must be less than 360 / count so sectors do not overlap",
                });
            }
            Ok(())
        }
    }
}

/// Indexed triangle builder that welds bit-identical points.
#[derive(Default)]
struct MeshBuilder {
    positions: Vec<[f64; 3]>,
    lookup: HashMap<[u64; 3], u32>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    fn vertex(&mut self, p: [f64; 3]) -> u32 {
        // `+ 0.0` turns -0.0 into 0.0 so equal points have equal bits.
        let p = [p[0] + 0.0, p[1] + 0.0, p[2] + 0.0];
        let key = p.map(f64::to_bits);
        if let Some(&index) = self.lookup.get(&key) {
            return index;
        }
        let index = self.positions.len() as u32;
        self.positions.push(p);
        self.lookup.insert(key, index);
        index
    }

    fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend([a, b, c]);
    }

    /// Two triangles (a, b, c) and (a, c, d).
    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.tri(a, b, c);
        self.tri(a, c, d);
    }
}

fn point(radius: f64, angle: f64, y: f64) -> [f64; 3] {
    [radius * angle.cos(), y, radius * angle.sin()]
}

fn build_extrusion(builder: &mut MeshBuilder, profile: Profile, length: f64, circle_segments: usize) {
    match profile {
        Profile::Disc { radius } => build_ring_body(builder, 0.0, radius, 0.0, TAU, true, circle_segments, length),
        Profile::Annulus { inner, outer } => {
            build_ring_body(builder, inner, outer, 0.0, TAU, true, circle_segments, length);
        }
        Profile::Sector { inner, outer, width_deg, first_angle_deg, count } => {
            let arc_segments = ((circle_segments as f64 * width_deg / 360.0).ceil() as usize).max(1);
            for copy in 0..count {
                let centre = (first_angle_deg + 360.0 * copy as f64 / count as f64).to_radians();
                let half = width_deg.to_radians() / 2.0;
                build_ring_body(builder, inner, outer, centre - half, centre + half, false, arc_segments, length);
            }
        }
    }
}

/// One closed body between radii `inner` (0 means a solid disc, full only) and
/// `outer`, over the angle range, from y = 0 to y = `length`.
#[allow(clippy::too_many_arguments)]
fn build_ring_body(
    builder: &mut MeshBuilder,
    inner: f64,
    outer: f64,
    start: f64,
    end: f64,
    full: bool,
    segments: usize,
    length: f64,
) {
    let angle = |j: usize| -> f64 {
        // Wrap a full ring so the last point is bit-identical to the first.
        let j = if full { j % segments } else { j };
        start + (end - start) * j as f64 / segments as f64
    };
    let hollow = inner > 0.0;

    // Vertex ids per ring position j in 0..=segments (bottom/top, inner/outer).
    let mut ob = Vec::new();
    let mut ot = Vec::new();
    let mut ib = Vec::new();
    let mut it = Vec::new();
    for j in 0..=segments {
        let a = angle(j);
        ob.push(builder.vertex(point(outer, a, 0.0)));
        ot.push(builder.vertex(point(outer, a, length)));
        if hollow {
            ib.push(builder.vertex(point(inner, a, 0.0)));
            it.push(builder.vertex(point(inner, a, length)));
        }
    }
    let centre_bottom = builder.vertex([0.0, 0.0, 0.0]);
    let centre_top = builder.vertex([0.0, length, 0.0]);

    for j in 0..segments {
        let k = j + 1;
        // Outer wall, normal outward.
        builder.quad(ob[j], ot[j], ot[k], ob[k]);
        if hollow {
            // Inner wall, normal toward the axis.
            builder.quad(ib[j], ib[k], it[k], it[j]);
            // Top cap (+Y) and bottom cap (-Y).
            builder.quad(it[j], it[k], ot[k], ot[j]);
            builder.quad(ib[j], ob[j], ob[k], ib[k]);
        } else {
            // Solid disc caps as fans about the axis.
            builder.tri(centre_top, ot[k], ot[j]);
            builder.tri(centre_bottom, ob[j], ob[k]);
        }
    }

    if !full {
        // Radial end walls. The start wall faces -theta, the end wall +theta.
        let (s, e) = (0, segments);
        if hollow {
            builder.quad(ib[s], it[s], ot[s], ob[s]);
            builder.quad(ib[e], ob[e], ot[e], it[e]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../fixtures/structures/dicot-stem.ron");

    fn structure(parts: &str) -> Result<Structure, StructureError> {
        Structure::from_ron_str(&format!(
            "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 48, parts: [{parts}])"
        ))
    }

    fn part(id: &str, parent: &str, colour: &str, geometry: &str) -> String {
        format!(
            "Part(id: \"{id}\", name: \"{id}\", parent: {parent}, description: \"d\", colour: {colour}, geometry: {geometry}),"
        )
    }

    const GREEN: &str = "(0.2, 0.6, 0.3)";

    fn annulus(inner: f64, outer: f64) -> String {
        format!("Some(Extrusion(axis_length: 10.0, profile: Annulus(inner: {inner}, outer: {outer})))")
    }

    /// Signed volume by the divergence theorem; positive means outward winding.
    fn signed_volume(mesh: &TriangleMesh3) -> f64 {
        let v = mesh.vertices();
        mesh.indices()
            .chunks_exact(3)
            .map(|t| {
                let a = v[t[0] as usize].to_array();
                let b = v[t[1] as usize].to_array();
                let c = v[t[2] as usize].to_array();
                let cross = [
                    b[1] * c[2] - b[2] * c[1],
                    b[2] * c[0] - b[0] * c[2],
                    b[0] * c[1] - b[1] * c[0],
                ];
                (a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0
            })
            .sum()
    }

    fn assert_closed(mesh: &TriangleMesh3) {
        let mut directed: HashMap<(u32, u32), usize> = HashMap::new();
        for t in mesh.indices().chunks_exact(3) {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *directed.entry((a, b)).or_default() += 1;
            }
        }
        for (&(a, b), &n) in &directed {
            assert_eq!(n, 1, "directed edge ({a},{b}) used {n} times");
            assert_eq!(directed.get(&(b, a)), Some(&1), "edge ({a},{b}) has no single opposite edge");
        }
    }

    fn polygon_area(segments: usize, outer: f64, inner: f64, step_angle: f64) -> f64 {
        segments as f64 / 2.0 * (outer * outer - inner * inner) * step_angle.sin()
    }

    #[test]
    fn annulus_is_closed_outward_and_has_polygon_volume() {
        let s = structure(&part("a", "None", GREEN, &annulus(2.0, 3.0))).unwrap();
        let g = s.build_geometry().unwrap();
        assert_closed(g.mesh());
        let expected = polygon_area(48, 3.0, 2.0, TAU / 48.0) * 10.0;
        assert!((signed_volume(g.mesh()) - expected).abs() < 1e-9);
    }

    #[test]
    fn disc_is_closed_outward_and_has_polygon_volume() {
        let geometry = "Some(Extrusion(axis_length: 10.0, profile: Disc(radius: 2.0)))";
        let s = structure(&part("d", "None", GREEN, geometry)).unwrap();
        let g = s.build_geometry().unwrap();
        assert_closed(g.mesh());
        let expected = polygon_area(48, 2.0, 0.0, TAU / 48.0) * 10.0;
        assert!((signed_volume(g.mesh()) - expected).abs() < 1e-9);
    }

    #[test]
    fn repeated_sectors_are_closed_with_the_expected_volume() {
        let geometry = "Some(Extrusion(axis_length: 10.0, profile: Sector(inner: 2.0, outer: 3.0, width_deg: 30.0, count: 8)))";
        let s = structure(&part("s", "None", GREEN, geometry)).unwrap();
        let g = s.build_geometry().unwrap();
        assert_closed(g.mesh());
        // 48 * 30/360 = 4 arc segments per sector, 8 sectors.
        let step = 30.0_f64.to_radians() / 4.0;
        let expected = 8.0 * polygon_area(4, 3.0, 2.0, step) * 10.0;
        assert!((signed_volume(g.mesh()) - expected).abs() < 1e-9);
    }

    #[test]
    fn ranges_are_contiguous_cover_the_mesh_and_map_triangles_to_parts() {
        let text = format!(
            "{}{}{}",
            part("group", "None", GREEN, "None"),
            part("a", "Some(\"group\")", GREEN, &annulus(2.0, 3.0)),
            part("b", "Some(\"group\")", GREEN, &annulus(3.0, 4.0)),
        );
        let s = structure(&text).unwrap();
        let g = s.build_geometry().unwrap();
        assert_eq!(g.ranges().len(), 2, "the grouping node has no geometry");
        assert_eq!(g.ranges()[0].first_triangle, 0);
        assert_eq!(
            g.ranges()[1].first_triangle,
            g.ranges()[0].triangle_count,
        );
        let total: usize = g.ranges().iter().map(|r| r.triangle_count).sum();
        assert_eq!(total * 3, g.mesh().indices().len());
        assert_eq!(g.colours().len(), g.mesh().vertices().len());
        assert_eq!(g.part_of_triangle(0), Some(1));
        assert_eq!(g.part_of_triangle(total - 1), Some(2));
        assert_eq!(g.part_of_triangle(total), None);
    }

    #[test]
    fn build_for_a_subset_contains_only_those_parts() {
        let text = format!(
            "{}{}",
            part("a", "None", GREEN, &annulus(2.0, 3.0)),
            part("b", "None", GREEN, &annulus(3.0, 4.0)),
        );
        let s = structure(&text).unwrap();
        let only_b = s.build_geometry_for(&[1]).unwrap();
        assert_eq!(only_b.ranges().len(), 1);
        assert_eq!(only_b.ranges()[0].part_index, 1);
        assert_eq!(s.build_geometry_for(&[7]).unwrap_err(), StructureError::UnknownPartIndex(7));
    }

    #[test]
    fn validation_rejects_each_error_class() {
        let ok = |id: &str| part(id, "None", GREEN, &annulus(2.0, 3.0));
        assert_eq!(
            structure(&format!("{}{}", ok("a"), ok("a"))).unwrap_err(),
            StructureError::DuplicateId("a".into())
        );
        assert!(matches!(
            structure(&part("a", "Some(\"zzz\")", GREEN, "None")).unwrap_err(),
            StructureError::UnknownParent { .. }
        ));
        assert!(matches!(
            structure(&format!(
                "{}{}",
                part("a", "Some(\"b\")", GREEN, "None"),
                part("b", "Some(\"a\")", GREEN, "None")
            ))
            .unwrap_err(),
            StructureError::ParentCycle(_)
        ));
        assert_eq!(
            structure(&part("a", "Some(\"a\")", GREEN, "None")).unwrap_err(),
            StructureError::ParentCycle("a".into())
        );
        assert_eq!(
            structure(&part("a", "None", "(1.5, 0.0, 0.0)", "None")).unwrap_err(),
            StructureError::InvalidColour("a".into())
        );
        assert!(matches!(
            structure(&part("a", "None", GREEN, &annulus(3.0, 3.0))).unwrap_err(),
            StructureError::InnerNotLessThanOuter(_)
        ));
        assert!(matches!(
            structure(&part("a", "None", GREEN, &annulus(-1.0, 3.0))).unwrap_err(),
            StructureError::InvalidDimension { what: "inner", .. }
        ));
        let zero_length = "Some(Extrusion(axis_length: 0.0, profile: Disc(radius: 1.0)))";
        assert!(matches!(
            structure(&part("a", "None", GREEN, zero_length)).unwrap_err(),
            StructureError::InvalidDimension { what: "axis_length", .. }
        ));
        let overlap = "Some(Extrusion(axis_length: 1.0, profile: Sector(inner: 1.0, outer: 2.0, width_deg: 50.0, count: 8)))";
        assert!(matches!(
            structure(&part("a", "None", GREEN, overlap)).unwrap_err(),
            StructureError::InvalidSector { .. }
        ));
        let zero_count = "Some(Extrusion(axis_length: 1.0, profile: Sector(inner: 1.0, outer: 2.0, width_deg: 10.0, count: 0)))";
        assert!(matches!(
            structure(&part("a", "None", GREEN, zero_count)).unwrap_err(),
            StructureError::InvalidSector { .. }
        ));
        assert_eq!(structure("").unwrap_err(), StructureError::NoParts);
        assert!(matches!(
            structure(&part("", "None", GREEN, "None")).unwrap_err(),
            StructureError::EmptyId { .. }
        ));
    }

    #[test]
    fn document_level_errors_are_reported() {
        let bad_version = "Structure(version: 2, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 48, parts: [])";
        assert_eq!(
            Structure::from_ron_str(bad_version).unwrap_err(),
            StructureError::UnsupportedVersion(2)
        );
        let few = "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 4, parts: [])";
        assert_eq!(Structure::from_ron_str(few).unwrap_err(), StructureError::TooFewCircleSegments(4));
        let unknown = "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 48, parts: [], views: [])";
        assert!(matches!(Structure::from_ron_str(unknown).unwrap_err(), StructureError::Parse(_)));
        assert!(matches!(Structure::from_ron_str("not ron").unwrap_err(), StructureError::Parse(_)));
    }

    fn layered(layers: &str) -> Result<Structure, StructureError> {
        let parts = format!(
            "{}{}{}",
            part("root", "None", GREEN, "None"),
            part("a", "Some(\"root\")", GREEN, &annulus(2.0, 3.0)),
            part("b", "Some(\"root\")", GREEN, &annulus(1.0, 2.0)),
        );
        Structure::from_ron_str(&format!(
            "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 48, parts: [{parts}], layers: [{layers}])"
        ))
    }

    fn layer(id: &str, parts: &str) -> String {
        format!("Layer(id: \"{id}\", name: \"{id}\", parts: [{parts}]),")
    }

    #[test]
    fn layers_are_validated() {
        assert!(layered("").unwrap().layers().is_empty());
        let ok = layered(&format!("{}{}", layer("l1", "\"a\""), layer("l2", "\"b\""))).unwrap();
        assert_eq!(ok.layers().len(), 2);
        assert_eq!(ok.layers()[0].parts(), &[1]);
        assert_eq!(
            layered(&format!("{}{}", layer("x", "\"a\""), layer("x", "\"b\""))).unwrap_err(),
            StructureError::DuplicateLayerId("x".into())
        );
        assert_eq!(layered(&layer("e", "")).unwrap_err(), StructureError::EmptyLayer("e".into()));
        assert_eq!(
            layered(&layer("", "\"a\"")).unwrap_err(),
            StructureError::EmptyLayerId { index: 0 }
        );
        assert_eq!(
            layered(&layer("u", "\"nope\"")).unwrap_err(),
            StructureError::UnknownLayerPart { layer: "u".into(), part: "nope".into() }
        );
        assert_eq!(
            layered(&format!("{}{}", layer("l1", "\"a\""), layer("l2", "\"a\", \"b\""))).unwrap_err(),
            StructureError::PartInTwoLayers { part: "a".into() }
        );
        // A grouping node expands to itself and its descendants.
        let group = layered(&layer("g", "\"root\"")).unwrap();
        assert_eq!(group.layers()[0].parts(), &[0, 1, 2]);
        // A part under a group already in another layer is a conflict.
        assert!(layered(&format!("{}{}", layer("g", "\"root\""), layer("l", "\"a\""))).is_err());
    }

    #[test]
    fn visible_parts_follows_peeling_and_isolation() {
        let s = layered(&format!("{}{}", layer("l1", "\"a\""), layer("l2", "\"b\""))).unwrap();
        assert_eq!(s.visible_parts(None, 0), vec![0, 1, 2]);
        assert_eq!(s.visible_parts(None, 1), vec![0, 2]);
        assert_eq!(s.visible_parts(None, 2), vec![0]);
        assert_eq!(s.visible_parts(None, 9), vec![0]);
        assert_eq!(s.visible_parts(Some(2), 0), vec![2]);
        assert_eq!(s.visible_parts(Some(1), 1), Vec::<usize>::new());
    }

    #[test]
    fn shipped_dicot_stem_has_four_layers_outermost_first() {
        let s = Structure::from_ron_str(EXAMPLE).unwrap();
        let ids: Vec<_> = s.layers().iter().map(Layer::id).collect();
        assert_eq!(ids, ["epidermis", "cortex", "vascular", "pith"]);
        let vascular = s.layers()[2].parts();
        for id in ["xylem", "phloem", "vascular-cambium", "medullary-rays"] {
            assert!(vascular.contains(&s.part_index(id).unwrap()), "{id}");
        }
        let visible = s.visible_parts(None, 3);
        assert!(visible.contains(&s.part_index("pith").unwrap()));
        assert!(!visible.contains(&s.part_index("cortex").unwrap()));
    }

    #[test]
    fn shipped_dicot_stem_example_parses_and_tiles_the_cross_section() {
        let s = Structure::from_ron_str(EXAMPLE).unwrap();
        for id in [
            "epidermis",
            "cortex",
            "pith",
            "xylem",
            "vascular-cambium",
            "phloem",
            "medullary-rays",
        ] {
            assert!(s.part_index(id).is_some(), "missing part {id}");
        }
        assert!(!s.reference().is_empty());
        let g = s.build_geometry().unwrap();
        assert_closed(g.mesh());

        // Epidermis + cortex + pith + bundle tissues + rays tile a solid
        // cylinder: total volume equals the outer disc's polygon volume.
        let outer_radius = 5.0;
        let length = 12.0;
        let polygon_disc = polygon_area(48, outer_radius, 0.0, TAU / 48.0) * length;
        let total = signed_volume(g.mesh());
        // Sector arcs use ceil(48*w/360) segments, so the tiling is exact only
        // up to polygon error; the 1 % bound is far above that and far below
        // any missing or duplicated tissue.
        assert!(((total - polygon_disc) / polygon_disc).abs() < 0.01, "total {total} vs {polygon_disc}");
    }
}
