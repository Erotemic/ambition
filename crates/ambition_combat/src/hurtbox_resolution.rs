//! Hurtboxes resolved from simulation state and clocks, independent of rendering.
//!
//! Combat geometry over prepared character facts; the actor kernel schedules the
//! two systems and registers the components. (Carved from the kernel's
//! `character_runtime`, D33, 2026-09-03.)
//!
//! Active move overrides use the move clock; body status poses use their own
//! deterministic timers or locomotion phase; default shapes are static. A body
//! without authored hurtboxes may use the sprite-derived compatibility box, but
//! rendered frames never become gameplay authority.

use bevy::prelude::*;

use ambition_entity_catalog::{HurtboxDoc, HurtboxVolume};

/// A body's authored hurtbox document. Absence selects the sprite-derived
/// compatibility box.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct AuthoredHurtboxes(pub HurtboxDoc);

/// Authoritative body-state pose and gait, each with its elapsed proper time.
/// Pose ids are gameplay facts from [`BODY_POSES`], not renderer animation
/// rows. The gait is how a grounded body moves ([`grounded_gait`]); it is kept
/// apart from the pose, so the hurtbox profiles keyed by pose id do not change
/// when a body walks.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct BodyPoseClock {
    pub pose: String,
    /// Seconds since this pose was entered, in the body's own proper time.
    pub elapsed_s: f32,
    pub gait: Gait,
    /// Seconds since this gait was entered, in the body's own proper time: the
    /// phase of a walk or run clip.
    pub gait_elapsed_s: f32,
}

impl BodyPoseClock {
    pub fn new(pose: impl Into<String>, elapsed_s: f32) -> Self {
        Self {
            pose: pose.into(),
            elapsed_s,
            gait: Gait::Standing,
            gait_elapsed_s: 0.0,
        }
    }
}

/// How a grounded body moves, from simulation facts only. Presentation picks
/// its Idle / Walk / Run row with the same rule ([`grounded_gait`]) but with
/// its own dead band, so the sprite row and the rig clip of a body agree
/// outside that band.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Gait {
    #[default]
    Standing,
    Walking,
    Running,
    Skidding,
}

/// Below this speed along its run axis a body stands. One value for every
/// body: the gait selects the rig clip that the hurt geometry and the
/// attachments are solved from, so it is a gameplay fact.
///
/// ⛔ NOT THE SPRITE DEAD BAND. The sprite pickers keep their own (12 for a
/// player, 8 for an actor) in `ambition_character_sprites`. Until 2026-09-30
/// this rule took the player's 12 when the body had a
/// `PlayerBlinkCameraState`, so two bodies at speed 11 with the same motion
/// facts had different hurt geometry because one had a camera state.
pub const STANDING_BELOW: f32 = 8.0;

/// The gait of a grounded body. `speed` is the size of its velocity along its
/// own run axis (not total speed: a body that slides down its own wall
/// stands). `running` and `skidding` are `BodyMotionFacts`.
pub fn grounded_gait(speed: f32, running: bool, skidding: bool, standing_below: f32) -> Gait {
    if skidding {
        Gait::Skidding
    } else if speed < standing_below {
        Gait::Standing
    } else if running {
        Gait::Running
    } else {
        Gait::Walking
    }
}

impl Default for BodyPoseClock {
    fn default() -> Self {
        Self::new(POSE_IDLE, 0.0)
    }
}

/// Body-state pose ids the engine writes. Content may author a profile for any of
/// them; an unauthored one falls through to the default shapes.
///
/// this list is a CONTRACT, not a wish list. An id documented here that
/// [`body_pose`] can never produce is worse than a missing feature: content
/// authors a profile for it, the profile validates, and it is silently never
/// selected. [`BODY_POSES`] and `body_pose`'s reachable set are pinned equal by
/// a test, so a pose cannot be named without also being written.
pub const POSE_IDLE: &str = "idle";
pub const POSE_HITSTUN: &str = "hitstun";
pub const POSE_AIRBORNE: &str = "airborne";
pub const POSE_CROUCH: &str = "crouch";

/// Every pose id the engine writes. See the note on [`POSE_IDLE`].
pub const BODY_POSES: [&str; 4] = [POSE_HITSTUN, POSE_CROUCH, POSE_AIRBORNE, POSE_IDLE];

