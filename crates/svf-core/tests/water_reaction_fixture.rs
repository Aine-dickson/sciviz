//! The hydrogen + oxygen -> water fixture (docs/animation-design.md): the atoms are conserved, the bond lengths and the
//! H-O-H angle are the textbook values, the old bonds go before the atoms move and the new ones appear where the atoms
//! arrive, and no two atoms overlap at any time.

use std::collections::HashMap;

use svf_core::structure::{Geometry, Structure};

const WATER: &str = include_str!("../../../fixtures/structures/water-reaction.ron");

const ATOMS: [(&str, f64); 6] = [("h1", 0.28), ("h2", 0.28), ("h3", 0.28), ("h4", 0.28), ("o1", 0.45), ("o2", 0.45)];

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
    Structure::from_ron_str(WATER).expect("water-reaction.ron parses and validates")
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Centre of an atom (authored position plus the pose offset) at time `t`.
fn atom_at(s: &Structure, id: &str, t: f64) -> [f64; 3] {
    let index = s.part_index(id).unwrap();
    let Some(Geometry::Ellipsoid { centre, .. }) = s.parts()[index].geometry() else { panic!("{id} is not an atom") };
    let offset = s.timelines()[0].pose_at(t).parts[index].offset;
    [centre[0] + offset[0], centre[1] + offset[1], centre[2] + offset[2]]
}

#[test]
fn the_parts_are_closed_and_the_atoms_are_six() {
    let s = load();
    assert!(!s.reference().is_empty());
    for (id, _) in ATOMS {
        assert!(s.part_index(id).is_some(), "missing atom {id}");
    }
    let hydrogen = s.part_index("hydrogen").unwrap();
    let oxygen = s.part_index("oxygen").unwrap();
    let children = |parent: usize| s.parts().iter().filter(|p| p.parent() == Some(parent)).count();
    assert_eq!((children(hydrogen), children(oxygen)), (4, 2));
    assert_closed(s.build_geometry().unwrap().mesh().indices());
}

#[test]
fn the_starting_molecules_and_the_water_molecules_have_the_textbook_geometry() {
    let s = load();
    let t = &s.timelines()[0];
    let end = t.duration();

    // Before: H-H 0.74 for two molecules, O=O 1.21.
    let at0 = |id: &str| atom_at(&s, id, 0.0);
    assert!((dist(at0("h1"), at0("h2")) - 0.74).abs() < 1e-9);
    assert!((dist(at0("h3"), at0("h4")) - 0.74).abs() < 1e-9);
    assert!((dist(at0("o1"), at0("o2")) - 1.21).abs() < 1e-9);

    // After: each oxygen has exactly two hydrogens at 0.96, and the angle between its bonds is 104.5 degrees.
    let at = |id: &str| atom_at(&s, id, end);
    for (o, hs) in [("o1", ["h1", "h2"]), ("o2", ["h3", "h4"])] {
        for h in hs {
            assert!((dist(at(o), at(h)) - 0.96).abs() < 1e-9, "{o}-{h}");
        }
        let (a, b) = (at(hs[0]), at(hs[1]));
        let (u, v) = ([a[0] - at(o)[0], a[1] - at(o)[1], a[2] - at(o)[2]], [b[0] - at(o)[0], b[1] - at(o)[1], b[2] - at(o)[2]]);
        let cos = (u[0] * v[0] + u[1] * v[1] + u[2] * v[2]) / (dist(a, at(o)) * dist(b, at(o)));
        assert!((cos.acos().to_degrees() - 104.5).abs() < 1e-6, "{o}");
        // The other molecule's hydrogens are not bonded to this oxygen.
        let other = if o == "o1" { ["h3", "h4"] } else { ["h1", "h2"] };
        assert!(other.iter().all(|h| dist(at(o), at(h)) > 2.5), "{o} is far from the other water's hydrogens");
    }
}

#[test]
fn old_bonds_fade_before_the_atoms_move_and_the_new_bonds_sit_on_the_final_atoms() {
    let s = load();
    let t = &s.timelines()[0];
    let starts: Vec<f64> = t.beats().iter().map(|b| b.start).collect();
    assert_eq!(starts, [0.0, 9.0, 14.0, 20.0, 29.0]);
    assert_eq!(t.duration(), 39.0);
    let shown = |id: &str, time: f64| {
        let p = &t.pose_at(time).parts[s.part_index(id).unwrap()];
        (p.shown, p.opacity)
    };

    // Start: old bonds there, new bonds not.
    assert_eq!(shown("bond-h1h2", 0.0), (true, 1.0));
    assert!(!shown("bond-o1h1", 0.0).0 && !shown("bond-o2h4", 0.0).0);
    // The old bonds are gone (faded out) by the time the hydrogens start to move.
    assert_eq!(shown("bond-o1o2", 11.0).1, 0.0);
    assert!(shown("bond-o1o2", 10.0).1 > 0.0 && shown("bond-o1o2", 10.0).1 < 1.0, "fading");
    assert_eq!(shown("bond-h3h4", 14.0).1, 0.0);
    for h in ["h1", "h2", "h3", "h4"] {
        assert_eq!(atom_at(&s, h, 14.0), atom_at(&s, h, 0.0), "{h} has not moved when the join begins");
    }
    // The new bonds appear after the atoms have arrived, and their ends are the atoms' final centres.
    assert!(!shown("bond-o1h1", 19.9).0 && shown("bond-o1h1", 20.0).0);
    let end = t.duration();
    for (bond, o, h) in [("bond-o1h1", "o1", "h1"), ("bond-o1h2", "o1", "h2"), ("bond-o2h3", "o2", "h3"), ("bond-o2h4", "o2", "h4")] {
        let Some(Geometry::Tube { path, .. }) = s.parts()[s.part_index(bond).unwrap()].geometry() else { panic!("{bond}") };
        let (first, last) = (path[0], path[path.len() - 1]);
        assert!(dist(first, atom_at(&s, o, end)) < 1e-3 && dist(last, atom_at(&s, h, end)) < 1e-3, "{bond} joins {o} and {h}");
    }
}

#[test]
fn atoms_are_conserved_and_never_overlap_while_they_move() {
    let s = load();
    let t = &s.timelines()[0];
    for step in 0..=390 {
        let time = f64::from(step) * 0.1;
        let pose = t.pose_at(time);
        for (id, _) in ATOMS {
            let p = &pose.parts[s.part_index(id).unwrap()];
            assert!(p.shown && p.opacity == 1.0, "atom {id} must always be shown and opaque at {time}");
        }
        for (i, (a, ra)) in ATOMS.iter().enumerate() {
            for (b, rb) in &ATOMS[i + 1..] {
                let d = dist(atom_at(&s, a, time), atom_at(&s, b, time));
                assert!(d >= ra + rb - 1e-9, "{a} and {b} overlap at {time}: distance {d}");
            }
        }
    }
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
