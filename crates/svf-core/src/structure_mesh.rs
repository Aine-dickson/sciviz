//! The `.svfmesh` container and the provenance block of a structure.
//! Design: `docs/heart-structure.md` sections 8 and 18.
//!
//! A container holds named closed triangle bodies. Layout (all little endian):
//! the 8 bytes `SVFMESH1`, a `u32` body count, then per body a `u32` name length
//! and the UTF-8 name, a `u32` point count, a `u32` triangle count, the points as
//! `f32` triples, the triangles as `u32` triples; and last a FNV-1a 64-bit
//! checksum of every byte before it (integrity against truncation and
//! corruption, not security). Positions are `f32` on disk and `f64` in memory.

use std::{error::Error, fmt};

use serde::Deserialize;

use crate::region_surface::{is_closed_manifold, signed_volume, P3};

const MAGIC: &[u8; 8] = b"SVFMESH1";

/// One named closed, outward-wound triangle body.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshBody {
    pub name: String,
    pub points: Vec<P3>,
    pub triangles: Vec<[u32; 3]>,
}

/// A set of bodies read from, or to be written to, one file.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MeshContainer {
    pub bodies: Vec<MeshBody>,
}

/// Why a container could not be written or read.
#[derive(Clone, Debug, PartialEq)]
pub enum MeshError {
    BadMagic,
    Truncated,
    BadChecksum,
    TrailingBytes,
    BadName,
    DuplicateBody(String),
    NonFinitePoint { body: String },
    IndexOutOfRange { body: String },
    /// A point does not survive the `f32` round trip (too large).
    NotRepresentable { body: String },
    EmptyBody(String),
    NotClosed(String),
    NotOutward(String),
    TooLarge,
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not an .svfmesh file (bad magic)"),
            Self::Truncated => write!(f, "the .svfmesh file is truncated"),
            Self::BadChecksum => write!(f, "the .svfmesh checksum does not match (corrupted file)"),
            Self::TrailingBytes => write!(f, "unexpected bytes after the last body"),
            Self::BadName => write!(f, "a body name is not valid UTF-8"),
            Self::DuplicateBody(n) => write!(f, "two bodies are named '{n}'"),
            Self::NonFinitePoint { body } => write!(f, "body '{body}' has a non-finite point"),
            Self::IndexOutOfRange { body } => write!(f, "body '{body}' has a triangle naming a missing point"),
            Self::NotRepresentable { body } => write!(f, "body '{body}' has a point too large for f32"),
            Self::EmptyBody(n) => write!(f, "body '{n}' has no triangles"),
            Self::NotClosed(n) => write!(f, "body '{n}' is not a closed 2-manifold"),
            Self::NotOutward(n) => write!(f, "body '{n}' is not wound outward (non-positive volume)"),
            Self::TooLarge => write!(f, "a count does not fit in 32 bits"),
        }
    }
}

impl Error for MeshError {}

/// FNV-1a, 64 bit.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn check_body(body: &MeshBody) -> Result<(), MeshError> {
    let name = || body.name.clone();
    if body.triangles.is_empty() {
        return Err(MeshError::EmptyBody(name()));
    }
    if body.points.iter().any(|p| p.iter().any(|v| !v.is_finite())) {
        return Err(MeshError::NonFinitePoint { body: name() });
    }
    if body.triangles.iter().flatten().any(|&i| i as usize >= body.points.len()) {
        return Err(MeshError::IndexOutOfRange { body: name() });
    }
    if !is_closed_manifold(&body.triangles) {
        return Err(MeshError::NotClosed(name()));
    }
    if signed_volume(&body.points, &body.triangles) <= 0.0 {
        return Err(MeshError::NotOutward(name()));
    }
    Ok(())
}