/// The pose selection rule, from authoritative simulation facts only.
///
/// Precedence is by how much the state overrides the body's shape: hitstun is a
/// reaction the body does not choose, a crouch is a stance it does, and airborne
/// is merely where it is. crouch outranks airborne because `BodyMode` only
/// reaches `Crouching` through a grounded stance change — the two are not
/// expected to be true at once, and if they ever are, the stance is the thing
/// that actually changed the silhouette.
pub fn body_pose(hitstun: bool, crouching: bool, airborne: bool) -> &'static str {
    if hitstun {
        POSE_HITSTUN
    } else if crouching {
        POSE_CROUCH
    } else if airborne {
        POSE_AIRBORNE
    } else {
        POSE_IDLE
    }
}

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
    /// The hurt parts of the body's semantic rig at its resolved pose
    /// (`crate::body_rig`). Outranks the authored pose profiles and default,
    /// and is outranked by a move override.
    RigDefault,
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
    resolve_hurtboxes_with_rig(Some(doc), || None, active_move, pose)
}

/// [`resolve_hurtboxes`] for a body that may also carry a semantic rig.
///
/// One precedence, no reconciliation: an authored MOVE override, else the
/// rig's hurt parts (`rig_volumes`, asked only when no move override
/// answered), else the authored pose profile, else the authored default.
/// A rig with no hurt parts answers `None` and changes nothing.
pub fn resolve_hurtboxes_with_rig(
    doc: Option<&HurtboxDoc>,
    rig_volumes: impl FnOnce() -> Option<Vec<HurtboxVolume>>,
    active_move: Option<(&str, f32)>,
    pose: Option<(&str, f32)>,
) -> ResolvedHurtboxes {
    if let (Some(doc), Some((move_id, elapsed_s))) = (doc, active_move) {
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
    if let Some(volumes) = rig_volumes() {
        return ResolvedHurtboxes {
            volumes,
            source: HurtboxSelection::RigDefault,
        };
    }
    match doc {
        Some(doc) => resolve_authored_defaults(doc, pose),
        None => ResolvedHurtboxes::default(),
    }
}

fn resolve_authored_defaults(doc: &HurtboxDoc, pose: Option<(&str, f32)>) -> ResolvedHurtboxes {
    // Ask the sources in precedence order individually rather than taking
    // `volumes_for`'s answer blind, because the CALLER needs to know which one
    // won: "the box is wrong" is unactionable until you know whether a move
    // override, the rig, a pose profile, or the default produced it.
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
/// Deliberately NOT a query for `CharacterAnimator`, `Sprite`, or anything in
/// `ambition_render`. This crate cannot even name them, which is the strongest
/// available form of §4.11's prohibition.
/// The set [`resolve_body_hurtboxes`] runs in — **published so a consumer can
/// order against a PHASE instead of against this function's identity.**
///
/// ⭐⭐ IT EXISTS BECAUSE A CONSUMER WAS NAMING THE SYSTEM, and the consumer said
/// so in its own words: *"the one intra-crate edge that is genuinely between two
/// systems: the character runtime resolves the silhouette this reads."* The edge
/// is real — `refresh_body_damageable_volumes` reads what this publishes — but
/// expressing it required `ambition_platformer2d_actor_monolith` to name a
/// function in this crate, which is private cross-domain ordering authority.
/// It did so because there was nothing else to name.
///
/// ⛔ THE MEMBERSHIP IS THE INSTALLER'S TO DECLARE. This crate publishes the
/// vocabulary; whoever composes the schedule decides which phase it sits in and
/// what shares it — the same split `HazardTickSet` above states, and the reason
/// this is a bare marker with no `configure_sets` beside it.
///
/// ⚠ ONE MEMBER TODAY, so `.after(BodyHurtboxesResolved)` is exactly
/// `.after(resolve_body_hurtboxes)` — and that is the point rather than a
/// caveat: the consumer gets the same guarantee it had, expressed in vocabulary
/// this crate can widen later without every consumer being rewritten.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyHurtboxesResolved;

pub fn resolve_body_hurtboxes(
    mut bodies: Query<(
        Option<&AuthoredHurtboxes>,
        Option<(&crate::body_rig::BodyRig, &crate::body_rig::BodyRigPose)>,
        Option<&crate::moveset::MovePlayback>,
        Option<&BodyPoseClock>,
        Option<&ambition_platformer2d_core::BodyKinematics>,
        &mut ResolvedHurtboxes,
    )>,
) {
    for (authored, rig, playback, pose, kin, mut resolved) in &mut bodies {
        if authored.is_none() && rig.is_none() {
            // Nothing authors this body's hurtboxes. Only what a RIG put here
            // is taken back: a body that loses its rig must not keep its last
            // rig pose as a hurt shape.
            if resolved.source == HurtboxSelection::RigDefault {
                *resolved = ResolvedHurtboxes::default();
            }
            continue;
        }
        let active_move = playback.map(|p| (p.spec.id.as_str(), p.t));
        let pose_clock = pose.map(|p| (p.pose.as_str(), p.elapsed_s));
        // The rig is feet-anchored and a hurt volume is centre-relative, so
        // the rig asks where the body's feet are below its centre.
        let rig_volumes = || {
            let (rig, rig_pose) = rig?;
            let feet_below_center = kin.map_or(0.0, |kin| kin.size.y * 0.5);
            rig_pose.hurt_volumes(&rig.0, feet_below_center)
        };
        let next = resolve_hurtboxes_with_rig(
            authored.map(|authored| &authored.0),
            rig_volumes,
            active_move,
            pose_clock,
        );
        // Change detection: a pose holds for many ticks, and this feeds consumers
        // that key on `Changed`.
        if *resolved != next {
            *resolved = next;
        }
    }
}

/// Write each body's pose clock from authoritative simulation state.
///
/// Hitstun outranks airborne outranks idle. The gait is written for a body on
/// the ground and is `Standing` in the air. Both the actors (in `WorldPrep`)
/// and the players (in `PlayerSimulation`) have moved this tick before this
/// runs in `Combat`, so the gait reads this tick's motion.
#[allow(clippy::type_complexity)]
pub fn advance_body_pose_clocks(
    world_time: Res<ambition_time::WorldTime>,
    mut bodies: Query<(
        (
            &ambition_characters::actor::BodyCombat,
            Option<&ambition_platformer2d_core::BodyGroundState>,
            Option<&ambition_platformer2d_core::BodyModeState>,
            Option<&ambition_time::ProperTimeScale>,
        ),
        (
            Option<&ambition_platformer2d_core::BodyKinematics>,
            Option<&ambition_platformer2d_core::BodyMotionFacts>,
            Option<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
        ),
        &mut BodyPoseClock,
    )>,
) {
    for ((combat, ground, body_mode, scale), (kinematics, facts, frame), mut clock) in &mut bodies {
        // The body's OWN proper time, the same clock `advance_move_playback` uses.
        // A dilated body's hitstun profile and its move profile must not disagree
        // about how much time passed, or a bullet-time hit resolves against a
        // silhouette from a different instant.
        let dt = world_time.entity_dt(scale.copied().unwrap_or_default());
        let pose = body_pose(
            combat.hitstun_timer > 0.0,
            body_mode.is_some_and(|m| {
                m.body_mode == ambition_platformer2d_core::player_state::BodyMode::Crouching
            }),
            ground.is_some_and(|g| !g.on_ground),
        );
        if clock.pose == pose {
            clock.elapsed_s += dt;
        } else {
            clock.pose = pose.to_string();
            clock.elapsed_s = 0.0;
        }
        let gait = match (ground.is_some_and(|g| g.on_ground), kinematics, facts) {
            (true, Some(kinematics), Some(facts)) => {
                let side = frame.map_or_else(
                    || ambition_platformer2d_core::AccelerationFrame::new(ambition_platformer2d_core::DEFAULT_GRAVITY_DIR),
                    |frame| frame.basis(),
                )
                .side;
                grounded_gait(kinematics.vel.dot(side).abs(), facts.running, facts.skidding, STANDING_BELOW)
            }
            _ => Gait::Standing,
        };
        if clock.gait == gait {
            clock.gait_elapsed_s += dt;
        } else {
            clock.gait = gait;
            clock.gait_elapsed_s = 0.0;
        }
    }
}

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
