//! Derive a controllable actor's melee attack hitbox from its sprite-sheet
//! manifest — the same data-driven path bosses use
//! (`boss_encounter::attack_geometry`), so the box you author and see in
//! `debug-hitboxes` IS the gameplay damage box.
//!
//! The manifest stores the hitbox as sprite-frame pixels. Turning those into
//! gameplay offsets is [`ambition_sprite_sheet::FrameToBody`]'s job and nothing
//! else's: a frame pixel is a coordinate in the sheet's ARTWORK, and the
//! artwork has a handedness that only the sheet knows. This module used to do
//! the mapping by hand from `facing` alone, which put every left-drawn sheet's
//! blade behind its owner.
//!
//! Resolution comes in two halves, and the split is the point:
//!
//! - `*_local` returns the volume BODY-LOCAL (`+x` forward, `+y` toward the
//!   feet, origin at the body centre) — no position, no facing, no gravity.
//!   That is what a spawned hitbox wants, because it mirrors and rotates itself
//!   at query time.
//! - `*_world` places that volume for a body that exists right now — what the
//!   debug overlay wants.
//!
//! Before the split there was one function taking a `facing`, and the moveset
//! path passed `1.0` to mean "don't place it" — a convention, which is to say
//! a thing that can be got wrong. Now there is no facing to pass wrongly.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d_core as ae;
use ambition_sprite_sheet::character::catalog_join;
use ambition_sprite_sheet::character::sheets;
use ambition_sprite_sheet::{FrameToBody, SheetRecord, SheetRegistry, baked_sheet_rons, frame_at};
use std::sync::OnceLock;

/// The player's sprite manifest file root. Both `robot` (enemy) and
/// `player_robot_v3` author `target: "robot"`, so the target-keyed registry
/// can't tell them apart — we key by file root instead.
const PLAYER_FILE_ROOT: &str = "player_robot_v3";
/// The player's catalog character id (drives the render-size spec lookup).
const PLAYER_CHARACTER_ID: &str = "player_robot_v3";

/// Baked sheets keyed by file root (not `record.target`), so the
/// player's `player_robot_v3` stays distinct from the enemy `robot`. Built
/// once, lazily.
///
/// §5 classification (per the old restructuring blueprint, folded into
/// `docs/planning/engine/architecture.md`): immutable asset cache —
/// derived once from the compile-time `BAKED_SHEET_RONS` table, pure and
/// override-free. Correctly a process-global `OnceLock`; not a content
/// registry, so it has no `install_*` seam.
fn file_root_registry() -> &'static SheetRegistry {
    static REG: OnceLock<SheetRegistry> = OnceLock::new();
    REG.get_or_init(|| SheetRegistry::from_baked_table(baked_sheet_rons::BAKED_SHEET_RONS))
}

/// Build the file-root index NOW, so the first attack does not.
///
/// ⛔⛔ MEASURED ON HARDWARE 2026-08-29, and it was a 189ms GAMEPLAY FRAME. The
/// `OnceLock` above is the right shape — an immutable, override-free cache of a
/// compile-time table — but "lazily" means *on whichever frame first asks*, and
/// the first asker is a punch:
///
/// ```text
/// [  3.214s] init_sheet_registry:   SheetRegistry: loaded 870 sheets
/// [ 23.927s] advance_move_playback: SheetRegistry: loaded 870 sheets   <- again
/// ```
///
/// Tracy priced that call at **189,032,871 ns against a 21us mean** over 10,078
/// calls, inside the 23.9s frame-spike cluster (a 198.3ms frame).
///
/// ⚠ THE TWO LINES ARE TWO DIFFERENT REGISTRIES, which is why nothing caught
/// this: `init_sheet_registry` fills the Bevy resource keyed by `record.target`,
/// while this one is keyed by FILE ROOT so `player_robot_v3` stays distinct from
/// `robot`. Both walk the same 870-entry baked table. ▢ Sharing one index would
/// remove the duplicated build AND its memory; warming is the cheap half.
pub fn warm_file_root_registry() {
    let _ = file_root_registry();
}

/// File roots the index REFUSED for naming several records, exposed so a
/// caller that owns a character catalog can decide whether any of them matter.
///
/// same division of labour as `SheetRegistry::shadowed_targets`: this side can
/// see that a root is ambiguous, but only a catalog knows whether a character
/// resolves its art by that root — and a character whose `manifest_target` were
/// refused here would silently lose its authored blade and fall back to the
/// shared hardcoded volume.
pub fn refused_file_roots() -> &'static [ambition_sprite_sheet::AmbiguousFileRoot] {
    file_root_registry().ambiguous_file_roots()
}

