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

use crate::engine_core as ae;
use ambition_sprite_sheet::{SheetRecord, SheetRegistry};
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
fn file_root_registry() -> &'static SheetRegistry {
    static REG: OnceLock<SheetRegistry> = OnceLock::new();
    REG.get_or_init(|| {
        SheetRegistry::from_baked_table_by_file_root(super::baked_sheet_rons::BAKED_SHEET_RONS)
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
    let pixel_to_world = |px: f32, py: f32| {
        let off_x = (px - feet_x) * scale.x * face;
        let off_y = (py - feet_y) * scale.y;
        ae::Vec2::new(body_pos.x + off_x, body_pos.y + collision.y * 0.5 + off_y)
    };

    // Authored convex polygon wins: a hitbox shape that conforms to the effect
    // (a blade arc, a cone) instead of the coarse bbox.
    if !hitbox.poly.is_empty() {
        let points: Vec<ae::Vec2> = hitbox.poly.iter().map(|(x, y)| pixel_to_world(*x, *y)).collect();
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
fn player_render_size(collision: ae::Vec2) -> Option<ae::Vec2> {
    static SPEC: OnceLock<Option<super::sheets::CharacterSheetSpec>> = OnceLock::new();
    let spec = SPEC
        .get_or_init(|| super::assets::sheet_for_character_id(PLAYER_CHARACTER_ID))
        .as_ref()?;
    Some(super::sheets::player_placeholder_render_size(
        spec, collision,
    ))
}

/// Resolve the player's melee attack hitbox for `animation` from the
/// baked manifest. Cheap per-frame (the file-root registry and the spec
/// are cached). Returns `None` when no hitbox is authored for that
/// animation — the caller falls back to its hardcoded `AttackSpec` volume.
///
/// TODO(non-player-centric): take the controlled actor's sheet/character
/// ids instead of the hardcoded player ones, so any possessed actor's
/// authored melee box drives its attack.
pub fn player_attack_hitbox_world(
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    facing: f32,
) -> Option<ae::CombatVolume> {
    let record = file_root_registry().get(PLAYER_FILE_ROOT)?;
    let render_size = player_render_size(collision)?;
    manifest_attack_hitbox_world(record, animation, body_pos, collision, facing, render_size)
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
) -> Option<ae::CombatVolume> {
    let file_root = crate::character_roster::EMBEDDED_CATALOG
        .characters
        .get(character_id)?
        .manifest_target()?;
    let record = file_root_registry().get(file_root)?;
    // Scale by the actor's rendered sprite size (same derivation its collision
    // came from); fall back to the collision box when no sheet spec resolves.
    let render_size = super::assets::sprite_body_collision_for_character_id(character_id, collision)
        .map(|b| b.render_size)
        .unwrap_or(collision);
    manifest_attack_hitbox_world(record, animation, body_pos, collision, facing, render_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collision() -> ae::Vec2 {
        ae::Vec2::new(30.0, 48.0)
    }

    fn player_box(facing: f32) -> ae::Aabb {
        player_attack_hitbox_world("attack_side", ae::Vec2::new(0.0, 0.0), collision(), facing)
            .expect("player_robot/attack_side has an authored manifest hitbox")
            .bounds()
    }

    #[test]
    fn player_attack_side_is_forward_disjoint_and_tall() {
        let body_right = collision().x * 0.5; // +15
        let aabb = player_box(1.0);
        // Disjoint + in front: the whole box sits to the RIGHT of the body.
        assert!(
            aabb.min.x > body_right,
            "hitbox should be disjoint in front of the body (min.x {} > {})",
            aabb.min.x,
            body_right
        );
        // At least as tall as the player body.
        let height = aabb.max.y - aabb.min.y;
        assert!(
            height >= collision().y,
            "hitbox should be at least body-height ({height} >= {})",
            collision().y
        );
    }

    #[test]
    fn player_attack_side_mirrors_with_facing() {
        let body_left = -collision().x * 0.5; // -15
        let aabb = player_box(-1.0);
        assert!(
            aabb.max.x < body_left,
            "left-facing hitbox should be disjoint in front on the LEFT (max.x {} < {})",
            aabb.max.x,
            body_left
        );
    }

    #[test]
    fn player_attack_side_is_an_authored_convex_blade() {
        // The robot's attack_side authors a poly (blade arc), so the player
        // slash resolves a Convex volume — not a box.
        let vol = player_attack_hitbox_world("attack_side", ae::Vec2::ZERO, collision(), 1.0)
            .expect("attack_side authored");
        assert!(
            matches!(vol, ae::CombatVolume::Convex { .. }),
            "expected a Convex blade, got {vol:?}"
        );
    }

    #[test]
    fn actor_attack_hitbox_resolves_an_authored_enemy_blade() {
        // The robot enemy (character_id "robot") authors an `attack_side` hitbox
        // in its sheet, so the actor-neutral path resolves a real box instead of
        // the hardcoded fallback — the unification payoff: an enemy swings the
        // authored blade you see in `debug-hitboxes`, not magic numbers.
        let aabb = actor_attack_hitbox_world(
            "robot",
            "attack_side",
            ae::Vec2::new(0.0, 0.0),
            collision(),
            1.0,
        );
        assert!(
            aabb.is_some(),
            "robot/attack_side should resolve an authored manifest hitbox"
        );
    }

    #[test]
    fn actor_attack_hitbox_is_none_for_unknown_character() {
        assert!(actor_attack_hitbox_world(
            "definitely_not_a_character",
            "attack_side",
            ae::Vec2::ZERO,
            collision(),
            1.0,
        )
        .is_none());
    }
}
