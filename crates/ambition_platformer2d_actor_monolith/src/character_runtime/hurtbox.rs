//! **Hurtboxes resolved from simulation clocks, never from what was drawn.** (§4.11)
//!
//! Hitboxes belong to the move — that was already true, `MoveWindow.volumes`.
//! Hurtboxes do not, and the prohibition is the whole design:
//!
//! > A hurtbox must never be derived from the rendered animation frame: not from
//! > whether an image loaded, not from the renderer's clock, not from frame
//! > interpolation, and it must exist in headless runs.
//!
//! Nothing in this module can reach an asset, a renderer, or a decoded frame. A
//! headless RL rollout and a windowed playtest compute identical volumes, because
//! there is no path by which "is anyone watching" could enter the answer. The
//! sprite-derived bbox stays the compatibility default when a body authored
//! nothing — never the authority.
//!
//! ## Selection, and which clock
//!
//! ```text
//! active move override   → the MOVE clock (owner's proper time since move start)
//! body pose/status       → that state's own timer (hitstun, tumble) or a
//!                          deterministic locomotion phase
//! default body shapes    → static
//! ```
//!
//! One format, several authoritative clocks. Hitstun is a body STATE, not a fake
//! move: moves initiate actions with cancels and motion locks, and modelling
//! hitstun as one would make it cancellable.

use bevy::prelude::*;

use ambition_entity_catalog::{HurtboxDoc, HurtboxVolume};

/// A body's authored hurtbox document.
///
/// Presence is the opt-in. A body without it keeps the sprite-derived
/// compatibility box, which is the pre-existing behaviour and still correct for
/// every character that has not authored volumes yet.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct AuthoredHurtboxes(pub HurtboxDoc);

/// The body-state pose this body is in, and how long it has been there.
///
/// Written by the sim from authoritative state, so the pose id is a gameplay fact
/// rather than an animation row. The engine knows the vocabulary
/// (`hitstun`, `tumble`, `crouch`, `shield`, `airborne`, `ledge_hang`, `run`,
/// `idle`); content authors profiles against it.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct BodyPoseClock {
    pub pose: String,
    /// Seconds since this pose was entered, in the body's own proper time.
    pub elapsed_s: f32,
}

impl BodyPoseClock {
    pub fn new(pose: impl Into<String>, elapsed_s: f32) -> Self {
        Self {
            pose: pose.into(),
            elapsed_s,
        }
    }
}

impl Default for BodyPoseClock {
    fn default() -> Self {
        Self::new(POSE_IDLE, 0.0)
    }
}

/// Body-state pose ids the engine writes. Content may author a profile for any of
/// them; an unauthored one falls through to the default shapes.
pub const POSE_IDLE: &str = "idle";
pub const POSE_HITSTUN: &str = "hitstun";
pub const POSE_AIRBORNE: &str = "airborne";

/// The volumes a body is damageable through THIS tick, and where they came from.
///
/// Derived every tick before anything tests against it, so it is rollback-derived
/// rather than rollback state.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct ResolvedHurtboxes {
    pub volumes: Vec<HurtboxVolume>,
    pub source: HurtboxSelection,
}

/// Which authored source answered, so a wrong box is diagnosable without
/// guessing which of three timelines won.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HurtboxSelection {
    /// A move-time override, sampled on the move clock.
    MoveOverride,
    /// A body pose/status profile, sampled on that state's timer.
    PoseProfile,
    /// The authored default shapes.
    Default,
    /// Nothing authored. The caller keeps its sprite-derived compatibility box —
    /// distinct from "authored an empty timeline", which would mean invulnerable.
    #[default]
    Unauthored,
}

/// Resolve one body's hurtboxes from its authored doc and its live clocks.
///
/// Pure, and takes the clocks as values: that is what makes it testable headless
/// and what keeps a renderer from being able to reach it.
pub fn resolve_hurtboxes(
    doc: &HurtboxDoc,
    active_move: Option<(&str, f32)>,
    pose: Option<(&str, f32)>,
) -> ResolvedHurtboxes {
    // Ask the sources in precedence order individually rather than taking
    // `volumes_for`'s answer blind, because the CALLER needs to know which one
    // won: "the box is wrong" is unactionable until you know whether a move
    // override, a pose profile, or the default produced it.
    if let Some((move_id, elapsed_s)) = active_move {
        if let Some(volumes) = doc
            .moves
            .get(move_id)
            .and_then(|timeline| timeline.volumes_at(elapsed_s))
        {
            return ResolvedHurtboxes {
                volumes: volumes.to_vec(),
                source: HurtboxSelection::MoveOverride,
            };
        }
    }
    if let Some((pose_id, elapsed_s)) = pose {
        if let Some(volumes) = doc
            .poses
            .get(pose_id)
            .and_then(|timeline| timeline.volumes_at(elapsed_s))
        {
            return ResolvedHurtboxes {
                volumes: volumes.to_vec(),
                source: HurtboxSelection::PoseProfile,
            };
        }
    }
    match doc
        .default
        .as_ref()
        .and_then(|timeline| timeline.volumes_at(0.0))
    {
        Some(volumes) => ResolvedHurtboxes {
            volumes: volumes.to_vec(),
            source: HurtboxSelection::Default,
        },
        None => ResolvedHurtboxes::default(),
    }
}

