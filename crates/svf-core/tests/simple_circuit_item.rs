//! The simple circuit as an item document (docs/item-document.md, C3). The circuit kit owns the rule: the
//! electrons flow and the lamp glows if and only if every lever is at its contact. These tests check the rule on
//! the compiled output, at every 0.1 s, for the authored story and for a second circuit with two switches.

use std::collections::HashMap;

use svf_core::item::{compile_str, CompiledItem, ItemError};
use svf_core::structure::Structure;

const ITEM: &str = include_str!("../../../fixtures/items/simple-circuit.item.ron");

fn assert_closed(indices: &[u32]) {
    let mut directed: HashMap<(u32, u32), usize> = HashMap::new();
    for t in indices.chunks_exact(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *directed.entry((a, b)).or_default() += 1;
        }
    }
    for (&(a, b), &n) in &directed {
        assert_eq!(n, 1, "directed edge ({a},{b}) used {n} times");
        assert_eq!(directed.get(&(b, a)), Some(&1), "edge ({a},{b}) has no opposite edge");
    }
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn point(item: &CompiledItem, name: &str) -> [f64; 3] {
    item.names.get(name).unwrap().point.unwrap_or_else(|| panic!("{name} has a point"))
}

/// The posed vertices of one part at `t`.
fn vertices_at(s: &Structure, part: &str, t: f64) -> Vec<[f64; 3]> {
    let pose = s.timelines()[0].pose_at(t);
    let index = s.part_index(part).unwrap();
    let all: Vec<usize> = (0..s.parts().len()).collect();
    let posed = s.build_geometry_for(&all).unwrap().apply_pose(&pose);
    let range = posed.ranges().iter().find(|r| r.part_index == index).expect("part range");
    let indices = &posed.mesh().indices()[range.first_triangle * 3..(range.first_triangle + range.triangle_count) * 3];
    indices.iter().map(|&i| posed.mesh().vertices()[i as usize].to_array()).collect()
}

/// The free end of a lever: the centre of the ring of vertices furthest from the pivot.
fn lever_tip(s: &Structure, lever: &str, pivot: [f64; 3], t: f64) -> [f64; 3] {
    let v = vertices_at(s, lever, t);
    let far = v.iter().map(|p| dist(*p, pivot)).fold(0.0, f64::max);
    let ring: Vec<&[f64; 3]> = v.iter().filter(|p| dist(**p, pivot) > far - 0.02).collect();
    #[allow(clippy::cast_precision_loss)]
    let n = ring.len() as f64;
    [ring.iter().map(|p| p[0]).sum::<f64>() / n, ring.iter().map(|p| p[1]).sum::<f64>() / n, ring.iter().map(|p| p[2]).sum::<f64>() / n]
}

fn touching(item: &CompiledItem, switch: &str, t: f64) -> bool {
    let tip = lever_tip(&item.structure, &format!("{switch}-lever"), point(item, &format!("{switch}.pivot")), t);
    dist(tip, point(item, &format!("{switch}.contact"))) < 0.01
}

fn flowing(s: &Structure, t: f64) -> bool {
    s.timelines()[0].pose_at(t).particles.iter().any(|g| !g.positions.is_empty())
}

fn compile() -> CompiledItem {
    compile_str(ITEM).expect("the circuit item compiles")
}

#[test]
fn the_item_compiles_to_a_structure_file_that_reads_back() {
    let item = compile();
    let text = item.to_ron();
    let back = Structure::from_ron_str(&text).expect("the compiled file parses and validates");
    assert_eq!(back.parts().len(), item.structure.parts().len());
    assert_eq!(back.timelines()[0].duration(), item.structure.timelines()[0].duration());
    assert!(text.starts_with("// Compiled from an item document"));
    assert!(text.contains("// Note: Electrons are drawn much faster"));
    assert_closed(item.structure.build_geometry().unwrap().mesh().indices());
    // Every part with geometry has a fidelity entry, and every part has a description.
    for p in item.structure.parts() {
        assert!(!p.description().is_empty(), "{} has a description", p.id());
        if p.geometry().is_some() {
            assert!(item.fidelity.iter().any(|(id, _)| id == p.id()), "{} has a fidelity entry", p.id());
        }
    }
}

#[test]
fn the_beats_follow_the_caption_pace() {
    let item = compile();
    let t = &item.structure.timelines()[0];
    for (beat, doc) in t.beats().iter().zip(&item.document.timelines[0].scenes[0].beats) {
        let words = match &doc.actions[0] {
            svf_core::structure_animation::ActionDocument::Narrate { text, .. } => text.split_whitespace().count(),
            other => panic!("each beat starts with its caption, not {other:?}"),
        };
        let length = beat.end - beat.start;
        #[allow(clippy::cast_precision_loss)]
        let needed = words as f64 / 2.5;
        assert!(length >= needed - 1e-9 && length < needed.max(3.0) + 0.5 + 1e-9, "beat {} lasts {length} for {words} words", beat.beat);
    }
}

