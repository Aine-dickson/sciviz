//! The chemistry kit's items (docs/item-document.md section 9, C4): the water reaction and salt dissolving, written as
//! item documents. The kit's rules are checked on the compiled output: a reaction only rearranges atoms into the
//! library's molecules; water turns its oxygen end to Na+ and a hydrogen end to Cl-, at the hydration distances.

use std::collections::HashMap;

use svf_core::chemistry_library::{self as lib, CL_O_HYDRATION, C_O_DOUBLE, H_H, H_O_H_DEG, NA_O_HYDRATION, O_H, O_O_DOUBLE};
use svf_core::item::{compile_str, CompiledItem};
use svf_core::structure::Structure;

const WATER: &str = include_str!("../../../fixtures/items/water-reaction.item.ron");
const SALT: &str = include_str!("../../../fixtures/items/salt-dissolving.item.ron");

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

fn angle(a: [f64; 3], centre: [f64; 3], b: [f64; 3]) -> f64 {
    let (u, v) = ([a[0] - centre[0], a[1] - centre[1], a[2] - centre[2]], [b[0] - centre[0], b[1] - centre[1], b[2] - centre[2]]);
    ((u[0] * v[0] + u[1] * v[1] + u[2] * v[2]) / (dist(a, centre) * dist(b, centre))).acos().to_degrees()
}

/// Posed centre and radius of every ball (atom or ion) at `t`, keyed by part id.
fn balls_at(s: &Structure, t: f64) -> HashMap<String, ([f64; 3], f64)> {
    let pose = s.timelines()[0].pose_at(t);
    let all: Vec<usize> = (0..s.parts().len()).collect();
    let posed = s.build_geometry_for(&all).unwrap().apply_pose(&pose);
    let mut out = HashMap::new();
    for r in posed.ranges() {
        let part = &s.parts()[r.part_index];
        if !matches!(part.name(), "Hydrogen atom" | "Oxygen atom" | "Carbon atom" | "Nitrogen atom" | "Sodium ion" | "Chloride ion") {
            continue;
        }
        let idx = &posed.mesh().indices()[r.first_triangle * 3..(r.first_triangle + r.triangle_count) * 3];
        let mut used: Vec<u32> = idx.to_vec();
        used.sort_unstable();
        used.dedup();
        let v: Vec<[f64; 3]> = used.iter().map(|&i| posed.mesh().vertices()[i as usize].to_array()).collect();
        #[allow(clippy::cast_precision_loss)]
        let n = v.len() as f64;
        let c = [v.iter().map(|p| p[0]).sum::<f64>() / n, v.iter().map(|p| p[1]).sum::<f64>() / n, v.iter().map(|p| p[2]).sum::<f64>() / n];
        let r = v.iter().map(|p| dist(*p, c)).fold(0.0, f64::max);
        out.insert(part.id().to_owned(), (c, r));
    }
    out
}

fn of_kind<'a>(s: &'a Structure, balls: &'a HashMap<String, ([f64; 3], f64)>, name: &'a str) -> impl Iterator<Item = [f64; 3]> + 'a {
    s.parts().iter().filter(move |p| p.name() == name).filter_map(move |p| balls.get(p.id()).map(|b| b.0))
}

fn water() -> CompiledItem {
    compile_str(WATER).expect("the water reaction compiles")
}

