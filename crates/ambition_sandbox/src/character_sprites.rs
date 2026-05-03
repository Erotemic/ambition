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
///
/// The boss has its own row set; see `boss_sprites::BossAnim`. Robot/goblin
/// share this 11-row layout.
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
    BlinkOut = 8,
    BlinkIn = 9,
    Dash = 10,
}

#[derive(Clone, Copy, Debug)]
pub struct AnimRow {
    pub frame_count: usize,
    pub duration_secs: f32,
}

/// Frame layout for one of the generated sheets.
///
/// Frames are 128×128 with a per-row label strip on the left whose width
/// differs between targets (robot 100, goblin 96). All rows have the same
/// vertical pitch but different frame counts (e.g. goblin slash is 7).
///
/// Tuning fields (`collision_scale`, `feet_anchor_y`, `frame_sample_inset`)
/// live per-spec so each target can be tuned without touching globals —
/// the prior version used module-level constants which forced identical
/// scale/anchor across robot and goblin even though their rendered bodies
/// occupy different fractions of the 128px frame.
#[derive(Clone, Copy, Debug)]
pub struct CharacterSheetSpec {
    pub label_width: u32,
    /// Per-frame width in source-image pixels. The generator now crops
    /// each sheet to the union of opaque-pixel bboxes across every frame,
    /// so this is *not* always 128 anymore — robot is 120, goblin 121.
    pub frame_width: u32,
    pub frame_height: u32,
    pub rows: [AnimRow; 11],
    /// Multiplier applied to the entity's collision-box max dimension to
    /// derive the rendered sprite's height. Width is derived from the
    /// cropped frame's aspect ratio so the character isn't squashed.
    pub collision_scale: f32,
    /// Sprite anchor y (normalized; negative shifts the sprite up so feet
    /// land near the collision-box bottom).
    pub feet_anchor_y: f32,
    /// Pixel inset on every URect to prevent bilinear filtering from
    /// pulling neighboring frame pixels at the seam.
    pub frame_sample_inset: u32,
}

// Frame counts, durations, label widths, and `feet_anchor_y` values are
// kept in sync with `tools/generators/gen2d` output. After regenerating
// sheets, mirror the new YAML headers + body_metrics here. When the
// runtime gains a YAML loader for the `body_metrics` field, these
// constants can be removed.

pub const ROBOT_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    // After the gen2d union-bbox crop the robot sheet is 120 wide × 128
    // tall (down from 128×128). Mirror that here.
    frame_width: 120,
    frame_height: 128,
    rows: [
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        }, // Idle
        AnimRow {
            frame_count: 8,
            duration_secs: 0.095,
        }, // Walk
        AnimRow {
            frame_count: 8,
            duration_secs: 0.075,
        }, // Run
        AnimRow {
            frame_count: 6,
            duration_secs: 0.095,
        }, // Jump
        AnimRow {
            frame_count: 6,
            duration_secs: 0.095,
        }, // Fall
        AnimRow {
            frame_count: 8,
            duration_secs: 0.075,
        }, // Slash
        AnimRow {
            frame_count: 5,
            duration_secs: 0.090,
        }, // Hit
        AnimRow {
            frame_count: 8,
            duration_secs: 0.110,
        }, // Death
        AnimRow {
            frame_count: 6,
            duration_secs: 0.062,
        }, // BlinkOut
        AnimRow {
            frame_count: 6,
            duration_secs: 0.062,
        }, // BlinkIn
        AnimRow {
            frame_count: 6,
            duration_secs: 0.065,
        }, // Dash
    ],
    collision_scale: 2.1,
    feet_anchor_y: -0.320,
    frame_sample_inset: 1,
};

pub const GOBLIN_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    // After the gen2d union-bbox crop the goblin sheet is 121×127.
    frame_width: 121,
    frame_height: 127,
    rows: [
        AnimRow {
            frame_count: 8,
            duration_secs: 0.120,
        }, // Idle
        AnimRow {
            frame_count: 8,
            duration_secs: 0.095,
        }, // Walk
        AnimRow {
            frame_count: 8,
            duration_secs: 0.075,
        }, // Run
        AnimRow {
            frame_count: 6,
            duration_secs: 0.095,
        }, // Jump
        AnimRow {
            frame_count: 6,
            duration_secs: 0.095,
        }, // Fall
        AnimRow {
            frame_count: 7,
            duration_secs: 0.075,
        }, // Slash (goblin: 7)
        AnimRow {
            frame_count: 5,
            duration_secs: 0.090,
        }, // Hit
        AnimRow {
            frame_count: 8,
            duration_secs: 0.110,
        }, // Death
        AnimRow {
            frame_count: 6,
            duration_secs: 0.062,
        }, // BlinkOut
        AnimRow {
            frame_count: 6,
            duration_secs: 0.062,
        }, // BlinkIn
        AnimRow {
            frame_count: 6,
            duration_secs: 0.065,
        }, // Dash
    ],
    collision_scale: 2.1,
    feet_anchor_y: -0.350,
    frame_sample_inset: 1,
};

