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
    pub frame_size: u32,
    pub rows: [AnimRow; 7],
    /// Multiplier applied to an entity's collision-box max dimension to
    /// derive its square sprite render size. Bosses tend to be large so
    /// this is typically smaller than the character default.
    pub collision_scale: f32,
    /// Custom sprite anchor on the y axis. Tuned per sheet because each
    /// generator's character takes different vertical space within the
    /// 128px frame.
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
    frame_size: 128,
    rows: [
        AnimRow { frame_count: 8, duration_secs: 0.120 }, // Rest
        AnimRow { frame_count: 7, duration_secs: 0.082 }, // FloorSlam
        AnimRow { frame_count: 7, duration_secs: 0.072 }, // SideSweep
        AnimRow { frame_count: 8, duration_secs: 0.092 }, // SpikeHalo
        AnimRow { frame_count: 7, duration_secs: 0.062 }, // DashEcho
        AnimRow { frame_count: 5, duration_secs: 0.090 }, // Hit
        AnimRow { frame_count: 8, duration_secs: 0.110 }, // Death
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
        let total_w = self.label_width + max_frames * self.frame_size;
        let total_h = self.rows.len() as u32 * self.frame_size;
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w, total_h));
        let inset = self.frame_sample_inset.min(self.frame_size / 4);
        for (row_idx, row) in self.rows.iter().enumerate() {
            for col in 0..row.frame_count {
                let x = self.label_width + col as u32 * self.frame_size;
                let y = row_idx as u32 * self.frame_size;
                let min = UVec2::new(x + inset, y + inset);
                let max = UVec2::new(
                    x + self.frame_size - inset,
                    y + self.frame_size - inset,
                );
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
        let side = collision.x.max(collision.y).max(8.0) * self.collision_scale;
        Vec2::splat(side)
    }

    pub fn anchor(&self) -> Anchor {
        Anchor(Vec2::new(0.0, self.feet_anchor_y))
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
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    if !Path::new(manifest_dir).join("assets").join(&rel).exists() {
        eprintln!(
            "[boss_sprites] boss spritesheet not found at assets/{rel} — falling back to entity sprite (boss_core.png)"
        );
        return None;
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
