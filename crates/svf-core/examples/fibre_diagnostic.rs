//! B1.2 SVF fibre diagnostic (`docs/heart-fibre-orientation.md` §7.3).
//!
//! Runs the svf-core parser, boundary extraction, triangulation and
//! `build_boundary_axial_glyphs` on the working copy of a Rodero case, then
//! derives the diagnostic-only quantities (ρ wall groups, LV axis, |α|,
//! near-face centroids) from the operation's exposed samples. It writes the
//! statistics set in the same JSON schema as `tools/rodero-fibre-audit/audit.py`,
//! plus a per-glyph CSV for `verify_glyphs.py`.
//!
//! Usage:
//! `cargo run -p svf-core --release --example fibre_diagnostic -- <01-svf-order.vtk>
//!  --source <01.vtk> --tags 1,2 --lv-tag 1 --bin 4 --length 3
//!  --out svf-case01.json --glyphs glyphs.csv`
//!
//! `--source` is hashed, never parsed (§7.4). Constants and arithmetic follow
//! `audit.py` so that `compare.py` can apply the §7.2 matching rule.

use std::{collections::BTreeMap, env, fmt::Write as _, fs, process, time::Instant};

use svf_core::{
    boundary_axial_glyphs::{build_boundary_axial_glyphs, BoundaryAxialGlyphOptions3, BoundaryAxialGlyphSet3},
    dataset::{Dataset, DatasetGeometry},
    sampling::{NonFiniteValue, SampleValue},
    surface_triangulation::TriangulatedSurface3,
    vtk_legacy::parse_legacy_ascii_unstructured_grid,
};

const SCHEMA: &str = "svf-b1.2-fibre-stats/1";
const GLYPH_SCHEMA: &str = "svf-b1.2-glyphs/1";
const EPSILON: f64 = 1.0e-6;
const NEAR_FACE_MM: f64 = 1.0e-9;
const ENDO_MAX: f64 = 0.2;
const EPI_MIN: f64 = 0.8;
const APEX_Z_MAX: f64 = 0.02;
const BASE_Z_MIN: f64 = 0.98;
const LV_V: f64 = -1.0;
const SENTINEL: f64 = -10.0;
const GROUPS: [&str; 3] = ["endo", "epi", "other"];
/// SVF's VTK parser contract (`vtk_legacy.rs`), stated in the audit's words
/// (design §7.2). `compare.py` checks that both sides declare the same one.
const SAMPLE_SCHEMA: &str = "svf-fibre-samples/1";
const NUMERIC_CONVERSION: &str =
    "vtk-declared-type/1: float=binary32 promoted to f64; double=binary64; integer types exact";

const USAGE: &str = "Usage: fibre_diagnostic <input.vtk> --source <original.vtk> --tags 1,2 --lv-tag 1 \
--bin <mm> --length <mm> --out <stats.json> --glyphs <glyphs.csv> [--samples <samples.csv>]";

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("fibre_diagnostic failed: {message}");
    process::exit(1);
}

fn usage(message: &str) -> ! {
    eprintln!("{message}\n{USAGE}");
    process::exit(2);
}

struct Args {
    input: String,
    source: String,
    tags: Vec<i64>,
    lv_tag: i64,
    bin_mm: f64,
    length_mm: f64,
    out: String,
    glyphs: String,
    samples: Option<String>,
}

fn parse_args() -> Args {
    let mut input = None;
    let mut options: BTreeMap<String, String> = BTreeMap::new();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(name) = arg.strip_prefix("--") {
            let Some(value) = args.next() else { usage(&format!("--{name} needs a value")) };
            if options.insert(name.to_owned(), value).is_some() {
                usage(&format!("--{name} given twice"));
            }
        } else if input.replace(arg).is_some() {
            usage("exactly one input file is expected");
        }
    }
    let mut take = |name: &str| options.remove(name).unwrap_or_else(|| usage(&format!("--{name} is required")));
    let number = |name: &str, text: String| -> f64 {
        match text.parse::<f64>() {
            Ok(value) if value.is_finite() && value > 0.0 => value,
            _ => usage(&format!("--{name} must be a positive finite number")),
        }
    };
    let source = take("source");
    let tags_text = take("tags");
    let lv_text = take("lv-tag");
    let bin_mm = number("bin", take("bin"));
    let length_mm = number("length", take("length"));
    let out = take("out");
    let glyphs = take("glyphs");
    let samples = options.remove("samples");
    if let Some(name) = options.keys().next() {
        usage(&format!("unknown option --{name}"));
    }
    let mut tags: Vec<i64> = tags_text
        .split(',')
        .map(|t| t.trim().parse().unwrap_or_else(|_| usage("--tags must be comma-separated integers")))
        .collect();
    tags.sort_unstable();
    tags.dedup();
    let lv_tag: i64 = lv_text.parse().unwrap_or_else(|_| usage("--lv-tag must be an integer"));
    if !tags.contains(&lv_tag) {
        usage("--lv-tag must be one of --tags");
    }
    let input = input.unwrap_or_else(|| usage("an input file is required"));
    Args { input, source, tags, lv_tag, bin_mm, length_mm, out, glyphs, samples }
}