#[test]
fn electrons_flow_and_the_lamp_glows_if_and_only_if_the_lever_is_at_its_contact() {
    let item = compile();
    let s = &item.structure;
    let t = &s.timelines()[0];
    let (filament, glow) = (s.part_index("lamp-filament").unwrap(), s.part_index("lamp-glow").unwrap());
    let mut flowed = 0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = (t.duration() * 10.0).round() as u32;
    for step in 0..=steps {
        let time = f64::from(step) * 0.1;
        let pose = t.pose_at(time);
        let on = flowing(s, time);
        assert_eq!(on, touching(&item, "switch", time), "electrons and lever disagree at {time}");
        // A flow still draws at its end instant (engine convention), when the lever has only started to lift.
        let end_instant = on && !flowing(s, time + 1e-6);
        assert_eq!(pose.parts[glow].shown, on && !end_instant, "the lamp's glow at {time}");
        assert_eq!(pose.key.iter().any(|k| k.text == "Electron"), on, "the electron key at {time}");
        if !on {
            let c = pose.parts[filament].colour.unwrap_or([0.35, 0.30, 0.30]);
            let just_off = !flowing(s, time) && (1..=3).any(|k| flowing(s, time - 0.1 * f64::from(k)));
            assert!(just_off || (c[0] - 0.35).abs() < 1e-9, "the filament is dark at {time}: {c:?}");
        }
        flowed += usize::from(on);
    }
    assert!(flowed > 300, "the current flows for most of the middle of the story ({flowed} samples)");
    // The current starts in the 'close' beat and runs on through the next beats until 'off' opens the switch.
    let beat = |id: &str| t.beats().iter().find(|b| b.beat == id).unwrap().clone();
    assert!(!flowing(s, beat("close").start + 1.4) && flowing(s, beat("close").start + 1.5));
    assert!(flowing(s, beat("same").start + 1.0) && flowing(s, beat("energy").end - 0.1));
    assert!(!flowing(s, beat("off").start + 0.1));
}

#[test]
fn electrons_leave_the_negative_terminal_and_go_once_round_the_loop() {
    let item = compile();
    let s = &item.structure;
    let path = match &item.document.timelines[0].scenes[0].beats.iter().flat_map(|b| &b.actions).find(|a| {
        matches!(a, svf_core::structure_animation::ActionDocument::Flow { .. })
    }) {
        Some(svf_core::structure_animation::ActionDocument::Flow { path, .. }) => path.clone(),
        _ => panic!("the kit emitted a flow"),
    };
    let p = |q: (f64, f64, f64)| [q.0, q.1, q.2];
    let negative = point(&item, "cell.negative");
    assert!(dist(p(path[0]), negative) < 1e-9, "the path starts at the negative terminal");
    assert!(dist(p(path[path.len() - 1]), negative) < 1e-9, "and comes back to it: one closed loop");
    // It passes the switch gap and the filament.
    let (pivot, contact, filament) = (point(&item, "switch.pivot"), point(&item, "switch.contact"), point(&item, "lamp.filament"));
    assert!(path.iter().any(|q| dist(p(*q), pivot) < 1e-9) && path.iter().any(|q| dist(p(*q), contact) < 1e-9));
    assert!(path.iter().any(|q| dist(p(*q), filament) < 0.25));
    // An electron just outside the negative terminal moves away from the cell, into the wire.
    let t = &s.timelines()[0];
    let start = t.beats().iter().find(|b| b.beat == "same").unwrap().start;
    let cell = point(&item, "cell");
    let mut checked = 0;
    for k in 0..40 {
        let time = start + f64::from(k) * 0.25;
        let now = &t.pose_at(time).particles[0].positions;
        let later = &t.pose_at(time + 0.1).particles[0].positions;
        for (a, b) in now.iter().zip(later) {
            if a[0] < negative[0] && a[0] > negative[0] - 1.0 && (a[1] - negative[1]).abs() < 0.3 {
                assert!(dist(*b, cell) > dist(*a, cell), "an electron at {a:?} moves towards the cell");
                checked += 1;
            }
        }
    }
    assert!(checked > 5, "enough electrons were seen leaving ({checked})");
}

#[test]
fn the_cell_and_the_lamp_have_their_stated_real_sizes() {
    let item = compile();
    let s = &item.structure;
    let span = |parts: &[&str], axis: usize| {
        let v: Vec<f64> = parts.iter().flat_map(|p| vertices_at(s, p, 0.0)).map(|q| q[axis]).collect();
        v.iter().copied().fold(f64::MIN, f64::max) - v.iter().copied().fold(f64::MAX, f64::min)
    };
    assert!((span(&["cell-body", "cell-top", "cell-positive"], 0) - 5.05).abs() < 1e-6, "an AA cell is 50.5 mm long");
    assert!((span(&["cell-body"], 1) - 1.45).abs() < 0.01, "and 14.5 mm across");
    assert!((span(&["lamp-glass"], 0) - 1.1).abs() < 0.01, "an MES bulb is 11 mm across");
    assert!((span(&["lamp-cap"], 0) - 1.0).abs() < 0.01, "on a 10 mm cap");
}

