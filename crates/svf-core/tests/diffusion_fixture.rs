//! The diffusion-in-a-beaker fixture (proof of concept, `docs/animation-design.md` section 17):
//! it parses, the dye particles stay inside the water region at every instant, they start as a
//! tight drop and spread out, and every drawn frame (beaker plus particles) is a closed mesh.

use std::collections::HashMap;

use svf_core::structure::Structure;

const BEAKER: &str = include_str!("../../../fixtures/structures/diffusion-beaker.ron");

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

fn spread(positions: &[[f64; 3]]) -> f64 {
    let n = positions.len() as f64;
    let mean = |a: usize| positions.iter().map(|p| p[a]).sum::<f64>() / n;
    let m = [mean(0), mean(1), mean(2)];
    (positions.iter().map(|p| (0..3).map(|a| (p[a] - m[a]).powi(2)).sum::<f64>()).sum::<f64>() / n).sqrt()
}

#[test]
fn the_dye_starts_as_a_drop_stays_in_the_beaker_and_spreads() {
    let s = Structure::from_ron_str(BEAKER).expect("diffusion-beaker.ron parses and validates");
    assert!(!s.reference().is_empty());
    let t = &s.timelines()[0];
    assert_eq!(t.id(), "spreading");
    assert_eq!(t.duration(), 28.0);

    // Before the dye is added there are no particles; then 150.
    assert!(t.pose_at(1.0).particles.is_empty());
    let at = |time: f64| t.pose_at(time).particles.remove(0).positions;
    assert_eq!(at(3.0).len(), 150);

    // The inside of the water: radius 2.9 about the axis, from 0.3 to 7.7 cm.
    for step in 0..=280 {
        let all = at(3.0 + f64::from(step) * 0.0892);
        assert_eq!(all.len(), 150);
        for p in &all {
            assert!(p[0] * p[0] + p[2] * p[2] <= 2.9 * 2.9 + 1e-9 && p[1] >= 0.3 && p[1] <= 7.7, "{p:?}");
        }
    }

    let (early, middle, late) = (spread(&at(3.0)), spread(&at(10.0)), spread(&at(28.0)));
    assert!(early < 0.5, "a tight drop at the start: {early}");
    assert!(early < middle && middle < late + 0.3, "{early} {middle} {late}");
    assert!(late > 3.0 * early, "{early} -> {late}");
    // The drop is added near the top and has not reached the bottom a few seconds later.
    assert!(at(3.0).iter().all(|p| p[1] > 5.5));
}

#[test]
fn every_drawn_frame_is_closed_and_leaves_the_beaker_parts_alone() {
    let s = Structure::from_ron_str(BEAKER).unwrap();
    let t = &s.timelines()[0];
    let beaker = s.build_geometry().unwrap();
    for step in 0..=28 {
        let pose = t.pose_at(f64::from(step));
        let drawn = beaker.apply_pose(&pose).with_particles(&pose.particles);
        assert_closed(drawn.mesh().indices());
        assert_eq!(drawn.ranges(), beaker.ranges());
        let spheres: usize = pose.particles.iter().map(|g| g.positions.len()).sum();
        assert_eq!(drawn.mesh().indices().len(), beaker.mesh().indices().len() + spheres * 20 * 3);
    }
}
