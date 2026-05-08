//! Boss spritesheet animation, parallel to `character_sprites` but with the
//! boss generator's own animation rows (rest / floor_slam / side_sweep /
//! spike_halo / dash_echo / hit / death) instead of the standard 8-row
//! `CharacterAnim` grid.
//!
//! Bosses don't walk/run/jump like a platforming character, so reusing the
//! `CharacterAnim` enum would either force the boss generator to emit
//! placeholder rows or force the gameplay layer to mis-label its
//! animations. The split keeps both clean and makes it obvious which sheet
//! a given pipeline expects.

#[cfg(not(target_os = "android"))]
use std::path::Path;

use bevy::math::URect;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::features::FeatureVisualKind;

/// Boss animation rows in the order the generator emits them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossAnim {
    Rest = 0,
    FloorSlam = 1,
    SideSweep = 2,
    SpikeHalo = 3,
    DashEcho = 4,
    Hit = 5,
    Death = 6,
}

#[derive(Clone, Copy, Debug)]
pub struct AnimRow {
    pub frame_count: usize,
    pub duration_secs: f32,
}

/// Frame layout for a boss sheet. Mirror of `character_sprites::CharacterSheetSpec`
/// but with 7 rows and per-target anchor/scale tuning so bosses can render at
/// a different size than playable characters.
#[derive(Clone, Copy, Debug)]
pub struct BossSheetSpec {
    pub label_width: u32,
    /// Per-frame size in source-image pixels after the gen2d union-bbox
    /// crop. Boss currently fills its 128×128 canvas (spike_halo radial
    /// pose), so the crop is a no-op for now; a future tighter pose set
    /// would shrink these values.
    pub frame_width: u32,
    pub frame_height: u32,
    pub rows: [AnimRow; 7],
    /// Multiplier applied to an entity's collision-box max dimension to
    /// derive the rendered sprite's height. Width is derived from the
    /// cropped frame's aspect ratio so the boss isn't squashed.
    pub collision_scale: f32,
    /// Custom sprite anchor on the y axis. Tuned per sheet because each
    /// generator's character takes different vertical space within the
    /// frame.
    pub feet_anchor_y: f32,
    /// Sample inset (pixels) on every URect to prevent bilinear filtering
    /// from sampling neighboring frames.
    pub frame_sample_inset: u32,
}

// `feet_anchor_y` matches the body-metrics measurement for the current
// generator output. Resync after regenerating the boss sheet by checking
// the manifest's `body_metrics.feet_anchor_norm.y`.
pub const BOSS_SHEET: BossSheetSpec = BossSheetSpec {
    label_width: 100,
    frame_width: 128,
    frame_height: 128,
    rows: [
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        }, // Rest
        AnimRow {
            frame_count: 7,
            duration_secs: 0.082,
        }, // FloorSlam
        AnimRow {
            frame_count: 7,
            duration_secs: 0.072,
        }, // SideSweep
        AnimRow {
            frame_count: 8,
            duration_secs: 0.092,
        }, // SpikeHalo
        AnimRow {
            frame_count: 7,
            duration_secs: 0.062,
        }, // DashEcho
        AnimRow {
            frame_count: 5,
            duration_secs: 0.090,
        }, // Hit
        AnimRow {
            frame_count: 8,
            duration_secs: 0.110,
        }, // Death
    ],
    // Bosses are visually larger than goblins; a slightly smaller scale
    // factor stops them from overpowering the rendered scene.
    collision_scale: 1.6,
    feet_anchor_y: -0.336,
    frame_sample_inset: 1,
};