fn main() {
    let args = parse_args();
    let start = Instant::now();

    let source_bytes = fs::read(&args.source).unwrap_or_else(|e| fail(format!("cannot read --source: {e}")));
    let source_sha256 = sha256_hex(&source_bytes);
    drop(source_bytes);
    let input_bytes = fs::read(&args.input).unwrap_or_else(|e| fail(format!("cannot read input: {e}")));
    let input_sha256 = sha256_hex(&input_bytes);
    let text = String::from_utf8(input_bytes).unwrap_or_else(|_| fail("input is not UTF-8 text"));
    let dataset = parse_legacy_ascii_unstructured_grid(&text).unwrap_or_else(|e| fail(format!("VTK parse failed: {e}")));
    drop(text);
    let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { fail("not an unstructured grid") };

    let mut incidence_histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for face in grid.volume_topology().faces() {
        *incidence_histogram.entry(face.incidence_count()).or_insert(0) += 1;
    }
    let surface = grid.boundary_surface().unwrap_or_else(|e| fail(format!("boundary extraction failed: {e}"))).triangulate();
    let options = BoundaryAxialGlyphOptions3 { bin_edge: args.bin_mm, length: args.length_mm };
    let result = build_boundary_axial_glyphs(&dataset, &surface, "fibres", "ID", &args.tags, options)
        .unwrap_or_else(|e| fail(format!("glyph operation failed: {e}")));

    let mut stats = statistics(&args, &dataset, &surface, &result, &incidence_histogram);
    stats.push(("provenance", obj(vec![("source_sha256", J::Str(source_sha256.clone())), ("input_sha256", J::Str(input_sha256.clone()))])));
    let runtime = (start.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    stats.push(("run", obj(vec![
        ("runtime_s", J::Float(runtime)),
        ("peak_memory_bytes", J::Null),
        ("peak_memory_method", J::Str("not measured by the diagnostic".to_owned())),
        ("implementation", J::Str(format!("svf-core {} fibre_diagnostic", svf_core::VERSION))),
        ("platform", J::Str(format!("{} {}", env::consts::OS, env::consts::ARCH))),
    ])));

    let mut json = String::new();
    obj(stats).write(&mut json, 0);
    json.push('\n');
    fs::write(&args.out, json).unwrap_or_else(|e| fail(format!("cannot write --out: {e}")));
    fs::write(&args.glyphs, glyph_csv(&args, &surface, &result, &source_sha256, &input_sha256))
        .unwrap_or_else(|e| fail(format!("cannot write --glyphs: {e}")));

    if let Some(path) = args.samples.as_deref() {
        let (csv, summary) = sample_export(&args, &dataset, &surface, &result, &source_sha256, &input_sha256);
        fs::write(path, csv).unwrap_or_else(|e| fail(format!("cannot write --samples: {e}")));
        fs::write(format!("{path}.summary.txt"), &summary).unwrap_or_else(|e| fail(format!("cannot write the samples summary: {e}")));
        print!("{summary}");
    }

    println!("boundary triangles: {}", surface.triangle_count());
    println!("eligible by tag: {:?}", result.eligible_by_tag());
    println!("glyphs: {} {:?}", result.glyphs().len(), result.glyphs_by_tag());
    println!("runtime: {runtime} s");
}

fn point_scalar(dataset: &Dataset, name: &str) -> Vec<f64> {
    let field = dataset.scalar_field(name).unwrap_or_else(|| fail(format!("missing point field {name}")));
    if field.len() != dataset.point_count() {
        fail(format!("{name} is not a point field"));
    }
    field.values().iter().map(|value| match *value {
        SampleValue::Finite(v) => v,
        SampleValue::NonFinite(NonFiniteValue::NaN) => f64::NAN,
        SampleValue::NonFinite(NonFiniteValue::PositiveInfinity) => f64::INFINITY,
        SampleValue::NonFinite(NonFiniteValue::NegativeInfinity) => f64::NEG_INFINITY,
    }).collect()
}

/// LV long axis (design 4.5): apex and base coordinate sums, summed in point
/// order as the audit does, with their point counts.
fn lv_axis_sums(grid: &svf_core::unstructured::UnstructuredGrid3, z: &[f64], v: &[f64]) -> ([f64; 3], [f64; 3], usize, usize) {
    let (mut apex, mut base) = ([0.0_f64; 3], [0.0_f64; 3]);
    let (mut n_apex, mut n_base) = (0_usize, 0_usize);
    for (i, point) in grid.points().iter().enumerate() {
        if v[i] != LV_V || z[i] == SENTINEL {
            continue;
        }
        let target = if z[i] < APEX_Z_MAX {
            n_apex += 1;
            &mut apex
        } else if z[i] > BASE_Z_MIN {
            n_base += 1;
            &mut base
        } else {
            continue;
        };
        let p = point.to_array();
        for k in 0..3 {
            target[k] += p[k];
        }
    }
    if n_apex == 0 || n_base == 0 {
        fail("cannot define the LV axis: no apex or base points");
    }
    (apex, base, n_apex, n_base)
}

fn statistics(
    args: &Args,
    dataset: &Dataset,
    surface: &TriangulatedSurface3,
    result: &BoundaryAxialGlyphSet3,
    incidence_histogram: &BTreeMap<usize, usize>,
) -> Vec<(&'static str, J)> {
    let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { unreachable!() };
    let rho = point_scalar(dataset, "RHO.dat");
    let z = point_scalar(dataset, "Z.dat");
    let v = point_scalar(dataset, "V.dat");

    let (apex, base, n_apex, n_base) = lv_axis_sums(grid, &z, &v);
    let axis = [0, 1, 2].map(|k| base[k] / n_base as f64 - apex[k] / n_apex as f64);
    let axis_length = norm(axis);
    let axis_unit = scale(axis, 1.0 / axis_length);

    let mut abs_cos: BTreeMap<i64, [Vec<f64>; 3]> = args.tags.iter().map(|&t| (t, Default::default())).collect();
    let mut alpha: [Vec<f64>; 3] = Default::default();
    let (mut helix_zero_projection, mut helix_degenerate) = (0_usize, 0_usize);
    let mut sentinel_rho_triangles = 0_usize;
    let mut near_face = 0_usize;
    let h = args.bin_mm;

    for sample in result.samples() {
        let triangle = surface.triangles()[sample.triangle_index()];
        let rhos = triangle.vertices().map(|vertex| rho[surface.source_point_ids()[vertex as usize] as usize]);
        if rhos.iter().any(|&r| r == SENTINEL) {
            sentinel_rho_triangles += 1;
        }
        let mean_rho = (rhos[0] + rhos[1] + rhos[2]) / 3.0;
        let group = if mean_rho < ENDO_MAX { 0 } else if mean_rho > EPI_MIN { 1 } else { 2 };
        abs_cos.get_mut(&sample.tag()).expect("sample tag is in the tag set")[group].push(sample.abs_cos());

        if sample.tag() == args.lv_tag {
            let f_hat = sample.unit_direction().to_vector().to_array();
            let n_hat = sample.unit_normal().to_vector().to_array();
            let t = sub(f_hat, scale(n_hat, dot(f_hat, n_hat)));
            let t_length = norm(t);
            let circ = cross(axis_unit, n_hat);
            let circ_length = norm(circ);
            if t_length <= EPSILON {
                helix_zero_projection += 1;
            } else if circ_length <= EPSILON {
                helix_degenerate += 1;
            } else {
                let value = dot(t, circ).abs() / (t_length * circ_length);
                alpha[group].push(value.min(1.0).acos().to_degrees());
            }
        }

        let centroid = sample.centroid().to_array();
        for (x, k) in centroid.iter().zip(sample.bin()) {
            let r = x - k as f64 * h;
            if r < NEAR_FACE_MM || h - r < NEAR_FACE_MM {
                near_face += 1;
                break;
            }
        }
    }

    let mut groups = Vec::new();
    let mut all_cos = Vec::new();
    for (&tag, lists) in &mut abs_cos {
        let mut entries = Vec::new();
        for (g, values) in lists.iter_mut().enumerate() {
            values.sort_by(f64::total_cmp);
            all_cos.extend_from_slice(values);
            let mut entry = vec![
                ("count", J::Int(values.len() as i64)),
                ("abs_cos_p50", percentile(values, 0.50)),
                ("abs_cos_p90", percentile(values, 0.90)),
                ("abs_cos_p99", percentile(values, 0.99)),
            ];
            if tag == args.lv_tag {
                let angles = &mut alpha[g];
                angles.sort_by(f64::total_cmp);
                entry.extend([
                    ("alpha_count", J::Int(angles.len() as i64)),
                    ("alpha_deg_p25", percentile(angles, 0.25)),
                    ("alpha_deg_p50", percentile(angles, 0.50)),
                    ("alpha_deg_p75", percentile(angles, 0.75)),
                ]);
            }
            entries.push((GROUPS[g].to_owned(), obj(entry)));
        }
        groups.push((tag.to_string(), J::Obj(entries)));
    }

    let eligible = all_cos.len();
    let lt01 = all_cos.iter().filter(|&&x| x < 0.1).count();
    let lt02 = all_cos.iter().filter(|&&x| x < 0.2).count();
    let ratio = |count: usize| if eligible == 0 { J::Null } else { J::Float(count as f64 / eligible as f64) };
    let counts = |map: &BTreeMap<i64, usize>| J::Obj(map.iter().map(|(k, v)| (k.to_string(), J::Int(*v as i64))).collect());

    vec![
        ("schema", J::Str(SCHEMA.to_owned())),
        ("parameters", obj(vec![
            ("tags", J::Arr(args.tags.iter().map(|&t| J::Int(t)).collect())),
            ("lv_tag", J::Int(args.lv_tag)),
            ("bin_mm", J::Float(args.bin_mm)),
            ("grid_origin_mm", J::Arr(vec![J::Float(0.0); 3])),
            ("epsilon", J::Float(EPSILON)),
            ("near_face_mm", J::Float(NEAR_FACE_MM)),
            ("endo_rho_max", J::Float(ENDO_MAX)),
            ("epi_rho_min", J::Float(EPI_MIN)),
            ("numeric_conversion", J::Str(NUMERIC_CONVERSION.to_owned())),
        ])),
        ("mesh", obj(vec![("points", J::Int(dataset.point_count() as i64)), ("cells", J::Int(dataset.cell_count() as i64))])),
        ("boundary_triangles", J::Int(surface.triangle_count() as i64)),
        ("boundary_by_tag", counts(result.boundary_by_tag())),
        ("selected_by_tag", counts(result.selected_by_tag())),
        ("eligible_by_tag", counts(&result.eligible_by_tag())),
        ("exclusions", obj(vec![
            ("zero_area", J::Int(result.zero_area_count() as i64)),
            ("invalid_fibre", J::Int(result.invalid_direction_count() as i64)),
            ("helix_zero_projection", J::Int(helix_zero_projection as i64)),
            ("helix_degenerate_circumferential", J::Int(helix_degenerate as i64)),
        ])),
        ("groups", J::Obj(groups)),
        ("fractions", obj(vec![
            ("eligible", J::Int(eligible as i64)),
            ("abs_cos_lt_0_1_count", J::Int(lt01 as i64)),
            ("abs_cos_lt_0_2_count", J::Int(lt02 as i64)),
            ("abs_cos_lt_0_1", ratio(lt01)),
            ("abs_cos_lt_0_2", ratio(lt02)),
        ])),
        ("lv_axis", obj(vec![
            ("vector_mm", J::Arr(axis.iter().map(|&x| J::Float(x)).collect())),
            ("length_mm", J::Float(axis_length)),
            ("apex_points", J::Int(n_apex as i64)),
            ("base_points", J::Int(n_base as i64)),
        ])),
        ("thinning", obj(vec![
            ("occupied_bins", J::Int(result.glyphs().len() as i64)),
            ("glyphs_by_tag", counts(&result.glyphs_by_tag())),
            ("near_face_centroids", J::Int(near_face as i64)),
        ])),
        ("audit_checks", obj(vec![
            ("face_incidence_histogram", J::Obj(incidence_histogram.iter().map(|(k, v)| (k.to_string(), J::Int(*v as i64))).collect())),
            ("eligible_triangles_with_sentinel_rho_vertex", J::Int(sentinel_rho_triangles as i64)),
        ])),
    ]
}

/// Nearest rank, index ⌊(n − 1)·p + 0.5⌋ (halves round up), as in the audit.
fn percentile(sorted: &[f64], p: f64) -> J {
    percentile_value(sorted, p).map_or(J::Null, J::Float)
}

fn percentile_value(sorted: &[f64], p: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    Some(sorted[((sorted.len() - 1) as f64 * p + 0.5).floor() as usize])
}

/// One eligible sample with the diagnostic-only quantities of the export.
struct ExportRow {
    surface_index: usize,
    cell: usize,
    tag: i64,
    group: usize,
    centroid: [f64; 3],
    abs_cos: f64,
    theta_deg: f64,
    mean_rho: f64,
    mean_z: f64,
    sentinel_rho: bool,
    sentinel_z: bool,
    axial_s: f64,
}

/// Exports the tail and RV "other" samples (design A4) and a short summary.
///
/// Both classes use the diagnostic's existing criteria and are not
/// redefined here:
/// * wall group: the mean of the three vertex rho values, endo if < 0.2, epi
///   if > 0.8, otherwise other (as `statistics` does);
/// * `rv_other`: a tag other than `--lv-tag`, in group other;
/// * `tail`: |cos| at or above that sample's (tag, group) p99, with the same
///   `percentile` rule as the statistics. The design reported the p99 but
///   defined no per-sample tail rule; this is the most literal reading.
/// `axial_s` is (p_c - apex centroid) . axis_unit / axis_length: 0 at the
/// apex centroid and 1 at the base centroid. Region proxies only, not anatomy.
fn sample_export(
    args: &Args,
    dataset: &Dataset,
    surface: &TriangulatedSurface3,
    result: &BoundaryAxialGlyphSet3,
    source: &str,
    input: &str,
) -> (String, String) {
    let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { unreachable!() };
    let rho = point_scalar(dataset, "RHO.dat");
    let z = point_scalar(dataset, "Z.dat");
    let v = point_scalar(dataset, "V.dat");
    let (apex, base, n_apex, n_base) = lv_axis_sums(grid, &z, &v);
    let apex_centroid = [0, 1, 2].map(|k| apex[k] / n_apex as f64);
    let axis = [0, 1, 2].map(|k| base[k] / n_base as f64 - apex[k] / n_apex as f64);
    let axis_length = norm(axis);
    let axis_unit = scale(axis, 1.0 / axis_length);

    let mut rows = Vec::with_capacity(result.samples().len());
    let mut cos_by_group: BTreeMap<(i64, usize), Vec<f64>> = BTreeMap::new();
    for sample in result.samples() {
        let triangle = surface.triangles()[sample.triangle_index()];
        let ids = triangle.vertices().map(|vertex| surface.source_point_ids()[vertex as usize] as usize);
        let rhos = ids.map(|i| rho[i]);
        let zs = ids.map(|i| z[i]);
        let mean_rho = (rhos[0] + rhos[1] + rhos[2]) / 3.0;
        let group = if mean_rho < ENDO_MAX { 0 } else if mean_rho > EPI_MIN { 1 } else { 2 };
        cos_by_group.entry((sample.tag(), group)).or_default().push(sample.abs_cos());
        let centroid = sample.centroid().to_array();
        rows.push(ExportRow {
            surface_index: sample.triangle_index(),
            cell: sample.source_cell_index(),
            tag: sample.tag(),
            group,
            centroid,
            abs_cos: sample.abs_cos(),
            theta_deg: sample.tangency_angle_degrees(),
            mean_rho,
            mean_z: (zs[0] + zs[1] + zs[2]) / 3.0,
            sentinel_rho: rhos.iter().any(|&r| r == SENTINEL),
            sentinel_z: zs.iter().any(|&x| x == SENTINEL),
            axial_s: dot(sub(centroid, apex_centroid), axis_unit) / axis_length,
        });
    }
    let p99: BTreeMap<(i64, usize), f64> = cos_by_group.into_iter().map(|(key, mut values)| {
        values.sort_by(f64::total_cmp);
        (key, percentile_value(&values, 0.99).expect("a populated group has a p99"))
    }).collect();

    let tail_of = |row: &ExportRow| row.abs_cos >= p99[&(row.tag, row.group)];
    let rv_other_of = |row: &ExportRow| row.tag != args.lv_tag && row.group == 2;

    let tags: Vec<String> = args.tags.iter().map(i64::to_string).collect();
    let mut csv = format!(
        "# {SAMPLE_SCHEMA} source_sha256={source} input_sha256={input} tags={} lv_tag={} bin_mm={:?} length_mm={:?} tail=abs_cos>=p99(tag,group) rv_other=tag!=lv_tag&group=other\n",
        tags.join(","), args.lv_tag, args.bin_mm, args.length_mm,
    );
    csv.push_str("surface_index,cell,tag,group,tail,rv_other,centroid_x,centroid_y,centroid_z,abs_cos,theta_deg,mean_rho,mean_z,sentinel_rho,sentinel_z,axial_s\n");
    let mut classes: BTreeMap<(i64, &str), Vec<&ExportRow>> = BTreeMap::new();
    for row in &rows {
        let (tail, rv_other) = (tail_of(row), rv_other_of(row));
        if !tail && !rv_other {
            continue;
        }
        if tail {
            classes.entry((row.tag, "tail")).or_default().push(row);
        }
        if rv_other {
            classes.entry((row.tag, "rv_other")).or_default().push(row);
        }
        let [cx, cy, cz] = row.centroid;
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{cx:?},{cy:?},{cz:?},{:?},{:?},{:?},{:?},{},{},{:?}",
            row.surface_index, row.cell, row.tag, GROUPS[row.group], tail as u8, rv_other as u8,
            row.abs_cos, row.theta_deg, row.mean_rho, row.mean_z,
            row.sentinel_rho as u8, row.sentinel_z as u8, row.axial_s,
        );
    }

    let mut summary = format!(
        "samples: {} eligible; per (tag, class) over exported rows; region proxies only (axial_s: 0 apex centroid, 1 base centroid)\n",
        rows.len(),
    );
    for ((tag, class), members) in &classes {
        let stat = |mut values: Vec<f64>| -> String {
            values.sort_by(f64::total_cmp);
            match (values.first(), percentile_value(&values, 0.5), values.last()) {
                (Some(min), Some(median), Some(max)) => format!("min {min:.4} median {median:.4} max {max:.4}"),
                _ => "n/a".to_owned(),
            }
        };
        let rho_rows: Vec<f64> = members.iter().filter(|r| !r.sentinel_rho).map(|r| r.mean_rho).collect();
        let z_rows: Vec<f64> = members.iter().filter(|r| !r.sentinel_z).map(|r| r.mean_z).collect();
        let _ = writeln!(
            summary,
            "tag {tag} {class}: {} rows\n  axial_s: {}\n  mean_z ({} rows without a sentinel vertex): {}\n  mean_rho ({} rows without a sentinel vertex): {}",
            members.len(),
            stat(members.iter().map(|r| r.axial_s).collect()),
            z_rows.len(), stat(z_rows),
            rho_rows.len(), stat(rho_rows),
        );
    }
    (csv, summary)
}

