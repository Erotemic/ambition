//! **The sprite is the authority for an actor's body geometry.**
//!
//! A sheet publishes, per animation row, the pixel rectangle its art actually
//! occupies inside the frame (`body_metrics.animations[row].hurtbox`, emitted by
//! the generator's `animation_key_map` opt-in). That rectangle answers two
//! questions at once — *how big* the body is and *where in the frame* it sits —
//! so it can drive BOTH halves of the visual/gameplay correspondence instead of
//! either half being hand-guessed against the other:
//!
//! * the collision + hurt box is the rectangle, scaled to world units;
//! * the sprite quad is the whole FRAME, scaled the same way and shifted so the
//!   art's rectangle lands exactly on the collision box.
//!
//! One scalar — [`SpritePosedBody::world_per_pixel`] — is the entire authored
//! input, and it means the obvious thing: how many world units one sheet pixel
//! covers. Everything else is read off the sheet. That is what makes a body
//! whose silhouette CHANGES SHAPE between poses expressible without bespoke
//! per-state boxes: a snake that withdraws into a cardboard box is a long low
//! serpent in `walk` and a small cube in `boxed_idle` because its art is, and
//! the box follows the art by construction.
//!
//! ## What the runtime does with it
//!
//! [`sync_sprite_posed_bodies`] resolves the pose once per tick and writes three
//! facts that must never disagree:
//!
//! * `BodyKinematics::size` — the collision box (feet-anchored, see below),
//! * [`ActorRenderSize`] — the sprite quad,
//! * [`ActorSpriteOffset`] — the quad's offset from the body centre.
//!
//! The resize is **feet-anchored**: the +gravity face of the box stays put, so a
//! body that shrinks does not sink into the floor and a body that grows does not
//! embed in it. That is the same rule the player's compact-stance path uses, and
//! it is what keeps this free of pushout.
//!
//! ## Which pose
//!
//! The sim reads the CONTENT pose pin ([`ActorAnimOverride`]) — the fact a shell
//! state machine already publishes — and falls back to `Idle` when nothing is
//! pinned. The presentation-side locomotion picker (`pick_actor_anim`) is
//! deliberately NOT consulted: it lives in the render plugin, so a headless or
//! RL build would resolve a different box than the drawn one, and a collision
//! box that depends on whether anyone is watching is not a collision box.
//!
//! ## When
//!
//! Before the movement phase, so the box a body sweeps with is the one it is
//! showing. That puts it BEFORE the content rule that pins the pose (which has
//! to run after movement, since it classifies contacts against resolved
//! positions), so the box trails the pin by exactly one tick. That is the right
//! way round: a stomp is judged against the body you actually landed on, and the
//! shape it collapses into applies from the next tick — not retroactively to the
//! contact that caused it.

use bevy::prelude::*;

use ambition_platformer2d_core as ae;
use ambition_sprite_sheet::character::sheets::record_for_target;
use ambition_sprite_sheet::character::{ActorAnimOverride, CharacterAnim};

use crate::features::{ActorRenderSize, ActorSpriteOffset};

/// **This actor's body geometry is authored by its spritesheet, per pose.**
///
/// Presence is the opt-in: an actor without it keeps whatever collision box its
/// spawn authored, exactly as before. Opting in hands the box to the art, which
/// is only meaningful for a sheet that publishes per-animation body metrics —
/// for one that doesn't, every pose resolves to the same static idle bbox and
/// this degenerates to "size the body to its art", which is still an improvement
/// on a hand-guessed rectangle but is not why the seam exists.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SpritePosedBody {
    /// The sheet manifest target the boxes are read from (`"solid_snake"`).
    pub target: String,
    /// World units per sheet pixel. The ONE authored number: it fixes the
    /// actor's on-screen scale, and every box follows from the art at that
    /// scale. Uniform by construction, so the art is never distorted.
    pub world_per_pixel: f32,
}

impl SpritePosedBody {
    pub fn new(target: impl Into<String>, world_per_pixel: f32) -> Self {
        Self {
            target: target.into(),
            world_per_pixel,
        }
    }
}

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
    let record = record_for_target(target)?;
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

/// **The sheet's AUTHORED gameplay body, in sheet pixels — `None` when it only
/// measured one.**
///
/// The question a caller is really asking is *"may I scale this character by
/// this rectangle?"*, and the honest answer is no for a measured alpha bbox: it
/// is the extent of the drawing, hat and outstretched arms included, and using
/// it as a body is how a collision box ends up 1.28× the character inside it.
/// So this refuses rather than returning a number that looks usable
/// (`BodyMetrics::authored_body` is the sheet's own claim, emitted only when a
/// target authored the box).
///
/// The `Idle` pose is the standing body — the same rectangle
/// [`sync_sprite_posed_bodies`] restores `base_size` to.
pub fn authored_body_pixel_size(target: &str) -> Option<ae::Vec2> {
    let record = record_for_target(target)?;
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
        // **The STANCE, which composes with the pose rather than competing with
        // it.** Absent ⇒ a body that never body-modes, and the pose IS the box.
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
        // **The STANDING box is this sheet's `Idle` rectangle**, and it is a
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
        // **The pose says how big the body IS; the MODE says what it is doing
        // with it, and the box is the composition of the two.**
        //
        // ⛔ writing `geometry.collision` straight into `kin.size` silently
        // undid every stance. The crouch is applied ONCE, on the tick the mode
        // changes — `body_mode::mechanics` does `if mode == target { continue }`
        // and `try_change_body_mode_clusters` early-returns on an unchanged mode
        // — so nothing re-asserts the shorter box afterwards, while THIS pass
        // runs every tick. A body on this seam crouched for one tick and then
        // stood back up inside its own crouch, with `BodyModeState` still saying
        // `Crouching`. Probed red before the fix
        // (`a_stance_survives_the_per_tick_resync`).
        //
        // `BodyMode::shape` is the same function the stance transition uses, so
        // the two cannot disagree about what crouching means — and applying it
        // to the POSE's rectangle rather than to `base_size` is what keeps this
        // right for a body whose silhouette changes shape: a boxed snake that
        // crouched would otherwise crouch from its sprawled height.
        let posed_collision = body_mode.map_or(geometry.collision, |mode| {
            mode.body_mode.shape(geometry.collision).size
        });
        if kin.size != posed_collision {
            // Feet-anchored, through the engine's one feet-planted resize op:
            // hold the +gravity face and move the centre by half the change, so a
            // withdraw/emerge never drives the body through the ground it is
            // standing on. This used to be a bare `kin.pos +=` here, which ADR
            // 0024 forbids precisely because it re-derives an authority that
            // already exists.
            //
            // ⛔ **and it used to pass `DEFAULT_GRAVITY_DIR`**, under a comment
            // promising that "a flipped-gravity variant passes its own gravity
            // instead" — a variant that did not exist, on a system that serves
            // every sprite-posed body. In a reversed or horizontal-gravity room
            // the resize anchored the wrong face and pushed the body into or off
            // its own support. The module's contract is that the +gravity face
            // stays planted; the direction has to be the body's, not the
            // default's (GPT 5.6 review, 2026-08-04).
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
            ae::resize_feet_planted(&mut kin, posed_collision, gravity_dir);
        }
        if render_size.map(|r| r.0) != Some(geometry.render) {
            commands
                .entity(entity)
                .try_insert(ActorRenderSize(geometry.render));
        }
        if offset.map(|o| o.0) != Some(geometry.sprite_offset) {
            commands
                .entity(entity)
                .try_insert(ActorSpriteOffset(geometry.sprite_offset));
        }
    }
}

#[cfg(test)]
mod tests;
