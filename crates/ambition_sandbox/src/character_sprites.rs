//! Sprite-sheet rendering for the player robot and goblin enemies.
//!
//! Sheets are produced by `tools/generators/gen2d` and copied into
//! `assets/sprites/`. If a PNG is missing at startup the corresponding
//! `Option` stays `None` and callers fall back to the colored-rectangle
//! visuals that predate this module — the game must always run.

use std::path::Path;

use ambition_engine as ae;
use bevy::math::URect;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::features::FeatureVisualKind;
use crate::SandboxRuntime;

/// Animation rows on each character sheet, in the order the generator emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterAnim {
    Idle = 0,
    Walk = 1,
    Run = 2,
    Jump = 3,
    Fall = 4,
    Slash = 5,
    Hit = 6,
    Death = 7,
}

#[derive(Clone, Copy, Debug)]
pub struct AnimRow {
    pub frame_count: usize,
    pub duration_secs: f32,
}

/// Frame layout for one of the generated sheets.
///
/// Frames are 128×128 with a per-row label strip on the left whose width
/// differs between targets (robot 100, goblin 96). All eight rows have the
/// same vertical pitch but different frame counts (e.g. goblin slash is 7).
#[derive(Clone, Copy, Debug)]
pub struct CharacterSheetSpec {
    pub label_width: u32,
    pub frame_size: u32,
    pub rows: [AnimRow; 8],
}

pub const ROBOT_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    frame_size: 128,
    rows: [
        AnimRow { frame_count: 8, duration_secs: 0.120 }, // Idle
        AnimRow { frame_count: 8, duration_secs: 0.095 }, // Walk
        AnimRow { frame_count: 8, duration_secs: 0.075 }, // Run
        AnimRow { frame_count: 6, duration_secs: 0.095 }, // Jump
        AnimRow { frame_count: 6, duration_secs: 0.095 }, // Fall
        AnimRow { frame_count: 8, duration_secs: 0.075 }, // Slash
        AnimRow { frame_count: 5, duration_secs: 0.090 }, // Hit
        AnimRow { frame_count: 8, duration_secs: 0.110 }, // Death
    ],
};

pub const GOBLIN_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 96,
    frame_size: 128,
    rows: [
        AnimRow { frame_count: 8, duration_secs: 0.120 }, // Idle
        AnimRow { frame_count: 8, duration_secs: 0.095 }, // Walk
        AnimRow { frame_count: 8, duration_secs: 0.075 }, // Run
        AnimRow { frame_count: 6, duration_secs: 0.095 }, // Jump
        AnimRow { frame_count: 6, duration_secs: 0.095 }, // Fall
        AnimRow { frame_count: 7, duration_secs: 0.075 }, // Slash (goblin: 7)
        AnimRow { frame_count: 5, duration_secs: 0.090 }, // Hit
        AnimRow { frame_count: 8, duration_secs: 0.110 }, // Death
    ],
};

/// Multiplier applied to an entity's collision-box max dimension to derive
/// its square sprite render size. The generator's character occupies only
/// part of the 128×128 frame, so the rendered quad must be larger than the
/// collision box for the visible body to roughly match the hitbox.
///
/// LIMITATION: this is a heuristic. A more polished pipeline would have
/// `tools/generators/gen2d` emit per-target metadata that pins the
/// character footprint inside the frame (feet pixel, height ratio) and
/// then compose the sprite at the exact collision-aware size at gameplay
/// time. Until then, callers can tweak `SPRITE_COLLISION_SCALE` and
/// `FEET_ANCHOR_Y` if the sprites look off-center.
///
/// TODO(gen2d-collision-aware): teach the generator to write a sidecar
/// (e.g. `*_spritesheet.metrics.yaml`) carrying `feet_y_pixel`,
/// `body_height_pixel`, `body_width_pixel`. Replace this constant + the
/// `FEET_ANCHOR_Y` constant with values derived from the metrics so
/// every archetype lines up with its hitbox without per-target tweaks.
pub const SPRITE_COLLISION_SCALE: f32 = 2.1;

/// Custom sprite anchor on the y axis. `0.0` is the sprite center;
/// negative values place the anchor below center, which shifts the
/// rendered sprite UP so the character's feet land near the collision
/// box's bottom edge instead of below it. Tuned to the procgen
/// character lab's roughly-centered output where feet sit ~10% above
/// the frame bottom.
pub const FEET_ANCHOR_Y: f32 = -0.18;

