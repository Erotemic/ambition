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
use ambition_sprite_sheet::character::sheets::{record_for_sheet_key, SpritePosedBody};
use ambition_sprite_sheet::character::{ActorAnimOverride, CharacterAnim};

use ambition_combat::components::{ActorRenderSize, ActorSpriteOffset};

/// The three geometry facts one pose resolves to, in world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PosedBodyGeometry {
    /// Collision + hurt box extents.
    pub collision: ae::Vec2,
    /// Sprite quad extents (the whole sheet frame).
    pub render: ae::Vec2,
    /// Where to draw the quad's centre, relative to the body's centre. Non-zero
    /// whenever the art does not sit dead-centre in its frame — which is the
    /// normal case, and exactly the placement a hand-authored box gets wrong.
    pub sprite_offset: ae::Vec2,
}

/// Resolve one pose's geometry from the baked sheet registry.
///
/// `None` when the target has no manifest record or the record publishes no
/// usable body metrics — the caller then leaves the body exactly as authored,
/// because a silent fallback to "the whole frame is the body" would inflate
/// every collision box on a sheet that simply forgot to publish.
pub fn posed_body_geometry(
    target: &str,
    anim: CharacterAnim,
    world_per_pixel: f32,
) -> Option<PosedBodyGeometry> {
    let record = record_for_sheet_key(target)?;
    let metrics = record.body_metrics.as_ref()?;
    let bbox = metrics.pose_body_bbox(anim)?;
    let frame_w = record.frame_width.max(1) as f32;
    let frame_h = record.frame_height.max(1) as f32;
    let (cx, cy) = bbox.center();
    Some(PosedBodyGeometry {
        collision: ae::Vec2::new(bbox.w as f32, bbox.h as f32) * world_per_pixel,
        render: ae::Vec2::new(frame_w, frame_h) * world_per_pixel,
        // Sheet pixel space and world space share the same handedness (both run
        // +y downward — see the `coordinate_system` block every actor sidecar
        // emits), so this is a plain scale with no axis flip. Drawing the frame
        // centre HERE puts the art's rectangle on the collision box.
        sprite_offset: ae::Vec2::new(frame_w * 0.5 - cx, frame_h * 0.5 - cy) * world_per_pixel,
    })
}

/// The sheet's AUTHORED gameplay body, in sheet pixels — `None` when it only
/// measured one.
///
/// So this refuses rather than returning a number that looks usable (`BodyMetrics::authored_body`
/// is the sheet's own claim, emitted only when a target authored the box).
///
/// The `Idle` pose is the standing body — the same rectangle
/// [`sync_sprite_posed_bodies`] restores `base_size` to.
pub fn authored_body_pixel_size(target: &str) -> Option<ae::Vec2> {
    let record = record_for_sheet_key(target)?;
    let metrics = record.body_metrics.as_ref()?;
    if !metrics.authored_body {
        return None;
    }
    // Asked of the same function the per-tick sync asks, at a scale of 1.0 so
    // the answer is in pixels — so the two cannot disagree about what the sheet
    // says.
    posed_body_geometry(target, CharacterAnim::Idle, 1.0)
        .map(|geometry| geometry.collision)
        .filter(|size| size.x > 0.0 && size.y > 0.0)
}

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
        Option<&ActorRenderSize>,
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
        // The pose says how big the body IS; the MODE says what it is doing
        // with it, and the box is the composition of the two.
        //
        // writing `geometry.collision` straight into `kin.size` silently undid every stance.
        // The crouch is applied ONCE, on the tick the mode changes — `body_mode::mechanics`
        // does `if mode == target { continue }` and `try_change_body_mode_clusters`
        // early-returns on an unchanged mode — so nothing re-asserts the shorter box
        // afterwards, while THIS pass runs every tick. A body on this seam crouched for one
        // tick and then stood back up inside its own crouch, with `BodyModeState` still saying
        // `Crouching`.
        //
        // `BodyMode::shape` is the same function the stance transition uses, so
        // the two cannot disagree about what crouching means — and applying it
        // to the POSE's rectangle rather than to `base_size` is what keeps this
        // right for a body whose silhouette changes shape: a boxed snake that
        // crouched would otherwise crouch from its sprawled height.
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
        if render_size.map(|r| r.0) != Some(geometry.render) {
            commands
                .entity(entity)
                .try_insert(ActorRenderSize(geometry.render));
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
