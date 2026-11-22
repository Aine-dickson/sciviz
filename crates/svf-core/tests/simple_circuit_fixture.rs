//! The simple circuit fixture (docs/animation-design.md section 26): the electrons flow only while the switch is closed
//! and the lamp is lit exactly then; their path is one closed loop that passes the switch and the filament; the lever really
//! closes the gap (and keeps its end at the pivot while it turns); every drawn frame is closed.

use std::collections::HashMap;

use svf_core::structure::Structure;

const CIRCUIT: &str = include_str!("../../../fixtures/structures/simple-circuit.ron");
const PIVOT: [f64; 3] = [-10.0, -1.5, 0.0];
const CONTACT: [f64; 3] = [-10.0, 1.5, 0.0];
const CLOSED: (f64, f64) = (22.5, 46.0); // the lever has arrived at 22.5 s and starts to lift at 46 s

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

fn load() -> Structure {
    Structure::from_ron_str(CIRCUIT).expect("simple-circuit.ron parses and validates")
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The posed lever's vertices at `t`.
fn lever_at(s: &Structure, t: f64) -> Vec<[f64; 3]> {
    let pose = s.timelines()[0].pose_at(t);
    let lever = s.part_index("switch-lever").unwrap();
    let all: Vec<usize> = (0..s.parts().len()).collect();
    let posed = s.build_geometry_for(&all).unwrap().apply_pose(&pose);
    let range = posed.ranges().iter().find(|r| r.part_index == lever).expect("lever range");
    let indices = &posed.mesh().indices()[range.first_triangle * 3..(range.first_triangle + range.triangle_count) * 3];
    indices.iter().map(|&i| posed.mesh().vertices()[i as usize].to_array()).collect()
}

#[test]
fn the_parts_are_closed_and_the_beats_are_in_order() {
    let s = load();
    assert!(!s.reference().is_empty());
    assert_closed(s.build_geometry().unwrap().mesh().indices());
    let t = &s.timelines()[0];
    let starts: Vec<f64> = t.beats().iter().map(|b| b.start).collect();
    assert_eq!(starts, [0.0, 12.0, 21.0, 47.5, 55.5]);
    assert_eq!(t.duration(), 65.5);
}

#[test]
fn electrons_flow_and_the_lamp_is_lit_only_while_the_switch_is_closed() {
    let s = load();
    let t = &s.timelines()[0];
    let (filament, glow) = (s.part_index("filament").unwrap(), s.part_index("glow").unwrap());
    for step in 0..=655 {
        let time = f64::from(step) * 0.1;
        let pose = t.pose_at(time);
        let flowing = pose.particles.iter().any(|g| !g.positions.is_empty());
        let lit = pose.parts[glow].shown;
        let closed = time >= CLOSED.0 - 1e-9 && time < CLOSED.1 - 1e-9;
        // A flow still draws at its end instant (46 s), when the lever is only starting to lift (no gap yet).
        assert_eq!(flowing, closed || (time - CLOSED.1).abs() < 1e-9, "electrons at {time}");
        assert_eq!(lit, closed, "lamp at {time}");
        // Glowing filament while lit (after its 0.3 s warm-up), dark when off (after its 0.3 s cool-down).
        let colour = pose.parts[filament].colour.unwrap_or([0.35, 0.30, 0.30]);
        if time >= CLOSED.0 + 0.3 - 1e-9 && time < CLOSED.1 - 1e-9 {
            assert_eq!(colour, [1.0, 0.95, 0.60], "filament at {time}");
        } else if time < CLOSED.0 - 1e-9 || time >= CLOSED.1 + 0.3 - 1e-9 {
            assert!((colour[0] - 0.35).abs() < 1e-9 && (colour[2] - 0.30).abs() < 1e-9, "filament at {time}: {colour:?}");
        }
    }
}

#[test]
fn the_electron_path_is_one_loop_through_the_switch_and_the_filament_inside_the_wires() {
    let s = load();
    let pose = s.timelines()[0].pose_at(30.0);
    let group = pose.particles.iter().find(|g| g.id == "electrons").expect("electrons");
    assert!(group.positions.len() >= 60, "about one electron per centimetre of loop");
    // Every electron is in the plane of the circuit and near one of: the wire loop, the switch, or the lamp.
    for p in &group.positions {
        assert!(p[2].abs() < 0.06, "{p:?} out of the circuit plane");
        let on_loop = (p[0] >= -10.06 && p[0] <= 10.06) && (p[1] >= -6.06 && p[1] <= 8.3);
        assert!(on_loop, "{p:?} outside the circuit");
    }
    // Over one lap every electron passes the switch gap and the filament.
    let t = &s.timelines()[0];
    let (mut through_switch, mut through_filament) = (false, false);
    for step in 0..200 {
        let time = CLOSED.0 + f64::from(step) * 0.1;
        for p in &t.pose_at(time).particles[0].positions {
            through_switch |= (p[0] + 10.0).abs() < 0.1 && p[1].abs() < 0.5;
            through_filament |= p[1] > 7.7 && p[0].abs() < 0.3;
        }
    }
    assert!(through_switch && through_filament);
}

#[test]
fn the_lever_closes_the_gap_and_turns_about_its_pivot() {
    let s = load();
    // Open (start and after it lifts again): its free end is well away from the contact.
    for time in [0.0, 50.0, 65.5] {
        let near = lever_at(&s, time).iter().map(|v| dist(*v, CONTACT)).fold(f64::INFINITY, f64::min);
        assert!(near > 1.0, "lever reaches within {near} of the contact at {time}");
    }
    // Closed: the lever lies along the gap from pivot to contact.
    for time in [CLOSED.0, 35.0, CLOSED.1] {
        for v in lever_at(&s, time) {
            assert!((v[0] + 10.0).abs() < 0.16 && v[1] > -1.66 && v[1] < 1.66, "{v:?} at {time}");
        }
        let near = lever_at(&s, time).iter().map(|v| dist(*v, CONTACT)).fold(f64::INFINITY, f64::min);
        assert!(near < 0.2, "closed lever ends {near} from the contact at {time}");
    }
    // While it turns, its lower end stays at the pivot.
    for step in 0..=15 {
        for start in [21.0, CLOSED.1] {
            let time = start + f64::from(step) * 0.1;
            let near = lever_at(&s, time).iter().map(|v| dist(*v, PIVOT)).fold(f64::INFINITY, f64::min);
            assert!(near < 0.25, "lever end is {near} from the pivot at {time}");
        }
    }
}

#[test]
fn every_drawn_frame_is_a_closed_mesh() {
    let s = load();
    let t = &s.timelines()[0];
    for step in 0..=131 {
        let pose = t.pose_at(f64::from(step) * 0.5);
        let visible: Vec<usize> =
            (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        let drawn = s.build_geometry_for(&visible).unwrap().apply_pose(&pose).with_particles(&pose.particles);
        assert_closed(drawn.mesh().indices());
    }
}

#[test]
fn labels_build_up_with_the_parts_and_the_electron_key_shows_only_with_the_current() {
    let s = load();
    let t = &s.timelines()[0];
    let texts = |time: f64| t.pose_at(time).labels.iter().map(|l| l.text.clone()).collect::<Vec<_>>();
    assert_eq!(texts(0.5), ["Cell"]);
    assert_eq!(texts(11.0).len(), 7, "{:?}", texts(11.0));
    assert_eq!(texts(t.duration()), texts(11.0), "labels stay to the end");
    let g = s.build_geometry().unwrap();
    for label in t.pose_at(30.0).labels {
        assert!(s.label_anchor(&g, &label, &t.pose_at(30.0)).is_some(), "{} has an anchor", label.text);
    }
    for step in 0..=655 {
        let time = f64::from(step) * 0.1;
        let pose = t.pose_at(time);
        let keyed = pose.key.iter().any(|k| k.text == "Electron");
        assert_eq!(keyed, !pose.particles.is_empty(), "electron key at {time}");
    }
}
