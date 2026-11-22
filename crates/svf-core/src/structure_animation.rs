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

use crate::structure::{Part, StructureError};
use crate::structure_lesson::ViewCamera;

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
    /// Scale a part, about its own centre, to `to` times its authored size.
    Grow {
        part: String,
        to: f64,
        for_s: f64,
        #[serde(default)]
        at_s: f64,
        #[serde(default)]
        ease: Ease,
    },
    /// Rotate a part, about its own centre, by `deg` about `axis`.
    Turn {
        part: String,
        axis: (f64, f64, f64),
        deg: f64,
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
    turns: Vec<(Span, [f64; 3], f64)>,
    /// `(span, from, to)`, non-overlapping, ascending.
    colours: Vec<(Span, [f64; 3], [f64; 3])>,
    /// `[from, to)` ranges.
    highlights: Vec<(f64, f64)>,
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
            highlights: Vec::new(),
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
    /// `None` keeps the part's authored colour.
    pub colour: Option<[f64; 3]>,
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
}

fn lerp(a: f64, b: f64, p: f64) -> f64 {
    a + (b - a) * p
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
        Pose { time: t, parts, camera, caption: caption.map(|(_, text)| text.clone()) }
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
    for (span, axis, degrees) in &track.turns {
        let p = span.progress(t);
        if p > 0.0 {
            turns.push((*axis, degrees * p));
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

    let highlighted = track.highlights.iter().any(|&(from, to)| t >= from && t < to);
    PartPose { shown, offset, scale, turns, colour, highlighted }
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
    let with_geometry = |part_id: &str, what: &str| -> Result<usize, StructureError> {
        let index = root_of(part_id)?;
        if parts[index].geometry().is_none() {
            return Err(bad(format!(
                "{what} needs a part with geometry; '{part_id}' is a grouping part"
            )));
        }
        Ok(index)
    };

    let mut tracks: Vec<PartTrack> = (0..parts.len()).map(|_| PartTrack::new()).collect();
    let mut cameras: Vec<(Span, ViewCamera)> = Vec::new();
    let mut captions: Vec<(f64, f64, String)> = Vec::new();
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
                    ActionDocument::Grow { part, to, for_s, at_s, ease } => {
                        let mut span = timing(*at_s, *for_s)?;
                        span.ease = *ease;
                        if !positive_finite(*to) {
                            return Err(bad(format!("beat '{}': a size must be finite and positive", beat.id)));
                        }
                        let i = with_geometry(part, "Grow")?;
                        tracks[i].scales.push((span, 1.0, *to));
                        length = length.max(at_s + for_s);
                    }
                    ActionDocument::Turn { part, axis, deg, for_s, at_s, ease } => {
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
                        let i = with_geometry(part, "Turn")?;
                        tracks[i].turns.push((span, [axis.0 / norm, axis.1 / norm, axis.2 / norm], *deg));
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
    })
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
    fn a_group_stands_for_its_descendants_in_moves_but_not_in_grow() {
        let t = timeline("Move(part: \"root\", by: (0.0, 1.0, 0.0), for_s: 1.0)");
        let p = t.pose_at(1.0);
        assert!(p.parts.iter().all(|part| part.offset == [0.0, 1.0, 0.0]));
        assert!(invalid("Grow(part: \"root\", to: 2.0, for_s: 1.0)").contains("grouping part"));
        // A part that has children still grows alone.
        let t = timeline("Grow(part: \"a\", to: 2.0, for_s: 1.0)");
        let p = t.pose_at(1.0);
        assert_eq!(p.parts[1].scale, 2.0);
        assert_eq!(p.parts[3].scale, 1.0);
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
