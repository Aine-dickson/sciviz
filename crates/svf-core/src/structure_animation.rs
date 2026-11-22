//! Animation timelines for a structure: timed steps that move, grow, turn,
//! recolour, show or hide parts, move the camera and show captions.
//! Design: `docs/animation-design.md`.
//!
//! The picture at presentation time `t` is a pure function of `t`:
//! [`Timeline::pose_at`] carries no state between calls, so a caller can play,
//! pause, step or scrub, and tests can sample any instant. This module only
//! says *what the pose is*; applying a pose to geometry is a separate step.
//!
//! Timelines are optional in a structure file. They describe and never change
//! the authored geometry.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::geometry::TriangleMesh3;
use crate::spatial::WorldPoint3;
use crate::structure::{Part, PartRange, StructureError, StructureGeometry};
use crate::structure_labels::{key_entry, label_parts, KeyEntry, LabelSide, PartLabel};
use crate::structure_lesson::ViewCamera;
use crate::structure_particles::{Flow, FlowSpec, ParticleGroup, ShapeDocument, Spread, SpreadSpec};

const EPS: f64 = 1e-9;

/// How a timed action moves from its start to its end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Ease {
    /// Constant speed.
    #[default]
    Linear,
    /// Slow start and end (smoothstep).
    Smooth,
}

impl Ease {
    fn apply(self, p: f64) -> f64 {
        match self {
            Self::Linear => p,
            Self::Smooth => p * p * (3.0 - 2.0 * p),
        }
    }
}

fn stay_by_default() -> bool {
    true
}

/// One action in a beat. Every action starts `at_s` seconds after its beat
/// starts (default 0). A beat lasts as long as its longest action.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ActionDocument {
    /// Show a caption for `for_s` seconds.
    Narrate {
        text: String,
        for_s: f64,
        #[serde(default)]
        at_s: f64,
    },
    /// Do nothing for `for_s` seconds.
    Wait {
        for_s: f64,
        #[serde(default)]
        at_s: f64,
    },
    /// Make a part (and its descendants) appear, instantly.
    Show {
        part: String,
        #[serde(default)]
        at_s: f64,
    },
    /// Make a part (and its descendants) disappear, instantly.
    Hide {
        part: String,
        #[serde(default)]
        at_s: f64,
    },
    /// Mark a part (and its descendants) until the end of the beat.
    Highlight {
        part: String,
        #[serde(default)]
        at_s: f64,
    },
    /// Move a part (and its descendants) by an offset.
    Move {
        part: String,
        by: (f64, f64, f64),
        for_s: f64,
        #[serde(default)]
        at_s: f64,
        #[serde(default)]
        ease: Ease,
    },
    /// Scale a part to `to` times its authored size, about its own centre. On a
    /// group (or a part with parts below it) the whole group scales as one body
    /// about the centre of its combined bounding box, so the parts keep their
    /// places relative to one another; with `each: true` every part with geometry
    /// scales about its own centre instead.
    Grow {
        part: String,
        to: f64,
        #[serde(default)]
        each: bool,
        for_s: f64,
        #[serde(default)]
        at_s: f64,
        #[serde(default)]
        ease: Ease,
    },
    /// Rotate a part by `deg` about `axis`. The turn is about the part's own
    /// centre unless `about` names a hinge point (a point of the structure's
    /// authored space): then the part swings about the line through `about`
    /// along `axis`. A hinge point stays with the part when it is moved.
    Turn {
        part: String,
        axis: (f64, f64, f64),
        deg: f64,
        #[serde(default)]
        about: Option<(f64, f64, f64)>,
        /// On a group: turn every part about its own centre (default: turn the
        /// whole group as one rigid body).
        #[serde(default)]
        each: bool,
        for_s: f64,
        #[serde(default)]
        at_s: f64,
        #[serde(default)]
        ease: Ease,
    },
    /// Change a part's (and its descendants') opacity to `to` (0 to 1) over `for_s`.
    /// A part that has faded to 0 is not drawn at all.
    Fade {
        part: String,
        to: f64,
        for_s: f64,
        #[serde(default)]
        at_s: f64,
        #[serde(default)]
        ease: Ease,
    },
    /// Change a part's (and its descendants') colour to `to`.
    Colour {
        part: String,
        to: (f64, f64, f64),
        for_s: f64,
        #[serde(default)]
        at_s: f64,
        #[serde(default)]
        ease: Ease,
    },
    /// Label a part (a group stands for its descendants) until `Unlabel` or a
    /// later `Label` of the same part. `text` defaults to the part's name; `at`
    /// is an authored anchor (default: on the part's surface). See `docs/labels.md`.
    Label {
        part: String,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        at: Option<(f64, f64, f64)>,
        #[serde(default)]
        side: Option<LabelSide>,
        #[serde(default)]
        at_s: f64,
    },
    /// Remove a part's label.
    Unlabel {
        part: String,
        #[serde(default)]
        at_s: f64,
    },
    /// Add a colour key entry, until `Unkey` with the same id.
    Key {
        id: String,
        colour: (f64, f64, f64),
        text: String,
        #[serde(default)]
        at_s: f64,
    },
    /// Remove a colour key entry.
    Unkey {
        id: String,
        #[serde(default)]
        at_s: f64,
    },
    /// Diffusion: `count` particles start in a small ball and wander inside
    /// `inside` for `for_s` seconds, then stay where they ended. They appear when it starts.
    Spread {
        id: String,
        count: usize,
        from: (f64, f64, f64),
        /// Radius of the start ball.
        radius: f64,
        inside: ShapeDocument,
        /// Distance moved per step (20 steps a second).
        step: f64,
        seed: u64,
        /// Radius of each drawn particle.
        particle_radius: f64,
        colour: (f64, f64, f64),
        for_s: f64,
        #[serde(default)]
        at_s: f64,
        /// Whether the particles stay, resting where the walk ended, after `for_s`
        /// (the default), or disappear when it ends.
        #[serde(default = "stay_by_default")]
        stay: bool,
        /// Key text shown, with the particles' colour, while the particles are shown.
        #[serde(default)]
        key: Option<String>,
    },
    /// Particles travelling along `path` at `speed` for `for_s` seconds, wrapping
    /// from the end to the start. Shown only while it runs.
    Flow {
        id: String,
        count: usize,
        path: Vec<(f64, f64, f64)>,
        /// When set, `path` lists corner points: each interior corner becomes an
        /// arc of this radius (see `structure_route`), sampled at `point_spacing`
        /// (default: a third of the radius).
        #[serde(default)]
        corner_radius: Option<f64>,
        #[serde(default)]
        point_spacing: Option<f64>,
        speed: f64,
        /// How far from the path a particle may sit.
        lateral: f64,
        seed: u64,
        particle_radius: f64,
        colour: (f64, f64, f64),
        for_s: f64,
        #[serde(default)]
        at_s: f64,
        /// Key text shown, with the particles' colour, while the particles are shown.
        #[serde(default)]
        key: Option<String>,
    },
    /// Move the camera, like a view's camera. Needs the timeline's `camera`.
    Camera {
        yaw_deg: f64,
        pitch_deg: f64,
        zoom: f64,
        for_s: f64,
        #[serde(default)]
        at_s: f64,
        #[serde(default)]
        ease: Ease,
    },
}

/// File representation of a beat.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Beat")]
pub struct BeatDocument {
    pub id: String,
    pub actions: Vec<ActionDocument>,
}

/// File representation of a scene: beats that play one after another.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Scene")]
pub struct SceneDocument {
    pub id: String,
    pub beats: Vec<BeatDocument>,
}

