//! Prepared-character physical facts shared by every body-construction path.
//!
//! [`PhysicalBaseline`] resolves health, mass, and construction geometry as pure
//! data. [`BaselineBoundary`] keeps creation separate from replacement because a
//! live body retains accumulated damage and geometry may have another runtime
//! authority. These facts are applied only at those boundaries, never per tick.

use bevy::ecs::system::EntityCommands;

use super::{BodySource, PreparedCharacterDefinition};
use ambition_platformer2d_core::Vec2;

/// Boundary at which prepared physical facts may be applied to a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineBoundary {
    /// A body is being built, or activated into a match. It starts at FULL
    /// health, because there is no damage yet for it to have taken.
    Construction,
    /// A live body changes which character it wears. Its accumulated damage is
    /// its own and survives; only the maximum moves, and the current value is
    /// clamped under it.
    ///
    /// geometry does not apply here, and that is a narrowing rather than an omission. A
    /// character whose silhouette must follow it onto every body authors
    /// [`BodySource::SpriteAuthored`], whose per-pose projection already reaches every body on
    /// every path; [`BodySource::Explicit`] is a construction-time size and says so.
    Replacement,
}

/// Values displaced by character replacement and eligible for later retraction.
///
/// Capture each field only when a character first overrides it. A field no
/// character has displaced stays `None` and must not be restored by this path.
/// This keeps change-detected rollback writes idempotent with respect to other
/// authorities over the same body fields.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DisplacedPhysicals {
    /// The pool a persona overwrote. `None` = no persona has ever set
    /// `max_health` on this body, so nothing here may write it.
    pub max_health: Option<i32>,
    /// Outer `None` = no persona has ever set mass. Inner `None` = the body
    /// carried no [`Mass`](ambition_platformer2d_shared_tangle::body::Mass) at all, so retraction REMOVES
    /// the component rather than inventing the ambient 1.0 — a distinction
    /// `Vitals::mass` already documents.
    pub mass: Option<Option<f32>>,
    /// The knockback weight a persona overwrote. `None` = no persona has ever
    /// set one, so nothing here may write it.
    ///
    /// a plain `Option`, not the nested one `mass` needs, and the
    /// difference is real rather than an inconsistency: weight lives on
    /// `CombatTuning`, which a clustered body always carries and which this path
    /// must never remove — removing it would take the body out of the actor
    /// cluster query. Retraction here restores a VALUE; there is no absence to
    /// restore.
    pub knockback_weight: Option<f32>,
}

impl DisplacedPhysicals {
    /// Record what `incoming` is about to displace, for the fields it authors
    /// and no persona has displaced yet. `live_*` are the body's values as they
    /// stand right now, before the write.
    pub fn displace(
        mut self,
        incoming: Option<PhysicalBaseline>,
        live_max_health: Option<i32>,
        live_mass: Option<f32>,
        live_knockback_weight: Option<f32>,
    ) -> Self {
        if self.max_health.is_none() && incoming.and_then(|incoming| incoming.max_health).is_some()
        {
            self.max_health = live_max_health;
        }
        if self.mass.is_none() && incoming.and_then(|incoming| incoming.mass).is_some() {
            self.mass = Some(live_mass);
        }
        if self.knockback_weight.is_none()
            && incoming
                .and_then(|incoming| incoming.knockback_weight)
                .is_some()
        {
            self.knockback_weight = live_knockback_weight;
        }
        self
    }
}

/// What a REPLACEMENT is licensed to put back, resolved for one swap.
///
/// Separate from [`DisplacedPhysicals`] because they answer different questions:
/// that one is the durable record on the body; this one is *of that record, what
/// does THIS incoming character leave unclaimed*. A field the incoming character
/// authors is not retracted — it is overwritten, which is the ordinary path.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PhysicalRetraction {
    pub max_health: Option<i32>,
    pub mass: Option<Option<f32>>,
    pub knockback_weight: Option<f32>,
}

