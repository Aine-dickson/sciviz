//! The 2D overlay drawn over the scene: label boxes with leaders, the colour
//! key and the caption band (`docs/labels.md`, slice L3). This module is CPU
//! only and testable without a GPU: it rasterises glyphs with `fontdue` into an
//! atlas and composes screen-space quads in pixels. The GPU side
//! (`overlay_gpu.rs`) uploads the atlas and draws the quads.

use std::collections::HashMap;

use svf_core::structure_label_layout::{layout_labels, LabelFrame, LabelRequest};
use svf_core::structure_labels::{KeyEntry, LabelSide};

/// The embedded font (DejaVu Sans 2.37; licence in `assets/fonts/DejaVu-LICENSE.txt`).
const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");

/// Side of the square glyph atlas, in pixels.
pub const ATLAS_SIZE: u32 = 1024;

/// One overlay vertex: position in pixels (top-left origin), atlas uv (negative for a solid colour) and RGBA.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OverlayVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub colour: [f32; 4],
}

/// Triangles to draw over the scene, in pixels of a `size` target.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    pub size: [f32; 2],
    pub vertices: Vec<OverlayVertex>,
}

#[derive(Clone, Copy, Debug)]
struct GlyphSlot {
    /// Top-left in the atlas, and size, in pixels.
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    /// Offset from the pen position on the baseline to the bitmap's top-left (y down).
    left: f32,
    top: f32,
    advance: f32,
}

/// The font and its glyph atlas. Glyphs are rasterised the first time a size needs them.
pub struct TextKit {
    font: fontdue::Font,
    atlas: Vec<u8>,
    slots: HashMap<(char, u32), GlyphSlot>,
    shelf_x: u32,
    shelf_y: u32,
    shelf_height: u32,
    /// Changes whenever the atlas pixels change, so the GPU copy can be refreshed.
    generation: u64,
}

impl Default for TextKit {
    fn default() -> Self {
        Self::new()
    }
}

impl TextKit {
    /// # Panics
    /// Only if the embedded font is unreadable, which the tests rule out.
    #[must_use]
    pub fn new() -> Self {
        let font = fontdue::Font::from_bytes(FONT_BYTES, fontdue::FontSettings::default())
            .expect("the embedded DejaVu Sans font parses");
        Self {
            font,
            atlas: vec![0; (ATLAS_SIZE * ATLAS_SIZE) as usize],
            slots: HashMap::new(),
            shelf_x: 1,
            shelf_y: 1,
            shelf_height: 0,
            generation: 0,
        }
    }

    /// Whether the font has a glyph for `c` (spaces and line breaks count as present).
    #[must_use]
    pub fn has_glyph(&self, c: char) -> bool {
        c.is_whitespace() || self.font.lookup_glyph_index(c) != 0
    }

    /// The atlas pixels (one byte of coverage each, `ATLAS_SIZE` square) and their generation.
    #[must_use]
    pub fn atlas(&self) -> (&[u8], u64) {
        (&self.atlas, self.generation)
    }

    fn line_metrics(&self, px: f32) -> (f32, f32) {
        let m = self.font.horizontal_line_metrics(px).expect("DejaVu Sans has horizontal metrics");
        (m.ascent, m.new_line_size)
    }

    /// Width of one line of text, and the line height, in pixels.
    #[must_use]
    pub fn measure(&self, text: &str, px: f32) -> [f32; 2] {
        let width = text.chars().map(|c| self.font.metrics(c, px).advance_width).sum();
        [width, self.line_metrics(px).1]
    }