impl BossSheetSpec {
    pub fn build_atlas(&self) -> TextureAtlasLayout {
        let max_frames = self.rows.iter().map(|r| r.frame_count).max().unwrap_or(0) as u32;
        let total_w = self.label_width + max_frames * self.frame_width;
        let total_h = self.rows.len() as u32 * self.frame_height;
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w, total_h));
        let inset = self
            .frame_sample_inset
            .min(self.frame_width.min(self.frame_height) / 4);
        for (row_idx, row) in self.rows.iter().enumerate() {
            for col in 0..row.frame_count {
                let x = self.label_width + col as u32 * self.frame_width;
                let y = row_idx as u32 * self.frame_height;
                let min = UVec2::new(x + inset, y + inset);
                let max = UVec2::new(x + self.frame_width - inset, y + self.frame_height - inset);
                layout.add_texture(URect { min, max });
            }
        }
        layout
    }

    pub fn flat_index(&self, anim: BossAnim, frame: usize) -> usize {
        let row = anim as usize;
        let frames_before: usize = self.rows[..row].iter().map(|r| r.frame_count).sum();
        let max_frame = self.rows[row].frame_count.saturating_sub(1);
        frames_before + frame.min(max_frame)
    }

    pub fn frame_count(&self, anim: BossAnim) -> usize {
        self.rows[anim as usize].frame_count
    }

    pub fn render_size(&self, collision: Vec2) -> Vec2 {
        // Height collision-driven; width preserves the cropped frame's
        // aspect ratio so the boss isn't squashed when frames are non-square.
        let height = collision.x.max(collision.y).max(8.0) * self.collision_scale;
        let width = height * (self.frame_width as f32 / self.frame_height as f32);
        Vec2::new(width, height)
    }

    pub fn anchor(&self) -> Anchor {
        Anchor(Vec2::new(0.0, self.feet_anchor_y))
    }

    /// Collision-aware anchor that places the rendered boss's feet on the
    /// bottom of the collision box rather than at its centre. Mirrors
    /// `character_sprites::feet_anchor_for` — see that function for the
    /// derivation.
    pub fn collision_anchor(&self, collision: Vec2) -> Anchor {
        let render_height = collision.x.max(collision.y).max(8.0) * self.collision_scale;
        let half_collision_y = collision.y * 0.5;
        let ay = self.feet_anchor_y + half_collision_y / render_height;
        Anchor(Vec2::new(0.0, ay))
    }
}

#[derive(Clone)]
pub struct BossSpriteAsset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub spec: BossSheetSpec,
}

const BOSS_FILENAME: &str = "boss_spritesheet.png";

/// Build the boss sprite asset. Returns `None` if the PNG isn't on disk —
/// callers fall back to the static `EntitySprite::BossCore` image, which in
/// turn falls back to the colored rectangle.
pub fn load_boss_sprite_in(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_folder: &str,
) -> Option<BossSpriteAsset> {
    let rel = format!("{sprite_folder}/{BOSS_FILENAME}");
    #[cfg(not(target_os = "android"))]
    {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        if !Path::new(manifest_dir).join("assets").join(&rel).exists() {
            eprintln!(
                "[boss_sprites] boss spritesheet not found at assets/{rel} — falling back to entity sprite (boss_core.png)"
            );
            return None;
        }
    }
    let layout = layouts.add(BOSS_SHEET.build_atlas());
    Some(BossSpriteAsset {
        texture: asset_server.load(rel),
        layout,
        spec: BOSS_SHEET,
    })
}

/// Per-entity boss animation cursor. Same shape as `CharacterAnimator` but
/// keyed off `BossAnim`.
#[derive(Component)]
pub struct BossAnimator {
    pub spec: BossSheetSpec,
    pub current: BossAnim,
    pub frame: usize,
    pub elapsed: f32,
    pub clip_held: bool,
}

impl BossAnimator {
    pub fn new(spec: BossSheetSpec) -> Self {
        Self {
            spec,
            current: BossAnim::Rest,
            frame: 0,
            elapsed: 0.0,
            clip_held: false,
        }
    }

    pub fn request(&mut self, anim: BossAnim) {
        if self.current == anim {
            return;
        }
        self.current = anim;
        self.frame = 0;
        self.elapsed = 0.0;
        self.clip_held = false;
    }

    pub fn tick(&mut self, dt: f32) -> usize {
        let row = self.spec.rows[self.current as usize];
        if row.frame_count == 0 || row.duration_secs <= 0.0 {
            return self.spec.flat_index(self.current, self.frame);
        }
        if self.clip_held {
            return self.spec.flat_index(self.current, self.frame);
        }
        self.elapsed += dt;
        while self.elapsed >= row.duration_secs {
            self.elapsed -= row.duration_secs;
            if self.frame + 1 >= row.frame_count {
                if non_looping(self.current) {
                    self.frame = row.frame_count - 1;
                    self.clip_held = true;
                    break;
                } else {
                    self.frame = 0;
                }
            } else {
                self.frame += 1;
            }
        }
        self.spec.flat_index(self.current, self.frame)
    }
}