impl MeshContainer {
    /// Checks every body (finite, indexed, closed, outward) and unique names.
    pub fn validate(&self) -> Result<(), MeshError> {
        for (i, body) in self.bodies.iter().enumerate() {
            if self.bodies[..i].iter().any(|b| b.name == body.name) {
                return Err(MeshError::DuplicateBody(body.name.clone()));
            }
            check_body(body)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn body(&self, name: &str) -> Option<&MeshBody> {
        self.bodies.iter().find(|b| b.name == name)
    }

    /// Serialises after validating. Points are rounded to `f32`; a body whose
    /// rounded points would no longer be finite is refused.
    pub fn to_bytes(&self) -> Result<Vec<u8>, MeshError> {
        self.validate()?;
        let count = |n: usize| u32::try_from(n).map_err(|_| MeshError::TooLarge);
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&count(self.bodies.len())?.to_le_bytes());
        for body in &self.bodies {
            out.extend_from_slice(&count(body.name.len())?.to_le_bytes());
            out.extend_from_slice(body.name.as_bytes());
            out.extend_from_slice(&count(body.points.len())?.to_le_bytes());
            out.extend_from_slice(&count(body.triangles.len())?.to_le_bytes());
            for p in &body.points {
                for &v in p {
                    let narrow = v as f32;
                    if !narrow.is_finite() {
                        return Err(MeshError::NotRepresentable { body: body.name.clone() });
                    }
                    out.extend_from_slice(&narrow.to_le_bytes());
                }
            }
            for &i in body.triangles.iter().flatten() {
                out.extend_from_slice(&i.to_le_bytes());
            }
        }
        let sum = fnv1a(&out);
        out.extend_from_slice(&sum.to_le_bytes());
        Ok(out)
    }

    /// Reads and validates a container. The checksum is verified first.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MeshError> {
        if bytes.len() < MAGIC.len() {
            return Err(MeshError::Truncated);
        }
        if &bytes[..MAGIC.len()] != MAGIC {
            return Err(MeshError::BadMagic);
        }
        if bytes.len() < MAGIC.len() + 4 + 8 {
            return Err(MeshError::Truncated);
        }
        let (payload, tail) = bytes.split_at(bytes.len() - 8);
        if fnv1a(payload).to_le_bytes() != tail {
            return Err(MeshError::BadChecksum);
        }

        let mut at = MAGIC.len();
        let take = |at: &mut usize, n: usize| -> Result<&[u8], MeshError> {
            let end = at.checked_add(n).filter(|&e| e <= payload.len()).ok_or(MeshError::Truncated)?;
            let slice = &payload[*at..end];
            *at = end;
            Ok(slice)
        };
        let word = |at: &mut usize| -> Result<u32, MeshError> {
            Ok(u32::from_le_bytes(take(at, 4)?.try_into().expect("4 bytes")))
        };

        let bodies_n = word(&mut at)? as usize;
        let mut bodies = Vec::new();
        for _ in 0..bodies_n {
            let name_len = word(&mut at)? as usize;
            let name = std::str::from_utf8(take(&mut at, name_len)?).map_err(|_| MeshError::BadName)?.to_owned();
            let (np, nt) = (word(&mut at)? as usize, word(&mut at)? as usize);
            // Length checks before allocating, so a corrupt count cannot ask for gigabytes.
            let need = np.checked_mul(12).and_then(|a| nt.checked_mul(12).and_then(|b| a.checked_add(b)));
            if need.is_none_or(|n| n > payload.len() - at) {
                return Err(MeshError::Truncated);
            }
            let mut points = Vec::with_capacity(np);
            for _ in 0..np {
                let mut p = [0.0f64; 3];
                for v in &mut p {
                    *v = f64::from(f32::from_le_bytes(take(&mut at, 4)?.try_into().expect("4 bytes")));
                }
                points.push(p);
            }
            let mut triangles = Vec::with_capacity(nt);
            for _ in 0..nt {
                triangles.push([word(&mut at)?, word(&mut at)?, word(&mut at)?]);
            }
            bodies.push(MeshBody { name, points, triangles });
        }
        if at != payload.len() {
            return Err(MeshError::TrailingBytes);
        }
        let container = Self { bodies };
        container.validate()?;
        Ok(container)
    }
}

