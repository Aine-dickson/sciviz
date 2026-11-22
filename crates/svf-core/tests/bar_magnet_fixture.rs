//! The bar-magnet fixture (docs/animation-design.md): the field lines, the compass that follows one of them and the
//! compasses placed round the magnet all agree with the field of the two-pole model the file states (north pole at
//! x = +3.6, south pole at x = -3.6). The lines go from the north pole to the south pole, and the timeline
//! shows the magnet, then the compass, then the lines, then the compasses.

use std::collections::HashMap;

use svf_core::structure::{Geometry, Structure};

const MAGNET: &str = include_str!("../../../fixtures/structures/bar-magnet.ron");
const POLE: f64 = 3.6;

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
    Structure::from_ron_str(MAGNET).expect("bar-magnet.ron parses and validates")
}

/// The field of the two-pole model: +1 at the north pole, -1 at the south pole.
fn field(p: [f64; 3]) -> [f64; 3] {
    let mut b = [0.0; 3];
    for (sign, cx) in [(1.0, POLE), (-1.0, -POLE)] {
        let d = [p[0] - cx, p[1], p[2]];
        let r3 = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).powf(1.5);
        for a in 0..3 {
            b[a] += sign * d[a] / r3;
        }
    }
    b
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn tube_path(s: &Structure, id: &str) -> Vec<[f64; 3]> {
    let Some(Geometry::Tube { path, .. }) = s.parts()[s.part_index(id).unwrap()].geometry() else {
        panic!("{id} is not a tube")
    };
    path.clone()
}

#[test]
fn the_parts_are_closed_and_the_poles_are_where_the_field_model_puts_them() {
    let s = load();
    assert!(!s.reference().is_empty());
    for id in ["north-pole", "south-pole", "compass"] {
        assert!(s.part_index(id).is_some(), "{id}");
    }
    assert_eq!((1..=5).filter(|k| s.part_index(&format!("field-line-{k}")).is_some()).count(), 5);
    assert_eq!((1..=8).filter(|k| s.part_index(&format!("needle-{k}")).is_some()).count(), 8);
    // North pole is the red half at +x; south the blue half at -x.
    let colour = |id: &str| s.parts()[s.part_index(id).unwrap()].colour();
    assert!(colour("north-pole")[0] > colour("north-pole")[2] && colour("south-pole")[2] > colour("south-pole")[0]);
    assert!(tube_path(&s, "north-pole").iter().all(|p| p[0] >= 0.0));
    assert!(tube_path(&s, "south-pole").iter().all(|p| p[0] <= 0.0));
    assert_closed(s.build_geometry().unwrap().mesh().indices());
}

#[test]
fn every_field_line_follows_the_field_from_the_north_pole_to_the_south_pole() {
    let s = load();
    for k in 1..=5 {
        let path = tube_path(&s, &format!("field-line-{k}"));
        assert!(path.len() >= 10, "line {k}");
        let (first, last) = (path[0], path[path.len() - 1]);
        // It leaves near the north pole and comes back at the south pole.
        assert!((first[0] - POLE).abs() < 1.6 && first[0] > 0.0, "line {k} starts at {first:?}");
        assert!((last[0] + POLE).abs() < 1.6 && last[0] < 0.0, "line {k} ends at {last:?}");
        // Along the line the direction of travel is the direction of the field.
        let mut worst: f64 = 1.0;
        let mut total = 0.0;
        for w in path.windows(2) {
            let step = unit([w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]]);
            let mid = [(w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0, (w[0][2] + w[1][2]) / 2.0];
            let c = dot(step, unit(field(mid)));
            worst = worst.min(c);
            total += c;
        }
        assert!(worst > 0.9, "line {k}: the line leaves the field direction (cos {worst})");
        assert!(total / (path.len() - 1) as f64 > 0.99, "line {k}");
    }
}

#[test]
fn the_lines_spread_out_away_from_the_magnet() {
    // Line 1 leaves the pole at the shallowest angle and reaches farthest; line 5 stays close to the magnet.
    let s = load();
    let widest = |k: usize| tube_path(&s, &format!("field-line-{k}")).iter().map(|p| p[1].abs()).fold(0.0, f64::max);
    assert!(widest(1) > widest(2) && widest(2) > widest(3) && widest(3) > widest(4) && widest(4) > widest(5));
    assert!(widest(5) < 4.0 && widest(1) > 10.0);
}

