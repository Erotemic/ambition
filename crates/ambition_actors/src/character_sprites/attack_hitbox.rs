//! Derive a controllable actor's melee attack hitbox in world space from
//! its sprite-sheet manifest — the same data-driven path bosses use
//! (`boss_encounter::attack_geometry`), so the box you author and see in
//! `debug-hitboxes` IS the gameplay damage box.
//!
//! The manifest stores the attack hitbox as a sprite-frame pixel rect
//! (`AnimationMetrics::hitbox`). We map frame pixels → world by planting
//! the manifest's `feet_pixel` at the collision box's bottom-centre (the
//! anchor the renderer also uses) and scaling by the *rendered* sprite
//! size — resolved via the same `player_placeholder_render_size` the
//! renderer uses, so the gameplay box lines up with the drawn blade.
//! Facing mirrors the box's forward offset.

use ambition_engine_core as ae;
use ambition_sprite_sheet::character::sheets;
use ambition_sprite_sheet::{baked_sheet_rons, SheetRecord, SheetRegistry};
use std::sync::OnceLock;

/// The player's sprite manifest file root. Both `robot` (enemy) and
/// `player_robot` author `target: "robot"`, so the target-keyed registry
/// can't tell them apart — we key by file root instead.
const PLAYER_FILE_ROOT: &str = "player_robot";
/// The player's catalog character id (drives the render-size spec lookup).
const PLAYER_CHARACTER_ID: &str = "player";

/// Baked sheets keyed by **file root** (not `record.target`), so the
/// player's `player_robot` stays distinct from the enemy `robot`. Built
/// once, lazily.
///
/// §5 classification (restructuring-blueprint): **immutable asset cache** —
/// derived once from the compile-time `BAKED_SHEET_RONS` table, pure and
/// override-free. Correctly a process-global `OnceLock`; not a content
/// registry, so it has no `install_*` seam.
fn file_root_registry() -> &'static SheetRegistry {
    static REG: OnceLock<SheetRegistry> = OnceLock::new();
    REG.get_or_init(|| {
        SheetRegistry::from_baked_table_by_file_root(baked_sheet_rons::BAKED_SHEET_RONS)
    })
}

/// Convert a sheet's per-animation attack hitbox (the coarse `bbox`) into
/// a world-space [`ae::Aabb`].
///
/// - `body_pos`: collision-box centre, world coords (y grows downward).
/// - `collision`: collision-box size (e.g. 30×48). Used to plant feet.
/// - `facing`: `+1` faces right, `-1` faces left (the box mirrors).
/// - `render_size`: the drawn sprite-quad size in world units (use the
///   renderer's own `player_placeholder_render_size` so the box matches
///   the visible blade).
///
/// Returns `None` when the sheet has no body metrics or no hitbox for
/// `animation`; the caller should then fall back to its hardcoded volume.
pub fn manifest_attack_hitbox_world(
    record: &SheetRecord,
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    facing: f32,
    render_size: ae::Vec2,
    // Live gravity DIRECTION at the body. The manifest authors the hitbox in the
    // sprite's screen frame (x = side, y = toward-feet); this rotates it into the
    // body's reference frame so the box lands toward the swing's forward under ANY
    // gravity — the SAME rotation `AttackSpec::into_world_frame` applies to the
    // slash, so the damage box and the VFX point the same way. Identity under
    // screen-down gravity (upright is byte-stable).
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    let metrics = record.body_metrics.as_ref()?;
    let hitbox = metrics.animations.get(animation)?.hitbox.as_ref()?;

    let fw = record.frame_width.max(1) as f32;
    let fh = record.frame_height.max(1) as f32;
    let scale = ae::Vec2::new(render_size.x / fw, render_size.y / fh);

    // Feet plant at the collision box's bottom-centre (world y-down → the
    // "bottom" of the body is at +y). Every frame pixel maps to world
    // relative to the feet, scaled by the render size. The sprite flips
    // horizontally with facing, so the forward x offset negates facing left.
    let (feet_x, feet_y) = metrics
        .feet_pixel
        .map(|p| (p.x, p.y))
        .unwrap_or((fw * 0.5, fh));
    let face = if facing < 0.0 { -1.0 } else { 1.0 };
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let pixel_to_world = |px: f32, py: f32| {
        let off_x = (px - feet_x) * scale.x * face;
        let off_y = (py - feet_y) * scale.y;
        // Body-LOCAL offset: x = gravity-perpendicular side (facing-signed),
        // y = toward-feet (the `+collision.y/2` plants the box's anchor at the
        // body's toward-gravity face). Rotate into world by the gravity frame so
        // the authored screen-axis box tracks gravity. Identity when gravity is
        // screen-down (`to_world` is the identity there), so upright is unchanged.
        body_pos + frame.to_world(ae::Vec2::new(off_x, collision.y * 0.5 + off_y))
    };

    // Authored convex polygon wins: a hitbox shape that conforms to the effect
    // (a blade arc, a cone) instead of the coarse bbox.
    if !hitbox.poly.is_empty() {
        let points: Vec<ae::Vec2> = hitbox
            .poly
            .iter()
            .map(|(x, y)| pixel_to_world(*x, *y))
            .collect();
        return Some(ae::CombatVolume::convex(points));
    }

    // Fallback: the single bbox as an axis-aligned volume.
    let bbox = hitbox.bbox?;
    let center = pixel_to_world(
        bbox.x as f32 + bbox.w as f32 * 0.5,
        bbox.y as f32 + bbox.h as f32 * 0.5,
    );
    let half = ae::Vec2::new(
        (bbox.w as f32 * 0.5 * scale.x).abs(),
        (bbox.h as f32 * 0.5 * scale.y).abs(),
    );
    Some(ae::CombatVolume::aabb(ae::Aabb::new(center, half)))
}