/// Whether the file-root index resolves `root` to a sheet.
///
/// exists so a caller can prove its own comparison is not vacuous: a check
/// that a character's `manifest_target` is not a REFUSED root means nothing
/// unless those two names live in the same namespace to begin with.
pub fn resolves_by_file_root(root: &str) -> bool {
    file_root_registry().get(root).is_some()
}

/// The body-local volume a sheet authors for `animation`, at `frame`.
///
/// `render_size` is the drawn sprite quad in world units (the renderer's own
/// `sprite_render_size`, so the box matches the visible blade); `collision` is
/// the body's collision box, which places the sheet's feet against the body's
/// toward-gravity face.
///
/// `clip_elapsed` is seconds into the animation row. The FRAME it selects is
/// the sheet's arithmetic, not the caller's: a row's `frame_duration_secs`
/// lives in the manifest, so a caller that holds a clock never has to hold a
/// frame rate too. A row publishing no per-frame geometry (every character
/// sheet today) resolves the coarse per-animation shape whatever the clock
/// says.
///
/// Returns `None` when the sheet has no body metrics or nothing authored for
/// `animation`; the caller falls back to its hardcoded volume.
pub fn manifest_attack_hitbox_local(
    record: &SheetRecord,
    animation: &str,
    collision: ae::Vec2,
    render_size: ae::Vec2,
    clip_elapsed: Option<f32>,
) -> Option<ae::CombatVolume> {
    let metrics = record.body_metrics.as_ref()?;
    let entry = metrics.animations.get(animation)?;
    let hitbox = entry.hitbox.as_ref()?;
    let frame = clip_elapsed.and_then(|elapsed| frame_at(entry, elapsed));
    FrameToBody::planting_feet(record, render_size, collision).volume(hitbox, frame)
}

/// [`manifest_attack_hitbox_local`] placed for a body that exists right now.
///
/// - `body_pos`: collision-box centre, world coords (y grows downward).
/// - `facing`: `+1` faces right, `-1` faces left.
/// - `gravity_dir`: live gravity DIRECTION at the body. The authored box is in
///   the body's own frame (x = side, y = toward-feet); this rotates it into
///   world so the box lands toward the swing's forward under ANY gravity — the
///   SAME rotation `AttackSpec::into_world_frame` applies to the slash, so the
///   damage box and the VFX point the same way. Identity under screen-down
///   gravity (upright is byte-stable).
#[allow(clippy::too_many_arguments)]
pub fn manifest_attack_hitbox_world(
    record: &SheetRecord,
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    facing: f32,
    render_size: ae::Vec2,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    Some(
        manifest_attack_hitbox_local(record, animation, collision, render_size, None)?
            .place_body_local(body_pos, facing, gravity_dir),
    )
}

/// Render size of the player's sprite quad, resolved from the supplied
/// App-local catalog the same way the renderer does. `None` if the player has
/// no sheet spec. The baked manifest registry remains the only immutable
/// process-wide cache; catalog-dependent sheet selection is never cached.
fn player_render_size(
    authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    catalog: &CharacterCatalog,
    collision: ae::Vec2,
) -> Option<ae::Vec2> {
    let spec = catalog_join::sheet_for_character_id_from_data(
        authored,
        catalog.data(),
        PLAYER_CHARACTER_ID,
    )?;
    Some(sheets::sprite_render_size(&spec, collision))
}

/// Resolve a controllable body's authored melee volume, BODY-LOCAL.
///
/// The combat-seam resolver (`combat::authored_volumes`) is installed as an
/// App-local Bevy resource by runtime composition. It receives the same
/// `CharacterCatalog` as spawning and rendering without naming this module.
/// `None` cid selects the player manifest root.
///
/// Body-local is the whole seam: combat spawns a hitbox that mirrors and
/// rotates itself against its owner every query, so handing it a placed volume
/// would mirror the swing twice. `clip_elapsed` is seconds into the clip, so a
/// sheet publishing per-frame geometry drives the box that is live right now.
pub fn authored_attack_volume_resolver(
    // U1 stage C: provider-authored sheets, captured by the composition root
    // into the resolver closure. Combat calls this without naming the type.
    authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    catalog: &CharacterCatalog,
    sprite_character_id: Option<&str>,
    animation: &str,
    collision: ae::Vec2,
    clip_elapsed: Option<f32>,
) -> Option<ae::CombatVolume> {
    match sprite_character_id {
        Some(cid) => {
            actor_attack_hitbox_local(authored, catalog, cid, animation, collision, clip_elapsed)
        }
        None => player_attack_hitbox_local(authored, catalog, animation, collision, clip_elapsed),
    }
}