fn non_looping(anim: BossAnim) -> bool {
    matches!(
        anim,
        BossAnim::FloorSlam
            | BossAnim::SideSweep
            | BossAnim::SpikeHalo
            | BossAnim::DashEcho
            | BossAnim::Hit
            | BossAnim::Death
    )
}

/// Snapshot of boss state used to drive its animation. Pulled from
/// `BossRuntime` by the rendering layer so this module stays free of
/// gameplay imports.
#[derive(Clone, Copy, Debug)]
pub struct BossAnimState {
    pub alive: bool,
    pub attack_active: bool,
    pub attack_windup: bool,
    pub hit_flash: bool,
    /// Boss-pattern timer used to vary which active-attack clip plays.
    pub pattern_timer: f32,
}

pub fn pick_boss_anim(state: BossAnimState) -> BossAnim {
    if !state.alive {
        return BossAnim::Death;
    }
    if state.hit_flash {
        return BossAnim::Hit;
    }
    if state.attack_windup {
        return BossAnim::SpikeHalo;
    }
    if state.attack_active {
        // Rotate active-attack clips so the boss reads with variety even
        // though the gameplay AI is currently a single pattern.
        let bucket = (state.pattern_timer.abs() as i32) % 3;
        return match bucket {
            0 => BossAnim::FloorSlam,
            1 => BossAnim::SideSweep,
            _ => BossAnim::DashEcho,
        };
    }
    BossAnim::Rest
}

/// True if a feature kind is "the boss". Kept here so the rendering layer
/// can ask `BossSprites::should_animate(kind)` without inlining the match.
pub fn is_boss_kind(kind: FeatureVisualKind) -> bool {
    matches!(kind, FeatureVisualKind::Boss)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_sheet_has_seven_animation_rows() {
        // The enum has 7 variants and the spec has 7 rows; if these
        // ever drift, indexing by `anim as usize` would panic at
        // runtime.
        assert_eq!(BOSS_SHEET.rows.len(), 7);
    }

    #[test]
    fn frame_count_matches_spec_rows() {
        assert_eq!(BOSS_SHEET.frame_count(BossAnim::Rest), 8);
        assert_eq!(BOSS_SHEET.frame_count(BossAnim::FloorSlam), 7);
        assert_eq!(BOSS_SHEET.frame_count(BossAnim::Death), 8);
    }

    #[test]
    fn flat_index_lays_rows_end_to_end() {
        // First frame of each row sits at the cumulative sum of prior
        // frame counts. The first row starts at 0.
        assert_eq!(BOSS_SHEET.flat_index(BossAnim::Rest, 0), 0);
        assert_eq!(BOSS_SHEET.flat_index(BossAnim::FloorSlam, 0), 8);
        assert_eq!(BOSS_SHEET.flat_index(BossAnim::SideSweep, 0), 8 + 7);
        assert_eq!(BOSS_SHEET.flat_index(BossAnim::SpikeHalo, 0), 8 + 7 + 7);
    }

    #[test]
    fn flat_index_clamps_to_last_frame_of_row() {
        // Asking for frame index past the end of a row clamps to the
        // last valid frame; this avoids out-of-bounds atlas reads when
        // an animation cursor overshoots due to a long delta-t.
        let last_rest = BOSS_SHEET.flat_index(BossAnim::Rest, 999);
        assert_eq!(last_rest, BOSS_SHEET.frame_count(BossAnim::Rest) - 1);
    }

    #[test]
    fn render_size_preserves_frame_aspect_ratio() {
        // BOSS_SHEET is 128x128 (square) → width / height = 1.
        let size = BOSS_SHEET.render_size(Vec2::new(50.0, 50.0));
        assert!((size.x - size.y).abs() < 1e-3);
    }

    #[test]
    fn render_size_floors_at_minimum_extent() {
        // collision_scale * max(min_extent, 8.0): collision smaller
        // than 8 should still produce a visible quad.
        let size = BOSS_SHEET.render_size(Vec2::new(2.0, 2.0));
        assert!(size.y >= 8.0 * BOSS_SHEET.collision_scale - 1e-3);
    }

    #[test]
    fn is_boss_kind_only_true_for_boss_variant() {
        assert!(is_boss_kind(FeatureVisualKind::Boss));
        assert!(!is_boss_kind(FeatureVisualKind::Enemy));
        assert!(!is_boss_kind(FeatureVisualKind::Hazard));
        assert!(!is_boss_kind(FeatureVisualKind::Chest));
    }
}