fn glyph_csv(args: &Args, surface: &TriangulatedSurface3, result: &BoundaryAxialGlyphSet3, source: &str, input: &str) -> String {
    let tags: Vec<String> = args.tags.iter().map(i64::to_string).collect();
    let mut csv = format!(
        "# {GLYPH_SCHEMA} source_sha256={source} input_sha256={input} tags={} bin_mm={:?} length_mm={:?} glyphs={}\n",
        tags.join(","), args.bin_mm, args.length_mm, result.glyphs().len(),
    );
    csv.push_str("surface_index,p0,p1,p2,cell,tag,bin_x,bin_y,bin_z,fibre_x,fibre_y,fibre_z,theta_deg,start_x,start_y,start_z,end_x,end_y,end_z\n");
    for glyph in result.glyphs() {
        let sample = glyph.sample();
        let mut ids = surface.triangles()[sample.triangle_index()].vertices().map(|v| surface.source_point_ids()[v as usize]);
        ids.sort_unstable();
        let [bx, by, bz] = sample.bin();
        let [fx, fy, fz] = sample.direction().to_vector().to_array();
        let [sx, sy, sz] = glyph.start().to_array();
        let [ex, ey, ez] = glyph.end().to_array();
        // `{:?}` prints the shortest text that round-trips to the same f64.
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{bx},{by},{bz},{fx:?},{fy:?},{fz:?},{:?},{sx:?},{sy:?},{sz:?},{ex:?},{ey:?},{ez:?}",
            sample.triangle_index(), ids[0], ids[1], ids[2], sample.source_cell_index(), sample.tag(),
            sample.tangency_angle_degrees(),
        );
    }
    csv
}

