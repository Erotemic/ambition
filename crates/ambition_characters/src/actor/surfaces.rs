//! Standing surfaces a body's art publishes, per row and frame: a place a
//! body can stand that moves as the art moves (the top of a giant's shoulder
//! that leans with his torso).
//!
//! This is SIMULATION data, as a landmark table is ([`super::landmarks`]). The
//! sheet publishes each surface as two named anchors of each frame
//! (`surface:<name>:a` and `surface:<name>:b`, its left end first for art that
//! faces right); `ambition_sprite_sheet` reads them into a [`SurfaceTable`] at
//! the scale the body is drawn at.
//!
//! Space: world units about the body's CENTRE (the centre of its collision
//! box), +x the way the art faces, +y DOWN.

use std::collections::BTreeMap;

use bevy::math::Vec2;

/// A surface in one frame: its two ends, the left one first for art that
/// faces right.
pub type SurfaceSegment = [Vec2; 2];

/// One row of a [`SurfaceTable`]: each frame's surfaces, by name.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceRow {
    pub frame_duration_s: f32,
    pub frames: Vec<BTreeMap<String, SurfaceSegment>>,
}

/// The standing surfaces one sheet publishes, per row and frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurfaceTable {
    pub rows: BTreeMap<String, SurfaceRow>,
    /// The row the body is drawn in when nothing pins one (its idle row).
    pub rest_row: Option<String>,
}

impl SurfaceTable {
    /// Surface `name` at `elapsed_s` seconds into the first of `rows` this
    /// table has (the rest row when `rows` is empty or names none of them).
    ///
    /// The sheet draws each frame for its length (a looping row wraps, a
    /// one-shot holds its last frame). The surface is that frame's in the
    /// MIDDLE of the frame's time, and goes on smoothly from the middle of
    /// one frame to the middle of the next: a body that stands on it is
    /// carried with no jump, and is never more than half a frame's step from
    /// the surface that is drawn. `None` when the row has no such surface.
    pub fn segment(&self, rows: &[String], elapsed_s: f32, looping: bool, name: &str) -> Option<SurfaceSegment> {
        let row = rows
            .iter()
            .find_map(|row| self.rows.get(row))
            .or_else(|| self.rest_row.as_ref().and_then(|rest| self.rows.get(rest)))?;
        let count = row.frames.len();
        if count == 0 {
            return None;
        }
        // Frames from the middle of the first.
        let at = elapsed_s.max(0.0) / row.frame_duration_s.max(f32::EPSILON) - 0.5;
        let at = if at.is_finite() { at } else { 0.0 };
        let at = if looping { at.rem_euclid(count as f32) } else { at.max(0.0) };
        let step = at.floor();
        let part = at - step;
        let step = step as usize;
        let (this, next, part) = if looping {
            (step % count, (step + 1) % count, part)
        } else if step + 1 >= count {
            (count - 1, count - 1, 0.0)
        } else {
            (step, step + 1, part)
        };
        let here = *row.frames[this].get(name)?;
        let Some(there) = row.frames[next].get(name) else {
            return Some(here);
        };
        Some([here[0].lerp(there[0], part), here[1].lerp(there[1], part)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> SurfaceTable {
        let frame = |y: f32| BTreeMap::from([("deck".to_string(), [Vec2::new(0.0, y), Vec2::new(10.0, y)])]);
        SurfaceTable {
            rows: BTreeMap::from([(
                "rest".to_string(),
                SurfaceRow { frame_duration_s: 0.1, frames: vec![frame(0.0), frame(-10.0), frame(-20.0)] },
            )]),
            rest_row: Some("rest".to_string()),
        }
    }

    #[test]
    fn a_surface_moves_on_toward_the_next_frame_and_wraps_or_holds() {
        let t = table();
        let y = |rows: &[String], s: f32, looping: bool| t.segment(rows, s, looping, "deck").map(|seg| seg[0].y);
        let rest = ["rest".to_string()];
        let near = |a: Option<f32>, b: f32| a.is_some_and(|a| (a - b).abs() < 1e-4);
        // In the middle of a frame, the frame's surface.
        assert!(near(y(&rest, 0.05, false), 0.0));
        assert!(near(y(&rest, 0.15, false), -10.0));
        // At the change of frames, half way from one to the next.
        assert!(near(y(&rest, 0.1, false), -5.0));
        // A one-shot starts on its first frame and holds its last.
        assert!(near(y(&rest, 0.0, false), 0.0));
        assert!(near(y(&rest, 0.29, false), -20.0));
        assert!(near(y(&rest, 9.0, false), -20.0));
        // A loop goes on from its last frame to its first.
        assert!(near(y(&rest, 0.3, true), -10.0));
        assert!(near(y(&rest, 0.0, true), -10.0));
        // No row pinned: the rest row.
        assert!(near(y(&[], 0.05, true), 0.0));
        assert_eq!(t.segment(&rest, 0.0, true, "other"), None);
    }
}
