//! Derive body and sprite geometry from per-pose sprite-sheet metrics.
//!
//! A sheet row supplies the occupied hurtbox rectangle in pixels.
//! [`SpritePosedBody::world_per_pixel`] scales that rectangle and the full frame into
//! world units so collision size, render size, and sprite offset stay coherent. Resizes
//! keep the gravity-side foot face fixed. The simulation uses authored
//! [`ActorAnimOverride`] rather than presentation locomotion selection, and resolves
//! geometry before movement.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_sprite_sheet::character::sheets::SpritePosedBody;
use ambition_sprite_sheet::character::{ActorAnimOverride, CharacterAnim};

use ambition_combat::components::{ActorRenderSize, ActorSpriteOffset};

// ⛔⛤ **`PosedBodyGeometry` / `posed_body_geometry` / `authored_body_pixel_size`
// MOVED DOWN TO `ambition_sprite_sheet` — 2026-09-21.** They are a pure
// projection of the BAKED SHEET REGISTRY (`record_for_sheet_key`), which that
// crate owns, and nothing about them is an ECS concern. The move is not
// tidying: `ambition_body_seed` builds a character's INITIAL body and cannot
// see this crate, so while the resolution lived here the seed had to derive
// its own answer from the catalog join — two derivations of one authored
// scale, which is the defect this move exists to make unrepresentable. Both
// roads now ask one function.
pub use ambition_sprite_sheet::character::sheets::{
    authored_body_pixel_size, posed_body_geometry, PosedBodyGeometry,
};

/// Keep every [`SpritePosedBody`] actor's collision box, sprite quad, and quad
/// offset equal to what its sheet says about the pose it is showing.
///
/// Runs in the sim so the box is authoritative in a headless build, and writes
/// nothing when the geometry is unchanged — the common case, since a pose holds
/// for many ticks and `ActorRenderSize` feeds a change-detecting render index.
pub fn sync_sprite_posed_bodies(
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &SpritePosedBody,
        Option<&ActorAnimOverride>,
        &mut ae::BodyKinematics,
        Option<&mut ae::BodyBaseSize>,
        Option<&mut ActorRenderSize>,
        Option<&ActorSpriteOffset>,
        // The STANCE, which composes with the pose rather than competing with
        // it. Absent  a body that never body-modes, and the pose IS the box.
        Option<&ae::BodyModeState>,
    )>,
    // The body's LOCAL gravity, resolved the same way movement and contact do.
    // Optional so a composition without a gravity field still poses bodies.
    gravity: Option<Res<ambition_platformer2d_shared_tangle::gravity::GravityField>>,
    zones: Option<Res<ambition_platformer2d_shared_tangle::gravity::GravityZones>>,
) {
    for (entity, posed, pinned, mut kin, base_size, render_size, offset, body_mode) in &mut bodies {
        let anim = pinned.map_or(CharacterAnim::Idle, |o| o.0);
        let Some(geometry) = posed_body_geometry(&posed.target, anim, posed.world_per_pixel) else {
            continue;
        };
        // The STANDING box is this sheet's `Idle` rectangle, and it is a
        // different fact from the box above: `size` is the pose showing NOW, and
        // `base_size` is what the body returns to — the denominator of every
        // stance ratio, and what a reset restores `size` to. Leaving it at the
        // spawn placeholder meant a sheet-authored body came back from a reset
        // wearing a box it had never had, and read as crouching forever to
        // anything dividing by it.
        //
        // Only the identity authority may write it (`reset_body_clusters`
        // restores, never redefines), and for a body whose geometry IS its art
        // that authority is this pass.
        if let Some(mut base) = base_size {
            if let Some(standing) =
                posed_body_geometry(&posed.target, CharacterAnim::Idle, posed.world_per_pixel)
            {
                if base.base_size != standing.collision {
                    base.base_size = standing.collision;
                }
            }
        }
        // The pose says how big the body IS; the MODE says what it is doing with
        // it, and the box is the composition of the two. The stance must be
        // re-applied here rather than left to the transition: the transition
        // writes the shorter box once, on the tick the mode changes, while this
        // pass writes every tick — so publishing `geometry.collision` alone
        // would stand a crouching body back up inside its own crouch.
        //
        // `BodyMode::shape` is the same function the stance transition uses, so
        // the two cannot disagree about what crouching means. It applies to the
        // POSE's rectangle, not to `base_size`, so a body whose silhouette
        // changes shape crouches from the height it is actually showing.
        let posed_collision = body_mode.map_or(geometry.collision, |mode| {
            mode.body_mode.shape(geometry.collision).size
        });
        // In a reversed or horizontal-gravity room a resize anchored to world +y
        // pushed the body into or off its own support. The module's contract is
        // that the +gravity face stays planted; the direction has to be the
        // body's, not the default's.
        let gravity_dir = match (gravity.as_deref(), zones.as_deref()) {
            (Some(field), Some(zones)) => {
                ambition_platformer2d_shared_tangle::gravity::gravity_dir_for(
                    ae::Aabb::new(kin.pos, kin.size * 0.5),
                    zones,
                    field.dir,
                )
            }
            (Some(field), None) => field.dir,
            _ => ae::DEFAULT_GRAVITY_DIR,
        };
        if kin.size != posed_collision {
            // Feet-anchored, through the engine's one feet-planted resize op: hold the +gravity
            // face and move the centre by half the change, so a withdraw/emerge never drives the
            // body through the ground it is standing on.
            ae::resize_feet_planted(&mut kin, posed_collision, gravity_dir);
        }
        // Written in the same instant as the box rather than through `Commands`:
        // both facts come from ONE `geometry`, so one mechanism is simpler than
        // two. `try_insert` remains for a body that has no `ActorRenderSize` yet,
        // where there is nothing to write into. Neither spelling is late for a
        // consumer ordered after this system — that ordering is itself a sync
        // point.
        match render_size {
            Some(mut existing) if existing.0 != geometry.render => {
                existing.0 = geometry.render;
            }
            None => {
                commands
                    .entity(entity)
                    .try_insert(ActorRenderSize(geometry.render));
            }
            Some(_) => {}
        }
        // The stance moved the body's CENTRE without moving its FEET, and the
        // quad is placed relative to that centre — so the placement owes the
        // same shift back, or the art is drawn where a STANDING body's centre
        // would have put it: a quarter of the body's height into the floor for
        // a half-height crouch.
        //
        // `geometry.sprite_offset` answers "where does the frame go so the
        // POSE's rectangle lands on the box", and that box is the one the SHEET
        // measured. `resize_feet_planted` then slid the centre by half the
        // stance shrink along gravity, so the same term reverses it. Both facts
        // are published from here, by the one pass that knows both.
        let stance_shift = gravity_dir * ((geometry.collision - posed_collision) * 0.5);
        let sprite_offset = geometry.sprite_offset - stance_shift;
        if offset.map(|o| o.0) != Some(sprite_offset) {
            commands
                .entity(entity)
                .try_insert(ActorSpriteOffset(sprite_offset));
        }
    }
}

#[cfg(test)]
mod tests;
