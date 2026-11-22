//! Screen layout for labels: the textbook callout columns (`docs/labels.md`
//! section 5, slice L2). A pure function of one frame's inputs, in pixels with
//! the origin at the top left and y down. It never measures text: the host
//! passes each label's box size in.
//!
//! Each label goes to the left or right column (its own `side`, else the side
//! of the picture its anchor is on). Within a column, boxes are sorted by
//! their anchor's y, placed at that y, then pushed apart and into the free
//! band. If a column cannot hold all its boxes, the labels set last are dropped
//! first. The leader runs from the anchor to the middle of the box's inner edge.

use crate::structure_labels::LabelSide;

/// The space labels may use.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelFrame {
    pub width: f64,
    pub height: f64,
    /// Distance kept from every viewport edge.
    pub margin: f64,
    /// Minimum vertical gap between two boxes in one column.
    pub gap: f64,
    /// Height reserved at the bottom (the caption band).
    pub bottom_reserved: f64,
    /// Height reserved at the top of the right column (the key box), from the top margin.
    pub right_top_reserved: f64,
}

/// One label to place, in the order the labels were set (earlier ones win space).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelRequest {
    pub anchor: [f64; 2],
    /// Width and height of the text box, padding included.
    pub size: [f64; 2],
    pub side: Option<LabelSide>,
}

/// A placed label.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedLabel {
    /// Index into the requests.
    pub request: usize,
    pub side: LabelSide,
    /// `[x, y, width, height]`, top-left corner first.
    pub rect: [f64; 4],
    /// From the anchor to the middle of the box's inner edge.
    pub leader: [[f64; 2]; 2],
}

/// Place labels in two columns. Requests whose anchor is outside the viewport,
/// whose box is wider than half the free width, or that do not fit in their
/// column are left out. The result is ordered by request index.
#[must_use]
pub fn layout_labels(frame: &LabelFrame, requests: &[LabelRequest]) -> Vec<PlacedLabel> {
    let finite = |v: f64| v.is_finite();
    let usable_width = frame.width - 2.0 * frame.margin;
    let mut placed = Vec::new();
    for side in [LabelSide::Left, LabelSide::Right] {
        let top = frame.margin + if side == LabelSide::Right { frame.right_top_reserved } else { 0.0 };
        let bottom = frame.height - frame.margin - frame.bottom_reserved;
        // Requests for this column, in request order (the priority for space).
        let mut column: Vec<usize> = requests
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                let [x, y] = r.anchor;
                finite(x)
                    && finite(y)
                    && (0.0..=frame.width).contains(&x)
                    && (0.0..=frame.height).contains(&y)
                    && r.size.iter().all(|&s| finite(s) && s > 0.0)
                    && r.size[0] <= usable_width / 2.0
                    && r.side.unwrap_or(if x < frame.width / 2.0 { LabelSide::Left } else { LabelSide::Right }) == side
            })
            .map(|(i, _)| i)
            .collect();
        // Drop the latest-set requests until the column's boxes and gaps fit.
        while !column.is_empty() {
            let need: f64 = column.iter().map(|&i| requests[i].size[1]).sum::<f64>() + frame.gap * (column.len() - 1) as f64;
            if need <= bottom - top {
                break;
            }
            column.pop();
        }
        if column.is_empty() {
            continue;
        }
        // Top to bottom by anchor y (ties by request index).
        column.sort_by(|&a, &b| requests[a].anchor[1].total_cmp(&requests[b].anchor[1]).then(a.cmp(&b)));
        let heights: Vec<f64> = column.iter().map(|&i| requests[i].size[1]).collect();
        let mut tops: Vec<f64> = column.iter().map(|&i| requests[i].anchor[1] - requests[i].size[1] / 2.0).collect();
        // Downward pass: no box above the band or overlapping the one before it.
        let mut floor = top;
        for (t, h) in tops.iter_mut().zip(&heights) {
            *t = t.max(floor);
            floor = *t + h + frame.gap;
        }
        // Upward pass: no box below the band or overlapping the one after it. The total fits, so the first stays inside.
        let mut ceiling = bottom;
        for (t, h) in tops.iter_mut().zip(&heights).rev() {
            *t = t.min(ceiling - h);
            ceiling = *t - frame.gap;
        }
        for (k, &i) in column.iter().enumerate() {
            let [w, h] = requests[i].size;
            let x = match side {
                LabelSide::Left => frame.margin,
                LabelSide::Right => frame.width - frame.margin - w,
            };
            let inner = match side {
                LabelSide::Left => [x + w, tops[k] + h / 2.0],
                LabelSide::Right => [x, tops[k] + h / 2.0],
            };
            placed.push(PlacedLabel { request: i, side, rect: [x, tops[k], w, h], leader: [requests[i].anchor, inner] });
        }
    }
    placed.sort_by_key(|p| p.request);
    placed
}

#[cfg(test)]
mod tests {
    use super::{layout_labels, LabelFrame, LabelRequest, PlacedLabel};
    use crate::structure_labels::LabelSide;

    const FRAME: LabelFrame =
        LabelFrame { width: 1600.0, height: 1200.0, margin: 16.0, gap: 8.0, bottom_reserved: 120.0, right_top_reserved: 150.0 };

    fn request(x: f64, y: f64) -> LabelRequest {
        LabelRequest { anchor: [x, y], size: [180.0, 32.0], side: None }
    }