/// Per-target sprite render size. The generator's character occupies only
/// part of the 128×128 frame, so the rendered quad must be larger than
/// the collision box for the visible body to roughly match the hitbox.
///
/// TODO(gen2d-collision-aware): teach the generator to write
/// `body_pixel_extent` + `feet_y_pixel` into the spritesheet YAML and
/// load them at runtime, replacing these per-spec constants with values
/// derived from each sheet's actual rendered body. The per-spec tuning
/// already isolates the override per target so the migration is local.
pub fn sprite_render_size(spec: CharacterSheetSpec, collision: Vec2) -> Vec2 {
    // Height is collision-driven; width preserves the cropped frame's
    // aspect ratio so the character isn't horizontally squashed when the
    // generator crop produces non-square frames (e.g. robot 120×128).
    let height = collision.x.max(collision.y).max(8.0) * spec.collision_scale;
    let width = height * (spec.frame_width as f32 / spec.frame_height as f32);
    Vec2::new(width, height)
}

/// Sprite anchor that places the character's feet near the bottom of the
/// collision box. Per-spec so each generator output can pick its own y.
pub fn feet_anchor_for(spec: CharacterSheetSpec) -> Anchor {
    Anchor(Vec2::new(0.0, spec.feet_anchor_y))
}

/// Back-compat default-anchor helper used at the player spawn site, which
/// still threads `ROBOT_SHEET` implicitly. Kept so existing call sites
/// don't need to plumb the spec just to fetch the anchor.
pub fn feet_anchor() -> Anchor {
    feet_anchor_for(ROBOT_SHEET)
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
    sprite.custom_size = Some(sprite_render_size(asset.spec, collision));
    sprite
}

impl CharacterSheetSpec {
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
                // Inset on every side so bilinear filtering at the frame
                // boundary cannot pull pixels from the next cell.
                let min = UVec2::new(x + inset, y + inset);
                let max = UVec2::new(x + self.frame_width - inset, y + self.frame_height - inset);
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
    // The boss uses the entity-sprite path (`EntitySprite::BossCore`) rather
    // than the character-spritesheet path: its generator emits non-standard
    // animation rows (rest/floor_slam/side_sweep/spike_halo/dash_echo/hit/
    // death) that don't fit `CharacterAnim`'s 8-variant grid. When/if the
    // boss gets a CharacterAnim-compatible sheet, add a `boss` field here.
}

impl CharacterSpriteAssets {
    pub fn enemy_asset(&self, kind: FeatureVisualKind) -> Option<&CharacterSpriteAsset> {
        match kind {
            FeatureVisualKind::Enemy | FeatureVisualKind::Sandbag => self.goblin.as_ref(),
            _ => None,
        }
    }
}

const ROBOT_FILENAME: &str = "robot_spritesheet.png";
const GOBLIN_FILENAME: &str = "goblin_spritesheet.png";

/// Probe the sandbox `assets/<sprite_folder>/` directory for spritesheets.
/// Missing files are not an error — callers fall back to colored rectangles.
pub fn load_character_sprites_in(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_folder: &str,
) -> CharacterSpriteAssets {
    let robot_rel = format!("{sprite_folder}/{ROBOT_FILENAME}");
    let goblin_rel = format!("{sprite_folder}/{GOBLIN_FILENAME}");

    let robot = build_optional(asset_server, layouts, &robot_rel, ROBOT_SHEET);
    let goblin = build_optional(asset_server, layouts, &goblin_rel, GOBLIN_SHEET);

    for (name, rel, present) in [
        ("robot", &robot_rel, robot.is_some()),
        ("goblin", &goblin_rel, goblin.is_some()),
    ] {
        if !present {
            eprintln!(
                "[character_sprites] {name} spritesheet not found at assets/{rel} — falling back to colored rectangle"
            );
        }
    }

    CharacterSpriteAssets { robot, goblin }
}

fn build_optional(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    rel_path: &str,
    spec: CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    if !asset_exists(rel_path) {
        return None;
    }
    let layout = layouts.add(spec.build_atlas());
    Some(CharacterSpriteAsset {
        texture: asset_server.load(rel_path.to_string()),
        layout,
        spec,
    })
}

fn asset_exists(rel_path: &str) -> bool {
    // Bevy's FileAssetReader resolves assets relative to CARGO_MANIFEST_DIR
    // when running through cargo. Mirror that here so the existence check
    // matches the asset server's lookup path.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .join("assets")
        .join(rel_path)
        .exists()
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
/// Priority: hit > slash > dash > airborne (jump/fall) > run/walk/idle.
/// Death is not represented yet — the player respawns instantly today.
/// `BlinkOut`/`BlinkIn` are not used yet because the runtime doesn't
/// track a per-blink anim window; once a `blink_anim_timer` is added
/// alongside `slash_anim_timer`, this function can switch on it.
pub fn pick_player_anim(runtime: &SandboxRuntime) -> CharacterAnim {
    if runtime.hitstun_timer > 0.05 {
        return CharacterAnim::Hit;
    }
    if runtime.slash_anim_timer > 0.0 {
        return CharacterAnim::Slash;
    }
    let player = &runtime.player;
    if player.dash_timer > 0.0 {
        return CharacterAnim::Dash;
    }
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
