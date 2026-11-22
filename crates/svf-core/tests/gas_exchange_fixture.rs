//! The gas-exchange fixture (docs/animation-design.md): an air sac and a capillary against it. Blood cells stay
//! inside the capillary, oxygen goes from the sac into the capillary and carbon dioxide the other way, and
//! every drawn frame is a closed mesh.

use std::collections::HashMap;

use svf_core::structure::Structure;

const GAS: &str = include_str!("../../../fixtures/structures/gas-exchange.ron");

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
    Structure::from_ron_str(GAS).expect("gas-exchange.ron parses and validates")
}

#[test]
fn the_parts_are_closed_translucent_bodies() {
    let s = load();
    assert!(!s.reference().is_empty());
    for id in ["alveolus", "capillary"] {
        let part = &s.parts()[s.part_index(id).unwrap_or_else(|| panic!("missing {id}"))];
        assert!(part.geometry().is_some() && part.opacity() < 1.0, "{id}");
    }
    assert_closed(s.build_geometry().unwrap().mesh().indices());
}

#[test]
fn blood_stays_in_the_capillary_dark_before_the_sac_and_bright_after() {
    let s = load();
    let t = &s.timelines()[0];
    let starts: Vec<f64> = t.beats().iter().map(|b| b.start).collect();
    assert_eq!(starts, [0.0, 7.0, 15.0, 23.0, 31.0]);
    assert_eq!(t.duration(), 39.0);
    assert!(t.pose_at(3.0).particles.is_empty(), "nothing moves while the sac is introduced");

    for step in 0..=60 {
        let time = 7.0 + f64::from(step) * 0.5;
        for group in t.pose_at(time).particles.iter().filter(|g| g.id.starts_with("blood")) {
            assert_eq!(group.positions.len(), 9);
            let (low, high) = if group.id.contains("poor") { (-16.0, -1.5) } else { (1.5, 16.0) };
            for p in &group.positions {
                assert!((p[0] - 12.2).hypot(p[2]) <= 1.1 + 1e-9, "{p:?} is outside the capillary axis");
                assert!(p[1] >= low - 1e-9 && p[1] <= high + 1e-9, "{} at {p:?}", group.id);
            }
        }
    }
    let colour = |id: &str| t.pose_at(10.0).particles.iter().find(|g| g.id == id).unwrap().colour;
    assert!(colour("blood-poor-a")[0] < colour("blood-rich-a")[0], "the blood is darker before the sac");
}

#[test]
fn oxygen_goes_from_the_sac_into_the_capillary_and_carbon_dioxide_comes_out() {
    let s = load();
    let t = &s.timelines()[0];

    // Oxygen starts in the third beat; carbon dioxide in the fourth.
    let ids = |time: f64, prefix: &str| -> Vec<String> {
        t.pose_at(time).particles.iter().filter(|g| g.id.starts_with(prefix)).map(|g| g.id.clone()).collect()
    };
    assert!(ids(10.0, "oxygen").is_empty() && ids(10.0, "co2").is_empty());
    assert_eq!(ids(18.0, "oxygen").len(), 5);
    assert!(ids(18.0, "co2").is_empty());
    assert_eq!(ids(26.0, "oxygen").len(), 5);
    assert_eq!(ids(26.0, "co2").len(), 4);

    let mean_dx = |time: f64, prefix: &str| -> f64 {
        let now = t.pose_at(time).particles;
        let later = t.pose_at(time + 0.05).particles;
        let (mut total, mut n) = (0.0, 0);
        for (a, b) in now.iter().zip(&later).filter(|(a, _)| a.id.starts_with(prefix)) {
            for (p, q) in a.positions.iter().zip(&b.positions) {
                // The sac's inside is x < about 9; the capillary's is x from about 10 to 14.5.
                assert!(p[0] >= 4.0 && p[0] <= 12.2, "{} at {p:?}", a.id);
                let dx = q[0] - p[0];
                if dx.abs() < 1.0 {
                    total += dx;
                    n += 1;
                }
            }
        }
        total / f64::from(n)
    };
    assert!(mean_dx(18.0, "oxygen") > 0.0, "oxygen moves towards the capillary");
    assert!(mean_dx(26.0, "co2") < 0.0, "carbon dioxide moves towards the sac");
    assert!(mean_dx(34.0, "oxygen") > 0.0 && mean_dx(34.0, "co2") < 0.0);
}

#[test]
fn every_drawn_frame_is_a_closed_mesh() {
    let s = load();
    let t = &s.timelines()[0];
    for step in 0..=39 {
        let pose = t.pose_at(f64::from(step));
        let visible: Vec<usize> =
            (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        let drawn = s.build_geometry_for(&visible).unwrap().apply_pose(&pose).with_particles(&pose.particles);
        assert_closed(drawn.mesh().indices());
    }
}