#[test]
fn the_compasses_round_the_magnet_point_along_the_field() {
    let s = load();
    for k in 1..=8 {
        let path = tube_path(&s, &format!("needle-{k}"));
        let (tail, tip) = (path[0], path[path.len() - 1]);
        let centre = path[1];
        let along = unit([tip[0] - tail[0], tip[1] - tail[1], tip[2] - tail[2]]);
        assert!(dot(along, unit(field(centre))) > 0.9999, "needle {k}");
    }
}

#[test]
fn the_moving_compass_keeps_pointing_along_the_field_while_it_travels() {
    let s = load();
    let t = &s.timelines()[0];
    let starts: Vec<f64> = t.beats().iter().map(|b| b.start).collect();
    assert_eq!(starts, [0.0, 10.0, 24.0, 34.0, 46.0]);
    assert_eq!(t.duration(), 56.0);

    let path = tube_path(&s, "compass");
    let (tail, tip) = (path[0], path[path.len() - 1]);
    let centre0 = path[1];
    let u0 = unit([tip[0] - tail[0], tip[1] - tail[1], tip[2] - tail[2]]);
    let phi0 = u0[1].atan2(u0[0]).to_degrees();
    let compass = s.part_index("compass").unwrap();

    let mut positions = Vec::new();
    for k in 0..=28 {
        let time = 10.0 + f64::from(k) * 0.5;
        let part = &t.pose_at(time).parts[compass];
        let pos = [centre0[0] + part.offset[0], centre0[1] + part.offset[1], centre0[2] + part.offset[2]];
        let mut turned = 0.0;
        for (axis, deg) in &part.turns {
            assert!(axis[2].abs() > 0.999_999, "turns are about z");
            turned += deg;
        }
        let phi = phi0 + turned;
        let b = unit(field(pos));
        let wanted = b[1].atan2(b[0]).to_degrees();
        let diff = ((phi - wanted + 180.0).rem_euclid(360.0) - 180.0).abs();
        // Exact at the end of each second; between them the needle turns steadily, so it is a little behind or ahead.
        let bound = if k % 2 == 0 { 0.05 } else { 12.0 };
        assert!(diff < bound, "at {time}: the needle is {diff} degrees off the field");
        positions.push(pos);
    }
    // It travels from near the north pole to near the south pole, and always outside the magnet.
    let (a, z) = (positions[0], positions[positions.len() - 1]);
    assert!(a[0] > 3.0 && z[0] < -3.0);
    assert!(positions.iter().all(|p| p[0].abs() > 4.0 || p[1].hypot(p[2]) > 1.0), "the compass never enters the magnet");
}

#[test]
fn the_magnet_then_the_compass_then_the_lines_then_the_compasses_appear() {
    let s = load();
    let t = &s.timelines()[0];
    let shown = |id: &str, time: f64| t.pose_at(time).parts[s.part_index(id).unwrap()].shown;
    for (time, lines, needles) in [(5.0, false, false), (15.0, false, false), (30.0, true, false), (40.0, true, false), (50.0, true, true)] {
        assert!(shown("north-pole", time) && shown("south-pole", time) && shown("compass", time), "{time}");
        assert_eq!(shown("field-line-1", time), lines, "lines at {time}");
        assert_eq!(shown("field-line-5", time), lines, "lines at {time}");
        assert_eq!(shown("needle-1", time), needles, "needles at {time}");
        assert_eq!(shown("needle-8", time), needles, "needles at {time}");
    }
}

#[test]
fn every_drawn_frame_is_a_closed_mesh() {
    let s = load();
    let t = &s.timelines()[0];
    for step in 0..=28 {
        let pose = t.pose_at(f64::from(step) * 2.0);
        let visible: Vec<usize> =
            (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        let drawn = s.build_geometry_for(&visible).unwrap().apply_pose(&pose).with_particles(&pose.particles);
        assert_closed(drawn.mesh().indices());
    }
}
