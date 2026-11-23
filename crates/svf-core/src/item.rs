//! Item documents: what an author writes. An item names objects from kits,
//! lists labels and tells its story in story verbs; it compiles to an ordinary
//! structure document. See `docs/item-document.md`.

use std::{collections::BTreeMap, error::Error, fmt};

use serde::Deserialize;

use crate::kits::chemistry::{ChemistryKit, ChemistryObject, CrystalKind};
use crate::kits::circuit::{CellKind, CircuitKit, CircuitObject, CircuitVerb, LampKind, SwitchState};
use crate::structure::{PartDocument, Structure, StructureDocument, StructureError};
use crate::structure_animation::{ActionDocument, BeatDocument, Ease, SceneDocument, TimelineDocument};
use crate::structure_labels::LabelSide;
use crate::structure_lesson::ViewCamera;

/// Caption pace (convention rule 6).
pub const WORDS_PER_SECOND: f64 = 2.5;
/// The shortest beat.
pub const MIN_BEAT_S: f64 = 3.0;
/// How long a beat's camera move takes.
pub const CAMERA_MOVE_S: f64 = 1.5;
const EPS: f64 = 1e-9;

/// The item document, as the author writes it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename = "Item")]
pub struct ItemDocument {
    /// Lowercase with hyphens: the root part's id.
    pub id: String,
    pub title: String,
    pub subject: Subject,
    /// The one idea the item teaches.
    pub idea: String,
    /// The account the item follows.
    pub reference: String,
    pub objects: Vec<Object>,
    /// Names labelled from the start.
    #[serde(default)]
    pub labels: Vec<String>,
    /// The camera at the start (default: the kit's).
    #[serde(default)]
    pub camera: Option<ViewCamera>,
    /// Extra honesty notes for the compiled file's header.
    #[serde(default)]
    pub notes: Vec<String>,
    pub story: Vec<Beat>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Subject {
    Physics,
    Chemistry,
    Biology,
}

/// One object from a kit.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum Object {
    /// Circuit kit: a cell (`kind: AA`).
    Cell {
        id: String,
        #[serde(default)]
        kind: CellKind,
    },
    /// Circuit kit: a knife switch with a hinged lever.
    Switch {
        id: String,
        #[serde(default)]
        starts: SwitchState,
    },
    /// Circuit kit: a filament lamp (`kind: MES`).
    Lamp {
        id: String,
        #[serde(default)]
        kind: LampKind,
    },
    /// Circuit kit: a series loop, starting at its cell, in the electrons' order.
    Loop(Vec<String>),
    /// Chemistry kit: a reaction such as `"2 H2 + O2 -> 2 H2O"`, built from the molecule library.
    Reaction { id: String, equation: String },
    /// Chemistry kit: a rock-salt crystal, `edge` ions along each edge (default 4).
    Crystal {
        id: String,
        #[serde(default)]
        kind: CrystalKind,
        #[serde(default = "default_edge")]
        edge: usize,
    },
}

fn default_edge() -> usize {
    4
}

/// One beat of the story.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename = "Beat")]
pub struct Beat {
    /// Optional id (default `beat-<n>`).
    #[serde(default)]
    pub id: Option<String>,
    /// The caption, shown for the whole beat.
    pub say: String,
    /// Names to highlight and label, spread evenly through the beat.
    #[serde(default)]
    pub point_at: Vec<String>,
    /// Names to label at the start of the beat.
    #[serde(default)]
    pub label: Vec<String>,
    /// Names whose labels are removed at the start of the beat.
    #[serde(default)]
    pub unlabel: Vec<String>,
    /// Kit verbs, all starting with the beat.
    #[serde(default, rename = "do")]
    pub verbs: Vec<Verb>,
    /// Move the camera here over the first 1.5 s.
    #[serde(default)]
    pub camera: Option<ViewCamera>,
    /// The beat's length, instead of the timing rule.
    #[serde(default)]
    pub hold_s: Option<f64>,
}

/// A kit verb.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Verb {
    /// Circuit kit: close a switch.
    Close(String),
    /// Circuit kit: open a switch.
    Open(String),
    /// Chemistry kit: the reaction happens: old bonds break, atoms move, new bonds form.
    React(String),
    /// Chemistry kit: draw a crystal's ions smaller and show its grid.
    OpenUp(String),
    /// Chemistry kit: water pulls `ions` corner ions off a crystal.
    Dissolve { crystal: String, ions: usize },
}