#[test]
fn the_reaction_conserves_atoms_and_ends_in_water_molecules_of_library_shape() {
    let item = water();
    let s = &item.structure;
    let t = &s.timelines()[0];
    let react = t.beats().iter().find(|b| b.beat == "react").unwrap().start;
    let before = balls_at(s, 0.0);
    let after = balls_at(s, t.duration());
    for (balls, when) in [(&before, "before"), (&after, "after")] {
        assert_eq!(of_kind(s, balls, "Hydrogen atom").count(), 4, "4 hydrogen atoms {when}");
        assert_eq!(of_kind(s, balls, "Oxygen atom").count(), 2, "2 oxygen atoms {when}");
    }
    // Before: two H2 and one O2 at the library's bond lengths.
    let h: Vec<[f64; 3]> = of_kind(s, &before, "Hydrogen atom").collect();
    let pairs = h.iter().filter(|a| h.iter().any(|b| (dist(**a, *b) - H_H).abs() < 1e-6)).count();
    assert_eq!(pairs, 4, "every hydrogen starts bonded to one other at {H_H} angstrom");
    let o: Vec<[f64; 3]> = of_kind(s, &before, "Oxygen atom").collect();
    assert!((dist(o[0], o[1]) - O_O_DOUBLE).abs() < 1e-6);
    // After: each oxygen has two hydrogens at O-H, at the H-O-H angle.
    let h: Vec<[f64; 3]> = of_kind(s, &after, "Hydrogen atom").collect();
    for o in of_kind(s, &after, "Oxygen atom") {
        let near: Vec<[f64; 3]> = h.iter().copied().filter(|p| (dist(*p, o) - O_H).abs() < 1e-6).collect();
        assert_eq!(near.len(), 2, "an oxygen with two hydrogens at {O_H}");
        assert!((angle(near[0], o, near[1]) - H_O_H_DEG).abs() < 1e-6);
    }
    // Atoms are never hidden, faded or made: every atom is shown and opaque at all times.
    let atoms: Vec<usize> = (0..s.parts().len()).filter(|&i| s.parts()[i].name().ends_with(" atom")).collect();
    for step in 0..=100 {
        let pose = t.pose_at(t.duration() * f64::from(step) / 100.0);
        assert!(atoms.iter().all(|&i| pose.parts[i].shown && (pose.parts[i].opacity - 1.0).abs() < 1e-12));
    }
    // Old bonds are gone and new bonds absent while the atoms move; new bonds appear when they arrive.
    let bonds = |group: &str| s.part_index(group).unwrap();
    let mid = t.pose_at(react + 1.5);
    assert!(!mid.parts[bonds("reaction-new-h2o-1")].shown && mid.parts[bonds("reaction-h2-1-bonds")].opacity < 1e-9);
    assert!(t.pose_at(react - 0.1).parts[bonds("reaction-h2-1-bonds")].opacity > 0.99);
    assert!(t.pose_at(react + 3.0).parts[bonds("reaction-new-h2o-1")].shown);
}

