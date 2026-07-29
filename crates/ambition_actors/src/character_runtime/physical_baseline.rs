//! **What a prepared character says about a BODY, applied one way everywhere.**
//!
//! A character's kit was unified at the finalization barrier: one fold, one
//! answer, every construction path reading the same value. Its *physical*
//! identity was not. Three paths built a body and each decided health, mass and
//! collision size for itself:
//!
//! | | max health | mass | `Explicit` box |
//! |---|---|---|---|
//! | spawned match fighter | `prepared.vitals` | `prepared.vitals` | yes |
//! | adopted match fighter | `prepared.vitals` | `prepared.vitals` | yes |
//! | worn exploration player | the CATALOG row | never set | no |
//!
//! So the same character was a different physical object depending on how it got
//! its body — and the versus duelists, who author 60 and 52 as a deliberate
//! trade, simply did not have those numbers outside a match (GPT 5.6,
//! 2026-07-29). This module is the one path all three now go through.
//!
//! # Why the numbers and the WRITE are separate
//!
//! [`PhysicalBaseline`] resolves *what the values are*. That is the part that has
//! to be identical everywhere, and it is a pure function of the prepared
//! definition. How they reach a body is legitimately different: two of these
//! paths assemble components for an entity that does not exist yet, and one
//! mutates a live one. A single `fn(&mut World)` would have to pretend those are
//! the same operation.
//!
//! # ⚠ What this does NOT do, deliberately
//!
//! **It does not run per tick.** These are construction facts. A projection that
//! rewrote a live body's health would delete the damage it had taken, and one
//! that rewrote its size every frame would be a second geometry authority beside
//! the transit seam (ADR 0024). [`BaselineBoundary`] names the two moments a body
//! may legitimately be told what it physically is.

use bevy::ecs::system::EntityCommands;

use super::{BodySource, PreparedCharacterDefinition};
use ambition_engine_core::Vec2;

/// **The moment a body is being told what it physically is.**
///
/// Not a verbosity knob — the two boundaries genuinely differ about current
/// health, and being explicit is what keeps a re-wear from healing a fighter
/// mid-round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineBoundary {
    /// A body is being built, or activated into a match. It starts at FULL
    /// health, because there is no damage yet for it to have taken.
    Construction,
    /// A live body changes which character it wears. Its accumulated damage is
    /// its own and survives; only the maximum moves, and the current value is
    /// clamped under it.
    ///
    /// ⚠ **geometry does not apply here**, and that is a narrowing rather than an
    /// omission. Resizing a body that is standing on a floor has to re-resolve
    /// its pose against the world, which is the transit seam's job and only its
    /// job (ADR 0024) — and Jon's standing rule is that nothing may shove a body
    /// out of geometry. A character whose silhouette must follow it onto every
    /// body authors [`BodySource::SpriteAuthored`], whose per-pose projection
    /// already reaches every body on every path; [`BodySource::Explicit`] is a
    /// construction-time size and says so.
    Replacement,
}

/// **The physical facts a prepared character states, resolved.**
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
    /// Full body size in world units, when the character authored an explicit
    /// one. `SpriteAuthored` is absent here on purpose: it is not a size, it is a
    /// policy, and its authority is the per-pose projection.
    explicit_size: Option<Vec2>,
}

impl PhysicalBaseline {
    /// Read one prepared character's physical identity.
    pub fn of(prepared: &PreparedCharacterDefinition) -> Self {
        Self {
            // Clamped once, here, rather than at each of the four call sites that
            // used to write `.max(1)` — a body with a zero maximum is dead before
            // its first frame and no author means that.
            max_health: prepared.vitals.max_health.map(|max| max.max(1)),
            mass: prepared.vitals.mass,
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

    /// **Apply to a body that already exists.**
    ///
    /// The live-body half of the seam. `health` and `size` are optional because
    /// not every body carries them in the caller's query — `None` is "this path
    /// cannot write that", which is a different claim from "leave it alone", and
    /// only the caller knows which is true.
    ///
    /// `size` must be the body's kinematic size and the caller is responsible for
    /// what follows it: seating passes it and then transits the body to its seat,
    /// which re-resolves the pose. A caller with nowhere to transit to passes
    /// `None` and gets [`BaselineBoundary::Replacement`]'s narrowing.
    pub fn apply_to_body(
        &self,
        boundary: BaselineBoundary,
        entity: &mut EntityCommands,
        health: Option<&mut ambition_characters::actor::BodyHealth>,
        size: Option<&mut Vec2>,
    ) {
        if let Some(health) = health {
            if let Some(max) = self.max_health {
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
        if let Some(mass) = self.mass {
            // `try_insert`: a session-scoped body can be despawned in the same
            // frame its worn identity last changed, and a torn-down entity is not
            // an error here.
            entity.try_insert(crate::features::Mass(mass));
        }
        if boundary == BaselineBoundary::Construction {
            if let (Some(authored), Some(size)) = (self.explicit_size, size) {
                *size = authored;
            }
        }
    }
}