    /// Break `text` into lines no wider than `max_width` where possible (a single
    /// word wider than that gets a line of its own).
    #[must_use]
    pub fn wrap(&self, text: &str, px: f32, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() { word.to_owned() } else { format!("{line} {word}") };
            if !line.is_empty() && self.measure(&candidate, px)[0] > max_width {
                lines.push(std::mem::take(&mut line));
                line = word.to_owned();
            } else {
                line = candidate;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }

    fn slot(&mut self, c: char, px: f32) -> Option<GlyphSlot> {
        let key = (c, px.to_bits());
        if let Some(slot) = self.slots.get(&key) {
            return Some(*slot);
        }
        let (m, coverage) = self.font.rasterize(c, px);
        let (width, height) = (m.width as u32, m.height as u32);
        if self.shelf_x + width + 1 > ATLAS_SIZE {
            self.shelf_x = 1;
            self.shelf_y += self.shelf_height + 1;
            self.shelf_height = 0;
        }
        if self.shelf_y + height + 1 > ATLAS_SIZE {
            return None; // Atlas full: the glyph is skipped (never expected at the sizes used).
        }
        let (x, y) = (self.shelf_x, self.shelf_y);
        for row in 0..height {
            let from = (row * width) as usize;
            let to = ((y + row) * ATLAS_SIZE + x) as usize;
            self.atlas[to..to + width as usize].copy_from_slice(&coverage[from..from + width as usize]);
        }
        self.shelf_x += width + 1;
        self.shelf_height = self.shelf_height.max(height);
        self.generation += 1;
        let slot = GlyphSlot {
            x,
            y,
            width,
            height,
            left: m.xmin as f32,
            top: -(m.ymin as f32 + m.height as f32),
            advance: m.advance_width,
        };
        self.slots.insert(key, slot);
        Some(slot)
    }

    /// Append the quads of one line of text whose top-left is `(x, y)`.
    fn push_text(&mut self, out: &mut Vec<OverlayVertex>, text: &str, px: f32, x: f32, y: f32, colour: [f32; 4]) {
        let ascent = self.line_metrics(px).0;
        let baseline = (y + ascent).round();
        let mut pen = x;
        for c in text.chars() {
            let Some(g) = self.slot(c, px) else { continue };
            if g.width > 0 && g.height > 0 {
                let left = (pen + g.left).round();
                let top = baseline + g.top;
                let s = ATLAS_SIZE as f32;
                let uv0 = [g.x as f32 / s, g.y as f32 / s];
                let uv1 = [(g.x + g.width) as f32 / s, (g.y + g.height) as f32 / s];
                push_quad(out, [left, top, g.width as f32, g.height as f32], uv0, uv1, colour);
            }
            pen += g.advance;
        }
    }
}

fn push_quad(out: &mut Vec<OverlayVertex>, rect: [f32; 4], uv0: [f32; 2], uv1: [f32; 2], colour: [f32; 4]) {
    let [x, y, w, h] = rect;
    let v = |px: f32, py: f32, u: f32, vv: f32| OverlayVertex { position: [px, py], uv: [u, vv], colour };
    out.extend_from_slice(&[
        v(x, y, uv0[0], uv0[1]),
        v(x + w, y, uv1[0], uv0[1]),
        v(x, y + h, uv0[0], uv1[1]),
        v(x + w, y, uv1[0], uv0[1]),
        v(x + w, y + h, uv1[0], uv1[1]),
        v(x, y + h, uv0[0], uv1[1]),
    ]);
}

fn push_rect(out: &mut Vec<OverlayVertex>, rect: [f32; 4], colour: [f32; 4]) {
    push_quad(out, rect, [-1.0, -1.0], [-1.0, -1.0], colour);
}

/// A straight line `width` pixels wide, as a quad.
fn push_line(out: &mut Vec<OverlayVertex>, a: [f32; 2], b: [f32; 2], width: f32, colour: [f32; 4]) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length = (dx * dx + dy * dy).sqrt();
    if length < 1e-3 {
        return;
    }
    let (nx, ny) = (-dy / length * width / 2.0, dx / length * width / 2.0);
    let v = |px: f32, py: f32| OverlayVertex { position: [px, py], uv: [-1.0, -1.0], colour };
    out.extend_from_slice(&[
        v(a[0] + nx, a[1] + ny),
        v(b[0] + nx, b[1] + ny),
        v(a[0] - nx, a[1] - ny),
        v(b[0] + nx, b[1] + ny),
        v(b[0] - nx, b[1] - ny),
        v(a[0] - nx, a[1] - ny),
    ]);
}

/// One label to draw: its text, its anchor already projected to pixels, and an optional column.
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenLabel {
    pub text: String,
    pub anchor: [f64; 2],
    pub side: Option<LabelSide>,
}

/// What one frame's overlay shows.
#[derive(Clone, Copy, Debug)]
pub struct OverlayInput<'a> {
    /// Target size in pixels.
    pub size: [f64; 2],
    pub caption: Option<&'a str>,
    pub key: &'a [KeyEntry],
    pub labels: &'a [ScreenLabel],
}

const PANEL: [f32; 4] = [0.02, 0.03, 0.05, 0.80];
const TEXT: [f32; 4] = [0.96, 0.96, 0.96, 1.0];
const LEADER: [f32; 4] = [0.93, 0.93, 0.93, 0.95];

/// Where things were put, for tests and for hosts that want to know.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OverlayPlacement {
    /// `[x, y, w, h]` of the caption band, if any.
    pub caption: Option<[f32; 4]>,
    /// `[x, y, w, h]` of the key box, if any.
    pub key: Option<[f32; 4]>,
    /// `(label index, [x, y, w, h])` of each placed label.
    pub labels: Vec<(usize, [f32; 4])>,
}

