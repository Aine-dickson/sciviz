//! Lesson additions to a structure: typed relations between parts and named
//! views. Design: `docs/cockroach-structure.md` (sections 4 and 5).
//!
//! Both are optional in the file. They describe and select; they never change
//! geometry.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::structure::{Part, StructureError};

/// How two parts are related. The parts tree already says "is part of".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum RelationKind {
    /// `from` physically contains `to`.
    Contains,
    /// `from` is joined to `to`.
    ConnectsTo,
    /// `from` empties or leads into `to`.
    OpensInto,
}

impl RelationKind {
    /// A short phrase for reading a relation aloud: "<from> <phrase> <to>".
    #[must_use]
    pub const fn phrase(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::ConnectsTo => "connects to",
            Self::OpensInto => "opens into",
        }
    }
}

/// File representation of one relation.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Relation")]
pub struct RelationDocument {
    pub from: String,
    pub kind: RelationKind,
    pub to: String,
}

/// One validated relation, by part index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relation {
    from: usize,
    kind: RelationKind,
    to: usize,
}

impl Relation {
    #[must_use]
    pub const fn from(&self) -> usize {
        self.from
    }
    #[must_use]
    pub const fn kind(&self) -> RelationKind {
        self.kind
    }
    #[must_use]
    pub const fn to(&self) -> usize {
        self.to
    }
}

/// A world axis for a view's section plane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum SectionAxis {
    X,
    Y,
    Z,
}

impl SectionAxis {
    /// 0, 1 or 2.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

/// A view's camera, relative to the default fitted camera.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewCamera {
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub zoom: f64,
}

/// A view's section plane. `offset` is along `axis`; `flip` keeps the other side.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewSection {
    pub axis: SectionAxis,
    pub offset: f64,
    #[serde(default)]
    pub flip: bool,
}

/// File representation of one view.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "View")]
pub struct ViewDocument {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub camera: Option<ViewCamera>,
    /// Part ids to show (a grouping node stands for its descendants). Empty
    /// means every part.
    #[serde(default)]
    pub show: Vec<String>,
    /// Number of outermost layers removed.
    #[serde(default)]
    pub peel: usize,
    #[serde(default)]
    pub section: Option<ViewSection>,
}

/// One validated view.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    id: String,
    name: String,
    camera: Option<ViewCamera>,
    show: Vec<usize>,
    peel: usize,
    section: Option<ViewSection>,
}

impl View {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub const fn camera(&self) -> Option<&ViewCamera> {
        self.camera.as_ref()
    }
    /// Part indices to show, ascending, descendants included; empty means all.
    #[must_use]
    pub fn show(&self) -> &[usize] {
        &self.show
    }
    #[must_use]
    pub const fn peel(&self) -> usize {
        self.peel
    }
    #[must_use]
    pub const fn section(&self) -> Option<&ViewSection> {
        self.section.as_ref()
    }
}

pub(crate) fn validate_relations(
    documents: &[RelationDocument],
    by_id: &HashMap<&str, usize>,
) -> Result<Vec<Relation>, StructureError> {
    documents
        .iter()
        .map(|d| {
            let find = |id: &String| {
                by_id
                    .get(id.as_str())
                    .copied()
                    .ok_or_else(|| StructureError::UnknownRelationPart { part: id.clone() })
            };
            let (from, to) = (find(&d.from)?, find(&d.to)?);
            if from == to {
                return Err(StructureError::SelfRelation { part: d.from.clone() });
            }
            Ok(Relation { from, kind: d.kind, to })
        })
        .collect()
}

pub(crate) fn validate_views(
    documents: &[ViewDocument],
    parts: &[Part],
    by_id: &HashMap<&str, usize>,
    layer_count: usize,
) -> Result<Vec<View>, StructureError> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut views = Vec::with_capacity(documents.len());
    for (index, d) in documents.iter().enumerate() {
        if d.id.is_empty() {
            return Err(StructureError::EmptyViewId { index });
        }
        if !seen.insert(d.id.as_str()) {
            return Err(StructureError::DuplicateViewId(d.id.clone()));
        }
        if d.peel > layer_count {
            return Err(StructureError::InvalidView {
                view: d.id.clone(),
                reason: "peel is larger than the number of layers",
            });
        }
        if let Some(c) = &d.camera {
            if !(c.yaw_deg.is_finite() && c.pitch_deg.is_finite() && c.zoom.is_finite() && c.zoom > 0.0) {
                return Err(StructureError::InvalidView {
                    view: d.id.clone(),
                    reason: "camera angles must be finite and zoom positive",
                });
            }
        }
        if let Some(s) = &d.section {
            if !s.offset.is_finite() {
                return Err(StructureError::InvalidView {
                    view: d.id.clone(),
                    reason: "section offset must be finite",
                });
            }
        }
        let mut shown = vec![false; parts.len()];
        for part_id in &d.show {
            let root = *by_id.get(part_id.as_str()).ok_or_else(|| StructureError::UnknownViewPart {
                view: d.id.clone(),
                part: part_id.clone(),
            })?;
            for (i, flag) in shown.iter_mut().enumerate() {
                let mut current = Some(i);
                while let Some(c) = current {
                    if c == root {
                        *flag = true;
                        break;
                    }
                    current = parts[c].parent();
                }
            }
        }
        views.push(View {
            id: d.id.clone(),
            name: d.name.clone(),
            camera: d.camera,
            show: shown.iter().enumerate().filter(|(_, s)| **s).map(|(i, _)| i).collect(),
            peel: d.peel,
            section: d.section,
        });
    }
    Ok(views)
}

