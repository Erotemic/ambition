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

use ambition_engine_core as ae;
use ambition_sprite_sheet::character::sheets::record_for_target;
use ambition_sprite_sheet::character::CharacterAnim;

use crate::features::{ActorAnimOverride, ActorRenderSize, ActorSpriteOffset};

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
        Option<&ActorRenderSize>,
        Option<&ActorSpriteOffset>,
    )>,
) {
    for (entity, posed, pinned, mut kin, render_size, offset) in &mut bodies {
        let anim = pinned.map_or(CharacterAnim::Idle, |o| o.0);
        let Some(geometry) = posed_body_geometry(&posed.target, anim, posed.world_per_pixel) else {
            continue;
        };
        if kin.size != geometry.collision {
            // Feet-anchored: hold the +gravity face and move the centre by half
            // the change, so a withdraw/emerge never drives the body through the
            // ground it is standing on. Bodies on this seam are ordinary
            // gravity-down actors; a flipped-gravity variant would read the
            // body's own gravity here.
            let shrink = kin.size - geometry.collision;
            kin.pos += ae::DEFAULT_GRAVITY_DIR * (shrink * 0.5);
            kin.size = geometry.collision;
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
