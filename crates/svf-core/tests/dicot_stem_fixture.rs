//! The dicot stem's `water-up` timeline (docs/animation-design.md): eight flows, one up the middle of each
//! xylem bundle. The water must stay inside the xylem bundles at every instant, the outer tissues must
//! fade out so that the xylem can be seen, and every drawn frame must be a closed mesh.

use std::collections::HashMap;

use svf_core::structure::Structure;

const STEM: &str = include_str!("../../../fixtures/structures/dicot-stem.ron");

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
    Structure::from_ron_str(STEM).expect("dicot-stem.ron parses and validates")
}

#[test]
fn the_water_stays_inside_the_xylem_bundles_all_the_time() {
    let s = load();
    let t = &s.timelines()[0];
    assert_eq!(t.id(), "water-up");
    assert_eq!(t.beats().len(), 3);
    // Nothing flows before the third beat starts.
    let flow_start = t.beats()[2].start;
    assert!(t.pose_at(flow_start - 0.1).particles.is_empty());

    let mut seen = 0;
    for step in 0..=300 {
        let time = flow_start + f64::from(step) * (t.duration() - flow_start) / 300.0;
        let groups = t.pose_at(time).particles;
        assert_eq!(groups.len(), 8);
        for group in &groups {
            assert_eq!(group.positions.len(), 14);
            for p in &group.positions {
                // Radius within the xylem (2.2 to 2.85), height within the stem (0 to 12).
                let r = p[0].hypot(p[2]);
                assert!((2.2..=2.85).contains(&r) && (0.0..=12.0).contains(&p[1]), "{p:?}");
                // Angle (measured from +x towards -z) inside a 30-degree bundle: 0..30 of every 45.
                let angle = (-p[2]).atan2(p[0]).to_degrees().rem_euclid(45.0);
                assert!((0.0..=30.0).contains(&angle), "angle {angle} for {p:?}");
                seen += 1;
            }
        }
    }
    assert_eq!(seen, 301 * 8 * 14);
}

#[test]
fn the_tissues_in_the_way_fade_out_and_the_xylem_stays_visible() {
    let s = load();
    let t = &s.timelines()[0];
    let opacity = |id: &str, time: f64| t.pose_at(time).parts[s.part_index(id).unwrap()].opacity;
    for id in ["epidermis", "cortex", "phloem", "xylem", "pith"] {
        assert_eq!(opacity(id, 0.0), 1.0, "{id} starts opaque");
    }
    let end = t.duration();
    for gone in ["cortex", "phloem", "vascular-cambium", "medullary-rays"] {
        assert_eq!(opacity(gone, end), 0.0, "{gone} fades out");
    }
    assert!(opacity("xylem", end) > 0.3 && opacity("xylem", end) < 1.0, "the xylem is translucent so the water shows");
    assert!(opacity("epidermis", end) > 0.0, "a faint outline of the stem remains");
}

#[test]
fn every_drawn_frame_is_a_closed_mesh() {
    let s = load();
    let t = &s.timelines()[0];
    for step in 0..=24 {
        let pose = t.pose_at(f64::from(step));
        let visible: Vec<usize> =
            (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        let drawn = s.build_geometry_for(&visible).unwrap().apply_pose(&pose).with_particles(&pose.particles);
        assert_closed(drawn.mesh().indices());
    }
}