#[cfg(test)]
mod tests {
    use crate::structure::{RelationKind, Structure, StructureError};

    fn structure(extra: &str) -> Result<Structure, StructureError> {
        let part = |id: &str, parent: &str| {
            format!(
                "Part(id: \"{id}\", name: \"{id}\", parent: {parent}, description: \"d\", \
                 colour: (0.5, 0.5, 0.5), geometry: None),"
            )
        };
        let parts = format!(
            "{}{}{}",
            part("root", "None"),
            part("a", "Some(\"root\")"),
            part("b", "Some(\"root\")")
        );
        Structure::from_ron_str(&format!(
            "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 48, \
             parts: [{parts}], layers: [Layer(id: \"l1\", name: \"l1\", parts: [\"a\"])]{extra})"
        ))
    }

    #[test]
    fn relations_are_validated_and_queryable() {
        let s = structure(
            ", relations: [Relation(from: \"a\", kind: OpensInto, to: \"b\"), \
             Relation(from: \"root\", kind: Contains, to: \"a\")]",
        )
        .unwrap();
        assert_eq!(s.relations().len(), 2);
        assert_eq!(s.relations()[0].kind(), RelationKind::OpensInto);
        assert_eq!(RelationKind::OpensInto.phrase(), "opens into");
        let a = s.part_index("a").unwrap();
        assert_eq!(s.relations_of(a).count(), 2);
        assert_eq!(s.relations_of(s.part_index("b").unwrap()).count(), 1);

        assert_eq!(
            structure(", relations: [Relation(from: \"a\", kind: Contains, to: \"zzz\")]").unwrap_err(),
            StructureError::UnknownRelationPart { part: "zzz".into() }
        );
        assert_eq!(
            structure(", relations: [Relation(from: \"a\", kind: Contains, to: \"a\")]").unwrap_err(),
            StructureError::SelfRelation { part: "a".into() }
        );
    }

    #[test]
    fn views_are_validated_and_expand_groups() {
        let s = structure(
            ", views: [View(id: \"v\", name: \"V\", camera: Some((yaw_deg: 90.0, pitch_deg: 40.0, zoom: 1.5)), \
             show: [\"root\"], peel: 1, section: Some((axis: Z, offset: 0.5, flip: true))), \
             View(id: \"all\", name: \"All\")]",
        )
        .unwrap();
        let v = &s.views()[0];
        assert_eq!((v.id(), v.name(), v.peel()), ("v", "V", 1));
        assert_eq!(v.show(), &[0, 1, 2], "a grouping node stands for its descendants");
        assert_eq!(v.camera().unwrap().zoom, 1.5);
        let section = v.section().unwrap();
        assert_eq!((section.axis.index(), section.offset, section.flip), (2, 0.5, true));
        let all = &s.views()[1];
        assert!(all.show().is_empty() && all.camera().is_none() && all.section().is_none());
        assert_eq!(all.peel(), 0);
    }

    #[test]
    fn view_errors_are_reported() {
        let view = |fields: &str| structure(&format!(", views: [View(id: \"v\", name: \"V\"{fields})]"));
        assert!(matches!(
            view(", show: [\"nope\"]").unwrap_err(),
            StructureError::UnknownViewPart { .. }
        ));
        assert!(matches!(view(", peel: 2").unwrap_err(), StructureError::InvalidView { .. }));
        assert!(matches!(
            view(", camera: Some((yaw_deg: 0.0, pitch_deg: 0.0, zoom: 0.0))").unwrap_err(),
            StructureError::InvalidView { .. }
        ));
        assert!(matches!(
            view(", section: Some((axis: X, offset: inf))").unwrap_err(),
            StructureError::InvalidView { .. }
        ));
        assert_eq!(
            structure(", views: [View(id: \"v\", name: \"1\"), View(id: \"v\", name: \"2\")]").unwrap_err(),
            StructureError::DuplicateViewId("v".into())
        );
        assert_eq!(
            structure(", views: [View(id: \"\", name: \"1\")]").unwrap_err(),
            StructureError::EmptyViewId { index: 0 }
        );
        assert!(matches!(
            structure(", views: [View(id: \"v\", name: \"V\", peal: 1)]").unwrap_err(),
            StructureError::Parse(_)
        ));
    }
}
