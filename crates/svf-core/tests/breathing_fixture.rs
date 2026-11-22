//! The breathing fixture (docs/animation-design.md): the parts are closed bodies, the timeline moves the
//! diaphragm, ribs and lungs together, goes back to where it started after each breath, and the air
//! particles stay on the airway paths and flow down on breathing in and up on breathing out.

use std::collections::HashMap;

use svf_core::structure::Structure;

const BREATHING: &str = include_str!("../../../fixtures/structures/breathing.ron");

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
    Structure::from_ron_str(BREATHING).expect("breathing.ron parses and validates")
}

#[test]
fn the_parts_exist_and_every_body_is_closed() {
    let s = load();
    assert!(!s.reference().is_empty());
    for id in ["ribs", "sternum", "diaphragm", "lungs", "trachea", "bronchi"] {
        let index = s.part_index(id).unwrap_or_else(|| panic!("missing part {id}"));
        assert!(s.parts()[index].geometry().is_some(), "{id} has no geometry");
    }
    assert!(s.parts()[s.part_index("lungs").unwrap()].opacity() < 1.0, "the lungs are translucent");
    assert_eq!(s.layers().len(), 4);
    assert_eq!(s.views().len(), 3);
    assert_closed(s.build_geometry().unwrap().mesh().indices());
}

#[test]
fn a_breath_in_then_out_moves_the_chest_and_returns_to_the_start() {
    let s = load();
    let t = &s.timelines()[0];
    let starts: Vec<f64> = t.beats().iter().map(|b| b.start).collect();
    assert_eq!(starts, [0.0, 5.0, 12.0, 19.0]);
    assert_eq!(t.duration(), 29.0);
    let part = |id: &str, time: f64| t.pose_at(time).parts[s.part_index(id).unwrap()].clone();

    // At rest nothing has moved.
    for id in ["diaphragm", "ribs", "lungs"] {
        assert_eq!((part(id, 0.0).offset, part(id, 0.0).scale), ([0.0; 3], 1.0), "{id}");
    }
    // End of breathing in: the diaphragm is down, the ribs up and out, the lungs bigger.
    let (d, r, l) = (part("diaphragm", 12.0), part("ribs", 12.0), part("lungs", 12.0));
    assert!((d.offset[1] + 1.8).abs() < 1e-9 && d.scale == 1.0);
    assert!((r.offset[1] - 0.5).abs() < 1e-9 && (r.scale - 1.08).abs() < 1e-9);
    assert!((l.scale - 1.15).abs() < 1e-9 && l.offset[1] < 0.0);
    // After breathing out, and after the second breath, everything is back.
    for time in [19.0, 29.0] {
        for id in ["diaphragm", "ribs", "sternum", "lungs"] {
            let p = part(id, time);
            assert!(p.offset.iter().all(|v| v.abs() < 1e-9), "{id} at {time}: {:?}", p.offset);
            assert!((p.scale - 1.0).abs() < 1e-9, "{id} at {time}");
        }
    }
    // The diaphragm is lowest at the end of each breath in.
    assert!(part("diaphragm", 9.0).offset[1] < part("diaphragm", 6.0).offset[1]);
    assert!(part("diaphragm", 24.0).offset[1] < -1.79 && part("diaphragm", 22.0).offset[1] > -1.79);
}

fn distance_to_path(path: &[[f64; 3]], p: [f64; 3]) -> f64 {
    path.windows(2)
        .map(|w| {
            let d = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
            let q = [p[0] - w[0][0], p[1] - w[0][1], p[2] - w[0][2]];
            let len2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
            let u = ((q[0] * d[0] + q[1] * d[1] + q[2] * d[2]) / len2).clamp(0.0, 1.0);
            let c = [w[0][0] + d[0] * u, w[0][1] + d[1] * u, w[0][2] + d[2] * u];
            ((p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2)).sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn air_follows_the_airways_down_on_breathing_in_and_up_on_breathing_out() {
    let s = load();
    let t = &s.timelines()[0];
    let right = [[0.0, 23.0, 0.0], [0.0, 20.0, 0.0], [0.0, 16.8, 0.0], [1.6, 15.6, 0.0], [3.4, 14.2, 0.0], [3.6, 12.5, 0.0]];
    let left = right.map(|[x, y, z]| [-x, y, z]);

    // No air moves at rest.
    assert!(t.pose_at(2.0).particles.is_empty());

    for (time, going_down) in [(7.0, true), (15.0, false), (21.0, true), (26.0, false)] {
        let now = t.pose_at(time).particles;
        let later = t.pose_at(time + 0.05).particles;
        assert_eq!(now.len(), 2, "one flow for each side at {time}");
        let mut total_dy = 0.0;
        for (a, b) in now.iter().zip(&later) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.positions.len(), 10);
            let path = if a.id.ends_with("right") { &right[..] } else { &left[..] };
            for (p, q) in a.positions.iter().zip(&b.positions) {
                assert!(distance_to_path(path, *p) <= 0.2 + 1e-9, "{} at {time}: {p:?}", a.id);
                let dy = q[1] - p[1];
                // A particle that wraps from the end of the path back to its start jumps by the whole
                // path; leave those out and look at the ones that simply moved.
                if dy.abs() < 1.0 {
                    total_dy += dy;
                }
            }
        }
        // Overall the air goes down (breathing in) or up (breathing out).
        if going_down {
            assert!(total_dy < 0.0, "air should move down at {time}: {total_dy}");
        } else {
            assert!(total_dy > 0.0, "air should move up at {time}: {total_dy}");
        }
    }
}

#[test]
fn every_drawn_frame_is_a_closed_mesh() {
    let s = load();
    let t = &s.timelines()[0];
    for step in 0..=29 {
        let pose = t.pose_at(f64::from(step));
        let visible: Vec<usize> =
            (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        let drawn = s.build_geometry_for(&visible).unwrap().apply_pose(&pose).with_particles(&pose.particles);
        assert_closed(drawn.mesh().indices());
    }
}
