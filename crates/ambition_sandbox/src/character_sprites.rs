//! Sprite-sheet rendering for the player robot and goblin enemies.
//!
//! All character sheets (player robot, goblins, sandbag, boss) are
//! produced by `tools/ambition_sprite2d_renderer` and copied into
//! `assets/sprites/`. If a PNG is missing at startup the corresponding
//! `Option` stays `None` and callers fall back to the colored-rectangle
//! visuals that predate this module — the game must always run.

use ambition_engine as ae;
use bevy::math::URect;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::features::FeatureVisualKind;
use crate::SandboxRuntime;

/// Animation ids that a character sheet may define.
///
/// The boss has its own row set; see `boss_sprites::BossAnim`. A sheet no
/// longer has to contain every row here: `CharacterSheetSpec` maps unsupported
/// animation requests back to `Idle`, so simple characters can emit only their
/// relevant rows.
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
/// Frames are 128x128 with a per-row label strip on the left whose width
/// differs between targets. Rows are sparse and ordered exactly as the
/// generator emits them, so a sandbag can list only idle/hit/death while the
/// player can still list the full movement/combat set.
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
    pub rows: &'static [(CharacterAnim, AnimRow)],
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
// kept in sync with `tools/ambition_sprite2d_renderer` output. After regenerating
// sheets, mirror the new YAML headers + body_metrics here. When the
// runtime gains a YAML loader for the `body_metrics` field, these
// constants can be removed.

pub const ROBOT_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    // After the gen2d union-bbox crop the robot sheet is 120 wide x 128
    // tall (down from 128x128). Mirror that here.
    frame_width: 120,
    frame_height: 128,
    rows: &[
        (
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.120,
            },
        ),
        (
            CharacterAnim::Walk,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Run,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Jump,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Fall,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Slash,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Hit,
            AnimRow {
                frame_count: 5,
                duration_secs: 0.090,
            },
        ),
        (
            CharacterAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
        (
            CharacterAnim::BlinkOut,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::BlinkIn,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::Dash,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.065,
            },
        ),
    ],
    collision_scale: 2.1,
    feet_anchor_y: -0.320,
    frame_sample_inset: 1,
};

pub const GOBLIN_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    // After the gen2d union-bbox crop the goblin sheet is 121x127.
    frame_width: 121,
    frame_height: 127,
    rows: &[
        (
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.120,
            },
        ),
        (
            CharacterAnim::Walk,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Run,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Jump,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Fall,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.095,
            },
        ),
        (
            CharacterAnim::Slash,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Hit,
            AnimRow {
                frame_count: 5,
                duration_secs: 0.090,
            },
        ),
        (
            CharacterAnim::Death,
            AnimRow {
                frame_count: 8,
                duration_secs: 0.110,
            },
        ),
        (
            CharacterAnim::BlinkOut,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::BlinkIn,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.062,
            },
        ),
        (
            CharacterAnim::Dash,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.065,
            },
        ),
    ],
    collision_scale: 2.1,
    feet_anchor_y: -0.350,
    frame_sample_inset: 1,
};