/// Where a derived structure came from, shown by the Explorer at startup.
/// Every field is a statement of fact about the data, not a rendering hint.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Provenance")]
pub struct Provenance {
    pub dataset: String,
    pub doi: String,
    pub licence: String,
    /// The attribution text the licence requires.
    pub attribution: String,
    pub source_sha256: String,
    pub tool: String,
    pub tool_commit: String,
    /// Named parameters of the derivation (`epsilon`, tag groups, ...).
    #[serde(default)]
    pub parameters: Vec<(String, String)>,
    /// Rigid frame transform, row-major 3x4 (rotation then translation), if any.
    #[serde(default)]
    pub frame_transform: Option<[f64; 12]>,
    /// Source volume in mm3 per body name, for checking the derived mesh.
    #[serde(default)]
    pub body_volumes_mm3: Vec<(String, f64)>,
    /// True when the geometry was changed from the source (simplified, re-framed).
    pub modified: bool,
}

impl Provenance {
    /// The lines the Explorer prints at startup.
    #[must_use]
    pub fn summary(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Data: {} (DOI {}), licence {}", self.dataset, self.doi, self.licence),
            format!("Attribution: {}", self.attribution),
            format!("Source SHA-256: {}", self.source_sha256),
            format!("Derived with {} at {}", self.tool, self.tool_commit),
        ];
        if self.modified {
            lines.push("This geometry is modified from the source data.".to_owned());
        }
        lines
    }

    pub(crate) fn check(&self) -> Result<(), &'static str> {
        let blank = |s: &str| s.trim().is_empty();
        if blank(&self.dataset) || blank(&self.doi) || blank(&self.licence) || blank(&self.attribution) {
            return Err("dataset, doi, licence and attribution must not be empty");
        }
        if self.source_sha256.len() != 64 || !self.source_sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("source_sha256 must be 64 hexadecimal characters");
        }
        if let Some(t) = &self.frame_transform {
            if t.iter().any(|v| !v.is_finite()) {
                return Err("frame_transform must be finite");
            }
        }
        if self.body_volumes_mm3.iter().any(|(_, v)| !(v.is_finite() && *v > 0.0)) {
            return Err("body volumes must be finite and positive");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tetra(name: &str, scale: f64) -> MeshBody {
        MeshBody {
            name: name.to_owned(),
            points: vec![[0.0, 0.0, 0.0], [scale, 0.0, 0.0], [0.0, scale, 0.0], [0.0, 0.0, scale]],
            triangles: vec![[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        }
    }

    fn sample() -> MeshContainer {
        MeshContainer { bodies: vec![tetra("a", 1.0), tetra("b", 2.5)] }
    }

    #[test]
    fn round_trips_with_f32_positions() {
        let c = sample();
        let back = MeshContainer::from_bytes(&c.to_bytes().unwrap()).unwrap();
        assert_eq!(back, c);
        let mut odd = sample();
        odd.bodies[0].points[1][0] = 0.1;
        let back = MeshContainer::from_bytes(&odd.to_bytes().unwrap()).unwrap();
        assert_eq!(back.bodies[0].points[1][0], f64::from(0.1f32));
        assert_eq!(back.body("b").unwrap().triangles.len(), 4);
        assert!(back.body("missing").is_none());
    }

    #[test]
    fn bytes_are_deterministic() {
        assert_eq!(sample().to_bytes().unwrap(), sample().to_bytes().unwrap());
    }

    #[test]
    fn damage_is_detected() {
        let good = sample().to_bytes().unwrap();
        assert_eq!(MeshContainer::from_bytes(&good[..4]), Err(MeshError::Truncated));
        let mut bad_magic = good.clone();
        bad_magic[0] = b'X';
        assert_eq!(MeshContainer::from_bytes(&bad_magic), Err(MeshError::BadMagic));
        for cut in [12, good.len() / 2, good.len() - 1] {
            assert!(MeshContainer::from_bytes(&good[..cut]).is_err(), "cut {cut}");
        }
        let mut flipped = good.clone();
        flipped[40] ^= 1;
        assert_eq!(MeshContainer::from_bytes(&flipped), Err(MeshError::BadChecksum));
        // Trailing data inside a valid checksum.
        let mut extra = good[..good.len() - 8].to_vec();
        extra.push(0);
        let sum = fnv1a(&extra);
        extra.extend_from_slice(&sum.to_le_bytes());
        assert_eq!(MeshContainer::from_bytes(&extra), Err(MeshError::TrailingBytes));
    }

    #[test]
    fn a_huge_count_with_a_valid_checksum_is_refused_without_allocating() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.push(b'x');
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        let sum = fnv1a(&bytes);
        bytes.extend_from_slice(&sum.to_le_bytes());
        assert_eq!(MeshContainer::from_bytes(&bytes), Err(MeshError::Truncated));
    }

    #[test]
    fn bodies_must_be_closed_outward_and_named_once() {
        let mut open = sample();
        open.bodies[0].triangles.pop();
        assert_eq!(open.to_bytes(), Err(MeshError::NotClosed("a".into())));

        let mut inside_out = sample();
        for t in &mut inside_out.bodies[1].triangles {
            t.swap(1, 2);
        }
        assert_eq!(inside_out.to_bytes(), Err(MeshError::NotOutward("b".into())));

        let mut twice = sample();
        twice.bodies[1].name = "a".into();
        assert_eq!(twice.to_bytes(), Err(MeshError::DuplicateBody("a".into())));

        let mut bad_index = sample();
        bad_index.bodies[0].triangles[0] = [0, 2, 9];
        assert_eq!(bad_index.to_bytes(), Err(MeshError::IndexOutOfRange { body: "a".into() }));

        let mut nan = sample();
        nan.bodies[0].points[0][1] = f64::NAN;
        assert_eq!(nan.to_bytes(), Err(MeshError::NonFinitePoint { body: "a".into() }));

        let mut huge = sample();
        huge.bodies[0] = tetra("a", 1e300);
        assert_eq!(huge.to_bytes(), Err(MeshError::NotRepresentable { body: "a".into() }));

        let empty = MeshContainer { bodies: vec![MeshBody { name: "e".into(), points: vec![], triangles: vec![] }] };
        assert_eq!(empty.to_bytes(), Err(MeshError::EmptyBody("e".into())));
    }

    #[test]
    fn provenance_summary_and_checks() {
        let p = Provenance {
            dataset: "Rodero 2021".into(),
            doi: "10.5281/zenodo.4590294".into(),
            licence: "CC BY 4.0".into(),
            attribution: "Rodero et al. 2021".into(),
            source_sha256: "a".repeat(64),
            tool: "svf-heart".into(),
            tool_commit: "abc1234".into(),
            parameters: vec![("epsilon".into(), "0.4".into())],
            frame_transform: None,
            body_volumes_mm3: vec![("lv".into(), 1.0)],
            modified: true,
        };
        assert_eq!(p.check(), Ok(()));
        let lines = p.summary();
        assert!(lines[0].contains("10.5281/zenodo.4590294") && lines[0].contains("CC BY 4.0"));
        assert!(lines.last().unwrap().contains("modified"));
        assert!(Provenance { source_sha256: "xyz".into(), ..p.clone() }.check().is_err());
        assert!(Provenance { attribution: " ".into(), ..p.clone() }.check().is_err());
        assert!(Provenance { body_volumes_mm3: vec![("a".into(), -1.0)], ..p.clone() }.check().is_err());
        let unmodified = Provenance { modified: false, ..p };
        assert!(!unmodified.summary().iter().any(|l| l.contains("modified")));
    }

    // ---- inside a structure

    use crate::structure::{Structure, StructureError};

    const PROVENANCE: &str = "provenance: Some(Provenance(dataset: \"Rodero 2021\", doi: \"10.5281/zenodo.4590294\",         licence: \"CC BY 4.0\", attribution: \"Rodero et al.\", source_sha256: \"%SHA%\", tool: \"t\",         tool_commit: \"abc\", parameters: [(\"epsilon\", \"0.4\")],         frame_transform: Some((1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0)),         body_volumes_mm3: [(\"a\", 1.0)], modified: true))";

    fn text(geometry: &str, extras: &str, provenance: &str) -> String {
        format!(
            "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 8,              parts: [Part(id: \"p\", name: \"p\", parent: None, description: \"d\", colour: (0.5, 0.5, 0.5),              geometry: Some({geometry}){extras})], {provenance})"
        )
        .replace("%SHA%", &"b".repeat(64))
    }

    fn resolver(bytes: Vec<u8>) -> impl Fn(&str) -> Result<Vec<u8>, String> {
        move |name| if name == "m.svfmesh" { Ok(bytes.clone()) } else { Err(format!("no file {name}")) }
    }

    const MESH_PART: &str = "Mesh(file: \"m.svfmesh\", body: \"b\")";

    #[test]
    fn a_mesh_part_builds_the_stored_body() {
        let bytes = sample().to_bytes().unwrap();
        let s = Structure::from_ron_with(&text(MESH_PART, "", PROVENANCE), &resolver(bytes)).unwrap();
        let g = s.build_geometry().unwrap();
        assert_eq!(g.mesh().indices().len(), 12);
        assert_eq!(g.mesh().vertices().len(), 4);
        assert_eq!(g.mesh().vertices()[1].to_array(), [2.5, 0.0, 0.0]);
        assert_eq!(g.ranges().len(), 1);
        let provenance = s.provenance().unwrap();
        assert_eq!(provenance.parameters, vec![("epsilon".to_owned(), "0.4".to_owned())]);
        assert_eq!(provenance.frame_transform.unwrap()[0], 1.0);
    }

    #[test]
    fn copies_and_mirror_apply_to_a_mesh_part() {
        let bytes = sample().to_bytes().unwrap();
        let s = Structure::from_ron_with(&text(MESH_PART, ", mirror: Some(X)", ""), &resolver(bytes)).unwrap();
        let g = s.build_geometry().unwrap();
        assert_eq!(g.mesh().indices().len(), 24);
        assert!(crate::region_surface::is_closed_manifold(
            &g.mesh().indices().chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect::<Vec<_>>()
        ));
    }

    #[test]
    fn a_plain_reader_refuses_mesh_parts() {
        assert_eq!(
            Structure::from_ron_str(&text(MESH_PART, "", "")),
            Err(StructureError::MeshNeedsResolver { part: "p".into() })
        );
    }

    #[test]
    fn mesh_errors_are_reported_by_file_and_part() {
        let bytes = sample().to_bytes().unwrap();
        let missing = text("Mesh(file: \"gone.svfmesh\", body: \"b\")", "", "");
        assert!(matches!(
            Structure::from_ron_with(&missing, &resolver(bytes.clone())),
            Err(StructureError::MeshFile { file, .. }) if file == "gone.svfmesh"
        ));
        let no_body = text("Mesh(file: \"m.svfmesh\", body: \"zzz\")", "", "");
        assert_eq!(
            Structure::from_ron_with(&no_body, &resolver(bytes.clone())),
            Err(StructureError::UnknownMeshBody { part: "p".into(), file: "m.svfmesh".into(), body: "zzz".into() })
        );
        let mut damaged = bytes;
        damaged[30] ^= 1;
        assert!(matches!(
            Structure::from_ron_with(&text(MESH_PART, "", ""), &resolver(damaged)),
            Err(StructureError::MeshFile { reason, .. }) if reason.contains("checksum")
        ));
    }

    #[test]
    fn bad_provenance_is_refused_and_recipe_files_are_unchanged() {
        let bad = PROVENANCE.replace("%SHA%", "short");
        let bytes = sample().to_bytes().unwrap();
        assert!(matches!(
            Structure::from_ron_with(&text(MESH_PART, "", &bad), &resolver(bytes)),
            Err(StructureError::InvalidProvenance(_))
        ));
        let recipe = text("Ellipsoid(centre: (0.0, 0.0, 0.0), radii: (1.0, 1.0, 1.0))", "", "");
        let s = Structure::from_ron_str(&recipe).unwrap();
        assert!(s.provenance().is_none());
        assert!(!s.build_geometry().unwrap().mesh().indices().is_empty());
    }
}