#[test]
fn labels_build_up_and_stay_and_every_label_has_an_anchor() {
    let item = compile();
    let s = &item.structure;
    let t = &s.timelines()[0];
    let texts = |time: f64| t.pose_at(time).labels.iter().map(|l| l.text.clone()).collect::<Vec<_>>();
    let first = texts(0.5);
    for want in ["Cell", "Positive terminal (+)", "Negative terminal (−)"] {
        assert!(first.iter().any(|l| l == want), "{want} at the start: {first:?}");
    }
    let end = texts(t.duration());
    for want in ["Cell", "Copper wire", "Switch", "Lamp", "Lever", "Filament"] {
        assert!(end.iter().any(|l| l == want), "{want} at the end: {end:?}");
    }
    let g = s.build_geometry().unwrap();
    let pose = t.pose_at(t.duration());
    for label in &pose.labels {
        assert!(s.label_anchor(&g, label, &pose).is_some(), "{} has an anchor", label.text);
    }
}

#[test]
fn every_drawn_frame_is_a_closed_mesh() {
    let item = compile();
    let s = &item.structure;
    let t = &s.timelines()[0];
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = (t.duration() * 2.0) as u32;
    for step in 0..=steps {
        let pose = t.pose_at(f64::from(step) * 0.5);
        let visible: Vec<usize> =
            (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        let drawn = s.build_geometry_for(&visible).unwrap().apply_pose(&pose).with_particles(&pose.particles);
        assert_closed(drawn.mesh().indices());
    }
}

fn two_switches(story: &str) -> Result<CompiledItem, ItemError> {
    compile_str(&format!(
        "Item(id: \"two-switches\", title: \"Two switches\", subject: Physics, idea: \"Both must be closed.\", reference: \"r\",
          objects: [Cell(id: \"cell\"), Switch(id: \"a\"), Lamp(id: \"lamp\"), Switch(id: \"b\"), Loop([\"cell\", \"a\", \"lamp\", \"b\"])],
          story: [{story}])"
    ))
}

#[test]
fn with_two_switches_in_series_the_current_flows_only_while_both_are_closed() {
    let item = two_switches(
        "Beat(say: \"Close a.\", do: [Close(\"a\")]), Beat(say: \"Close b.\", do: [Close(\"b\")]), \
         Beat(say: \"Open a.\", do: [Open(\"a\")]), Beat(say: \"Close a again.\", do: [Close(\"a\")]), Beat(say: \"End.\")",
    )
    .expect("compiles");
    let s = &item.structure;
    let t = &s.timelines()[0];
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = (t.duration() * 10.0).round() as u32;
    let mut changes = 0;
    let mut before = false;
    for step in 0..=steps {
        let time = f64::from(step) * 0.1;
        let both = touching(&item, "a", time) && touching(&item, "b", time);
        let on = flowing(s, time);
        assert_eq!(on, both, "current and the two levers disagree at {time}");
        changes += usize::from(on != before);
        before = on;
    }
    assert_eq!(changes, 3, "on when b closes, off when a opens, on again when a closes");
    // Starting closed: the current flows from the start.
    let closed = compile_str(
        "Item(id: \"c\", title: \"T\", subject: Physics, idea: \"I\", reference: \"r\",
          objects: [Cell(id: \"cell\"), Switch(id: \"s\", starts: Closed), Lamp(id: \"lamp\"), Loop([\"cell\", \"s\", \"lamp\"])],
          story: [Beat(say: \"On.\"), Beat(say: \"Off.\", do: [Open(\"s\")])])",
    )
    .unwrap();
    assert!(flowing(&closed.structure, 0.0) && !flowing(&closed.structure, 3.5));
}

#[test]
fn misuse_is_an_error_with_a_reason() {
    let err = |story: &str| two_switches(story).unwrap_err().to_string();
    assert!(err("Beat(say: \"x\", do: [Open(\"a\")])").contains("already open"));
    assert!(err("Beat(say: \"x\", do: [Close(\"lamp\")])").contains("not a switch"));
    assert!(err("Beat(say: \"x\", do: [Close(\"a\"), Open(\"a\")])").contains("still moving"));
    let unknown = err("Beat(say: \"x\", point_at: [\"lamp.filement\"])");
    assert!(unknown.contains("unknown name 'lamp.filement'") && unknown.contains("lamp.filament"), "{unknown}");
    let objects = |objects: &str| {
        compile_str(&format!(
            "Item(id: \"x\", title: \"T\", subject: Physics, idea: \"I\", reference: \"r\", objects: [{objects}], story: [Beat(say: \"s\")])"
        ))
        .unwrap_err()
        .to_string()
    };
    assert!(objects("Cell(id: \"cell\"), Lamp(id: \"lamp\"), Loop([\"lamp\", \"cell\"])").contains("start at its one cell"));
    assert!(objects("Cell(id: \"cell\"), Lamp(id: \"lamp\"), Lamp(id: \"spare\"), Loop([\"cell\", \"lamp\"])").contains("not in the loop"));
    assert!(objects("Cell(id: \"Cell\"), Lamp(id: \"lamp\"), Loop([\"Cell\", \"lamp\"])").contains("lowercase"));
    assert!(objects("Cell(id: \"cell\"), Lamp(id: \"lamp\")").contains("exactly one Loop"));
}