/// What the compiler needs from a kit (`docs/item-document.md` section 2).
pub trait Kit {
    /// How long a verb takes.
    fn verb_seconds(&self, verb: &Verb) -> f64;
    /// The motion of a verb that starts at `at`; the kit records what it changes.
    fn apply(&mut self, verb: &Verb, at: f64) -> Result<Vec<Timed>, ItemError>;
    /// Everything that follows from the kit's rules over a story that ends at `end`.
    fn consequences(&self, end: f64) -> Result<Vec<Timed>, ItemError>;
}

impl Kit for CircuitKit {
    fn verb_seconds(&self, verb: &Verb) -> f64 {
        CircuitKit::verb_seconds(self, verb)
    }
    fn apply(&mut self, verb: &Verb, at: f64) -> Result<Vec<Timed>, ItemError> {
        match verb {
            Verb::Close(_) | Verb::Open(_) => CircuitKit::apply(self, verb, at),
            other => Err(ItemError::Invalid(format!("{other:?} is not a circuit verb"))),
        }
    }
    fn consequences(&self, end: f64) -> Result<Vec<Timed>, ItemError> {
        CircuitKit::consequences(self, end)
    }
}

impl Kit for ChemistryKit {
    fn verb_seconds(&self, verb: &Verb) -> f64 {
        ChemistryKit::verb_seconds(self, verb)
    }
    fn apply(&mut self, verb: &Verb, at: f64) -> Result<Vec<Timed>, ItemError> {
        ChemistryKit::apply(self, verb, at)
    }
    fn consequences(&self, end: f64) -> Result<Vec<Timed>, ItemError> {
        ChemistryKit::consequences(self, end)
    }
}

/// A fidelity level (`docs/authoring-convention.md` section 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    F0,
    F1,
    F2,
    F3,
}

/// A part's fidelity on the three axes, with where it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Fidelity {
    pub form: Level,
    pub behaviour: Level,
    pub appearance: Level,
    pub source: String,
}

/// The default label of a name.
#[derive(Clone, Debug, PartialEq)]
pub struct LabelDefault {
    /// The engine part the label belongs to.
    pub part: String,
    pub text: String,
    pub at: Option<[f64; 3]>,
    pub side: Option<LabelSide>,
}

/// What a name stands for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Named {
    /// The engine part highlighted by `point_at`.
    pub part: Option<String>,
    /// A point: a label anchor or a route end.
    pub point: Option<[f64; 3]>,
    pub label: Option<LabelDefault>,
}

/// An action at an absolute time; the compiler puts it into its beat.
#[derive(Clone, Debug, PartialEq)]
pub struct Timed {
    pub at: f64,
    pub action: ActionDocument,
}

/// What a kit hands to the compiler when it builds its objects.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KitBuild {
    pub parts: Vec<PartDocument>,
    pub names: Vec<(String, Named)>,
    /// Fidelity per engine part id.
    pub fidelity: Vec<(String, Fidelity)>,
    pub notes: Vec<String>,
    pub camera: Option<ViewCamera>,
    pub units: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ItemError {
    Parse(String),
    Invalid(String),
    UnknownName { name: String, known: Vec<String> },
    Structure(StructureError),
}

impl fmt::Display for ItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "the item document does not parse: {e}"),
            Self::Invalid(e) => write!(f, "{e}"),
            Self::UnknownName { name, known } => {
                write!(f, "unknown name '{name}'; the names in this item are: {}", known.join(", "))
            }
            Self::Structure(e) => write!(f, "the compiled structure is invalid: {e}"),
        }
    }
}

impl Error for ItemError {}

/// Whether `id` follows convention rule 1: lowercase letters, digits and single hyphens.
#[must_use]
pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The item's name table.
#[derive(Clone, Debug, Default)]
pub struct Names(BTreeMap<String, Named>);

impl Names {
    fn insert(&mut self, name: String, named: Named) -> Result<(), ItemError> {
        if self.0.insert(name.clone(), named).is_some() {
            return Err(ItemError::Invalid(format!("the name '{name}' is defined twice")));
        }
        Ok(())
    }