/// The authored polys are sized to the visual blade; this scales the player's strike reach +
/// size about the feet anchor so the directional swings connect more forgivingly, WITHOUT
/// touching the visual sprite or any actor's authored size. `1.0` = authored size. Pure feel
/// knob — TUNE LIVE.
const PLAYER_ATTACK_HITBOX_SCALE: f32 = 1.3;

/// The player's authored melee volume for `animation`, BODY-LOCAL.
///
/// Cheap per-frame because the file-root registry is an immutable baked-asset
/// cache. `None` when no hitbox is authored for that animation, so the caller
/// falls back to its `AttackSpec` volume.
pub fn player_attack_hitbox_local(
    authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    catalog: &CharacterCatalog,
    animation: &str,
    collision: ae::Vec2,
    clip_elapsed: Option<f32>,
) -> Option<ae::CombatVolume> {
    // Authored first, baked second — the same order every other sheet lookup
    // uses since U1, so a provider that authored its protagonist's sheet gets
    // its own attack bboxes instead of the engine player's.
    let record = authored
        .get(PLAYER_FILE_ROOT)
        .or_else(|| file_root_registry().get(PLAYER_FILE_ROOT))?;
    // Enlarge the hitbox by scaling the render size the poly/bbox offsets derive
    // from — grows reach + size about the feet anchor, player-only.
    let render_size =
        player_render_size(authored, catalog, collision)? * PLAYER_ATTACK_HITBOX_SCALE;
    manifest_attack_hitbox_local(record, animation, collision, render_size, clip_elapsed)
}

/// [`player_attack_hitbox_local`] placed for a body that exists right now.
#[allow(clippy::too_many_arguments)]
pub fn player_attack_hitbox_world(
    authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    catalog: &CharacterCatalog,
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    facing: f32,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    Some(
        player_attack_hitbox_local(authored, catalog, animation, collision, None)?
            .place_body_local(body_pos, facing, gravity_dir),
    )
}

/// ANY catalog actor's authored melee volume for `animation`, BODY-LOCAL — the
/// actor-neutral generalization of [`player_attack_hitbox_local`].
///
/// The actor's sheet is resolved by its catalog `character_id` through the
/// file-root registry (so robot-family characters — the player and the robot
/// enemy both author `target: "robot"` — stay distinct), and pixel rects scale
/// by the actor's rendered sprite size.
///
/// Returns `None` when the character has no catalog row, no baked sheet, or no
/// authored hitbox for `animation`; the caller falls back to its shared
/// hardcoded melee volume. This is the same sprite-metadata-then-fallback shape
/// the player uses, so an enemy with an authored blade swings the box you see
/// in `debug-hitboxes`, not a divergent hardcoded rectangle.
pub fn actor_attack_hitbox_local(
    authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    catalog: &CharacterCatalog,
    character_id: &str,
    animation: &str,
    collision: ae::Vec2,
    clip_elapsed: Option<f32>,
) -> Option<ae::CombatVolume> {
    let file_root = catalog.get(character_id)?.manifest_target()?;
    let record = authored
        .get(file_root)
        .or_else(|| file_root_registry().get(file_root))?;
    // Scale by the actor's rendered sprite size (same derivation its collision
    // came from); fall back to the collision box when no sheet spec resolves.
    let render_size = catalog_join::sprite_body_collision_for_character_id_from_data(
        authored,
        catalog.data(),
        character_id,
        collision,
    )
    .map(|b| b.render_size)
    .unwrap_or(collision);
    manifest_attack_hitbox_local(record, animation, collision, render_size, clip_elapsed)
}

/// [`actor_attack_hitbox_local`] placed for a body that exists right now.
#[allow(clippy::too_many_arguments)]
pub fn actor_attack_hitbox_world(
    authored: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    catalog: &CharacterCatalog,
    character_id: &str,
    animation: &str,
    body_pos: ae::Vec2,
    collision: ae::Vec2,
    facing: f32,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    Some(
        actor_attack_hitbox_local(authored, catalog, character_id, animation, collision, None)?
            .place_body_local(body_pos, facing, gravity_dir),
    )
}

#[cfg(test)]
mod tests;