impl PhysicalRetraction {
    /// A construction boundary retracts nothing: there is no outgoing persona
    /// whose contribution could be left behind, so a silent character leaves the
    /// body's freshly-built numbers exactly as it found them.
    pub const NONE: Self = Self {
        max_health: None,
        mass: None,
        knockback_weight: None,
    };

    /// What `incoming` leaves unclaimed, out of what personas have displaced.
    pub fn resolve(incoming: Option<PhysicalBaseline>, displaced: DisplacedPhysicals) -> Self {
        Self {
            max_health: incoming
                .and_then(|incoming| incoming.max_health)
                .is_none()
                .then_some(displaced.max_health)
                .flatten(),
            mass: incoming
                .and_then(|incoming| incoming.mass)
                .is_none()
                .then_some(displaced.mass)
                .flatten(),
            knockback_weight: incoming
                .and_then(|incoming| incoming.knockback_weight)
                .is_none()
                .then_some(displaced.knockback_weight)
                .flatten(),
        }
    }
}

/// The physical facts a prepared character states, resolved.
///
/// Every field is an `Option` carrying the same meaning it has on the
/// definition: `None` is *the author said nothing*, so whatever the body's own
/// construction established stands. That distinction is the whole reason this
/// type can be applied universally — a flat `max_health` with a default of 1
/// could not be, because it would hand a one-hit pool to every character that
/// had never thought about health.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalBaseline {
    max_health: Option<i32>,
    mass: Option<f32>,
    knockback_weight: Option<f32>,
    /// Full body size in world units, when the character authored an explicit
    /// one. `SpriteAuthored` is absent here on purpose: it is not a size, it is a
    /// policy, and its authority is the per-pose projection.
    explicit_size: Option<Vec2>,
}

impl PhysicalBaseline {
    /// Read one prepared character's physical identity.
    pub fn of(prepared: &PreparedCharacterDefinition) -> Self {
        Self {
            max_health: prepared.vitals.max_health.map(|max| max.max(1)),
            mass: prepared.vitals.mass,
            knockback_weight: prepared.vitals.knockback_weight,
            explicit_size: match prepared.body.as_ref() {
                Some(BodySource::Explicit { half_extents }) => {
                    Some(Vec2::new(half_extents.0 * 2.0, half_extents.1 * 2.0))
                }
                Some(BodySource::SpriteAuthored { .. }) | None => None,
            },
        }
    }

    /// The health pool for a body being built from nothing, given the pool its
    /// host would otherwise use.
    ///
    /// For the construction paths that assemble a `Health` before an entity
    /// exists. `standing` is the host's own answer — `DEFAULT_PLAYER_HEALTH` for
    /// the exploration player, the seed's for a seated fighter — and it is what a
    /// character that authored nothing keeps.
    pub fn max_health_over(&self, standing: i32) -> i32 {
        self.max_health.unwrap_or(standing)
    }

    /// The authored body size, for a construction path sizing a body it is about
    /// to spawn. `None` means the caller's own placeholder stands.
    pub fn explicit_size(&self) -> Option<Vec2> {
        self.explicit_size
    }

    /// The authored mass, when there is one.
    pub fn mass(&self) -> Option<f32> {
        self.mass
    }

    /// The authored knockback weight, when there is one. For a construction path
    /// assembling a `CombatTuning` before an entity exists.
    pub fn knockback_weight(&self) -> Option<f32> {
        self.knockback_weight
    }

