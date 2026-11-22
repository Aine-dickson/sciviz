//! The digestive-system fixture (docs/animation-design.md): the organs are closed bodies, the timeline follows one
//! bite of food organ by organ, and what is shown in each organ stays inside it: chewing in the mouth, churning in
//! the stomach, a stream along each tube, nutrients leaving the small intestine, water leaving the large one.

use std::collections::HashMap;

use svf_core::structure::Structure;

const DIGESTIVE: &str = include_str!("../../../fixtures/structures/digestive-system.ron");

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
    Structure::from_ron_str(DIGESTIVE).expect("digestive-system.ron parses and validates")
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn the_organs_are_closed_translucent_bodies() {
    let s = load();
    assert!(!s.reference().is_empty());
    for id in ["mouth", "oesophagus", "stomach", "liver", "small-intestine", "large-intestine"] {
        let part = &s.parts()[s.part_index(id).unwrap_or_else(|| panic!("missing {id}"))];
        assert!(part.geometry().is_some() && part.opacity() < 1.0, "{id}");
    }
    assert_eq!(s.layers().len(), 5);
    assert_eq!(s.views().len(), 3);
    assert_closed(s.build_geometry().unwrap().mesh().indices());
}

fn groups(s: &Structure, time: f64, prefix: &str) -> Vec<svf_core::structure_particles::ParticleGroup> {
    s.timelines()[0].pose_at(time).particles.into_iter().filter(|g| g.id.starts_with(prefix)).collect()
}

#[test]
fn each_organ_is_shown_in_turn_and_the_food_stays_inside_it() {
    let s = load();
    let t = &s.timelines()[0];
    let starts: Vec<f64> = t.beats().iter().map(|b| b.start).collect();
    assert_eq!(starts, [0.0, 8.0, 16.0, 24.0, 34.0, 42.0, 56.0, 68.0]);
    assert_eq!(t.duration(), 75.0);
    assert!(t.pose_at(4.0).particles.is_empty(), "nothing moves in the introduction");

    for step in 0..=16 {
        let time = 8.0 + f64::from(step) * 0.5;
        for g in groups(&s, time, "chewing") {
            assert_eq!(g.positions.len(), 14);
            assert!(g.positions.iter().all(|p| dist(*p, [0.0, 40.5, 0.5]) <= 1.3 + 1e-9));
        }
    }
    assert!(!groups(&s, 12.0, "chewing").is_empty() && groups(&s, 17.0, "chewing").is_empty(), "chewing ends with its beat");

    for step in 0..=20 {
        let time = 24.0 + f64::from(step) * 0.5;
        for g in groups(&s, time, "churning") {
            assert_eq!(g.positions.len(), 30);
            assert!(g.positions.iter().all(|p| dist(*p, [3.0, 25.0, 0.0]) <= 3.0 + 1e-9), "{time}");
        }
    }
    assert!(!groups(&s, 30.0, "churning").is_empty() && groups(&s, 36.0, "churning").is_empty());

    // The gullet: close to the line from the mouth to the stomach.
    let oes = [[0.0, 39.2, 0.0], [0.0, 34.0, 0.0], [0.5, 30.0, 0.0], [1.5, 28.3, 0.0]];
    for g in groups(&s, 20.0, "swallow") {
        for p in &g.positions {
            let near = oes
                .windows(2)
                .map(|w| {
                    let d = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
                    let q = [p[0] - w[0][0], p[1] - w[0][1], p[2] - w[0][2]];
                    let u = ((q[0] * d[0] + q[1] * d[1] + q[2] * d[2]) / (d[0] * d[0] + d[1] * d[1] + d[2] * d[2])).clamp(0.0, 1.0);
                    dist(*p, [w[0][0] + d[0] * u, w[0][1] + d[1] * u, w[0][2] + d[2] * u])
                })
                .fold(f64::INFINITY, f64::min);
            assert!(near <= 0.3 + 1e-9, "{p:?}");
        }
    }
}

#[test]
fn the_intestines_carry_the_food_and_pass_substances_out_through_their_walls() {
    let s = load();
    let t = &s.timelines()[0];

    // Small intestine: a coil in the middle of the abdomen, a little behind the large intestine.
    let chyme = groups(&s, 48.0, "chyme");
    assert_eq!(chyme.len(), 1);
    for p in &chyme[0].positions {
        // The last stretch slopes forward (z up to 1.8) to join the large intestine at the lower left.
        assert!((-7.4..=5.8).contains(&p[0]), "{p:?}");
        assert!((5.8..=20.0).contains(&p[1]) && (-0.6..=1.9).contains(&p[2]), "{p:?}");
    }
    // Nutrients leave backwards (towards -z) from the small intestine.
    let mean_dz = |time: f64, prefix: &str| -> f64 {
        let now = groups(&s, time, prefix);
        let later = t.pose_at(time + 0.05).particles.into_iter().filter(|g| g.id.starts_with(prefix)).collect::<Vec<_>>();
        let (mut total, mut n) = (0.0, 0);
        for (a, b) in now.iter().zip(&later) {
            for (p, q) in a.positions.iter().zip(&b.positions) {
                let dz = q[2] - p[2];
                if dz.abs() < 1.0 {
                    total += dz;
                    n += 1;
                }
            }
        }
        total / f64::from(n)
    };
    assert_eq!(groups(&s, 48.0, "nutrients").len(), 4);
    assert!(groups(&s, 60.0, "nutrients").is_empty());
    assert!(mean_dz(48.0, "nutrients") < 0.0);
    for g in groups(&s, 48.0, "nutrients") {
        assert!(g.positions.iter().all(|p| (-3.4..=-0.1).contains(&p[2])));
    }

    // Large intestine: in front of the small one; water leaves sideways from the ascending colon.
    let waste = groups(&s, 62.0, "leftovers");
    assert_eq!(waste.len(), 1);
    for p in &waste[0].positions {
        assert!((-7.6..=7.6).contains(&p[0]) && (0.4..=18.4).contains(&p[1]) && (1.4..=2.2).contains(&p[2]), "{p:?}");
    }
    let water = groups(&s, 62.0, "water");
    assert_eq!(water.len(), 4);
    for g in &water {
        assert!(g.positions.iter().all(|p| (-9.7..=-7.1).contains(&p[0])));
    }
    let mean_dx = |time: f64| -> f64 {
        let now = groups(&s, time, "water");
        let later = groups(&s, time + 0.05, "water");
        let (mut total, mut n) = (0.0, 0);
        for (a, b) in now.iter().zip(&later) {
            for (p, q) in a.positions.iter().zip(&b.positions) {
                if (q[0] - p[0]).abs() < 1.0 {
                    total += q[0] - p[0];
                    n += 1;
                }
            }
        }
        total / f64::from(n)
    };
    assert!(mean_dx(62.0) < 0.0, "water moves out to the side");
}

#[test]
fn every_drawn_frame_is_a_closed_mesh() {
    let s = load();
    let t = &s.timelines()[0];
    for step in 0..=75 {
        let pose = t.pose_at(f64::from(step));
        let visible: Vec<usize> =
            (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        let drawn = s.build_geometry_for(&visible).unwrap().apply_pose(&pose).with_particles(&pose.particles);
        assert_closed(drawn.mesh().indices());
    }
}