/// Compose the overlay: caption band at the bottom, key box at the top right,
/// labels in two columns with leaders (layout from `svf-core`). Sizes scale
/// with the target height (1200 px is the reference).
pub fn compose_overlay(kit: &mut TextKit, input: &OverlayInput<'_>) -> (Overlay, OverlayPlacement) {
    let [width, height] = [input.size[0] as f32, input.size[1] as f32];
    let s = (height / 1200.0).clamp(0.5, 2.0);
    let (margin, pad, gap) = (16.0 * s, 8.0 * s, 6.0 * s);
    let (label_px, caption_px) = ((22.0 * s).round(), (26.0 * s).round());
    let mut out = Vec::new();
    let mut placement = OverlayPlacement::default();

    // Caption band: wrapped text on a full-width panel at the bottom.
    let mut bottom_reserved = 0.0;
    if let Some(caption) = input.caption.filter(|c| !c.trim().is_empty()) {
        let line_height = kit.measure("", caption_px)[1];
        let lines = kit.wrap(caption, caption_px, width - 2.0 * (margin + pad));
        let band = [margin, 0.0, width - 2.0 * margin, lines.len() as f32 * line_height + 2.0 * pad];
        let band = [band[0], height - margin - band[3], band[2], band[3]];
        push_rect(&mut out, band, PANEL);
        for (i, line) in lines.iter().enumerate() {
            kit.push_text(&mut out, line, caption_px, band[0] + pad, band[1] + pad + i as f32 * line_height, TEXT);
        }
        bottom_reserved = band[3] + gap;
        placement.caption = Some(band);
    }

    // Key box: one row per entry, a swatch then the text.
    let mut right_top_reserved = 0.0;
    if !input.key.is_empty() {
        let line_height = kit.measure("", label_px)[1];
        let swatch = (line_height * 0.7).round();
        let text_width = input.key.iter().map(|k| kit.measure(&k.text, label_px)[0]).fold(0.0, f32::max);
        let w = (pad + swatch + pad + text_width + pad).min(width / 2.0 - margin);
        let h = input.key.len() as f32 * line_height + 2.0 * pad;
        let rect = [width - margin - w, margin, w, h];
        push_rect(&mut out, rect, PANEL);
        for (i, entry) in input.key.iter().enumerate() {
            let y = rect[1] + pad + i as f32 * line_height;
            let c = entry.colour;
            push_rect(&mut out, [rect[0] + pad, y + (line_height - swatch) / 2.0, swatch, swatch], [c[0] as f32, c[1] as f32, c[2] as f32, 1.0]);
            kit.push_text(&mut out, &entry.text, label_px, rect[0] + 2.0 * pad + swatch, y, TEXT);
        }
        right_top_reserved = h + gap;
        placement.key = Some(rect);
    }

    // Labels: measured here, placed by the core layout.
    let line_height = kit.measure("", label_px)[1];
    let requests: Vec<LabelRequest> = input
        .labels
        .iter()
        .map(|l| LabelRequest {
            anchor: l.anchor,
            size: [f64::from(kit.measure(&l.text, label_px)[0] + 2.0 * pad), f64::from(line_height + pad)],
            side: l.side,
        })
        .collect();
    let frame = LabelFrame {
        width: f64::from(width),
        height: f64::from(height),
        margin: f64::from(margin),
        gap: f64::from(gap),
        bottom_reserved: f64::from(bottom_reserved),
        right_top_reserved: f64::from(right_top_reserved),
    };
    for placed in layout_labels(&frame, &requests) {
        let rect = placed.rect.map(|v| v as f32);
        let [a, b] = placed.leader.map(|p| [p[0] as f32, p[1] as f32]);
        push_line(&mut out, a, b, (2.0 * s).max(1.0), LEADER);
        let dot = (6.0 * s).round();
        push_rect(&mut out, [a[0] - dot / 2.0, a[1] - dot / 2.0, dot, dot], LEADER);
        push_rect(&mut out, rect, PANEL);
        kit.push_text(&mut out, &input.labels[placed.request].text, label_px, rect[0] + pad, rect[1] + pad / 2.0, TEXT);
        placement.labels.push((placed.request, rect));
    }

    (Overlay { size: [width, height], vertices: out }, placement)
}

#[cfg(test)]
mod tests {
    use super::{compose_overlay, OverlayInput, ScreenLabel, TextKit, ATLAS_SIZE};
    use svf_core::structure_labels::KeyEntry;