    fn unknown(&self, name: &str) -> ItemError {
        ItemError::UnknownName { name: name.to_owned(), known: self.0.keys().cloned().collect() }
    }

    /// What `name` stands for.
    pub fn get(&self, name: &str) -> Result<&Named, ItemError> {
        self.0.get(name).ok_or_else(|| self.unknown(name))
    }

    fn part(&self, name: &str) -> Result<&str, ItemError> {
        self.get(name)?
            .part
            .as_deref()
            .ok_or_else(|| ItemError::Invalid(format!("'{name}' is a point, not a part, so it cannot be pointed at")))
    }

    fn label(&self, name: &str) -> Result<&LabelDefault, ItemError> {
        self.get(name)?.label.as_ref().ok_or_else(|| ItemError::Invalid(format!("'{name}' has no label")))
    }

    /// Every name, in order.
    pub fn all(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }
}

/// A compiled item.
#[derive(Clone, Debug)]
pub struct CompiledItem {
    pub document: StructureDocument,
    pub structure: Structure,
    pub names: Names,
    /// Fidelity per engine part id, in part order.
    pub fidelity: Vec<(String, Fidelity)>,
    /// Header notes: the kits' simplifications, then the author's.
    pub notes: Vec<String>,
}

impl CompiledItem {
    /// The compiled structure file, with a header that records where it came from.
    ///
    /// # Panics
    /// Never in practice: every structure document type serialises.
    #[must_use]
    pub fn to_ron(&self) -> String {
        let mut out = String::new();
        out.push_str("// Compiled from an item document by svf_core::item (docs/item-document.md). Do not edit: edit the item.\n");
        for note in &self.notes {
            out.push_str(&format!("// Note: {note}\n"));
        }
        out.push_str("// Fidelity per part (form, behaviour, appearance; source):\n");
        for (part, f) in &self.fidelity {
            out.push_str(&format!(
                "//   {part}: {:?}, {:?}, {:?}; {}\n",
                f.form, f.behaviour, f.appearance, f.source
            ));
        }
        let config = ron::ser::PrettyConfig::new().struct_names(true);
        out.push_str(&ron::ser::to_string_pretty(&self.document, config).expect("structure documents serialise"));
        out.push('\n');
        out
    }
}

/// Parse and compile an item document.
pub fn compile_str(text: &str) -> Result<CompiledItem, ItemError> {
    let item: ItemDocument = ron::from_str(text).map_err(|e| ItemError::Parse(e.to_string()))?;
    compile(&item)
}

fn round_up_half(seconds: f64) -> f64 {
    (seconds * 2.0 - EPS).ceil() / 2.0
}

/// How long a caption needs (convention rule 6), before verbs are considered.
#[must_use]
pub fn caption_seconds(text: &str) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let words = text.split_whitespace().count() as f64;
    round_up_half((words / WORDS_PER_SECOND).max(MIN_BEAT_S))
}

fn set_at(action: &mut ActionDocument, value: f64) {
    use ActionDocument as A;
    match action {
        A::Narrate { at_s, .. }
        | A::Wait { at_s, .. }
        | A::Show { at_s, .. }
        | A::Hide { at_s, .. }
        | A::Highlight { at_s, .. }
        | A::Move { at_s, .. }
        | A::Grow { at_s, .. }
        | A::Turn { at_s, .. }
        | A::Fade { at_s, .. }
        | A::Colour { at_s, .. }
        | A::Label { at_s, .. }
        | A::Unlabel { at_s, .. }
        | A::Key { at_s, .. }
        | A::Unkey { at_s, .. }
        | A::Spread { at_s, .. }
        | A::Flow { at_s, .. }
        | A::Camera { at_s, .. } => *at_s = value,
    }
}

fn label_action(label: &LabelDefault) -> ActionDocument {
    ActionDocument::Label {
        part: label.part.clone(),
        text: Some(label.text.clone()),
        at: label.at.map(|p| (p[0], p[1], p[2])),
        side: label.side,
        at_s: 0.0,
    }
}

fn timed(at: f64, action: ActionDocument) -> Timed {
    Timed { at, action }
}