#[test]
fn the_reaction_item_reads_back_has_its_key_and_closed_frames() {
    let item = water();
    let s = Structure::from_ron_str(&item.to_ron()).expect("the compiled file reads back");
    let t = &s.timelines()[0];
    let key: Vec<String> = t.pose_at(1.0).key.iter().map(|k| k.text.clone()).collect();
    assert_eq!(key, ["Hydrogen atom (H)", "Oxygen atom (O)"]);
    let g = s.build_geometry().unwrap();
    for time in [1.0, t.duration()] {
        let pose = t.pose_at(time);
        assert!(!pose.labels.is_empty());
        for label in &pose.labels {
            assert!(s.label_anchor(&g, label, &pose).is_some(), "{} has an anchor", label.text);
        }
    }
    for step in 0..=100 {
        let pose = t.pose_at(t.duration() * f64::from(step) / 100.0);
        let visible: Vec<usize> = (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        assert_closed(s.build_geometry_for(&visible).unwrap().apply_pose(&pose).mesh().indices());
    }
}

fn reaction(equation: &str, verbs: &str) -> Result<CompiledItem, String> {
    compile_str(&format!(
        "Item(id: \"r\", title: \"T\", subject: Chemistry, idea: \"I\", reference: \"ref\",
          objects: [Reaction(id: \"x\", equation: \"{equation}\")], story: [Beat(say: \"Go.\", do: [{verbs}]), Beat(say: \"Done.\")])"
    ))
    .map_err(|e| e.to_string())
}

#[test]
fn another_reaction_works_from_the_same_kit_and_misuse_is_refused() {
    let item = reaction("CH4 + 2 O2 -> CO2 + 2 H2O", "React(\"x\")").expect("methane burning compiles");
    let s = &item.structure;
    let end = balls_at(s, s.timelines()[0].duration());
    let c = of_kind(s, &end, "Carbon atom").next().unwrap();
    let o: Vec<[f64; 3]> = of_kind(s, &end, "Oxygen atom").filter(|p| (dist(*p, c) - C_O_DOUBLE).abs() < 1e-6).collect();
    assert_eq!(o.len(), 2, "carbon dioxide: two oxygens at C=O");
    assert!((angle(o[0], c, o[1]) - 180.0).abs() < 1e-6, "and straight");
    assert!(reaction("H2 + O2 -> H2O", "").unwrap_err().contains("does not balance"));
    assert!(reaction("2 H2 + O2 -> 2 H2O2", "").unwrap_err().contains("not in the molecule library"));
    assert!(reaction("2 H2 + O2 -> 2 H2O", "React(\"x\"), React(\"x\")").unwrap_err().contains("already reacted"));
    assert!(reaction("2 H2 + O2 = 2 H2O", "").unwrap_err().contains("needs an arrow"));
    assert!(reaction("2 H2 + O2 -> 2 H2O", "Close(\"x\")").unwrap_err().contains("not a switch"));
    // Without React, the products' bonds never appear.
    let quiet = reaction("2 H2 + O2 -> 2 H2O", "").unwrap();
    let t = &quiet.structure.timelines()[0];
    assert!(!t.pose_at(t.duration()).parts[quiet.structure.part_index("x-new-h2o-1").unwrap()].shown);
    assert!(lib::species("H2O").is_some());
}

fn salt() -> CompiledItem {
    compile_str(SALT).expect("the salt item compiles")
}

#[test]
fn the_crystal_has_equal_ions_at_their_ionic_radii_on_the_lattice() {
    let item = salt();
    let s = &item.structure;
    let start = balls_at(s, 0.0);
    let na: Vec<[f64; 3]> = of_kind(s, &start, "Sodium ion").collect();
    let cl: Vec<[f64; 3]> = of_kind(s, &start, "Chloride ion").collect();
    assert_eq!((na.len(), cl.len()), (32, 32));
    for p in s.parts().iter().filter(|p| p.name() == "Sodium ion") {
        assert!((start[p.id()].1 - 1.02).abs() < 1e-6, "Na+ is drawn at its ionic radius");
    }
    // Every sodium's nearest neighbours are chloride ions, half a lattice constant away.
    for a in &na {
        let nearest = cl.iter().map(|b| dist(*a, *b)).fold(f64::INFINITY, f64::min);
        assert!((nearest - lib::NACL_LATTICE / 2.0).abs() < 1e-6);
        assert!(na.iter().filter(|b| *b != a).all(|b| dist(*a, *b) > lib::NACL_LATTICE / 2.0 + 0.1));
    }
    // Opened up, the ions are drawn at 0.4 of their size, in place.
    let t = &s.timelines()[0];
    let opened = balls_at(s, t.beats().iter().find(|b| b.beat == "neighbours").unwrap().start);
    for p in s.parts().iter().filter(|p| p.name() == "Chloride ion") {
        assert!((opened[p.id()].1 - 1.81 * 0.4).abs() < 1e-6 && dist(opened[p.id()].0, start[p.id()].0) < 1e-9);
    }
}

#[test]
fn water_turns_oxygen_to_sodium_and_hydrogen_to_chloride_at_the_hydration_distances() {
    let item = salt();
    let s = &item.structure;
    let t = &s.timelines()[0];
    let end = balls_at(s, t.duration());
    let start = balls_at(s, 0.0);
    let waters: Vec<&str> = s.parts().iter().filter(|p| p.name() == "Water molecule").map(|p| p.id()).collect();
    let pose = t.pose_at(t.duration());
    let shown_waters: Vec<&str> = waters.iter().copied().filter(|w| pose.parts[s.part_index(w).unwrap()].shown).collect();
    assert_eq!(shown_waters.len(), 4 * 6, "six water molecules for each of the four dissolved ions");
    let moved: Vec<&str> = s
        .parts()
        .iter()
        .filter(|p| p.name().ends_with(" ion") && dist(start[p.id()].0, end[p.id()].0) > 1.0)
        .map(|p| p.id())
        .collect();
    assert_eq!(moved.len(), 4, "four ions have left the crystal");
    for ion in &moved {
        let (c, _) = end[*ion];
        let cation = ion.contains("-na-");
        let shell: Vec<&str> = shown_waters
            .iter()
            .copied()
            .filter(|w| dist(end[&format!("{w}-o")].0, c) < 3.5)
            .collect();
        assert_eq!(shell.len(), 6, "{ion} has six waters round it");
        for w in shell {
            let o = end[&format!("{w}-o")].0;
            let (h1, h2) = (end[&format!("{w}-h1")].0, end[&format!("{w}-h2")].0);
            assert!((dist(h1, o) - O_H).abs() < 1e-6 && (dist(h2, o) - O_H).abs() < 1e-6);
            assert!((angle(h1, o, h2) - H_O_H_DEG).abs() < 1e-6, "the water keeps its shape");
            if cation {
                assert!((dist(o, c) - NA_O_HYDRATION).abs() < 1e-6, "Na-O {}", dist(o, c));
                assert!(dist(h1, c) > dist(o, c) && dist(h2, c) > dist(o, c), "oxygen end towards Na+");
            } else {
                assert!((dist(o, c) - CL_O_HYDRATION).abs() < 1e-6, "Cl-O {}", dist(o, c));
                assert!((dist(h1, c) - (CL_O_HYDRATION - O_H)).abs() < 1e-6, "one O-H points straight at Cl-");
            }
        }
    }
    // No ball overlaps another (drawn sizes), except atoms within one water molecule.
    let ids: Vec<&String> = end.keys().filter(|id| {
        let p = &s.parts()[s.part_index(id).unwrap()];
        pose.parts[s.part_index(id).unwrap()].shown && p.parent().is_some_and(|q| pose.parts[q].shown)
    }).collect();
    for (i, a) in ids.iter().enumerate() {
        for b in &ids[i + 1..] {
            let same_water = a.rsplit_once('-').map(|x| x.0) == b.rsplit_once('-').map(|x| x.0) && a.contains("-w");
            if same_water {
                continue;
            }
            let (pa, ra) = end[*a];
            let (pb, rb) = end[*b];
            assert!(dist(pa, pb) > ra + rb - 1e-6, "{a} and {b} overlap: {} < {}", dist(pa, pb), ra + rb);
        }
    }
}

#[test]
fn the_salt_item_misuse_frames_and_key() {
    let item = salt();
    let s = &item.structure;
    let t = &s.timelines()[0];
    let key = |time: f64| t.pose_at(time).key.iter().map(|k| k.text.clone()).collect::<Vec<_>>();
    assert_eq!(key(1.0), ["Sodium ion (Na⁺)", "Chloride ion (Cl⁻)"]);
    assert_eq!(key(t.duration()).len(), 4, "water atoms join the key once water arrives");
    for step in 0..=27 {
        let pose = t.pose_at(f64::from(step) * 4.0);
        let visible: Vec<usize> = (0..s.parts().len()).filter(|&p| pose.parts[p].shown && pose.parts[p].opacity >= 0.005).collect();
        assert_closed(s.build_geometry_for(&visible).unwrap().apply_pose(&pose).mesh().indices());
    }
    let crystal = |verbs: &str| {
        compile_str(&format!(
            "Item(id: \"s\", title: \"T\", subject: Chemistry, idea: \"I\", reference: \"r\", objects: [Crystal(id: \"c\")], story: [Beat(say: \"x\", do: [{verbs}])])"
        ))
        .map(|_| ())
        .map_err(|e| e.to_string())
    };
    assert!(crystal("Dissolve(crystal: \"c\", ions: 1)").unwrap_err().contains("open up"));
    assert!(crystal("OpenUp(\"c\"), Dissolve(crystal: \"c\", ions: 1)").unwrap_err().contains("still changing"));
    assert!(crystal("OpenUp(\"c\")").is_ok());
}