    /// Apply to a body that already exists.
    ///
    /// The live-body half of the seam. `health` and `size` are optional because
    /// not every body carries them in the caller's query — `None` is "this path
    /// cannot write that", which is a different claim from "leave it alone", and
    /// only the caller knows which is true.
    ///
    /// `geometry` must be the body's LIVE kinematic size and its IDENTITY base
    /// size, together. The caller is responsible for what follows: seating passes
    /// them and then transits the body to its seat, which re-resolves the pose. A
    /// caller with nowhere to transit to passes `None` and gets
    /// [`BaselineBoundary::Replacement`]'s narrowing.
    ///
    /// both, or the size does not survive the first lifecycle transition. This wrote only the
    /// live collider. `BodyBaseSize` is the identity-derived canonical shape: a round reset
    /// restores the collider FROM it, and every body-mode transition (crouch, stand, morph)
    /// computes its target shape FROM it. The two seats diverged again one lifecycle transition
    /// after the seam that exists to stop them diverging.
    ///
    /// Match activation is a CONSTRUCTION boundary, and at a construction
    /// boundary an explicit identity size IS the body's new base.
    /// `retraction` is what this character's SILENCE puts back, and passing
    /// it is the difference between replacement and accumulation. See
    /// [`PhysicalRetraction`]; [`PhysicalRetraction::NONE`] is the construction
    /// reading.
    pub fn apply_to_body(
        &self,
        boundary: BaselineBoundary,
        entity: &mut EntityCommands,
        health: Option<&mut ambition_characters::actor::BodyHealth>,
        combat_tuning: Option<&mut crate::combat::CombatTuning>,
        geometry: Option<BodyGeometry<'_>>,
        retraction: PhysicalRetraction,
    ) {
        if let Some(health) = health {
            // The incoming character's answer, or — at a replacement, and only
            // for a field some persona actually took — the body's own. Never the
            // outgoing character's, which is what "write it only if authored"
            // quietly meant.
            if let Some(max) = self.max_health.or(retraction.max_health) {
                health.health.max = max;
            }
            match boundary {
                BaselineBoundary::Construction => health.health.current = health.health.max,
                // The damage is the body's, not the character's. Clamping rather
                // than refilling is what keeps a mid-round re-wear from being a
                // free heal.
                BaselineBoundary::Replacement => {
                    health.health.current = health.health.current.min(health.health.max)
                }
            }
        }
        // the weight is WRITTEN IN PLACE, never inserted or removed.
        // `CombatTuning` is part of `ActorClusterQueryData`; a body without it
        // leaves the actor cluster query entirely and stops being simulated.
        // Only the field moves.
        if let Some(tuning) = combat_tuning {
            if let Some(weight) = self.knockback_weight.or(retraction.knockback_weight) {
                tuning.weight = weight;
            }
        }
        match (self.mass, retraction.mass) {
            // `try_insert`: a session-scoped body can be despawned in the same
            // frame its worn identity last changed, and a torn-down entity is not
            // an error here.
            (Some(mass), _) => {
                entity.try_insert(ambition_platformer2d_shared_tangle::body::Mass(mass));
            }
            // Silent character putting back what a persona took: the body's own
            // mass, or the ABSENCE of one. Same shape as the
            // `AuthoredMovementTuning` insert/remove pair the re-wear path
            // already runs a few lines later — absence there has always been a
            // retraction, and it was only here that it meant "keep".
            (None, Some(Some(own))) => {
                entity.try_insert(ambition_platformer2d_shared_tangle::body::Mass(own));
            }
            (None, Some(None)) => {
                entity.try_remove::<ambition_platformer2d_shared_tangle::body::Mass>();
            }
            // Nothing to put back: either a construction, or a body no persona
            // has ever given a mass to. Its own value stands.
            (None, None) => {}
        }
        if boundary == BaselineBoundary::Construction {
            if let (Some(authored), Some(geometry)) = (self.explicit_size, geometry) {
                *geometry.live = authored;
                *geometry.base = authored;
            }
        }
    }
}

/// `live` is the collider the movement seam sweeps this frame. `base` is the
/// IDENTITY size a reset restores to and a body-mode transition derives from.
/// They are separate fields on purpose — a crouching body's live size is
/// legitimately not its base — but an identity CHANGE has to move both.
pub struct BodyGeometry<'a> {
    pub live: &'a mut Vec2,
    pub base: &'a mut Vec2,
}