/// Publish every authored body's resolved hurtboxes for this tick.
///
/// Reads the MOVE clock from `MovePlayback.t` — the owner's proper time since the
/// move started, which is the same clock the move's own hit windows use, so a
/// move's hurtbox timeline and its hitbox timeline cannot disagree about when they
/// are. The pose clock comes from the body state.
///
/// ⚠ Deliberately NOT a query for `CharacterAnimator`, `Sprite`, or anything in
/// `ambition_render`. This crate cannot even name them, which is the strongest
/// available form of §4.11's prohibition.
pub fn resolve_body_hurtboxes(
    mut bodies: Query<(
        &AuthoredHurtboxes,
        Option<&ambition_combat::moveset::MovePlayback>,
        Option<&BodyPoseClock>,
        &mut ResolvedHurtboxes,
    )>,
) {
    for (authored, playback, pose, mut resolved) in &mut bodies {
        let active_move = playback.map(|p| (p.spec.id.as_str(), p.t));
        let pose_clock = pose.map(|p| (p.pose.as_str(), p.elapsed_s));
        let next = resolve_hurtboxes(&authored.0, active_move, pose_clock);
        // Change detection: a pose holds for many ticks, and this feeds consumers
        // that key on `Changed`.
        if *resolved != next {
            *resolved = next;
        }
    }
}

/// Write each body's pose clock from authoritative simulation state.
///
/// Hitstun outranks airborne outranks idle. The timer is the state's OWN clock —
/// `hitstun_timer` counts DOWN, so elapsed is measured from when the state was
/// entered by tracking the peak, which the pose clock does by only resetting on a
/// pose CHANGE.
pub fn advance_body_pose_clocks(
    world_time: Res<ambition_time::WorldTime>,
    mut bodies: Query<(
        &ambition_characters::actor::BodyCombat,
        Option<&ambition_platformer2d_core::BodyGroundState>,
        Option<&ambition_time::ProperTimeScale>,
        &mut BodyPoseClock,
    )>,
) {
    for (combat, ground, scale, mut clock) in &mut bodies {
        // The body's OWN proper time, the same clock `advance_move_playback` uses.
        // A dilated body's hitstun profile and its move profile must not disagree
        // about how much time passed, or a bullet-time hit resolves against a
        // silhouette from a different instant.
        let dt = world_time.entity_dt(scale.copied().unwrap_or_default());
        let pose = if combat.hitstun_timer > 0.0 {
            POSE_HITSTUN
        } else if ground.is_some_and(|g| !g.on_ground) {
            POSE_AIRBORNE
        } else {
            POSE_IDLE
        };
        if clock.pose == pose {
            clock.elapsed_s += dt;
        } else {
            clock.pose = pose.to_string();
            clock.elapsed_s = 0.0;
        }
    }
}

#[cfg(test)]
mod damage_tests;
#[cfg(test)]
mod tests;

/// Place one entity-local hurtbox volume into the world.
///
/// `facing` mirrors the x offset the same way hit volumes are mirrored, so a
/// character's authored right-facing silhouette is correct when it turns around
/// without authoring a second timeline. A circle becomes its bounding square: this
/// is the COARSE pass (§7.10) and the damageable-volume seam speaks AABBs; a true
/// circle-vs-box test is a later refinement, not a correctness gap, because a
/// slightly generous hurtbox errs toward being hittable rather than invulnerable.
pub fn hurtbox_world_aabb(
    volume: &HurtboxVolume,
    body_center: ambition_platformer2d_core::Vec2,
    facing: f32,
) -> ambition_platformer2d_core::CenteredAabb {
    let mirror = if facing < 0.0 { -1.0 } else { 1.0 };
    let (offset, half_extents) = match volume.shape {
        ambition_entity_catalog::VolumeShape::Rect {
            offset,
            half_extents,
        } => (offset, half_extents),
        ambition_entity_catalog::VolumeShape::Circle { offset, radius } => {
            (offset, (radius, radius))
        }
    };
    let center = ambition_platformer2d_core::Vec2::new(
        body_center.x + offset.0 * mirror,
        body_center.y + offset.1,
    );
    ambition_platformer2d_core::CenteredAabb::from_center_size(
        center,
        ambition_platformer2d_core::Vec2::new(half_extents.0 * 2.0, half_extents.1 * 2.0),
    )
}

impl ResolvedHurtboxes {
    /// This body's damageable volumes in world space, or `None` when nothing was
    /// authored — the caller then keeps its coarse body AABB.
    ///
    /// `Some(vec![])` is a real answer and means INVULNERABLE HERE, which is why
    /// the unauthored case has to be a distinct variant rather than an empty list.
    pub fn world_volumes(
        &self,
        body_center: ambition_platformer2d_core::Vec2,
        facing: f32,
    ) -> Option<Vec<ambition_platformer2d_core::CenteredAabb>> {
        if self.source == HurtboxSelection::Unauthored {
            return None;
        }
        Some(
            self.volumes
                .iter()
                .map(|volume| hurtbox_world_aabb(volume, body_center, facing))
                .collect(),
        )
    }
}