    #[test]
    fn every_character_in_every_fixture_has_a_glyph() {
        let kit = TextKit::new();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/structures");
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "ron") {
                let text = std::fs::read_to_string(&path).unwrap();
                let missing: Vec<char> = text.chars().filter(|&c| !kit.has_glyph(c)).collect();
                assert!(missing.is_empty(), "{}: no glyph for {missing:?}", path.display());
                checked += 1;
            }
        }
        assert!(checked >= 10, "read {checked} fixtures");
        for c in ['₂', '⁺', '⁻', '→', '°', 'é'] {
            assert!(kit.has_glyph(c), "{c}");
        }
    }

    #[test]
    fn measuring_and_wrapping() {
        let kit = TextKit::new();
        let [short, line] = kit.measure("Filament", 22.0);
        assert!(short > 50.0 && short < 150.0 && line > 20.0, "{short} {line}");
        assert!(kit.measure("Filament supports", 22.0)[0] > short);
        assert_eq!(kit.measure("", 22.0)[0], 0.0);
        let text = "The cell gives the charge energy. In the thin filament that energy is transferred as heat and light.";
        let lines = kit.wrap(text, 26.0, 400.0);
        assert!(lines.len() >= 3);
        assert!(lines.iter().all(|l| kit.measure(l, 26.0)[0] <= 400.0));
        assert_eq!(lines.join(" "), text);
    }

    #[test]
    fn glyphs_are_packed_without_overlap_and_the_generation_moves() {
        let mut kit = TextKit::new();
        let (_, before) = kit.atlas();
        let mut out = Vec::new();
        for px in [14.0, 22.0, 26.0, 40.0] {
            kit.push_text(&mut out, "Sodium ion Na⁺ and chloride Cl⁻: 2H₂ + O₂ → 2H₂O", px, 0.0, 0.0, [1.0; 4]);
        }
        assert!(kit.atlas().1 > before);
        let slots: Vec<_> = kit.slots.values().filter(|s| s.width > 0).collect();
        for (i, a) in slots.iter().enumerate() {
            assert!(a.x + a.width <= ATLAS_SIZE && a.y + a.height <= ATLAS_SIZE);
            for b in &slots[i + 1..] {
                let apart = a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y;
                assert!(apart, "{a:?} {b:?}");
            }
        }
        // Asking again rasterises nothing new.
        let generation = kit.atlas().1;
        kit.push_text(&mut out, "Sodium", 22.0, 0.0, 0.0, [1.0; 4]);
        assert_eq!(kit.atlas().1, generation);
    }

    #[test]
    fn the_overlay_puts_caption_key_and_labels_where_they_belong() {
        let mut kit = TextKit::new();
        let key = [
            KeyEntry { colour: [0.25, 0.55, 1.0], text: "Electron".to_owned() },
            KeyEntry { colour: [0.6, 0.45, 0.9], text: "Sodium ion, Na⁺".to_owned() },
        ];
        let labels = [
            ScreenLabel { text: "Switch".to_owned(), anchor: [300.0, 600.0], side: None },
            ScreenLabel { text: "Lamp".to_owned(), anchor: [900.0, 300.0], side: None },
            ScreenLabel { text: "Off screen".to_owned(), anchor: [-50.0, 300.0], side: None },
        ];
        let caption = "Close the switch. The lever touches the contact and the loop is complete.";
        let input = OverlayInput { size: [1600.0, 1200.0], caption: Some(caption), key: &key, labels: &labels };
        let (overlay, placed) = compose_overlay(&mut kit, &input);
        assert_eq!(overlay.size, [1600.0, 1200.0]);
        assert!(!overlay.vertices.is_empty() && overlay.vertices.len() % 3 == 0);
        for v in &overlay.vertices {
            assert!(v.position.iter().all(|p| p.is_finite()));
            assert!(v.position[0] >= -1.0 && v.position[0] <= 1601.0 && v.position[1] >= -1.0 && v.position[1] <= 1201.0, "{v:?}");
        }
        let band = placed.caption.unwrap();
        assert!((band[1] + band[3] - (1200.0 - 16.0)).abs() < 1e-3, "caption band sits on the bottom margin");
        let key_box = placed.key.unwrap();
        assert!((key_box[0] + key_box[2] - (1600.0 - 16.0)).abs() < 1e-3 && (key_box[1] - 16.0).abs() < 1e-3);
        assert_eq!(placed.labels.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [0, 1], "the off-screen label is left out");
        for (_, r) in &placed.labels {
            assert!(r[1] + r[3] <= band[1], "label above the caption band");
            assert!(r[0] >= 16.0 - 1e-3 && r[0] + r[2] <= 1600.0 - 16.0 + 1e-3);
        }
        let lamp = placed.labels[1].1;
        assert!(lamp[1] >= key_box[1] + key_box[3], "right-column label below the key box");
    }

    #[test]
    fn an_empty_input_draws_nothing() {
        let mut kit = TextKit::new();
        let input = OverlayInput { size: [800.0, 600.0], caption: None, key: &[], labels: &[] };
        assert!(compose_overlay(&mut kit, &input).0.vertices.is_empty());
    }
}