/// Compile an item document to a structure.
pub fn compile(item: &ItemDocument) -> Result<CompiledItem, ItemError> {
    let invalid = |s: String| ItemError::Invalid(s);
    if !is_valid_id(&item.id) {
        return Err(invalid(format!("the item id '{}' must be lowercase letters, digits and hyphens", item.id)));
    }
    if item.title.trim().is_empty() || item.idea.trim().is_empty() || item.reference.trim().is_empty() {
        return Err(invalid("an item needs a title, an idea and a reference".to_owned()));
    }
    if item.story.is_empty() {
        return Err(invalid("an item needs a story of at least one beat".to_owned()));
    }

    // Objects go to their kit. An item uses one kit: kits differ in units and layout.
    let mut circuit_objects = Vec::new();
    let mut chemistry_objects = Vec::new();
    for object in &item.objects {
        match object {
            Object::Cell { id, kind } => circuit_objects.push(CircuitObject::Cell { id: id.clone(), kind: *kind }),
            Object::Switch { id, starts } => circuit_objects.push(CircuitObject::Switch { id: id.clone(), starts: *starts }),
            Object::Lamp { id, kind } => circuit_objects.push(CircuitObject::Lamp { id: id.clone(), kind: *kind }),
            Object::Loop(members) => circuit_objects.push(CircuitObject::Loop(members.clone())),
            Object::Reaction { id, equation } => {
                chemistry_objects.push(ChemistryObject::Reaction { id: id.clone(), equation: equation.clone() });
            }
            Object::Crystal { id, kind, edge } => {
                chemistry_objects.push(ChemistryObject::Crystal { id: id.clone(), kind: *kind, edge: *edge });
            }
        }
    }
    for id in item.objects.iter().filter_map(|o| match o {
        Object::Reaction { id, .. } | Object::Crystal { id, .. } => Some(id),
        _ => None,
    }) {
        if !is_valid_id(id) || *id == item.id {
            return Err(invalid(format!("the object id '{id}' must be lowercase letters, digits and hyphens, and not the item's id")));
        }
    }
    let (mut kit, build): (Box<dyn Kit>, KitBuild) = match (circuit_objects.is_empty(), chemistry_objects.is_empty()) {
        (false, true) => {
            let (k, b) = CircuitKit::build(&circuit_objects, &item.id)?;
            (Box::new(k), b)
        }
        (true, false) => {
            let (k, b) = ChemistryKit::build(&chemistry_objects, &item.id)?;
            (Box::new(k), b)
        }
        (true, true) => return Err(invalid("an item needs at least one object".to_owned())),
        (false, false) => return Err(invalid("an item uses one kit: circuit and chemistry objects cannot be mixed".to_owned())),
    };

    let mut parts = vec![PartDocument {
        id: item.id.clone(),
        name: item.title.clone(),
        parent: None,
        description: item.idea.clone(),
        colour: (0.5, 0.5, 0.5),
        opacity: 1.0,
        geometry: None,
        repeat: None,
        mirror: None,
    }];
    parts.extend(build.parts);
    let mut names = Names::default();
    names.insert(
        item.id.clone(),
        Named { part: Some(item.id.clone()), point: None, label: None },
    )?;
    for (name, named) in build.names {
        names.insert(name, named)?;
    }
    let mut notes = build.notes;
    notes.extend(item.notes.iter().cloned());

    // Time the story, and collect its actions at absolute times.
    let mut starts = Vec::with_capacity(item.story.len());
    let mut lengths = Vec::with_capacity(item.story.len());
    let mut actions: Vec<Timed> = Vec::new();
    let mut clock = 0.0;
    for label in &item.labels {
        actions.push(timed(0.0, label_action(names.label(label)?)));
    }
    let camera = item.camera.or(build.camera).unwrap_or(ViewCamera { yaw_deg: 0.0, pitch_deg: 0.0, zoom: 1.0 });
    for (n, beat) in item.story.iter().enumerate() {
        if beat.say.trim().is_empty() {
            return Err(invalid(format!("beat {} has nothing to say", n + 1)));
        }
        let longest_verb = beat.verbs.iter().map(|v| kit.verb_seconds(v)).fold(0.0, f64::max);
        let mut length = caption_seconds(&beat.say);
        if longest_verb > 0.0 {
            length = length.max(round_up_half(longest_verb + 1.0));
        }
        if beat.camera.is_some() {
            length = length.max(round_up_half(CAMERA_MOVE_S + 1.0));
        }
        if let Some(hold) = beat.hold_s {
            if !(hold.is_finite() && hold > 0.0) {
                return Err(invalid(format!("beat {}: hold_s must be positive", n + 1)));
            }
            if hold + EPS < longest_verb {
                return Err(invalid(format!("beat {}: hold_s is shorter than its verbs", n + 1)));
            }
            length = hold;
        }
        let start = clock;
        actions.push(timed(start, ActionDocument::Narrate { text: beat.say.clone(), for_s: length, at_s: 0.0 }));
        #[allow(clippy::cast_precision_loss)]
        let step = length / beat.point_at.len().max(1) as f64;
        for (i, name) in beat.point_at.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let at = start + step * i as f64;
            actions.push(timed(at, ActionDocument::Highlight { part: names.part(name)?.to_owned(), at_s: 0.0 }));
            actions.push(timed(at, label_action(names.label(name)?)));
        }
        for name in &beat.label {
            actions.push(timed(start, label_action(names.label(name)?)));
        }
        for name in &beat.unlabel {
            actions.push(timed(start, ActionDocument::Unlabel { part: names.label(name)?.part.clone(), at_s: 0.0 }));
        }
        if let Some(c) = beat.camera {
            actions.push(timed(
                start,
                ActionDocument::Camera {
                    yaw_deg: c.yaw_deg,
                    pitch_deg: c.pitch_deg,
                    zoom: c.zoom,
                    for_s: CAMERA_MOVE_S,
                    at_s: 0.0,
                    ease: Ease::Smooth,
                },
            ));
        }
        for verb in &beat.verbs {
            actions.extend(kit.apply(verb, start)?);
        }
        starts.push(start);
        lengths.push(length);
        clock += length;
    }
    // The consequences of the kits' rules, over the whole story.
    actions.extend(kit.consequences(clock)?);

    // Put every action into the beat it starts in.
    let mut beat_actions: Vec<Vec<ActionDocument>> = vec![Vec::new(); item.story.len()];
    actions.sort_by(|a, b| a.at.total_cmp(&b.at));
    for Timed { at, mut action } in actions {
        let index = starts.iter().rposition(|&s| s <= at + EPS).unwrap_or(0);
        set_at(&mut action, (at - starts[index]).max(0.0));
        beat_actions[index].push(action);
    }
    let beats = item
        .story
        .iter()
        .zip(beat_actions)
        .zip(&lengths)
        .enumerate()
        .map(|(n, ((beat, actions), &length))| BeatDocument {
            id: beat.id.clone().unwrap_or_else(|| format!("beat-{}", n + 1)),
            actions,
            length_s: Some(length),
        })
        .collect();

    let document = StructureDocument {
        version: crate::structure::SUPPORTED_VERSION,
        name: item.title.clone(),
        units: build.units.to_owned(),
        reference: item.reference.clone(),
        circle_segments: 24,
        parts,
        layers: Vec::new(),
        relations: Vec::new(),
        views: Vec::new(),
        timelines: vec![TimelineDocument {
            id: "story".to_owned(),
            name: item.title.clone(),
            camera: Some(camera),
            scenes: vec![SceneDocument { id: "story".to_owned(), beats }],
        }],
        provenance: None,
    };
    let structure = Structure::from_document(document.clone()).map_err(ItemError::Structure)?;
    let fidelity = build.fidelity;
    Ok(CompiledItem { document, structure, names, fidelity, notes })
}

impl Verb {
    /// The object the verb acts on.
    #[must_use]
    pub fn object(&self) -> &str {
        match self {
            Self::Close(id) | Self::Open(id) | Self::React(id) | Self::OpenUp(id) => id,
            Self::Dissolve { crystal, .. } => crystal,
        }
    }

    pub(crate) fn circuit(&self) -> Option<CircuitVerb> {
        match self {
            Self::Close(_) => Some(CircuitVerb::Close),
            Self::Open(_) => Some(CircuitVerb::Open),
            _ => None,
        }
    }
}