/// File representation of a timeline.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename = "Timeline")]
pub struct TimelineDocument {
    pub id: String,
    pub name: String,
    /// The camera at time 0. Required when the timeline has `Camera` actions.
    #[serde(default)]
    pub camera: Option<ViewCamera>,
    pub scenes: Vec<SceneDocument>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Span {
    start: f64,
    end: f64,
    ease: Ease,
}

impl Span {
    fn progress(&self, t: f64) -> f64 {
        if t >= self.end {
            1.0
        } else if t <= self.start {
            0.0
        } else {
            self.ease.apply((t - self.start) / (self.end - self.start))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PartTrack {
    initially_shown: bool,
    /// `(time, shown)`, ascending in time.
    visibility: Vec<(f64, bool)>,
    moves: Vec<(Span, [f64; 3])>,
    /// `(span, from, to)`, non-overlapping, ascending.
    scales: Vec<(Span, f64, f64)>,
    /// `(span, unit axis, degrees)`, ascending in start.
    turns: Vec<(Span, [f64; 3], f64, Option<[f64; 3]>)>,
    /// `(span, from, to)`, non-overlapping, ascending.
    colours: Vec<(Span, [f64; 3], [f64; 3])>,
    /// The part's authored opacity, used until the first fade starts.
    base_opacity: f64,
    /// `(span, from, to)`, non-overlapping, ascending.
    fades: Vec<(Span, f64, f64)>,
    /// `[from, to)` ranges.
    highlights: Vec<(f64, f64)>,
    /// The group this part scales and turns with as one body, once a `Grow` or
    /// `Turn` has set it (`Some(None)`: about its own centre).
    body: Option<Option<usize>>,
}

impl PartTrack {
    fn new() -> Self {
        Self {
            initially_shown: true,
            visibility: Vec::new(),
            moves: Vec::new(),
            scales: Vec::new(),
            turns: Vec::new(),
            colours: Vec::new(),
            base_opacity: 1.0,
            fades: Vec::new(),
            highlights: Vec::new(),
            body: None,
        }
    }
}

/// Where one beat sits on the timeline.
#[derive(Clone, Debug, PartialEq)]
pub struct BeatSpan {
    pub scene: String,
    pub beat: String,
    pub start: f64,
    pub end: f64,
}

/// One validated timeline.
#[derive(Clone, Debug, PartialEq)]
pub struct Timeline {
    id: String,
    name: String,
    duration: f64,
    beats: Vec<BeatSpan>,
    tracks: Vec<PartTrack>,
    start_camera: Option<ViewCamera>,
    /// `(span, from, to)`, non-overlapping, ascending.
    cameras: Vec<(Span, ViewCamera, ViewCamera)>,
    /// `(from, to, text)`.
    captions: Vec<(f64, f64, String)>,
    emitters: Vec<Emitter>,
    /// `(time, part, label or None to remove)`, ascending in time, file order kept within a time.
    label_events: Vec<(f64, usize, Option<PartLabel>)>,
    /// `(time, key id, entry or None to remove)`, ascending in time, file order kept within a time.
    key_events: Vec<(f64, String, Option<KeyEntry>)>,
    /// `(group part, the parts with geometry in it)` for every group scaled or turned as one body.
    bodies: Vec<(usize, Vec<usize>)>,
}

/// How one part looks at one instant. `turns` are applied in order, each
/// about the part's own centre, after `scale`; `offset` is added last.
#[derive(Clone, Debug, PartialEq)]
pub struct PartPose {
    pub shown: bool,
    pub offset: [f64; 3],
    pub scale: f64,
    /// `(unit axis, degrees)`, in the order the turns began.
    pub turns: Vec<([f64; 3], f64)>,
    /// One entry per entry of `turns`: the hinge point it swings about, or
    /// `None` for a turn about the part's own centre.
    pub turn_pivots: Vec<Option<[f64; 3]>>,
    /// The group whose combined centre this part scales and turns about (see
    /// `Pose::bodies`), or `None` for its own centre.
    pub body: Option<usize>,
    /// `None` keeps the part's authored colour.
    pub colour: Option<[f64; 3]>,
    /// Opacity, 0 to 1: the authored opacity, changed by any `Fade`.
    pub opacity: f64,
    pub highlighted: bool,
}

/// The whole picture at one instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub time: f64,
    /// One entry per part of the structure, in part order.
    pub parts: Vec<PartPose>,
    /// `None` when the timeline has no camera.
    pub camera: Option<ViewCamera>,
    pub caption: Option<String>,
    /// `(group part, parts with geometry in it)` for each group that scales or
    /// turns as one body; a part's `body` names one of these groups.
    pub bodies: Vec<(usize, Vec<usize>)>,
    /// Particle groups running at this time, in the order they were written.
    pub particles: Vec<ParticleGroup>,
    /// Labels shown at this time, in the order they were (last) set.
    pub labels: Vec<PartLabel>,
    /// Colour key: authored entries in the order they were (last) added, then
    /// the keys of the particles shown, without repeats.
    pub key: Vec<KeyEntry>,
}

/// A particle generator placed on the timeline.
#[derive(Clone, Debug, PartialEq)]
struct Emitter {
    /// Whether it is still shown after `end` (a `Spread` that stays).
    stays: bool,
    id: String,
    start: f64,
    end: f64,
    kind: EmitterKind,
    radius: f64,
    colour: [f64; 3],
    key: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum EmitterKind {
    /// Shown from `start` on, resting where the walk ended.
    Spread(Spread),
    /// Shown from `start` to `end` only.
    Flow(Flow),
}

/// Exact at both ends (`p` 0 gives `a`, 1 gives `b`), so a finished change shows its target value.
fn lerp(a: f64, b: f64, p: f64) -> f64 {
    a * (1.0 - p) + b * p
}

impl Timeline {
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Total length in seconds.
    #[must_use]
    pub const fn duration(&self) -> f64 {
        self.duration
    }
    /// Every beat in play order with its start and end time, for stepping.
    #[must_use]
    pub fn beats(&self) -> &[BeatSpan] {
        &self.beats
    }

    /// The picture at presentation time `t` (clamped to `0..=duration`).
    #[must_use]
    pub fn pose_at(&self, t: f64) -> Pose {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, self.duration) };
        let parts = self.tracks.iter().map(|track| part_pose(track, t)).collect();

        let mut camera = self.start_camera;
        for (span, from, to) in &self.cameras {
            if t < span.start {
                break;
            }
            let p = span.progress(t);
            camera = Some(ViewCamera {
                yaw_deg: lerp(from.yaw_deg, to.yaw_deg, p),
                pitch_deg: lerp(from.pitch_deg, to.pitch_deg, p),
                zoom: lerp(from.zoom, to.zoom, p),
            });
        }

        // The caption that started most recently wins.
        let mut caption: Option<(&f64, &String)> = None;
        for (from, to, text) in &self.captions {
            if t >= *from && t < *to && caption.is_none_or(|(best, _)| from >= best) {
                caption = Some((from, text));
            }
        }
        let mut particles = Vec::new();
        let mut particle_keys: Vec<KeyEntry> = Vec::new();
        for e in &self.emitters {
            if t < e.start {
                continue;
            }
            let local = t - e.start;
            let positions = match &e.kind {
                EmitterKind::Spread(_) if !e.stays && t > e.end => continue,
                EmitterKind::Spread(spread) => spread.positions_at(local),
                EmitterKind::Flow(flow) if t <= e.end => flow.positions_at(local),
                EmitterKind::Flow(_) => continue,
            };
            if let Some(text) = &e.key {
                particle_keys.push(KeyEntry { colour: e.colour, text: text.clone() });
            }
            particles.push(ParticleGroup { id: e.id.clone(), positions, radius: e.radius, colour: e.colour });
        }

        // The latest event at or before `t` wins for each part or key id; order by when it was set.
        let mut latest: HashMap<usize, Option<usize>> = HashMap::new();
        for (seq, (time, part, label)) in self.label_events.iter().enumerate() {
            if *time <= t {
                latest.insert(*part, label.as_ref().map(|_| seq));
            }
        }
        let labels: Vec<PartLabel> = self
            .label_events
            .iter()
            .enumerate()
            .filter_map(|(seq, (_, part, label))| {
                label.as_ref().filter(|_| latest.get(part) == Some(&Some(seq))).cloned()
            })
            .collect();
        let mut latest_key: HashMap<&str, Option<usize>> = HashMap::new();
        for (seq, (time, id, entry)) in self.key_events.iter().enumerate() {
            if *time <= t {
                latest_key.insert(id.as_str(), entry.as_ref().map(|_| seq));
            }
        }
        let mut key: Vec<KeyEntry> = Vec::new();
        for (seq, (_, id, entry)) in self.key_events.iter().enumerate() {
            if let Some(entry) = entry {
                if latest_key.get(id.as_str()) == Some(&Some(seq)) && !key.contains(entry) {
                    key.push(entry.clone());
                }
            }
        }
        for entry in particle_keys {
            if !key.contains(&entry) {
                key.push(entry);
            }
        }

        Pose { time: t, parts, camera, caption: caption.map(|(_, text)| text.clone()), bodies: self.bodies.clone(), particles, labels, key }
    }
}

/// Rotate `v` about the unit `axis` by `degrees` (Rodrigues' formula).
fn rotate(v: [f64; 3], axis: [f64; 3], degrees: f64) -> [f64; 3] {
    let (s, c) = degrees.to_radians().sin_cos();
    let dot = axis[0] * v[0] + axis[1] * v[1] + axis[2] * v[2];
    let cross = [
        axis[1] * v[2] - axis[2] * v[1],
        axis[2] * v[0] - axis[0] * v[2],
        axis[0] * v[1] - axis[1] * v[0],
    ];
    [
        v[0] * c + cross[0] * s + axis[0] * dot * (1.0 - c),
        v[1] * c + cross[1] * s + axis[1] * dot * (1.0 - c),
        v[2] * c + cross[2] * s + axis[2] * dot * (1.0 - c),
    ]
}

/// The vertices a part's triangles use (ascending, no repeats) and the centre
/// of their bounding box, which is what a pose scales and turns about.
pub(crate) fn range_vertices_and_centre(points: &[[f64; 3]], indices: &[u32], range: &PartRange) -> (Vec<usize>, [f64; 3]) {
    let mut used: Vec<usize> = indices[range.first_triangle * 3..(range.first_triangle + range.triangle_count) * 3]
        .iter()
        .map(|&i| i as usize)
        .collect();
    used.sort_unstable();
    used.dedup();
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for &i in &used {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(points[i][axis]);
            hi[axis] = hi[axis].max(points[i][axis]);
        }
    }
    (used, [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0])
}

/// The centre `range`'s part is scaled and turned about under `pose`: its own
/// bounding-box centre, or, for a part in a body (a `Grow` or `Turn` on a
/// group), the centre of the bounding box of every part in that body.
/// `points` are the authored (unposed) vertices and `ranges` all the geometry's ranges.
pub(crate) fn pose_centre(points: &[[f64; 3]], indices: &[u32], ranges: &[PartRange], pose: &Pose, range: &PartRange) -> [f64; 3] {
    let own = range_vertices_and_centre(points, indices, range).1;
    let Some(group) = pose.parts.get(range.part_index).and_then(|p| p.body) else { return own };
    let Some((_, members)) = pose.bodies.iter().find(|(g, _)| *g == group) else { return own };
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for r in ranges.iter().filter(|r| members.contains(&r.part_index)) {
        let (used, _) = range_vertices_and_centre(points, indices, r);
        for &i in &used {
            for axis in 0..3 {
                lo[axis] = lo[axis].min(points[i][axis]);
                hi[axis] = hi[axis].max(points[i][axis]);
            }
        }
    }
    if lo[0].is_finite() {
        [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0]
    } else {
        own
    }
}

/// One point of a part under its pose: scaled about `centre`, turned in order about it, then moved.
pub(crate) fn pose_point(point: [f64; 3], centre: [f64; 3], part: &PartPose) -> [f64; 3] {
    let mut rel = [
        (point[0] - centre[0]) * part.scale,
        (point[1] - centre[1]) * part.scale,
        (point[2] - centre[2]) * part.scale,
    ];
    for (k, &(axis, degrees)) in part.turns.iter().enumerate() {
        match part.turn_pivots.get(k).copied().flatten() {
            None => rel = rotate(rel, axis, degrees),
            Some(pivot) => {
                // Turn about the hinge: work relative to it, then come back to the centre frame.
                let about = [pivot[0] - centre[0], pivot[1] - centre[1], pivot[2] - centre[2]];
                let r = rotate([rel[0] - about[0], rel[1] - about[1], rel[2] - about[2]], axis, degrees);
                rel = [r[0] + about[0], r[1] + about[1], r[2] + about[2]];
            }
        }
    }
    [centre[0] + rel[0] + part.offset[0], centre[1] + rel[1] + part.offset[1], centre[2] + rel[2] + part.offset[2]]
}

/// How far a highlighted part's colour moves towards white.
const HIGHLIGHT_LIGHTEN: f64 = 0.45;

impl StructureGeometry {
    /// The geometry as it looks under `pose`. A highlighted part is
    /// lightened towards white. Each part is scaled about the
    /// centre of its bounding box, turned in order about that same centre,
    /// then moved by its offset. Vertex and triangle counts, ranges and
    /// winding are unchanged; a colour in the pose replaces the part's colour.
    /// Visibility is not applied here: build only the visible parts.
    /// Parts the pose has no entry for are left as they are.
    #[must_use]
    pub fn apply_pose(&self, pose: &Pose) -> Self {
        let mut points: Vec<[f64; 3]> =
            self.mesh().vertices().iter().map(|p| p.to_array()).collect();
        let mut colours = self.colours().to_vec();
        let indices = self.mesh().indices();
        // Centres come from the authored points, before any part has moved.
        let centres: Vec<[f64; 3]> =
            self.ranges().iter().map(|r| pose_centre(&points, indices, self.ranges(), pose, r)).collect();

        for (range, centre) in self.ranges().iter().zip(centres) {
            let Some(part) = pose.parts.get(range.part_index) else { continue };
            let (used, _) = range_vertices_and_centre(&points, indices, range);
            for &i in &used {
                points[i] = pose_point(points[i], centre, part);
                if let Some(colour) = part.colour {
                    colours[i] = colour;
                }
                if part.highlighted {
                    colours[i] = colours[i].map(|c| lerp(c, 1.0, HIGHLIGHT_LIGHTEN));
                }
            }
        }

        let vertices = points.iter().map(|p| WorldPoint3::new(p[0], p[1], p[2])).collect();
        // Same indices, finite inputs: an affine map of a valid mesh stays valid.
        let mesh = TriangleMesh3::new(vertices, indices.to_vec())
            .expect("a posed mesh keeps its topology and stays finite");
        Self::from_parts(mesh, colours, self.ranges().to_vec())
    }
}

fn part_pose(track: &PartTrack, t: f64) -> PartPose {
    let mut shown = track.initially_shown;
    for &(time, value) in &track.visibility {
        if t >= time {
            shown = value;
        }
    }

    let mut offset = [0.0; 3];
    for (span, by) in &track.moves {
        let p = span.progress(t);
        for axis in 0..3 {
            offset[axis] += by[axis] * p;
        }
    }

    let mut scale = 1.0;
    for (span, from, to) in &track.scales {
        if t < span.start {
            break;
        }
        scale = lerp(*from, *to, span.progress(t));
    }

    let mut turns = Vec::new();
    let mut turn_pivots = Vec::new();
    for (span, axis, degrees, pivot) in &track.turns {
        let p = span.progress(t);
        if p > 0.0 {
            turns.push((*axis, degrees * p));
            turn_pivots.push(*pivot);
        }
    }

    let mut colour = None;
    for (span, from, to) in &track.colours {
        if t < span.start {
            break;
        }
        let p = span.progress(t);
        colour = Some([lerp(from[0], to[0], p), lerp(from[1], to[1], p), lerp(from[2], to[2], p)]);
    }

    let mut opacity = track.base_opacity;
    for (span, from, to) in &track.fades {
        if t < span.start {
            break;
        }
        opacity = lerp(*from, *to, span.progress(t));
    }

    let highlighted = track.highlights.iter().any(|&(from, to)| t >= from && t < to);
    PartPose { shown, offset, scale, turns, turn_pivots, body: track.body.flatten(), colour, opacity, highlighted }
}

fn positive_finite(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

fn non_negative_finite(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

/// Validates the timelines of a structure against its parts.
pub(crate) fn validate_timelines(
    documents: &[TimelineDocument],
    parts: &[Part],
    by_id: &HashMap<&str, usize>,
) -> Result<Vec<Timeline>, StructureError> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut timelines = Vec::with_capacity(documents.len());
    for (index, d) in documents.iter().enumerate() {
        if d.id.is_empty() {
            return Err(StructureError::EmptyTimelineId { index });
        }
        if !seen.insert(d.id.as_str()) {
            return Err(StructureError::DuplicateTimelineId(d.id.clone()));
        }
        timelines.push(compile(d, parts, by_id)?);
    }
    Ok(timelines)
}

fn compile(d: &TimelineDocument, parts: &[Part], by_id: &HashMap<&str, usize>) -> Result<Timeline, StructureError> {
    let bad = |reason: String| StructureError::InvalidTimeline { timeline: d.id.clone(), reason };
    if let Some(c) = &d.camera {
        if !(c.yaw_deg.is_finite() && c.pitch_deg.is_finite() && positive_finite(c.zoom)) {
            return Err(bad("the start camera needs finite angles and a positive zoom".to_owned()));
        }
    }
    if d.scenes.is_empty() {
        return Err(bad("a timeline needs at least one scene".to_owned()));
    }

    let root_of = |part_id: &str| -> Result<usize, StructureError> {
        by_id.get(part_id).copied().ok_or_else(|| StructureError::UnknownTimelinePart {
            timeline: d.id.clone(),
            part: part_id.to_owned(),
        })
    };
    // The part and all its descendants, in part order.
    let with_descendants = |root: usize| -> Vec<usize> {
        (0..parts.len())
            .filter(|&i| {
                let mut current = Some(i);
                while let Some(c) = current {
                    if c == root {
                        return true;
                    }
                    current = parts[c].parent();
                }
                false
            })
            .collect()
    };
    // The parts a Grow or Turn acts on, and the group they act as one body about (None: each about
    // its own centre, which is also the case for a single part).
    let transform_targets = |part_id: &str, what: &str, each: bool| -> Result<(Vec<usize>, Option<usize>), StructureError> {
        let root = root_of(part_id)?;
        let members: Vec<usize> =
            with_descendants(root).into_iter().filter(|&i| parts[i].geometry().is_some()).collect();
        if members.is_empty() {
            return Err(bad(format!("{what} needs geometry in '{part_id}' or below it")));
        }
        let body = if each || members == [root] { None } else { Some(root) };
        Ok((members, body))
    };
    let mut bodies: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();

    let mut tracks: Vec<PartTrack> = (0..parts.len()).map(|_| PartTrack::new()).collect();
    let mut cameras: Vec<(Span, ViewCamera)> = Vec::new();
    let mut captions: Vec<(f64, f64, String)> = Vec::new();
    let mut emitters: Vec<Emitter> = Vec::new();
    let mut label_events: Vec<(f64, usize, Option<PartLabel>)> = Vec::new();
    let mut key_events: Vec<(f64, String, Option<KeyEntry>)> = Vec::new();
    let mut beats: Vec<BeatSpan> = Vec::new();
    let mut scene_ids: HashSet<&str> = HashSet::new();
    let mut clock = 0.0_f64;

    for scene in &d.scenes {
        if scene.id.is_empty() {
            return Err(bad("a scene has an empty id".to_owned()));
        }
        if !scene_ids.insert(scene.id.as_str()) {
            return Err(bad(format!("duplicate scene id '{}'", scene.id)));
        }
        if scene.beats.is_empty() {
            return Err(bad(format!("scene '{}' has no beats", scene.id)));
        }
        let mut beat_ids: HashSet<&str> = HashSet::new();
        for beat in &scene.beats {
            if beat.id.is_empty() {
                return Err(bad(format!("a beat in scene '{}' has an empty id", scene.id)));
            }
            if !beat_ids.insert(beat.id.as_str()) {
                return Err(bad(format!("duplicate beat id '{}' in scene '{}'", beat.id, scene.id)));
            }
            if beat.actions.is_empty() {
                return Err(bad(format!("beat '{}' has no actions", beat.id)));
            }
            let mut length = 0.0_f64;
            let mut marked: Vec<(f64, usize)> = Vec::new();

            for action in &beat.actions {
                let timing = |at_s: f64, for_s: f64| -> Result<Span, StructureError> {
                    if !non_negative_finite(at_s) || !non_negative_finite(for_s) {
                        return Err(bad(format!("beat '{}': times must be finite and not negative", beat.id)));
                    }
                    Ok(Span { start: clock + at_s, end: clock + at_s + for_s, ease: Ease::Linear })
                };
                match action {
                    ActionDocument::Narrate { text, for_s, at_s } => {
                        let span = timing(*at_s, *for_s)?;
                        if text.is_empty() || *for_s <= 0.0 {
                            return Err(bad(format!(
                                "beat '{}': a caption needs text and a positive duration",
                                beat.id
                            )));
                        }
                        captions.push((span.start, span.end, text.clone()));
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Wait { for_s, at_s } => {
                        timing(*at_s, *for_s)?;
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Show { part, at_s } | ActionDocument::Hide { part, at_s } => {
                        let span = timing(*at_s, 0.0)?;
                        let value = matches!(action, ActionDocument::Show { .. });
                        for i in with_descendants(root_of(part)?) {
                            tracks[i].visibility.push((span.start, value));
                        }
                        length = length.max(*at_s);
                    }
                    ActionDocument::Highlight { part, at_s } => {
                        let span = timing(*at_s, 0.0)?;
                        for i in with_descendants(root_of(part)?) {
                            marked.push((span.start, i));
                        }
                        length = length.max(*at_s);
                    }
                    ActionDocument::Move { part, by, for_s, at_s, ease } => {
                        let mut span = timing(*at_s, *for_s)?;
                        span.ease = *ease;
                        if !(by.0.is_finite() && by.1.is_finite() && by.2.is_finite()) {
                            return Err(bad(format!("beat '{}': a move must be finite", beat.id)));
                        }
                        for i in with_descendants(root_of(part)?) {
                            tracks[i].moves.push((span, [by.0, by.1, by.2]));
                        }
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Grow { part, to, each, for_s, at_s, ease } => {
                        let mut span = timing(*at_s, *for_s)?;
                        span.ease = *ease;
                        if !positive_finite(*to) {
                            return Err(bad(format!("beat '{}': a size must be finite and positive", beat.id)));
                        }
                        let (members, body) = transform_targets(part, "Grow", *each)?;
                        for &i in &members {
                            set_body(&mut tracks[i], body, parts[i].id()).map_err(&bad)?;
                            tracks[i].scales.push((span, 1.0, *to));
                        }
                        if let Some(group) = body {
                            bodies.insert(group, members);
                        }
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Turn { part, axis, deg, about, each, for_s, at_s, ease } => {
                        let mut span = timing(*at_s, *for_s)?;
                        span.ease = *ease;
                        let norm = (axis.0 * axis.0 + axis.1 * axis.1 + axis.2 * axis.2).sqrt();
                        if !(axis.0.is_finite() && axis.1.is_finite() && axis.2.is_finite())
                            || !positive_finite(norm)
                            || !deg.is_finite()
                        {
                            return Err(bad(format!(
                                "beat '{}': a turn needs a finite non-zero axis and finite degrees",
                                beat.id
                            )));
                        }
                        if let Some(p) = about {
                            if !(p.0.is_finite() && p.1.is_finite() && p.2.is_finite()) {
                                return Err(bad(format!("beat '{}': a hinge point must be finite", beat.id)));
                            }
                        }
                        let (members, body) = transform_targets(part, "Turn", *each)?;
                        let pivot = about.map(|p| [p.0, p.1, p.2]);
                        for &i in &members {
                            set_body(&mut tracks[i], body, parts[i].id()).map_err(&bad)?;
                            tracks[i].turns.push((span, [axis.0 / norm, axis.1 / norm, axis.2 / norm], *deg, pivot));
                        }
                        if let Some(group) = body {
                            bodies.insert(group, members);
                        }
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Fade { part, to, for_s, at_s, ease } => {
                        let mut span = timing(*at_s, *for_s)?;
                        span.ease = *ease;
                        if !(to.is_finite() && (0.0..=1.0).contains(to)) {
                            return Err(bad(format!("beat '{}': an opacity must be in 0 to 1", beat.id)));
                        }
                        for i in with_descendants(root_of(part)?) {
                            tracks[i].fades.push((span, parts[i].opacity(), *to));
                        }
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Colour { part, to, for_s, at_s, ease } => {
                        let mut span = timing(*at_s, *for_s)?;
                        span.ease = *ease;
                        if ![to.0, to.1, to.2].iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)) {
                            return Err(bad(format!("beat '{}': colour values must be in 0 to 1", beat.id)));
                        }
                        for i in with_descendants(root_of(part)?) {
                            let from = parts[i].colour();
                            tracks[i].colours.push((span, from, [to.0, to.1, to.2]));
                        }
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Label { part, text, at, side, at_s } => {
                        let span = timing(*at_s, 0.0)?;
                        let index = root_of(part)?;
                        label_parts(text.as_ref(), *at).map_err(|e| bad(format!("beat '{}': {e}", beat.id)))?;
                        let text = text.clone().unwrap_or_else(|| parts[index].name().to_owned());
                        let at = at.map(|a| [a.0, a.1, a.2]);
                        label_events.push((span.start, index, Some(PartLabel { part: index, text, at, side: *side })));
                        length = length.max(*at_s);
                    }
                    ActionDocument::Unlabel { part, at_s } => {
                        let span = timing(*at_s, 0.0)?;
                        label_events.push((span.start, root_of(part)?, None));
                        length = length.max(*at_s);
                    }
                    ActionDocument::Key { id, colour, text, at_s } => {
                        let span = timing(*at_s, 0.0)?;
                        if id.is_empty() {
                            return Err(bad(format!("beat '{}': a key entry needs a non-empty id", beat.id)));
                        }
                        let entry = key_entry(*colour, text).map_err(|e| bad(format!("beat '{}': {e}", beat.id)))?;
                        key_events.push((span.start, id.clone(), Some(entry)));
                        length = length.max(*at_s);
                    }
                    ActionDocument::Unkey { id, at_s } => {
                        let span = timing(*at_s, 0.0)?;
                        if !key_events.iter().any(|(_, k, e)| k == id && e.is_some()) {
                            return Err(bad(format!("beat '{}': Unkey '{id}' has no Key before it", beat.id)));
                        }
                        key_events.push((span.start, id.clone(), None));
                        length = length.max(*at_s);
                    }
                    ActionDocument::Spread {
                        id, count, from, radius, inside, step, seed, particle_radius, colour, for_s, at_s, stay, key,
                    } => {
                        let span = timing(*at_s, *for_s)?;
                        let colour = emitter_colour(&bad, &beat.id, id, &emitters, *particle_radius, *colour, *for_s)?;
                        let container = inside.container().map_err(|e| bad(format!("Spread '{id}': {e}")))?;
                        let walk = Spread::new(
                            &SpreadSpec {
                                count: *count,
                                start: [from.0, from.1, from.2],
                                start_radius: *radius,
                                step_length: *step,
                                steps_per_second: SPREAD_STEPS_PER_SECOND,
                                duration_s: *for_s,
                                seed: *seed,
                            },
                            container.as_ref(),
                        )
                        .map_err(|e| bad(format!("Spread '{id}': {e}")))?;
                        emitters.push(Emitter {
                            stays: *stay,
                            id: id.clone(),
                            start: span.start,
                            end: span.end,
                            kind: EmitterKind::Spread(walk),
                            radius: *particle_radius,
                            colour,
                            key: particle_key(&bad, id, key.as_ref())?,
                        });
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Flow { id, count, path, corner_radius, point_spacing, speed, lateral, seed, particle_radius, colour, for_s, at_s, key } => {
                        let span = timing(*at_s, *for_s)?;
                        let colour = emitter_colour(&bad, &beat.id, id, &emitters, *particle_radius, *colour, *for_s)?;
                        let mut flow_path: Vec<[f64; 3]> = path.iter().map(|p| [p.0, p.1, p.2]).collect();
                        match (corner_radius, point_spacing) {
                            (Some(radius), spacing) => {
                                flow_path = crate::structure_route::rounded_route(
                                    &flow_path,
                                    *radius,
                                    spacing.unwrap_or(radius / 3.0),
                                )
                                .map_err(|e| bad(format!("Flow '{id}': {e}")))?;
                            }
                            (None, Some(_)) => {
                                return Err(bad(format!("Flow '{id}': point_spacing needs a corner_radius")));
                            }
                            (None, None) => {}
                        }
                        let flow = Flow::new(&FlowSpec {
                            count: *count,
                            speed: *speed,
                            path: flow_path,
                            lateral_radius: *lateral,
                            seed: *seed,
                        })
                        .map_err(|e| bad(format!("Flow '{id}': {e}")))?;
                        emitters.push(Emitter {
                            stays: false,
                            id: id.clone(),
                            start: span.start,
                            end: span.end,
                            kind: EmitterKind::Flow(flow),
                            radius: *particle_radius,
                            colour,
                            key: particle_key(&bad, id, key.as_ref())?,
                        });
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Camera { yaw_deg, pitch_deg, zoom, for_s, at_s, ease } => {
                        let mut span = timing(*at_s, *for_s)?;
                        span.ease = *ease;
                        if d.camera.is_none() {
                            return Err(bad(
                                "Camera actions need the timeline's start camera (`camera: Some((...))`)".to_owned(),
                            ));
                        }
                        if !(yaw_deg.is_finite() && pitch_deg.is_finite() && positive_finite(*zoom)) {
                            return Err(bad(format!(
                                "beat '{}': a camera needs finite angles and a positive zoom",
                                beat.id
                            )));
                        }
                        cameras.push((span, ViewCamera { yaw_deg: *yaw_deg, pitch_deg: *pitch_deg, zoom: *zoom }));
                        length = length.max(at_s + for_s);
                    }
                }
            }
            let end = clock + length;
            for (from, i) in marked {
                tracks[i].highlights.push((from, end));
            }
            beats.push(BeatSpan { scene: scene.id.clone(), beat: beat.id.clone(), start: clock, end });
            clock = end;
        }
    }

    // Order each track and chain the "from" values.
    for (index, track) in tracks.iter_mut().enumerate() {
        let part_id = parts[index].id();
        track.visibility.sort_by(|a, b| a.0.total_cmp(&b.0));
        if let Some(&(_, first)) = track.visibility.first() {
            track.initially_shown = !first;
        }
        track.moves.sort_by(|a, b| a.0.start.total_cmp(&b.0.start));
        track.turns.sort_by(|a, b| a.0.start.total_cmp(&b.0.start));

        track.scales.sort_by(|a, b| a.0.start.total_cmp(&b.0.start));
        let mut previous: Option<(f64, f64)> = None; // (end, value)
        for (span, from, to) in &mut track.scales {
            if let Some((end, value)) = previous {
                if span.start < end - EPS {
                    return Err(bad(format!("two Grow actions on '{part_id}' overlap in time")));
                }
                *from = value;
            }
            previous = Some((span.end, *to));
        }

        track.base_opacity = parts[index].opacity();
        track.fades.sort_by(|a, b| a.0.start.total_cmp(&b.0.start));
        let mut previous: Option<(f64, f64)> = None;
        for (span, from, to) in &mut track.fades {
            if let Some((end, value)) = previous {
                if span.start < end - EPS {
                    return Err(bad(format!("two Fade actions on '{part_id}' overlap in time")));
                }
                *from = value;
            }
            previous = Some((span.end, *to));
        }

        track.colours.sort_by(|a, b| a.0.start.total_cmp(&b.0.start));
        let mut previous: Option<(f64, [f64; 3])> = None;
        for (span, from, to) in &mut track.colours {
            if let Some((end, value)) = previous {
                if span.start < end - EPS {
                    return Err(bad(format!("two Colour actions on '{part_id}' overlap in time")));
                }
                *from = value;
            }
            previous = Some((span.end, *to));
        }
    }

    cameras.sort_by(|a, b| a.0.start.total_cmp(&b.0.start));
    let mut chained = Vec::with_capacity(cameras.len());
    let mut previous: Option<(f64, ViewCamera)> = None;
    for (span, to) in cameras {
        let from = match previous {
            Some((end, value)) => {
                if span.start < end - EPS {
                    return Err(bad("two Camera actions overlap in time".to_owned()));
                }
                value
            }
            None => d.camera.expect("checked when the action was read"),
        };
        previous = Some((span.end, to));
        chained.push((span, from, to));
    }

    Ok(Timeline {
        id: d.id.clone(),
        name: d.name.clone(),
        duration: clock,
        beats,
        tracks,
        start_camera: d.camera,
        cameras: chained,
        captions,
        emitters,
        label_events: {
            // Stable: events at the same time keep their file order.
            label_events.sort_by(|a, b| a.0.total_cmp(&b.0));
            label_events
        },
        bodies: bodies.into_iter().collect(),
        key_events: {
            key_events.sort_by(|a, b| a.0.total_cmp(&b.0));
            key_events
        },
    })
}

/// Time steps per second of a `Spread` walk.
const SPREAD_STEPS_PER_SECOND: f64 = 20.0;

/// Records the body a part scales and turns with. All the Grow and Turn actions on one part must agree.
fn set_body(track: &mut PartTrack, body: Option<usize>, part_id: &str) -> Result<(), String> {
    match track.body {
        Some(existing) if existing != body => Err(format!(
            "'{part_id}' is grown or turned both as part of a group and on its own (or in another group); \
             use one way for each part, or `each: true` on the group"
        )),
        _ => {
            track.body = Some(body);
            Ok(())
        }
    }
}

/// Checks what every particle action shares; returns its colour.
fn particle_key(
    bad: &dyn Fn(String) -> StructureError,
    id: &str,
    key: Option<&String>,
) -> Result<Option<String>, StructureError> {
    match key {
        Some(text) if text.trim().is_empty() => Err(bad(format!("particles '{id}': a key needs text"))),
        other => Ok(other.cloned()),
    }
}

fn emitter_colour(
    bad: &dyn Fn(String) -> StructureError,
    beat: &str,
    id: &str,
    existing: &[Emitter],
    particle_radius: f64,
    colour: (f64, f64, f64),
    for_s: f64,
) -> Result<[f64; 3], StructureError> {
    if id.is_empty() || existing.iter().any(|e| e.id == id) {
        return Err(bad(format!("beat '{beat}': a particle action needs a unique, non-empty id ('{id}')")));
    }
    if !positive_finite(particle_radius) || !positive_finite(for_s) {
        return Err(bad(format!("particles '{id}': the particle radius and the duration must be positive")));
    }
    let c = [colour.0, colour.1, colour.2];
    if !c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)) {
        return Err(bad(format!("particles '{id}': colour values must be in 0 to 1")));
    }
    Ok(c)
}

#[cfg(test)]
mod tests {
    use crate::structure::{Structure, StructureError};

    /// root (group) with `a` and `b` (geometry), and `c` a child of `a`.
    fn structure(timelines: &str) -> Result<Structure, StructureError> {
        let shape = "Some(Extrusion(axis_length: 1.0, profile: Disc(radius: 1.0)))";
        let part = |id: &str, parent: &str, geometry: &str| {
            format!(
                "Part(id: \"{id}\", name: \"{id}\", parent: {parent}, description: \"d\", \
                 colour: (0.5, 0.5, 0.5), geometry: {geometry}),"
            )
        };
        let parts = format!(
            "{}{}{}{}",
            part("root", "None", "None"),
            part("a", "Some(\"root\")", shape),
            part("b", "Some(\"root\")", shape),
            part("c", "Some(\"a\")", shape)
        );
        Structure::from_ron_str(&format!(
            "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 16, \
             parts: [{parts}]{timelines})"
        ))
    }

    fn one(actions: &str) -> String {
        format!(
            ", timelines: [Timeline(id: \"t\", name: \"T\", camera: Some((yaw_deg: 10.0, pitch_deg: 20.0, zoom: 1.0)), \
             scenes: [Scene(id: \"s\", beats: [Beat(id: \"b\", actions: [{actions}])])])]"
        )
    }

    fn timeline(actions: &str) -> crate::structure_animation::Timeline {
        structure(&one(actions)).unwrap().timelines()[0].clone()
    }

    fn invalid(actions: &str) -> String {
        match structure(&one(actions)).unwrap_err() {
            StructureError::InvalidTimeline { reason, .. } => reason,
            other => panic!("unexpected error {other:?}"),
        }
    }

    fn signed_volume(geometry: &crate::structure::StructureGeometry, range: usize) -> f64 {
        let r = geometry.ranges()[range];
        let v = geometry.mesh().vertices();
        let idx = geometry.mesh().indices();
        let mut total = 0.0;
        for tri in r.first_triangle..r.first_triangle + r.triangle_count {
            let p = |k: usize| {
                v[idx[tri * 3 + k] as usize].to_array()
            };
            let (a, b, c) = (p(0), p(1), p(2));
            total += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0;
        }
        total
    }

    fn posed(actions: &str, t: f64) -> (crate::structure::StructureGeometry, crate::structure::StructureGeometry) {
        let s = structure(&one(actions)).unwrap();
        let base = s.build_geometry().unwrap();
        let pose = s.timelines()[0].pose_at(t);
        let after = base.apply_pose(&pose);
        (base, after)
    }

    // Built ranges are a, b, c (the group `root` has no geometry): b is range 1.
    #[test]
    fn a_pose_move_keeps_counts_and_volume() {
        let (base, after) = posed("Move(part: \"b\", by: (10.0, 2.0, -3.0), for_s: 4.0)", 4.0);
        assert_eq!(after.mesh().indices(), base.mesh().indices());
        assert_eq!(after.mesh().vertices().len(), base.mesh().vertices().len());
        assert_eq!(after.ranges(), base.ranges());
        assert!((signed_volume(&after, 1) - signed_volume(&base, 1)).abs() < 1e-9);
        let (b0, b1) = (base.mesh().vertices(), after.mesh().vertices());
        let r = base.ranges()[1];
        let first = base.mesh().indices()[r.first_triangle * 3] as usize;
        assert!((b1[first].to_array()[0] - b0[first].to_array()[0] - 10.0).abs() < 1e-9);
        // Parts not moved are untouched.
        let a = base.mesh().indices()[base.ranges()[0].first_triangle * 3] as usize;
        assert_eq!(b1[a], b0[a]);
    }

    #[test]
    fn a_pose_scale_multiplies_volume_by_the_cube() {
        let (base, after) = posed("Grow(part: \"b\", to: 2.0, for_s: 1.0)", 1.0);
        let ratio = signed_volume(&after, 1) / signed_volume(&base, 1);
        assert!((ratio - 8.0).abs() < 1e-9, "ratio {ratio}");
    }

    #[test]
    fn a_pose_turn_keeps_volume_and_centre() {
        let (base, after) = posed("Turn(part: \"b\", axis: (0.0, 1.0, 1.0), deg: 90.0, for_s: 1.0)", 1.0);
        assert!((signed_volume(&after, 1) - signed_volume(&base, 1)).abs() < 1e-9);
        let centre = |g: &crate::structure::StructureGeometry| {
            let r = g.ranges()[1];
            let mut used: Vec<usize> = g.mesh().indices()
                [r.first_triangle * 3..(r.first_triangle + r.triangle_count) * 3]
                .iter()
                .map(|&i| i as usize)
                .collect();
            used.sort_unstable();
            used.dedup();
            let v = g.mesh().vertices();
            let n = used.len() as f64;
            [
                used.iter().map(|&i| v[i].to_array()[0]).sum::<f64>() / n,
                used.iter().map(|&i| v[i].to_array()[1]).sum::<f64>() / n,
                used.iter().map(|&i| v[i].to_array()[2]).sum::<f64>() / n,
            ]
        };
        // The disc extrusion is symmetric, so the vertex mean is its bounding-box centre.
        let (c0, c1) = (centre(&base), centre(&after));
        for axis in 0..3 {
            assert!((c0[axis] - c1[axis]).abs() < 1e-9);
        }
        assert_ne!(base.mesh().vertices(), after.mesh().vertices());
    }

    /// The vertices of range 1 (part `b`) of a built geometry.
    fn range_one_points(g: &crate::structure::StructureGeometry) -> Vec<[f64; 3]> {
        let r = g.ranges()[1];
        let mut used: Vec<usize> = g.mesh().indices()[r.first_triangle * 3..(r.first_triangle + r.triangle_count) * 3]
            .iter()
            .map(|&i| i as usize)
            .collect();
        used.sort_unstable();
        used.dedup();
        used.iter().map(|&i| g.mesh().vertices()[i].to_array()).collect()
    }

    #[test]
    fn a_hinged_turn_swings_the_part_about_the_hinge_point() {
        let s0 = structure(&one("Wait(for_s: 1.0)")).unwrap();
        let base0 = s0.build_geometry().unwrap();
        let p = range_one_points(&base0)[0];
        let actions = format!(
            "Turn(part: \"b\", axis: (0.0, 0.0, 2.0), deg: 90.0, about: Some(({}, {}, {})), for_s: 1.0)",
            p[0], p[1], p[2]
        );
        let (base, after) = posed(&actions, 1.0);
        let (before, now) = (range_one_points(&base), range_one_points(&after));
        for (b, a) in before.iter().zip(&now) {
            // A quarter turn about z through p: (dx, dy) goes to (-dy, dx); z is unchanged.
            let (dx, dy) = (b[0] - p[0], b[1] - p[1]);
            assert!((a[0] - (p[0] - dy)).abs() < 1e-9 && (a[1] - (p[1] + dx)).abs() < 1e-9 && (a[2] - b[2]).abs() < 1e-9);
        }
        assert!((signed_volume(&after, 1) - signed_volume(&base, 1)).abs() < 1e-9);
        // Halfway through, the hinge point itself has not moved at all.
        let (_, half) = posed(&actions, 0.5);
        let hinge = range_one_points(&half)
            .into_iter()
            .find(|q| (q[0] - p[0]).abs() < 1e-9 && (q[1] - p[1]).abs() < 1e-9 && (q[2] - p[2]).abs() < 1e-9);
        assert!(hinge.is_some(), "the vertex at the hinge point stays put");
        // Other parts are untouched.
        assert_eq!(after.mesh().vertices()[base.mesh().indices()[0] as usize], base.mesh().vertices()[base.mesh().indices()[0] as usize]);
    }

    #[test]
    fn a_hinge_point_travels_with_a_move_and_an_unhinged_turn_is_unchanged() {
        let s0 = structure(&one("Wait(for_s: 1.0)")).unwrap();
        let p = range_one_points(&s0.build_geometry().unwrap())[0];
        let hinged = format!(
            "Move(part: \"b\", by: (5.0, 0.0, 0.0), for_s: 1.0),              Turn(part: \"b\", axis: (0.0, 0.0, 1.0), deg: 90.0, about: Some(({}, {}, {})), for_s: 1.0)",
            p[0], p[1], p[2]
        );
        let (base, after) = posed(&hinged, 1.0);
        let moved_pivot = [p[0] + 5.0, p[1], p[2]];
        for (b, a) in range_one_points(&base).iter().zip(&range_one_points(&after)) {
            let (dx, dy) = (b[0] - p[0], b[1] - p[1]);
            assert!((a[0] - (moved_pivot[0] - dy)).abs() < 1e-9 && (a[1] - (moved_pivot[1] + dx)).abs() < 1e-9);
        }
        // `about: None` (the default) is the old behaviour: the file without `about` poses identically.
        let (_, plain) = posed("Turn(part: \"b\", axis: (0.0, 1.0, 1.0), deg: 90.0, for_s: 1.0)", 1.0);
        let (_, none) = posed("Turn(part: \"b\", axis: (0.0, 1.0, 1.0), deg: 90.0, about: None, for_s: 1.0)", 1.0);
        assert_eq!(plain.mesh().vertices(), none.mesh().vertices());
    }

    /// A group `root` holding two balls (radius 1) at x = 10 and x = 2, so the group's centre is x = 6.
    fn two_balls(actions: &str) -> (crate::structure::StructureGeometry, crate::structure::StructureGeometry) {
        let ball = |id: &str, x: f64| {
            format!(
                "Part(id: \"{id}\", name: \"{id}\", parent: Some(\"root\"), description: \"d\", colour: (0.5, 0.5, 0.5), \
                 geometry: Some(Ellipsoid(centre: ({x}, 0.0, 0.0), radii: (1.0, 1.0, 1.0))))"
            )
        };
        let text = format!(
            "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 16, parts: [\
             Part(id: \"root\", name: \"root\", parent: None, description: \"d\", colour: (0.5, 0.5, 0.5), geometry: None), \
             {}, {}]{})",
            ball("a", 10.0),
            ball("b", 2.0),
            one(actions)
        );
        let s = Structure::from_ron_str(&text).unwrap();
        let base = s.build_geometry().unwrap();
        let after = base.apply_pose(&s.timelines()[0].pose_at(1.0));
        (base, after)
    }

    /// Mean of the vertices of part range `r`; for a ball this is its centre.
    fn ball_centre(g: &crate::structure::StructureGeometry, r: usize) -> [f64; 3] {
        let pts = range_one_points_of(g, r);
        let n = pts.len() as f64;
        [
            pts.iter().map(|p| p[0]).sum::<f64>() / n,
            pts.iter().map(|p| p[1]).sum::<f64>() / n,
            pts.iter().map(|p| p[2]).sum::<f64>() / n,
        ]
    }

    fn range_one_points_of(g: &crate::structure::StructureGeometry, r: usize) -> Vec<[f64; 3]> {
        let range = g.ranges()[r];
        let mut used: Vec<usize> = g.mesh().indices()
            [range.first_triangle * 3..(range.first_triangle + range.triangle_count) * 3]
            .iter()
            .map(|&i| i as usize)
            .collect();
        used.sort_unstable();
        used.dedup();
        used.iter().map(|&i| g.mesh().vertices()[i].to_array()).collect()
    }

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1e-9)
    }

    #[test]
    fn a_group_grows_as_one_body_about_its_combined_centre() {
        // Group centre x = 6. Doubling about it sends the balls from 10 and 2 to 14 and -2.
        let (_, after) = two_balls("Grow(part: \"root\", to: 2.0, for_s: 1.0)");
        assert!(close(ball_centre(&after, 0), [14.0, 0.0, 0.0]), "{:?}", ball_centre(&after, 0));
        assert!(close(ball_centre(&after, 1), [-2.0, 0.0, 0.0]), "{:?}", ball_centre(&after, 1));
        // The balls doubled in size too: the farthest vertex is 2 from the centre.
        let far = range_one_points_of(&after, 0).iter().map(|p| (p[0] - 14.0).abs()).fold(0.0, f64::max);
        assert!((far - 2.0).abs() < 1e-9);
        // With `each: true` every ball grows about itself and stays where it is.
        let (_, each) = two_balls("Grow(part: \"root\", to: 2.0, each: true, for_s: 1.0)");
        assert!(close(ball_centre(&each, 0), [10.0, 0.0, 0.0]));
        assert!(close(ball_centre(&each, 1), [2.0, 0.0, 0.0]));
    }

    #[test]
    fn a_group_turns_as_one_rigid_body_and_each_turns_the_parts_in_place() {
        // A quarter turn about z through the group centre (6, 0, 0): (10, 0, 0) -> (6, 4, 0), (2, 0, 0) -> (6, -4, 0).
        let (base, after) = two_balls("Turn(part: \"root\", axis: (0.0, 0.0, 1.0), deg: 90.0, for_s: 1.0)");
        assert!(close(ball_centre(&after, 0), [6.0, 4.0, 0.0]), "{:?}", ball_centre(&after, 0));
        assert!(close(ball_centre(&after, 1), [6.0, -4.0, 0.0]), "{:?}", ball_centre(&after, 1));
        // Rigid: the distance between the two balls is unchanged.
        let d = |g: &crate::structure::StructureGeometry| {
            let (a, b) = (ball_centre(g, 0), ball_centre(g, 1));
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
        };
        assert!((d(&base) - d(&after)).abs() < 1e-9);
        let (_, each) = two_balls("Turn(part: \"root\", axis: (0.0, 0.0, 1.0), deg: 90.0, each: true, for_s: 1.0)");
        assert!(close(ball_centre(&each, 0), [10.0, 0.0, 0.0]));
        assert!(close(ball_centre(&each, 1), [2.0, 0.0, 0.0]));
        // An explicit hinge point applies to the whole group: about (0, 0, 0), (10, 0, 0) goes to (0, 10, 0).
        let (_, hinged) = two_balls(
            "Turn(part: \"root\", axis: (0.0, 0.0, 1.0), deg: 90.0, about: Some((0.0, 0.0, 0.0)), for_s: 1.0)",
        );
        assert!(close(ball_centre(&hinged, 0), [0.0, 10.0, 0.0]), "{:?}", ball_centre(&hinged, 0));
        assert!(close(ball_centre(&hinged, 1), [0.0, 2.0, 0.0]), "{:?}", ball_centre(&hinged, 1));
    }

    #[test]
    fn a_pose_at_time_zero_equals_the_unposed_geometry() {
        let (base, after) = posed(
            "Move(part: \"b\", by: (5.0, 0.0, 0.0), for_s: 2.0), Grow(part: \"a\", to: 3.0, for_s: 2.0)",
            0.0,
        );
        assert_eq!(base, after);
    }

    #[test]
    fn a_highlighted_part_is_lightened_and_others_are_not() {
        let (base, after) = posed("Highlight(part: \"b\"), Wait(for_s: 1.0)", 0.5);
        let r = base.ranges()[1];
        let b = base.mesh().indices()[r.first_triangle * 3] as usize;
        assert!(after.colours()[b][0] > base.colours()[b][0]);
        assert_eq!(after.colours()[0], base.colours()[0]);
    }

    #[test]
    fn a_pose_colour_replaces_only_that_part() {
        let (base, after) = posed("Colour(part: \"b\", to: (1.0, 0.0, 0.0), for_s: 1.0)", 1.0);
        let r = base.ranges()[1];
        let b = base.mesh().indices()[r.first_triangle * 3] as usize;
        assert_eq!(after.colours()[b], [1.0, 0.0, 0.0]);
        assert_eq!(after.colours()[0], base.colours()[0]);
    }

    const DIFFUSION: &str = "Spread(id: \"dye\", count: 30, from: (0.0, 0.5, 0.0), radius: 0.2, \
        inside: Cylinder(base: (0.0, 0.0, 0.0), radius: 1.0, height: 2.0), step: 0.1, seed: 3, \
        particle_radius: 0.05, colour: (1.0, 0.0, 0.0), for_s: 5.0, at_s: 1.0)";

    #[test]
    fn a_spread_appears_when_it_starts_and_rests_where_it_ended() {
        let t = timeline(DIFFUSION);
        assert_eq!(t.duration(), 6.0);
        assert!(t.pose_at(0.5).particles.is_empty());
        let begun = t.pose_at(1.0);
        assert_eq!(begun.particles.len(), 1);
        assert_eq!(begun.particles[0].positions.len(), 30);
        assert_eq!(begun.particles[0].id, "dye");
        assert_ne!(t.pose_at(2.0).particles, begun.particles);
        // At the end the particles rest where the walk ended, and the answer is repeatable.
        assert_eq!(t.pose_at(6.0).particles, t.pose_at(6.0).particles);
        assert_eq!(t.pose_at(6.0).particles, t.pose_at(100.0).particles);
    }

    #[test]
    fn a_spread_that_does_not_stay_disappears_when_its_walk_ends() {
        let t = timeline(&DIFFUSION.replace("at_s: 1.0)", "at_s: 1.0, stay: false), Wait(for_s: 9.0)"));
        assert_eq!(t.pose_at(3.0).particles.len(), 1);
        assert_eq!(t.pose_at(6.0).particles.len(), 1, "still there at the end of the walk");
        assert!(t.pose_at(6.5).particles.is_empty());
        // The default is unchanged: it rests where it ended.
        let stays = timeline(&DIFFUSION.replace("at_s: 1.0)", "at_s: 1.0), Wait(for_s: 9.0)"));
        assert_eq!(stays.pose_at(9.0).particles.len(), 1);
    }

    #[test]
    fn a_flow_is_shown_only_while_it_runs() {
        let t = timeline(
            "Flow(id: \"sap\", count: 10, path: [(0.0, 0.0, 0.0), (0.0, 4.0, 0.0)], speed: 1.0, lateral: 0.1, \
             seed: 1, particle_radius: 0.05, colour: (0.2, 0.4, 1.0), for_s: 3.0, at_s: 1.0), Wait(for_s: 6.0)",
        );
        assert!(t.pose_at(0.9).particles.is_empty());
        assert_eq!(t.pose_at(2.0).particles[0].positions.len(), 10);
        assert!(t.pose_at(4.5).particles.is_empty());
        assert_eq!(t.pose_at(2.0).particles, t.pose_at(2.0).particles);
    }

    #[test]
    fn a_flow_with_a_corner_radius_rounds_its_bends() {
        let flow = |extra: &str| {
            timeline(&format!(
                "Flow(id: \"f\", count: 60, path: [(0.0, 0.0, 0.0), (10.0, 0.0, 0.0), (10.0, 10.0, 0.0)]{extra},                  speed: 1.0, lateral: 0.0, seed: 3, particle_radius: 0.05, colour: (0.2, 0.4, 1.0), for_s: 20.0)"
            ))
        };
        let nearest_to_corner = |t: &crate::structure_animation::Timeline| {
            (0..200)
                .flat_map(|k| t.pose_at(k as f64 * 0.09).particles[0].positions.clone())
                .map(|p| ((p[0] - 10.0).powi(2) + p[1].powi(2) + p[2].powi(2)).sqrt())
                .fold(f64::INFINITY, f64::min)
        };
        // A radius of 2 at a right angle cuts the corner off by 2 * (sqrt(2) - 1) = 0.828.
        assert!(nearest_to_corner(&flow("")) < 0.05, "the sharp path passes through the corner");
        assert!(nearest_to_corner(&flow(", corner_radius: Some(2.0)")) > 0.8);
        assert!(invalid("Flow(id: \"f\", count: 3, path: [(0.0, 0.0, 0.0), (5.0, 0.0, 0.0), (5.0, 1.0, 0.0)], corner_radius: Some(3.0), speed: 1.0, lateral: 0.1, seed: 1, particle_radius: 0.05, colour: (0.2, 0.4, 1.0), for_s: 1.0)")
            .contains("straight run"));
        assert!(invalid("Flow(id: \"f\", count: 3, path: [(0.0, 0.0, 0.0), (5.0, 0.0, 0.0)], point_spacing: Some(0.5), speed: 1.0, lateral: 0.1, seed: 1, particle_radius: 0.05, colour: (0.2, 0.4, 1.0), for_s: 1.0)")
            .contains("needs a corner_radius"));
    }

    #[test]
    fn bad_particle_actions_are_refused_with_a_reason() {
        let spread = |extra: &str| {
            format!(
                "Spread(id: \"d\", count: 5, from: (0.0, 0.5, 0.0), radius: 0.2, \
                 inside: Cylinder(base: (0.0, 0.0, 0.0), radius: 1.0, height: 2.0), step: 0.1, seed: 1, \
                 particle_radius: 0.05, colour: (1.0, 0.0, 0.0), for_s: 2.0{extra})"
            )
        };
        assert!(invalid(&spread(", at_s: -1.0")).contains("not negative"));
        assert!(invalid(&spread("").replace("count: 5", "count: 0")).contains("count 0"));
        assert!(invalid(&spread("").replace("from: (0.0, 0.5, 0.0)", "from: (9.0, 0.5, 0.0)")).contains("inside the container"));
        assert!(invalid(&spread("").replace("colour: (1.0, 0.0, 0.0)", "colour: (2.0, 0.0, 0.0)")).contains("0 to 1"));
        assert!(invalid(&spread("").replace("particle_radius: 0.05", "particle_radius: 0.0")).contains("radius"));
        assert!(invalid(&spread("").replace("radius: 1.0, height", "radius: -1.0, height")).contains("container radius"));
        let twice = format!("{}, {}", spread(""), spread(""));
        assert!(invalid(&twice).contains("unique"));
        assert!(invalid(
            "Flow(id: \"f\", count: 3, path: [(0.0, 0.0, 0.0)], speed: 1.0, lateral: 0.1, seed: 1, \
             particle_radius: 0.05, colour: (0.0, 0.0, 1.0), for_s: 1.0)"
        )
        .contains("path"));
    }

    #[test]
    fn a_fade_runs_from_the_authored_opacity_and_chains() {
        let t = timeline(
            "Fade(part: \"b\", to: 0.2, for_s: 2.0), Fade(part: \"b\", to: 1.0, for_s: 2.0, at_s: 3.0)",
        );
        let b = 2;
        assert_eq!(t.pose_at(0.0).parts[b].opacity, 1.0);
        assert!((t.pose_at(1.0).parts[b].opacity - 0.6).abs() < 1e-12);
        assert_eq!(t.pose_at(2.5).parts[b].opacity, 0.2);
        assert!((t.pose_at(4.0).parts[b].opacity - 0.6).abs() < 1e-12);
        assert_eq!(t.pose_at(5.0).parts[b].opacity, 1.0);
        // Other parts are untouched.
        assert_eq!(t.pose_at(1.0).parts[1].opacity, 1.0);
    }

    #[test]
    fn a_fade_reaches_the_descendants_of_a_group_and_starts_from_each_authored_opacity() {
        let text = ", timelines: [Timeline(id: \"t\", name: \"T\", scenes: [Scene(id: \"s\", beats: [Beat(id: \"b\", \
                    actions: [Fade(part: \"root\", to: 0.0, for_s: 1.0)])])])]";
        let s = structure(text).unwrap();
        let pose = s.timelines()[0].pose_at(1.0);
        assert!(pose.parts.iter().all(|p| p.opacity == 0.0));
        assert!((s.timelines()[0].pose_at(0.5).parts[3].opacity - 0.5).abs() < 1e-12);
    }

    #[test]
    fn a_later_fade_grow_or_colour_starts_where_the_previous_one_ended() {
        let t = timeline(
            "Fade(part: \"b\", to: 0.2, for_s: 1.0), Grow(part: \"b\", to: 0.5, for_s: 1.0), \
             Colour(part: \"b\", to: (1.0, 0.0, 0.0), for_s: 1.0), \
             Fade(part: \"b\", to: 1.0, for_s: 2.0, at_s: 2.0), Grow(part: \"b\", to: 1.5, for_s: 2.0, at_s: 2.0), \
             Colour(part: \"b\", to: (0.0, 0.0, 1.0), for_s: 2.0, at_s: 2.0)",
        );
        let b = 2; // root, a, b, c
        // Between the two: held at the first one's end.
        let held = &t.pose_at(1.5).parts[b];
        assert!((held.opacity - 0.2).abs() < 1e-12 && (held.scale - 0.5).abs() < 1e-12);
        assert_eq!(held.colour, Some([1.0, 0.0, 0.0]));
        // Half-way through the second one: half-way from the first one's end, with no jump back.
        let mid = &t.pose_at(3.0).parts[b];
        assert!((mid.opacity - 0.6).abs() < 1e-12);
        assert!((mid.scale - 1.0).abs() < 1e-12);
        assert_eq!(mid.colour, Some([0.5, 0.0, 0.5]));
        let end = &t.pose_at(4.0).parts[b];
        assert!((end.opacity - 1.0).abs() < 1e-12 && (end.scale - 1.5).abs() < 1e-12);
    }

    #[test]
    fn bad_and_overlapping_fades_are_refused() {
        assert!(invalid("Fade(part: \"a\", to: 1.5, for_s: 1.0)").contains("0 to 1"));
        assert!(invalid("Fade(part: \"a\", to: -0.1, for_s: 1.0)").contains("0 to 1"));
        assert!(invalid("Fade(part: \"a\", to: 0.5, for_s: 2.0), Fade(part: \"a\", to: 0.2, for_s: 1.0, at_s: 1.0)")
            .contains("overlap"));
    }

    #[test]
    fn a_structure_without_timelines_loads_unchanged() {
        assert!(structure("").unwrap().timelines().is_empty());
    }

    #[test]
    fn a_move_is_a_pure_function_of_time() {
        let t = timeline("Move(part: \"b\", by: (10.0, 0.0, 0.0), for_s: 4.0)");
        assert_eq!(t.duration(), 4.0);
        let b = 2; // root, a, b, c
        assert_eq!(t.pose_at(0.0).parts[b].offset, [0.0; 3]);
        assert_eq!(t.pose_at(1.0).parts[b].offset, [2.5, 0.0, 0.0]);
        assert_eq!(t.pose_at(4.0).parts[b].offset, [10.0, 0.0, 0.0]);
        // Clamped outside the timeline, and the same answer however often it is asked.
        assert_eq!(t.pose_at(99.0), t.pose_at(4.0));
        assert_eq!(t.pose_at(-3.0), t.pose_at(0.0));
        assert_eq!(t.pose_at(f64::NAN), t.pose_at(0.0));
        assert_eq!(t.pose_at(1.0), t.pose_at(1.0));
    }

    #[test]
    fn smooth_easing_starts_slow_and_meets_the_end() {
        let t = timeline("Move(part: \"b\", by: (8.0, 0.0, 0.0), for_s: 4.0, ease: Smooth)");
        assert!(t.pose_at(1.0).parts[2].offset[0] < 2.0);
        assert!((t.pose_at(2.0).parts[2].offset[0] - 4.0).abs() < 1e-12);
        assert_eq!(t.pose_at(4.0).parts[2].offset[0], 8.0);
    }

    #[test]
    fn a_group_stands_for_its_descendants_in_moves_grows_and_turns() {
        let t = timeline("Move(part: \"root\", by: (0.0, 1.0, 0.0), for_s: 1.0)");
        let p = t.pose_at(1.0);
        assert!(p.parts.iter().all(|part| part.offset == [0.0, 1.0, 0.0]));
        // Grow on the group scales every part with geometry in it, as one body about the group.
        let t = timeline("Grow(part: \"root\", to: 2.0, for_s: 1.0)");
        let p = t.pose_at(1.0);
        assert_eq!(p.parts[0].scale, 1.0);
        assert!([1, 2, 3].iter().all(|&i| p.parts[i].scale == 2.0 && p.parts[i].body == Some(0)));
        assert_eq!(p.bodies, vec![(0, vec![1, 2, 3])]);
        // A part that has children grows with them (it is a group too)...
        let t = timeline("Grow(part: \"a\", to: 2.0, for_s: 1.0)");
        let p = t.pose_at(1.0);
        assert_eq!((p.parts[1].scale, p.parts[3].scale, p.parts[2].scale), (2.0, 2.0, 1.0));
        assert_eq!(p.parts[1].body, Some(1));
        // ...unless `each: true` says every part grows about its own centre.
        let t = timeline("Grow(part: \"a\", to: 2.0, each: true, for_s: 1.0)");
        let p = t.pose_at(1.0);
        assert_eq!((p.parts[1].scale, p.parts[3].scale), (2.0, 2.0));
        assert_eq!((p.parts[1].body, p.parts[3].body), (None, None));
        assert!(p.bodies.is_empty());
    }

    #[test]
    fn one_part_cannot_be_grown_two_ways() {
        let both = invalid("Grow(part: \"root\", to: 2.0, for_s: 1.0), Grow(part: \"b\", to: 3.0, for_s: 1.0, at_s: 2.0)");
        assert!(both.contains("both as part of a group and on its own"), "{both}");
        // Using `each: true` on the group makes the two agree.
        assert!(structure(&one(
            "Grow(part: \"root\", to: 2.0, each: true, for_s: 1.0), Grow(part: \"b\", to: 3.0, for_s: 1.0, at_s: 2.0)"
        ))
        .is_ok());
    }

    #[test]
    fn grow_chains_from_the_previous_size() {
        let t = timeline(
            "Grow(part: \"b\", to: 2.0, for_s: 2.0), Grow(part: \"b\", to: 4.0, for_s: 2.0, at_s: 2.0)",
        );
        assert_eq!(t.pose_at(0.0).parts[2].scale, 1.0);
        assert_eq!(t.pose_at(1.0).parts[2].scale, 1.5);
        assert_eq!(t.pose_at(2.0).parts[2].scale, 2.0);
        assert_eq!(t.pose_at(3.0).parts[2].scale, 3.0);
        assert!(invalid("Grow(part: \"b\", to: 2.0, for_s: 2.0), Grow(part: \"b\", to: 4.0, for_s: 2.0, at_s: 1.0)")
            .contains("overlap"));
    }

    #[test]
    fn the_first_show_or_hide_decides_whether_a_part_starts_shown() {
        let t = timeline("Show(part: \"b\", at_s: 2.0), Wait(for_s: 3.0)");
        assert!(!t.pose_at(1.9).parts[2].shown);
        assert!(t.pose_at(2.0).parts[2].shown);
        assert!(t.pose_at(0.0).parts[1].shown);
        let t = timeline("Hide(part: \"b\", at_s: 2.0), Wait(for_s: 3.0)");
        assert!(t.pose_at(1.9).parts[2].shown);
        assert!(!t.pose_at(2.0).parts[2].shown);
        // Hiding a part hides its descendants too.
        let t = timeline("Hide(part: \"a\", at_s: 1.0)");
        let p = t.pose_at(1.0);
        assert!(!p.parts[1].shown && !p.parts[3].shown && p.parts[2].shown);
    }

    #[test]
    fn turns_keep_their_order_and_start_from_zero() {
        let t = timeline(
            "Turn(part: \"b\", axis: (0.0, 2.0, 0.0), deg: 90.0, for_s: 2.0), \
             Turn(part: \"b\", axis: (1.0, 0.0, 0.0), deg: 40.0, for_s: 2.0, at_s: 2.0)",
        );
        assert!(t.pose_at(0.0).parts[2].turns.is_empty());
        assert_eq!(t.pose_at(1.0).parts[2].turns, vec![([0.0, 1.0, 0.0], 45.0)]);
        assert_eq!(
            t.pose_at(3.0).parts[2].turns,
            vec![([0.0, 1.0, 0.0], 90.0), ([1.0, 0.0, 0.0], 20.0)]
        );
        assert!(invalid("Turn(part: \"b\", axis: (0.0, 0.0, 0.0), deg: 10.0, for_s: 1.0)").contains("non-zero axis"));
    }

    #[test]
    fn colour_starts_from_the_authored_colour() {
        let t = timeline("Colour(part: \"b\", to: (1.0, 0.0, 0.0), for_s: 2.0)");
        assert_eq!(t.pose_at(0.0).parts[2].colour, Some([0.5, 0.5, 0.5]));
        assert_eq!(t.pose_at(1.0).parts[2].colour, Some([0.75, 0.25, 0.25]));
        assert_eq!(t.pose_at(0.0).parts[1].colour, None);
        assert!(invalid("Colour(part: \"b\", to: (2.0, 0.0, 0.0), for_s: 1.0)").contains("0 to 1"));
    }

    #[test]
    fn the_camera_moves_from_the_start_camera() {
        let t = timeline("Camera(yaw_deg: 90.0, pitch_deg: 20.0, zoom: 3.0, for_s: 2.0)");
        let at = |s: f64| t.pose_at(s).camera.unwrap();
        assert_eq!((at(0.0).yaw_deg, at(0.0).zoom), (10.0, 1.0));
        assert_eq!((at(1.0).yaw_deg, at(1.0).zoom), (50.0, 2.0));
        assert_eq!((at(2.0).yaw_deg, at(2.0).zoom), (90.0, 3.0));
        // Without a start camera, a Camera action is refused.
        let no_start = ", timelines: [Timeline(id: \"t\", name: \"T\", scenes: [Scene(id: \"s\", beats: \
             [Beat(id: \"b\", actions: [Camera(yaw_deg: 1.0, pitch_deg: 1.0, zoom: 1.0, for_s: 1.0)])])])]";
        assert!(matches!(structure(no_start).unwrap_err(), StructureError::InvalidTimeline { .. }));
    }

    #[test]
    fn captions_show_during_their_time_and_the_newest_wins() {
        let t = timeline(
            "Narrate(text: \"first\", for_s: 4.0), Narrate(text: \"second\", for_s: 1.0, at_s: 2.0)",
        );
        assert_eq!(t.pose_at(1.0).caption.as_deref(), Some("first"));
        assert_eq!(t.pose_at(2.5).caption.as_deref(), Some("second"));
        assert_eq!(t.pose_at(3.5).caption.as_deref(), Some("first"));
        assert_eq!(t.pose_at(4.0).caption, None);
        assert!(invalid("Narrate(text: \"\", for_s: 1.0)").contains("caption"));
        assert!(invalid("Narrate(text: \"x\", for_s: 0.0)").contains("caption"));
    }

    #[test]
    fn a_highlight_lasts_until_the_end_of_its_beat() {
        let t = timeline("Highlight(part: \"b\", at_s: 1.0), Wait(for_s: 3.0)");
        assert!(!t.pose_at(0.5).parts[2].highlighted);
        assert!(t.pose_at(1.0).parts[2].highlighted);
        assert!(t.pose_at(2.9).parts[2].highlighted);
        assert!(!t.pose_at(3.0).parts[2].highlighted);
    }

    #[test]
    fn beats_play_one_after_another_and_scenes_after_scenes() {
        let s = structure(
            ", timelines: [Timeline(id: \"t\", name: \"T\", scenes: [\
             Scene(id: \"one\", beats: [\
               Beat(id: \"x\", actions: [Wait(for_s: 2.0)]), \
               Beat(id: \"y\", actions: [Move(part: \"b\", by: (1.0, 0.0, 0.0), for_s: 1.0, at_s: 0.5)])]), \
             Scene(id: \"two\", beats: [Beat(id: \"z\", actions: [Wait(for_s: 1.0)])])])]",
        )
        .unwrap();
        let t = &s.timelines()[0];
        let spans: Vec<(&str, f64, f64)> = t.beats().iter().map(|b| (b.beat.as_str(), b.start, b.end)).collect();
        assert_eq!(spans, vec![("x", 0.0, 2.0), ("y", 2.0, 3.5), ("z", 3.5, 4.5)]);
        assert_eq!(t.duration(), 4.5);
        // The move starts half a second into its beat, so at 2.5 it has just begun.
        assert_eq!(t.pose_at(2.5).parts[2].offset, [0.0; 3]);
        assert_eq!(t.pose_at(3.0).parts[2].offset, [0.5, 0.0, 0.0]);
    }

    #[test]
    fn bad_timelines_are_refused_with_a_reason() {
        assert!(matches!(
            structure(&one("Move(part: \"zzz\", by: (1.0, 0.0, 0.0), for_s: 1.0)")).unwrap_err(),
            StructureError::UnknownTimelinePart { .. }
        ));
        assert!(invalid("Move(part: \"b\", by: (1.0, 0.0, 0.0), for_s: -1.0)").contains("not negative"));
        assert!(invalid("Grow(part: \"b\", to: 0.0, for_s: 1.0)").contains("positive"));
        assert!(matches!(
            structure(", timelines: [Timeline(id: \"\", name: \"T\", scenes: [])]").unwrap_err(),
            StructureError::EmptyTimelineId { .. }
        ));
        assert!(matches!(
            structure(&format!(
                ", timelines: [Timeline(id: \"t\", name: \"1\", scenes: [Scene(id: \"s\", beats: [Beat(id: \"b\",                  actions: [Wait(for_s: 1.0)])])]), Timeline(id: \"t\", name: \"2\", scenes: [])]"
            ))
            .unwrap_err(),
            StructureError::DuplicateTimelineId(_)
        ));
        assert!(matches!(
            structure(", timelines: [Timeline(id: \"t\", name: \"T\", scenes: [])]").unwrap_err(),
            StructureError::InvalidTimeline { .. }
        ));
        // Unknown fields are caught, as for views.
        assert!(matches!(
            structure(&one("Wait(for_s: 1.0, sooner: 2.0)")).unwrap_err(),
            StructureError::Parse(_)
        ));
    }
}