/// Sample inset (pixels) applied to every frame URect when building the
/// atlas. Without this inset, bilinear filtering at the frame edges
/// samples the neighboring frame and produces faint colored seams that
/// flicker as the animation advances. 1px is enough at the current
/// 128px frame size and bilinear filter; bump if seams reappear after
/// switching filtering modes.
pub const FRAME_SAMPLE_INSET: u32 = 1;

/// Compute the square render size for a sprite given an entity's
/// collision box.
pub fn sprite_render_size(collision: Vec2) -> Vec2 {
    let side = collision.x.max(collision.y).max(8.0) * SPRITE_COLLISION_SCALE;
    Vec2::splat(side)
}

/// Sprite anchor that places the character's feet near the bottom of the
/// collision box for the procgen character lab's frame layout.
pub fn feet_anchor() -> Anchor {
    Anchor(Vec2::new(0.0, FEET_ANCHOR_Y))
}

/// Build the textured sprite for a character given its collision-box size.
/// The sprite is square (the source frames are 128×128, distortion would
/// look bad), so any non-square hitbox uses the larger axis for sizing.
pub fn build_character_sprite(asset: &CharacterSpriteAsset, collision: Vec2) -> Sprite {
    let mut sprite = Sprite::from_atlas_image(
        asset.texture.clone(),
        bevy::image::TextureAtlas {
            layout: asset.layout.clone(),
            index: asset.spec.flat_index(CharacterAnim::Idle, 0),
        },
    );
    sprite.custom_size = Some(sprite_render_size(collision));
    sprite
}

impl CharacterSheetSpec {
    pub fn build_atlas(&self) -> TextureAtlasLayout {
        let max_frames = self.rows.iter().map(|r| r.frame_count).max().unwrap_or(0) as u32;
        let total_w = self.label_width + max_frames * self.frame_size;
        let total_h = self.rows.len() as u32 * self.frame_size;
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w, total_h));
        let inset = FRAME_SAMPLE_INSET.min(self.frame_size / 4);
        for (row_idx, row) in self.rows.iter().enumerate() {
            for col in 0..row.frame_count {
                let x = self.label_width + col as u32 * self.frame_size;
                let y = row_idx as u32 * self.frame_size;
                // Inset on every side so bilinear filtering at the frame
                // boundary cannot pull pixels from the next cell.
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

    pub fn flat_index(&self, anim: CharacterAnim, frame: usize) -> usize {
        let row = anim as usize;
        let frames_before: usize = self.rows[..row].iter().map(|r| r.frame_count).sum();
        let max_frame = self.rows[row].frame_count.saturating_sub(1);
        frames_before + frame.min(max_frame)
    }

    pub fn frame_count(&self, anim: CharacterAnim) -> usize {
        self.rows[anim as usize].frame_count
    }

    pub fn frame_duration(&self, anim: CharacterAnim) -> f32 {
        self.rows[anim as usize].duration_secs
    }
}

#[derive(Clone)]
pub struct CharacterSpriteAsset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub spec: CharacterSheetSpec,
}

/// Holds optional spritesheet handles. `None` = file missing → fallback.
#[derive(Resource, Default, Clone)]
pub struct CharacterSpriteAssets {
    pub robot: Option<CharacterSpriteAsset>,
    pub goblin: Option<CharacterSpriteAsset>,
}

impl CharacterSpriteAssets {
    pub fn enemy_asset(&self, kind: FeatureVisualKind) -> Option<&CharacterSpriteAsset> {
        match kind {
            FeatureVisualKind::Enemy | FeatureVisualKind::Sandbag => self.goblin.as_ref(),
            _ => None,
        }
    }
}

pub const ROBOT_SPRITE_PATH: &str = "sprites/robot_spritesheet.png";
pub const GOBLIN_SPRITE_PATH: &str = "sprites/goblin_spritesheet.png";

