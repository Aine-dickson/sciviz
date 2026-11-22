//! The salt (NaCl) lattice and dissolving fixture (docs/animation-design.md section 25): the lattice is rock salt with the
//! stated spacing and ionic radii, every ion has six oppositely charged neighbours, ions are conserved, the freed ions end
//! with six correctly turned water molecules at the stated distances, and no two balls overlap at any sampled time.

use std::collections::HashMap;

use svf_core::structure::{Geometry, Structure};

const SALT: &str = include_str!("../../../fixtures/structures/salt-dissolving.ron");
const NA_CL: f64 = 2.82;
const FREED: [&str; 4] = ["na-330", "cl-333", "na-033", "cl-030"];

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
    Structure::from_ron_str(SALT).expect("salt-dissolving.ron parses and validates")
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Every ball: (part index, id, authored centre, authored radius).
fn balls(s: &Structure) -> Vec<(usize, String, [f64; 3], f64)> {
    s.parts()
        .iter()
        .enumerate()
        .filter_map(|(i, p)| match p.geometry() {
            Some(Geometry::Ellipsoid { centre, radii, .. }) => {
                assert!(radii[0] == radii[1] && radii[1] == radii[2], "{} is a ball", p.id());
                Some((i, p.id().to_string(), *centre, radii[0]))
            }
            _ => None,
        })
        .collect()
}

fn centre_at(s: &Structure, id: &str, t: f64) -> [f64; 3] {
    let i = s.part_index(id).unwrap();
    let Some(Geometry::Ellipsoid { centre, .. }) = s.parts()[i].geometry() else { panic!("{id} is not a ball") };
    let o = s.timelines()[0].pose_at(t).parts[i].offset;
    [centre[0] + o[0], centre[1] + o[1], centre[2] + o[2]]
}

fn ions(s: &Structure) -> Vec<(usize, String, [f64; 3], f64)> {
    balls(s).into_iter().filter(|b| b.1.starts_with("na-") || b.1.starts_with("cl-")).collect()
}

#[test]
fn the_crystal_is_rock_salt_with_the_stated_spacing_and_radii() {
    let s = load();
    assert!(!s.reference().is_empty());
    assert_closed(s.build_geometry().unwrap().mesh().indices());
    let ions = ions(&s);
    let na = ions.iter().filter(|b| b.1.starts_with("na-")).count();
    assert_eq!((na, ions.len() - na), (32, 32), "one sodium ion for each chloride ion");
    for (_, id, c, r) in &ions {
        let expected = if id.starts_with("na-") { 1.02 } else { 1.81 };
        assert_eq!(*r, expected, "{id} radius");
        // Nearest neighbours are 2.82 away and always the other kind; inner ions have exactly six.
        let near: Vec<&String> = ions.iter().filter(|o| (dist(*c, o.2) - NA_CL).abs() < 1e-6).map(|o| &o.1).collect();
        assert!(near.iter().all(|o| o[..2] != id[..2]), "{id} has a neighbour of its own kind");
        assert!(ions.iter().filter(|o| o.1 != *id).all(|o| dist(*c, o.2) > NA_CL - 1e-6), "{id} too close");
        let inner = id[3..].chars().all(|d| d == '1' || d == '2');
        if inner {
            assert_eq!(near.len(), 6, "{id} has six neighbours");
        }
    }
}

#[test]
fn the_beats_show_the_lattice_then_the_neighbours_then_the_water() {
    let s = load();
    let t = &s.timelines()[0];
    let starts: Vec<f64> = t.beats().iter().map(|b| b.start).collect();
    assert_eq!(starts, [0.0, 10.0, 19.0, 30.0, 38.0, 50.0, 60.0, 70.0, 78.0]);
    let pose = |time: f64, id: &str| t.pose_at(time).parts[s.part_index(id).unwrap()].clone();

    // At first, real relative sizes and no grid or water; then everything is drawn at 0.4.
    assert_eq!(pose(5.0, "na-000").scale, 1.0);
    assert!(!pose(5.0, "grid-x0").shown && !pose(5.0, "w-na-330-px-o").shown);
    for (_, id, _, _) in ions(&s) {
        assert!((pose(14.0, &id).scale - 0.4).abs() < 1e-12, "{id}");
        assert!((pose(t.duration(), &id).scale - 0.4).abs() < 1e-12, "{id}");
    }
    assert!(pose(14.0, "grid-x0").shown);
    // The six neighbours of na-112 are marked and the rest fade; then they come back.
    for id in ["na-112", "cl-012", "cl-212", "cl-102", "cl-122", "cl-111", "cl-113"] {
        assert!(pose(25.0, id).highlighted && pose(25.0, id).opacity == 1.0, "{id}");
    }
    assert!(pose(25.0, "na-000").opacity < 0.2 && !pose(25.0, "na-000").highlighted);
    assert_eq!(pose(33.0, "na-000").opacity, 1.0);
    // The grid has gone before the water arrives.
    assert_eq!(pose(40.0, "grid-x0").opacity, 0.0);
    assert!(pose(40.0, "w-na-330-px-o").shown && !pose(40.0, "w-na-330-nx-o").shown);
}