/// Render size of the player's sprite quad, resolved (and cached) the
/// same way the renderer does. `None` if the player has no sheet spec.
///
/// §5 classification: **immutable asset cache** — the resolved sheet spec is
/// derived once from baked metadata; an `OnceLock`, no override seam.
fn player_render_size(collision: ae::Vec2) -> Option<ae::Vec2> {
    static SPEC: OnceLock<Option<sheets::CharacterSheetSpec>> = OnceLock::new();
    let spec = SPEC
        .get_or_init(|| super::assets::sheet_for_character_id(PLAYER_CHARACTER_ID))
        .as_ref()?;
    Some(sheets::player_placeholder_render_size(spec, collision))
}

/// Resolve the player's melee attack hitbox for `animation` from the
/// baked manifest. Cheap per-frame (the file-root registry and the spec
/// are cached). Returns `None` when no hitbox is authored for that
/// animation — the caller falls back to its hardcoded `AttackSpec` volume.
///
/// TODO(non-player-centric): take the controlled actor's sheet/character
/// ids instead of the hardcoded player ones, so any possessed actor's

/// The combat-seam resolver (`combat::authored_volumes`): one entry point the
/// runtime assembly installs so the strike paths resolve authored volumes
/// without naming this module. `None` cid = the player manifest root.
pub fn authored_attack_volume_resolver(
    sprite_character_id: Option<&str>,
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    facing: f32,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    match sprite_character_id {
        Some(cid) => {
            actor_attack_hitbox_world(cid, animation, body_pos, collision, facing, gravity_dir)
        }
        None => player_attack_hitbox_world(animation, body_pos, collision, facing, gravity_dir),
    }
}

/// Player-only GAMEPLAY hitbox enlargement (blind fix 2026-07-12, Jon: "make
/// dair / up-tilt easier to pogo + test"). The authored polys are sized to the
/// visual blade; this scales the player's strike reach + size about the feet
/// anchor so the directional swings connect more forgivingly, WITHOUT touching
/// the visual sprite or any actor's authored size. `1.0` = authored size. Pure
/// feel knob — TUNE LIVE.
const PLAYER_ATTACK_HITBOX_SCALE: f32 = 1.3;

/// authored melee box drives its attack.
pub fn player_attack_hitbox_world(
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    facing: f32,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    let record = file_root_registry().get(PLAYER_FILE_ROOT)?;
    // Enlarge the hitbox by scaling the render size the poly/bbox offsets derive
    // from — grows reach + size about the feet anchor, player-only.
    let render_size = player_render_size(collision)? * PLAYER_ATTACK_HITBOX_SCALE;
    manifest_attack_hitbox_world(
        record,
        animation,
        body_pos,
        collision,
        facing,
        render_size,
        gravity_dir,
    )
}

/// Resolve ANY catalog actor's melee attack hitbox for `animation` from its
/// baked manifest — the actor-neutral generalization of
/// [`player_attack_hitbox_world`] (the `TODO(non-player-centric)` above). The
/// actor's sheet is resolved by its catalog `character_id` through the
/// file-root registry (so robot-family characters — the player and the robot
/// enemy both author `target: "robot"` — stay distinct), and pixel rects scale
/// by the actor's rendered sprite size.
///
/// Returns `None` when the character has no catalog row, no baked sheet, or no
/// authored hitbox for `animation`; the caller falls back to its shared
/// hardcoded melee volume. This is the same sprite-metadata-then-fallback shape
/// the player uses, so an enemy with an authored blade swings the box you see
/// in `debug-hitboxes`, not a divergent hardcoded rectangle.
pub fn actor_attack_hitbox_world(
    character_id: &str,
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    facing: f32,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    let file_root = crate::character_roster::catalog()
        .characters
        .get(character_id)?
        .manifest_target()?;
    let record = file_root_registry().get(file_root)?;
    // Scale by the actor's rendered sprite size (same derivation its collision
    // came from); fall back to the collision box when no sheet spec resolves.
    let render_size =
        super::assets::sprite_body_collision_for_character_id(character_id, collision)
            .map(|b| b.render_size)
            .unwrap_or(collision);
    manifest_attack_hitbox_world(
        record,
        animation,
        body_pos,
        collision,
        facing,
        render_size,
        gravity_dir,
    )
}

#[cfg(test)]
mod tests;