    /// The guarantees of `docs/labels.md` section 5 for one layout.
    fn check(frame: &LabelFrame, requests: &[LabelRequest], placed: &[PlacedLabel]) {
        for p in placed {
            let [x, y, w, h] = p.rect;
            assert!(x >= frame.margin - 1e-9 && x + w <= frame.width - frame.margin + 1e-9, "{p:?} leaves the sides");
            assert!(y >= frame.margin - 1e-9, "{p:?} above the top");
            assert!(y + h <= frame.height - frame.margin - frame.bottom_reserved + 1e-9, "{p:?} in the caption band");
            if p.side == LabelSide::Right {
                assert!(y >= frame.margin + frame.right_top_reserved - 1e-9, "{p:?} in the key box");
            }
            assert_eq!(p.leader[0], requests[p.request].anchor);
            assert_eq!([w, h], requests[p.request].size);
        }
        for (i, a) in placed.iter().enumerate() {
            for b in &placed[i + 1..] {
                let apart_x = a.rect[0] + a.rect[2] <= b.rect[0] || b.rect[0] + b.rect[2] <= a.rect[0];
                let apart_y = a.rect[1] + a.rect[3] + frame.gap <= b.rect[1] + 1e-9 || b.rect[1] + b.rect[3] + frame.gap <= a.rect[1] + 1e-9;
                assert!(apart_x || apart_y, "{a:?} and {b:?} overlap");
                // Order within a column follows the anchors.
                if a.side == b.side {
                    let (ya, yb) = (requests[a.request].anchor[1], requests[b.request].anchor[1]);
                    if ya < yb {
                        assert!(a.rect[1] < b.rect[1], "order lost: {a:?} {b:?}");
                    } else if yb < ya {
                        assert!(b.rect[1] < a.rect[1], "order lost: {a:?} {b:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_free_label_sits_level_with_its_anchor_on_its_side() {
        let requests = [request(400.0, 500.0), request(1300.0, 700.0)];
        let placed = layout_labels(&FRAME, &requests);
        check(&FRAME, &requests, &placed);
        assert_eq!(placed[0].side, LabelSide::Left);
        assert_eq!(placed[0].rect, [16.0, 484.0, 180.0, 32.0]);
        assert_eq!(placed[0].leader, [[400.0, 500.0], [196.0, 500.0]]);
        assert_eq!(placed[1].side, LabelSide::Right);
        assert_eq!(placed[1].rect, [1404.0, 684.0, 180.0, 32.0]);
        assert_eq!(placed[1].leader[1], [1404.0, 700.0]);
    }

    #[test]
    fn an_authored_side_overrides_the_anchor_rule() {
        let requests = [LabelRequest { side: Some(LabelSide::Right), ..request(300.0, 500.0) }];
        assert_eq!(layout_labels(&FRAME, &requests)[0].side, LabelSide::Right);
    }

    #[test]
    fn close_anchors_are_pushed_apart_keeping_their_order() {
        let requests = [request(500.0, 600.0), request(520.0, 590.0), request(480.0, 605.0)];
        let placed = layout_labels(&FRAME, &requests);
        check(&FRAME, &requests, &placed);
        assert_eq!(placed.len(), 3);
        // By anchor y: request 1 (590), 0 (600), 2 (605).
        assert!(placed[1].rect[1] < placed[0].rect[1] && placed[0].rect[1] < placed[2].rect[1]);
    }

    #[test]
    fn labels_near_the_edges_are_kept_out_of_the_caption_band_and_the_key_box() {
        let requests = [request(1500.0, 20.0), request(100.0, 1190.0), request(100.0, 2.0)];
        let placed = layout_labels(&FRAME, &requests);
        check(&FRAME, &requests, &placed);
        assert_eq!(placed[0].rect[1], 16.0 + 150.0);
        assert_eq!(placed[1].rect[1], 1200.0 - 16.0 - 120.0 - 32.0);
        assert_eq!(placed[2].rect[1], 16.0);
    }

    #[test]
    fn anchors_off_screen_and_boxes_too_wide_are_left_out() {
        let wide = LabelRequest { size: [900.0, 30.0], ..request(500.0, 500.0) };
        let requests = [request(-5.0, 500.0), request(500.0, 1300.0), wide, request(f64::NAN, 3.0), request(500.0, 500.0)];
        let placed = layout_labels(&FRAME, &requests);
        assert_eq!(placed.iter().map(|p| p.request).collect::<Vec<_>>(), [4]);
    }

    #[test]
    fn a_full_column_drops_the_labels_set_last() {
        // 40 labels of 32 px with 8 px gaps need 1592 px; the left column has 1048.
        let requests: Vec<LabelRequest> = (0..40).map(|i| request(300.0, 100.0 + f64::from(i) * 5.0)).collect();
        let placed = layout_labels(&FRAME, &requests);
        check(&FRAME, &requests, &placed);
        let kept: Vec<usize> = placed.iter().map(|p| p.request).collect();
        assert_eq!(kept, (0..26).collect::<Vec<_>>(), "26 boxes fit (26 x 32 + 25 x 8 = 1032)");
    }

    #[test]
    fn random_scenes_keep_every_guarantee_and_are_deterministic() {
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        for _ in 0..500 {
            let count = (next() * 30.0) as usize;
            let requests: Vec<LabelRequest> = (0..count)
                .map(|_| LabelRequest {
                    anchor: [next() * 1700.0 - 50.0, next() * 1300.0 - 50.0],
                    size: [40.0 + next() * 300.0, 20.0 + next() * 40.0],
                    side: [None, Some(LabelSide::Left), Some(LabelSide::Right)][(next() * 3.0) as usize % 3],
                })
                .collect();
            let placed = layout_labels(&FRAME, &requests);
            check(&FRAME, &requests, &placed);
            assert_eq!(placed, layout_labels(&FRAME, &requests));
        }
    }
}