/// Probe the sandbox `assets/` directory for the spritesheet PNGs. Missing
/// files are not an error — callers fall back to colored rectangles.
pub fn load_character_sprites(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> CharacterSpriteAssets {
    let robot = build_optional(asset_server, layouts, ROBOT_SPRITE_PATH, ROBOT_SHEET);
    let goblin = build_optional(asset_server, layouts, GOBLIN_SPRITE_PATH, GOBLIN_SHEET);

    if robot.is_none() {
        eprintln!(
            "[character_sprites] robot spritesheet not found at assets/{ROBOT_SPRITE_PATH} \
             — falling back to colored rectangle"
        );
    }
    if goblin.is_none() {
        eprintln!(
            "[character_sprites] goblin spritesheet not found at assets/{GOBLIN_SPRITE_PATH} \
             — falling back to colored rectangle"
        );
    }

    CharacterSpriteAssets { robot, goblin }
}

fn build_optional(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    rel_path: &'static str,
    spec: CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    if !asset_exists(rel_path) {
        return None;
    }
    let layout = layouts.add(spec.build_atlas());
    Some(CharacterSpriteAsset {
        texture: asset_server.load(rel_path),
        layout,
        spec,
    })
}

fn asset_exists(rel_path: &str) -> bool {
    // Bevy's FileAssetReader resolves assets relative to CARGO_MANIFEST_DIR
    // when running through cargo. Mirror that here so the existence check
    // matches the asset server's lookup path.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("assets").join(rel_path).exists()
}

/// Per-character animation cursor.
#[derive(Component)]
pub struct CharacterAnimator {
    pub spec: CharacterSheetSpec,
    pub current: CharacterAnim,
    pub frame: usize,
    pub elapsed: f32,
    /// Once a non-looping clip (Slash/Hit/Death) finishes its last frame
    /// we hold there until `set` switches to a new animation.
    pub clip_held: bool,
}

impl CharacterAnimator {
    pub fn new(spec: CharacterSheetSpec) -> Self {
        Self {
            spec,
            current: CharacterAnim::Idle,
            frame: 0,
            elapsed: 0.0,
            clip_held: false,
        }
    }

    pub fn request(&mut self, anim: CharacterAnim) {
        if self.current == anim {
            return;
        }
        self.current = anim;
        self.frame = 0;
        self.elapsed = 0.0;
        self.clip_held = false;
    }

    /// Advance the animation. Returns the flat atlas index for the current frame.
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

fn non_looping(anim: CharacterAnim) -> bool {
    matches!(
        anim,
        CharacterAnim::Slash | CharacterAnim::Hit | CharacterAnim::Death
    )
}

/// Pick the player's animation from runtime state.
///
/// Priority: hit > slash > airborne (jump/fall) > run/walk/idle. Death is
/// not represented yet — the player respawns instantly today, so there is
/// no on-entity death window to animate.
pub fn pick_player_anim(runtime: &SandboxRuntime) -> CharacterAnim {
    if runtime.hitstun_timer > 0.05 {
        return CharacterAnim::Hit;
    }
    if runtime.slash_anim_timer > 0.0 {
        return CharacterAnim::Slash;
    }
    let player = &runtime.player;
    if !player.on_ground {
        // Engine uses top-left coords: vel.y < 0 = moving up.
        if player.vel.y < -10.0 {
            return CharacterAnim::Jump;
        }
        return CharacterAnim::Fall;
    }
    let speed = player.vel.x.abs();
    if speed < 12.0 {
        CharacterAnim::Idle
    } else if speed < 220.0 {
        CharacterAnim::Walk
    } else {
        CharacterAnim::Run
    }
}

/// Snapshot of an enemy's per-frame state used to drive its animation.
#[derive(Clone, Copy, Debug)]
pub struct EnemyAnimState {
    pub vel: ae::Vec2,
    pub facing: f32,
    pub alive: bool,
    pub attack_active: bool,
    pub attack_windup: bool,
    pub hit_flash: bool,
}

pub fn pick_enemy_anim(state: EnemyAnimState) -> CharacterAnim {
    if !state.alive {
        return CharacterAnim::Death;
    }
    if state.hit_flash {
        return CharacterAnim::Hit;
    }
    if state.attack_active || state.attack_windup {
        return CharacterAnim::Slash;
    }
    if state.vel.x.abs() > 8.0 {
        CharacterAnim::Walk
    } else {
        CharacterAnim::Idle
    }
}