#[test]
fn ions_are_conserved_and_freed_ions_end_with_six_turned_water_molecules() {
    let s = load();
    let t = &s.timelines()[0];
    let end = t.duration();
    for time in [0.0, 30.0, 55.0, end] {
        let pose = t.pose_at(time);
        for (i, id, _, _) in ions(&s) {
            assert!(pose.parts[i].shown && pose.parts[i].opacity > 0.05, "ion {id} at {time}");
        }
    }
    for id in FREED {
        let ion = centre_at(&s, id, end);
        let na = id.starts_with("na-");
        let waters: Vec<String> = balls(&s)
            .into_iter()
            .filter(|b| b.1.starts_with(&format!("w-{id}-")) && b.1.ends_with("-o"))
            .map(|b| b.1.trim_end_matches("-o").to_string())
            .collect();
        assert_eq!(waters.len(), 6, "{id}");
        let mut directions = Vec::new();
        for w in &waters {
            let (o, h1, h2) = (centre_at(&s, &format!("{w}-o"), end), centre_at(&s, &format!("{w}-h1"), end), centre_at(&s, &format!("{w}-h2"), end));
            let r = dist(o, ion);
            assert!((r - if na { 2.4 } else { 3.2 }).abs() < 1e-6, "{w}: ion to oxygen {r}");
            assert!((dist(o, h1) - 0.96).abs() < 1e-6 && (dist(o, h2) - 0.96).abs() < 1e-6, "{w}: O-H");
            let (u, v) = (sub(h1, o), sub(h2, o));
            let cos = (u[0] * v[0] + u[1] * v[1] + u[2] * v[2]) / (0.96 * 0.96);
            assert!((cos.acos().to_degrees() - 104.5).abs() < 1e-4, "{w}: H-O-H");
            // Oxygen ends face Na+; hydrogen ends face Cl-.
            for h in [h1, h2] {
                assert_eq!(dist(h, ion) > r, na, "{w} is turned the wrong way");
            }
            let d = sub(o, ion);
            directions.push([(d[0] / r).round() as i32, (d[1] / r).round() as i32, (d[2] / r).round() as i32]);
        }
        directions.sort();
        let mut axes = vec![[-1, 0, 0], [1, 0, 0], [0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1]];
        axes.sort();
        assert_eq!(directions, axes, "{id}: one water molecule on each side");
        // Away from the crystal: farther from every remaining ion than a neighbour would be.
        for (_, other, c, _) in ions(&s) {
            if !FREED.contains(&other.as_str()) {
                assert!(dist(ion, c) > 2.0 * NA_CL, "{id} is still next to {other}");
            }
        }
    }
}

#[test]
fn no_two_balls_overlap_at_any_sampled_time() {
    let s = load();
    let t = &s.timelines()[0];
    let balls = balls(&s);
    // Atoms of one water molecule overlap on purpose (they share a parent).
    let molecule = |i: usize| s.parts()[i].parent().filter(|&p| s.parts()[p].id().starts_with("w-"));
    let steps = (t.duration() * 10.0).round() as u32;
    for step in 0..=steps {
        let time = f64::from(step) * 0.1;
        let pose = t.pose_at(time);
        let drawn: Vec<(usize, &str, [f64; 3], f64)> = balls
            .iter()
            .filter(|b| pose.parts[b.0].shown && pose.parts[b.0].opacity >= 0.005)
            .map(|b| {
                let p = &pose.parts[b.0];
                (b.0, b.1.as_str(), [b.2[0] + p.offset[0], b.2[1] + p.offset[1], b.2[2] + p.offset[2]], b.3 * p.scale)
            })
            .collect();
        for (k, a) in drawn.iter().enumerate() {
            for b in &drawn[k + 1..] {
                if molecule(a.0).is_some() && molecule(a.0) == molecule(b.0) {
                    continue;
                }
                // Na+ and Cl- touch in the real crystal (1.02 + 1.81 = 2.83 against 2.82).
                let d = dist(a.2, b.2);
                assert!(d >= a.3 + b.3 - 0.011, "{} and {} overlap at {time}: {d} < {}", a.1, b.1, a.3 + b.3);
            }
        }
    }
}

#[test]
fn every_drawn_frame_is_a_closed_mesh() {
    let s = load();
    let t = &s.timelines()[0];
    let steps = (t.duration() / 2.0).ceil() as u32;
    for step in 0..=steps {
        let pose = t.pose_at(f64::from(step) * 2.0);
        let visible: Vec<usize> =
            (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        let drawn = s.build_geometry_for(&visible).unwrap().apply_pose(&pose).with_particles(&pose.particles);
        assert_closed(drawn.mesh().indices());
    }
}
