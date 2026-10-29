//! The standing surfaces a sheet publishes, as the simulation reads them
//! ([`ambition_characters::actor::SurfaceTable`]).
//!
//! A surface is two anchors of each frame of a row: `surface:<name>:a` and
//! `surface:<name>:b` ([`SURFACE_ANCHOR`]), its left end first for art that
//! faces right, as parts of the frame (0 to 1), as the portal seam anchors
//! are (`seam:<slot>:a` / `:b`). A frame that does not state both ends has no
//! such surface. Unlike a seam, a surface is a SIMULATION input: the sheet's
//! mechanical projection carries it (`sheet_mechanics`).

use std::collections::BTreeMap;

use ambition_characters::actor::{SurfaceRow, SurfaceTable};
use bevy::math::Vec2;

use crate::character::CharacterAnim;
use crate::SheetRecord;

/// The prefix of a frame anchor that is an end of a standing surface.
pub const SURFACE_ANCHOR: &str = "surface:";

/// The name of the surface an anchor key is an end of, and whether it is the
/// `a` end. `None` for an anchor that is not a surface end.
pub fn surface_end(key: &str) -> Option<(&str, bool)> {
    let rest = key.strip_prefix(SURFACE_ANCHOR)?;
    if let Some(name) = rest.strip_suffix(":a") {
        return Some((name, true));
    }
    rest.strip_suffix(":b").map(|name| (name, false))
}

/// The standing surfaces sheet `key` publishes, in world units about the
/// centre of its authored body box at `world_per_pixel`, or `None` when the
/// sheet publishes none (or has no authored body box to place them from).
pub fn published_surfaces(content: &crate::SpriteContent, key: &str, world_per_pixel: f32) -> Option<SurfaceTable> {
    surfaces_of(crate::character::sheets::record_for_sheet_key(&content, key)?, world_per_pixel)
}

/// [`published_surfaces`] of one record.
pub fn surfaces_of(record: &SheetRecord, world_per_pixel: f32) -> Option<SurfaceTable> {
    let body = record.body_metrics.as_ref()?.body_pixel_bbox?;
    let centre = Vec2::new(body.x as f32 + body.w as f32 * 0.5, body.y as f32 + body.h as f32 * 0.5);
    let frame = Vec2::new(record.frame_width as f32, record.frame_height as f32);
    let place = |x: f32, y: f32| (Vec2::new(x, y) * frame - centre) * world_per_pixel;
    let mut rows = BTreeMap::new();
    for row in record.rows.iter().filter(|row| row.mirror_of.is_none()) {
        let frames: Vec<BTreeMap<String, [Vec2; 2]>> = row
            .rects
            .iter()
            .map(|rect| {
                let mut ends: BTreeMap<&str, [Option<Vec2>; 2]> = BTreeMap::new();
                for (key, at) in &rect.anchors {
                    if let Some((name, a)) = surface_end(key) {
                        ends.entry(name).or_default()[usize::from(!a)] = Some(place(at.x, at.y));
                    }
                }
                ends.into_iter()
                    .filter_map(|(name, [a, b])| Some((name.to_string(), [a?, b?])))
                    .collect()
            })
            .collect();
        if frames.iter().any(|frame| !frame.is_empty()) {
            rows.insert(row.animation.clone(), SurfaceRow { frame_duration_s: row.duration_secs, frames });
        }
    }
    if rows.is_empty() {
        return None;
    }
    let rest_row = record
        .rows
        .iter()
        .map(|row| row.animation.as_str())
        .find(|name| rows.contains_key(*name) && CharacterAnim::from_name(name) == Some(CharacterAnim::Idle))
        .map(str::to_string);
    Some(SurfaceTable { rows, rest_row })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_surface_is_read_from_its_two_anchors_about_the_body_centre() {
        let text = r#"[(target: "giant", image: "g.png", label_width: 0, frame_width: 100, frame_height: 50,
            body_metrics: Some((body_pixel_bbox: Some((x: 40, y: 20, w: 20, h: 20)), authored_body: true)),
            rows: [(animation: "rest", row_index: 0, frame_count: 2, duration_ms: 100, duration_secs: 0.1,
                rects: [
                    (x: 0, y: 0, w: 100, h: 50, anchors: {"surface:deck:a": (x: 0.4, y: 0.4), "surface:deck:b": (x: 0.6, y: 0.4),
                                                          "seam:hand:a": (x: 0.1, y: 0.1)}),
                    (x: 0, y: 0, w: 100, h: 50, anchors: {"surface:deck:a": (x: 0.4, y: 0.2)}),
                ])])]"#;
        let records: Vec<SheetRecord> = ron::from_str(text).expect("the fixture parses");
        let table = surfaces_of(&records[0], 2.0).expect("the sheet publishes a surface");
        let row = &table.rows["rest"];
        // (40, 20) and (60, 20) in pixels; the centre is (50, 30).
        let [a, b] = row.frames[0]["deck"];
        assert!(a.distance(Vec2::new(-20.0, -20.0)) < 1e-3 && b.distance(Vec2::new(20.0, -20.0)) < 1e-3, "{a} {b}");
        // A frame with one end has no surface.
        assert!(row.frames[1].is_empty());
        assert_eq!(table.rest_row.as_deref(), Some("rest"));
    }
}