pub const SANDBAG_SHEET: CharacterSheetSpec = CharacterSheetSpec {
    label_width: 100,
    frame_width: 128,
    frame_height: 128,
    rows: &[
        (
            CharacterAnim::Idle,
            AnimRow {
                frame_count: 6,
                duration_secs: 0.120,
            },
        ),
        (
            CharacterAnim::Hit,
            AnimRow {
                frame_count: 4,
                duration_secs: 0.075,
            },
        ),
        (
            CharacterAnim::Death,
            AnimRow {
                frame_count: 7,
                duration_secs: 0.112,
            },
        ),
    ],
    collision_scale: 1.38,
    feet_anchor_y: -0.438,
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

/// Sprite anchor that places the rendered character's feet on the bottom
/// of the collision box (rather than at its centre).
///
/// `spec.feet_anchor_y` records where feet sit *within the sprite frame*
/// (Bevy convention — typically about -0.32 for these sheets, meaning feet
/// are below sprite centre). If we used that anchor verbatim the feet
/// would coincide with `transform.translation`, which is the collision
/// *centre* — leaving everyone visually floating by half a collision
/// height. Adding `collision.y / (2 * render_height)` shifts the anchor
/// up inside the sprite, drawing the sprite lower in world space so the
/// feet land on the collision bottom edge.
pub fn feet_anchor_for(spec: CharacterSheetSpec, collision: Vec2) -> Anchor {
    let render_height = collision.x.max(collision.y).max(8.0) * spec.collision_scale;
    let half_collision_y = collision.y * 0.5;
    let ay = spec.feet_anchor_y + half_collision_y / render_height;
    Anchor(Vec2::new(0.0, ay))
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
    fn row_index(&self, anim: CharacterAnim) -> Option<usize> {
        self.rows.iter().position(|(row_anim, _)| *row_anim == anim)
    }

    pub fn resolve_anim(&self, anim: CharacterAnim) -> CharacterAnim {
        if self.row_index(anim).is_some() {
            anim
        } else {
            CharacterAnim::Idle
        }
    }

    fn row(&self, anim: CharacterAnim) -> AnimRow {
        let resolved = self.resolve_anim(anim);
        let idx = self
            .row_index(resolved)
            .expect("character sprite sheet must define an Idle row");
        self.rows[idx].1
    }

    pub fn build_atlas(&self) -> TextureAtlasLayout {
        let max_frames = self
            .rows
            .iter()
            .map(|(_, row)| row.frame_count)
            .max()
            .unwrap_or(0) as u32;
        let total_w = self.label_width + max_frames * self.frame_width;
        let total_h = self.rows.len() as u32 * self.frame_height;
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w, total_h));
        let inset = self
            .frame_sample_inset
            .min(self.frame_width.min(self.frame_height) / 4);
        for (row_idx, (_, row)) in self.rows.iter().enumerate() {
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
        let resolved = self.resolve_anim(anim);
        let row = self
            .row_index(resolved)
            .expect("character sprite sheet must define an Idle row");
        let frames_before: usize = self.rows[..row]
            .iter()
            .map(|(_, row)| row.frame_count)
            .sum();
        let max_frame = self.rows[row].1.frame_count.saturating_sub(1);
        frames_before + frame.min(max_frame)
    }

    pub fn frame_count(&self, anim: CharacterAnim) -> usize {
        self.row(anim).frame_count
    }

    pub fn frame_duration(&self, anim: CharacterAnim) -> f32 {
        self.row(anim).duration_secs
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
    pub sandbag: Option<CharacterSpriteAsset>,
    // The boss uses the entity-sprite path (`EntitySprite::BossCore`) rather
    // than the character-spritesheet path: its generator emits non-standard
    // animation rows (rest/floor_slam/side_sweep/spike_halo/dash_echo/hit/
    // death) that don't fit `CharacterAnim`'s 8-variant grid. When/if the
    // boss gets a CharacterAnim-compatible sheet, add a `boss` field here.
}

impl CharacterSpriteAssets {
    pub fn enemy_asset(&self, kind: FeatureVisualKind) -> Option<&CharacterSpriteAsset> {
        match kind {
            FeatureVisualKind::Enemy => self.goblin.as_ref(),
            FeatureVisualKind::Sandbag => self.sandbag.as_ref().or(self.goblin.as_ref()),
            _ => None,
        }
    }
}

const ROBOT_FILENAME: &str = "robot_spritesheet.png";
const GOBLIN_FILENAME: &str = "goblin_spritesheet.png";
const SANDBAG_FILENAME: &str = "sandbag_spritesheet.png";

/// Probe the sandbox `assets/<sprite_folder>/` directory for spritesheets.
/// Missing files are not an error — callers fall back to colored rectangles.
pub fn load_character_sprites_in(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_folder: &str,
) -> CharacterSpriteAssets {
    let robot_rel = format!("{sprite_folder}/{ROBOT_FILENAME}");
    let goblin_rel = format!("{sprite_folder}/{GOBLIN_FILENAME}");
    let sandbag_rel = format!("{sprite_folder}/{SANDBAG_FILENAME}");

    let robot = build_optional(asset_server, layouts, &robot_rel, ROBOT_SHEET);
    let goblin = build_optional(asset_server, layouts, &goblin_rel, GOBLIN_SHEET);
    let sandbag = build_optional(asset_server, layouts, &sandbag_rel, SANDBAG_SHEET);

    for (name, rel, present) in [
        ("robot", &robot_rel, robot.is_some()),
        ("goblin", &goblin_rel, goblin.is_some()),
        ("sandbag", &sandbag_rel, sandbag.is_some()),
    ] {
        if !present {
            eprintln!(
                "[character_sprites] {name} spritesheet not found at assets/{rel} — falling back to colored rectangle"
            );
        }
    }

    CharacterSpriteAssets {
        robot,
        goblin,
        sandbag,
    }
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
    // Android assets live inside the APK, not under the host-side
    // CARGO_MANIFEST_DIR. Let Bevy's Android asset reader try the load.
    #[cfg(target_os = "android")]
    {
        let _ = rel_path;
        true
    }

    // Bevy's FileAssetReader resolves assets relative to CARGO_MANIFEST_DIR
    // when running through cargo. Mirror that here so the existence check
    // matches the asset server's lookup path.
    #[cfg(not(target_os = "android"))]
    {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir)
            .join("assets")
            .join(rel_path)
            .exists()
    }
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
        let anim = self.spec.resolve_anim(anim);
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
        let row = self.spec.row(self.current);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_render_size_uses_max_collision_axis() {
        // Tall narrow body: render height tracks collision.y (the
        // larger axis), scaled by collision_scale.
        let collision = Vec2::new(28.0, 46.0);
        let size = sprite_render_size(ROBOT_SHEET, collision);
        let expected_height = 46.0 * ROBOT_SHEET.collision_scale;
        assert!((size.y - expected_height).abs() < 1e-3);
    }

    #[test]
    fn sprite_render_size_clamps_at_minimum_eight() {
        // Tiny collision boxes hit the 8.0 floor so micro-entities
        // (debris-sized actors) still render visibly.
        let collision = Vec2::new(2.0, 1.0);
        let size = sprite_render_size(ROBOT_SHEET, collision);
        let expected_height = 8.0 * ROBOT_SHEET.collision_scale;
        assert!((size.y - expected_height).abs() < 1e-3);
    }

    #[test]
    fn sprite_render_size_preserves_frame_aspect() {
        // Width tracks the frame's source aspect, not the collision
        // box, so cropped non-square frames don't get distorted.
        let collision = Vec2::new(28.0, 46.0);
        let size = sprite_render_size(ROBOT_SHEET, collision);
        let expected_aspect = ROBOT_SHEET.frame_width as f32 / ROBOT_SHEET.frame_height as f32;
        let actual_aspect = size.x / size.y;
        assert!(
            (actual_aspect - expected_aspect).abs() < 1e-3,
            "expected aspect {expected_aspect}, got {actual_aspect}"
        );
    }

    #[test]
    fn flat_index_zero_for_first_frame_of_first_row() {
        let idx = ROBOT_SHEET.flat_index(CharacterAnim::Idle, 0);
        assert_eq!(idx, 0);
    }

    #[test]
    fn frame_count_positive_for_every_row() {
        for (anim, _) in ROBOT_SHEET.rows {
            assert!(
                ROBOT_SHEET.frame_count(*anim) > 0,
                "anim {:?} has zero frames",
                anim
            );
        }
    }

    #[test]
    fn flat_index_clamps_to_last_frame_of_row() {
        // Asking for frame past the end of a row clamps to the last
        // valid frame; this avoids out-of-bounds atlas reads when the
        // animation cursor overshoots due to a long delta-t.
        let last = ROBOT_SHEET.flat_index(CharacterAnim::Idle, 9_999);
        let expected = ROBOT_SHEET.frame_count(CharacterAnim::Idle) - 1;
        assert_eq!(last, expected);
    }

    #[test]
    fn frame_duration_positive_for_every_row() {
        // Zero or negative duration would wedge the animation cursor
        // (advance_anim divides by it). Pin the contract.
        for (anim, _) in ROBOT_SHEET.rows {
            assert!(
                ROBOT_SHEET.frame_duration(*anim) > 0.0,
                "anim {:?} has non-positive duration",
                anim
            );
        }
    }
}