// Explicit arithmetic in the audit's operation order.
fn sub(u: [f64; 3], v: [f64; 3]) -> [f64; 3] { [u[0] - v[0], u[1] - v[1], u[2] - v[2]] }
fn cross(u: [f64; 3], v: [f64; 3]) -> [f64; 3] {
    [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
}
fn dot(u: [f64; 3], v: [f64; 3]) -> f64 { u[0] * v[0] + u[1] * v[1] + u[2] * v[2] }
fn norm(u: [f64; 3]) -> f64 { dot(u, u).sqrt() }
fn scale(u: [f64; 3], s: f64) -> [f64; 3] { [u[0] * s, u[1] * s, u[2] * s] }

// ------------------------------------------------------------------ JSON

#[derive(Clone)]
enum J {
    Null,
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

fn obj(entries: Vec<(&str, J)>) -> J {
    J::Obj(entries.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

impl J {
    fn write(&self, out: &mut String, indent: usize) {
        let pad = |n: usize| "  ".repeat(n);
        match self {
            J::Null => out.push_str("null"),
            J::Int(v) => { let _ = write!(out, "{v}"); }
            J::Float(v) => {
                if !v.is_finite() {
                    fail(format!("non-finite statistic {v}"));
                }
                let _ = write!(out, "{v:?}");
            }
            J::Str(s) => {
                debug_assert!(!s.contains(['"', '\\']) && s.chars().all(|c| !c.is_control()));
                let _ = write!(out, "\"{s}\"");
            }
            J::Arr(items) if items.is_empty() => out.push_str("[]"),
            J::Obj(items) if items.is_empty() => out.push_str("{}"),
            J::Arr(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    out.push_str(&pad(indent + 1));
                    item.write(out, indent + 1);
                    out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
                }
                out.push_str(&pad(indent));
                out.push(']');
            }
            J::Obj(items) => {
                out.push_str("{\n");
                for (i, (key, item)) in items.iter().enumerate() {
                    let _ = write!(out, "{}\"{key}\": ", pad(indent + 1));
                    item.write(out, indent + 1);
                    out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
                }
                out.push_str(&pad(indent));
                out.push('}');
            }
        }
    }
}

// ------------------------------------------------------------------ SHA-256 (FIPS 180-4)

fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut tail = data[data.len() - data.len() % 64..].to_vec();
    tail.push(0x80);
    while tail.len() % 64 != 56 {
        tail.push(0);
    }
    tail.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    let full = &data[..data.len() - data.len() % 64];
    for block in full.chunks_exact(64).chain(tail.chunks_exact(64)) {
        let mut w = [0_u32; 64];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (state, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *state = state.wrapping_add(value);
        }
    }
    h.iter().map(|word| format!("{word:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_fips_vectors() {
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(sha256_hex(&[b'a'; 1_000_000]), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
    }

    #[test]
    fn svf_parser_matches_the_audit_declared_type_conversion() {
        // The same strings as test_audit.py: 1 + 2^-24 ± 2^-60 round to the
        // binary32 midpoint in f64, so only correct rounding separates them.
        let above = "1.000000059604644776257986737988403547205962240695953369140625";
        let below = "1.000000059604644774523263262011596452794037759304046630859375";
        let vtk = format!(
            "# vtk DataFile Version 3.0\nt\nASCII\nDATASET UNSTRUCTURED_GRID\nPOINTS 4 float\n\
             {above} 0.1 0\n{below} 0 0\n0 1 0\n0 0 1\nCELLS 1 5\n4 0 1 2 3\nCELL_TYPES 1\n10\n\
             CELL_DATA 1\nSCALARS d double 1\nLOOKUP_TABLE default\n0.1\n"
        );
        let dataset = parse_legacy_ascii_unstructured_grid(&vtk).unwrap();
        let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { unreachable!() };
        let p = |i: usize| grid.points()[i].to_array();
        assert_eq!(p(0)[0], 1.0 + 2.0_f64.powi(-23));
        assert_eq!(p(1)[0], 1.0);
        assert_eq!(p(0)[1], f64::from(0.1_f32));
        assert_eq!(dataset.scalar_field("d").unwrap().values()[0], SampleValue::Finite(0.1));
    }

    /// The same table as `EDGE_CASES` in tools/rodero-fibre-audit/test_audit.py.
    #[test]
    fn svf_parser_accepts_and_rejects_the_audit_edge_cases() {
        let cases: &[(&str, &str, Option<f64>)] = &[
            ("short", "-32768", Some(-32768.0)), ("short", "32767", Some(32767.0)),
            ("short", "-32769", None), ("short", "32768", None),
            ("unsigned_short", "0", Some(0.0)), ("unsigned_short", "65535", Some(65535.0)),
            ("unsigned_short", "65536", None), ("unsigned_short", "-1", None), ("unsigned_short", "-0", None),
            ("int", "-2147483648", Some(-2147483648.0)), ("int", "2147483647", Some(2147483647.0)),
            ("int", "2147483648", None), ("int", "-2147483649", None),
            ("int", "+7", Some(7.0)), ("int", "007", Some(7.0)), ("int", "-0", Some(0.0)),
            ("int", "1.0", None), ("int", "1_0", None), ("int", "\u{0663}", None), ("int", "+", None),
            ("unsigned_int", "4294967295", Some(4294967295.0)), ("unsigned_int", "4294967296", None),
            ("unsigned_int", "+1", Some(1.0)),
            ("unsigned_char", "255", Some(255.0)), ("unsigned_char", "256", None), ("unsigned_char", "-0", None),
            ("double", "1e5", Some(100000.0)), ("double", "-0.25", Some(-0.25)), ("double", "1_0.5", None),
            ("double", "\u{0661}.5", None), ("double", "abc", None),
            ("float", "0.5", Some(0.5)), ("float", "1_0", None), ("float", "1e39", Some(f64::INFINITY)),
            ("FLOAT", "0.5", Some(0.5)), ("Double", "-0.25", Some(-0.25)), ("INT", "7", Some(7.0)), ("Unsigned_Char", "255", Some(255.0)),
            ("float", "inf", Some(f64::INFINITY)), ("float", "INFINITY", Some(f64::INFINITY)), ("float", "-Infinity", Some(f64::NEG_INFINITY)),
            ("float", "+inf", Some(f64::INFINITY)), ("double", "Inf", Some(f64::INFINITY)), ("float", "NaN", Some(f64::NAN)), ("float", "-nan", Some(f64::NAN)),
            ("double", "nan", Some(f64::NAN)), ("double", "+NAN", Some(f64::NAN)),
            ("float", "infinit", None), ("float", "infinityy", None), ("float", "in", None), ("float", "nan(1)", None),
            ("float", "nan1", None), ("float", "snan", None), ("double", "qnan", None), ("int", "inf", None),
            ("double", "1.", Some(1.0)), ("double", ".5", Some(0.5)), ("double", "+.5", Some(0.5)), ("double", "1E+5", Some(100000.0)),
            ("double", ".", None), ("double", "-.", None), ("double", "1e", None), ("double", "1e+", None),
            ("double", "e5", None), ("double", "0x10", None), ("double", "1d5", None), ("double", "1.5f", None),
            ("double", "+-1", None), ("double", "1,5", None), ("double", "1e400", Some(f64::INFINITY)), ("float", "1e-400", Some(0.0)),
        ];
        for &(declared, text, expected) in cases {
            let vtk = format!(
                "# vtk DataFile Version 3.0\nt\nASCII\nDATASET UNSTRUCTURED_GRID\nPOINTS 4 double\n\
                 0 0 0\n1 0 0\n0 1 0\n0 0 1\nCELLS 1 5\n4 0 1 2 3\nCELL_TYPES 1\n10\n\
                 CELL_DATA 1\nSCALARS s {declared} 1\nLOOKUP_TABLE default\n{text}\n"
            );
            let parsed = parse_legacy_ascii_unstructured_grid(&vtk)
                .map(|dataset| dataset.scalar_field("s").unwrap().values()[0]);
            match expected {
                None => assert!(parsed.is_err(), "{declared} {text:?} should be rejected, got {parsed:?}"),
                Some(value) => assert_eq!(parsed, Ok(SampleValue::from_f64(value)), "{declared} {text:?}"),
            }
        }
    }

    #[test]
    fn percentile_rounds_halves_up() {
        let values = [1.0, 2.0, 3.0];
        assert!(matches!(percentile(&values, 0.25), J::Float(v) if v == 2.0));
        assert!(matches!(percentile(&[], 0.5), J::Null));
    }
}
