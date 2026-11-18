//! Projectile emission captures source facts before deferred construction.
//!
//! Producers submit physical requests through `ProjectileWriter`. It captures
//! source identity, scope, allegiance, and presentation before construction.
//! Both first-step boundaries use the same captured request channel.

use bevy::prelude::Entity;

use ambition_projectiles::{build_in_flight_projectile, InFlightProjectile, ProjectileKind};

/// When a newly materialized projectile begins advancing.
///
/// This is an explicit gameplay-timing fact, not a proxy for who fired the shot.
/// Actor/item/boss projectiles historically materialize before the projectile
/// step and therefore advance on the firing tick. The charged named-body-fire
/// path historically fires after that step and therefore begins on the next tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileStart {
    /// Materialize before this tick's projectile step.
    StepThisTick,
    /// Materialize after this tick's projectile step; first advance next tick.
    StepNextTick,
}

/// How the presentation layer identifies a projectile.
///
/// These variants intentionally say nothing about faction. A sentry or wielded
/// weapon can emit an open-visual projectile on a player's side, and allegiance
/// is captured from the source body at emission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectilePresentation {
    /// A named projectile vocabulary entry (Fireball / Hadouken / …).
    /// The kind component and its registered visual id are both stamped.
    NamedKind(ProjectileKind),
    /// A named kind fired at a charge tier: the kind, and the visual its tier
    /// is drawn with ([`ProjectileKind::visual_id_at`]).
    NamedCharged(ProjectileKind, u8),
    /// An open visual id authored by an actor/item/boss technique. The empty
    /// string means the generic open-projectile look.
    OpenVisual(String),
}

/// Physical intent passed to the native projectile writer.
///
/// `owner == Entity::PLACEHOLDER` means there is deliberately no firing body
/// (for example, an environmental volley). A real owner is stamped as
/// [`ambition_projectiles::ProjectileOwner`] and is the source of faction/team allegiance and
/// presentation-source inheritance at emission.
#[derive(Clone, Debug)]
pub struct ProjectileSpawnRequest {
    pub owner: Entity,
    pub projectile: InFlightProjectile,
    pub presentation: ProjectilePresentation,
    pub start: ProjectileStart,
    /// The use of the owner's move that made this shot, if a move made it.
    ///
    /// `None` is a correct answer, not a placeholder: a gun, a thrown bomb and an
    /// environmental volley have no move. Only the moveset road sets it.
    ///
    /// A shot can land after its move stops. This field lets the damage result
    /// name the move that fired it, not the move playing at impact. Do not
    /// compute it again from the owner's playback at spawn time.
    pub move_instance: Option<u32>,
    /// The owner's ranged weapon fired this shot. See [`ambition_projectiles::WeaponShot`].
    pub weapon_shot: bool,
    /// The growing launch this shot deals, if its weapon authors one. See
    /// [`ambition_projectiles::ProjectileLaunch`].
    pub launch: Option<ambition_projectiles::ProjectileLaunch>,
}

impl ProjectileSpawnRequest {
    /// Build an open-visual projectile from authored spawn data.
    pub fn open(
        owner: Entity,
        spawn: ambition_projectile_spec::ProjectileSpawn,
        start: ProjectileStart,
    ) -> Self {
        let visual_id = spawn.visual_id.clone();
        Self {
            owner,
            projectile: build_in_flight_projectile(spawn),
            presentation: ProjectilePresentation::OpenVisual(visual_id),
            start,
            move_instance: None,
            weapon_shot: false,
            launch: None,
        }
    }

    /// Materialize a named projectile whose gameplay body is already prepared.
    pub fn named(
        owner: Entity,
        projectile: InFlightProjectile,
        kind: ProjectileKind,
        start: ProjectileStart,
    ) -> Self {
        Self {
            owner,
            projectile,
            presentation: ProjectilePresentation::NamedKind(kind),
            start,
            move_instance: None,
            weapon_shot: false,
            launch: None,
        }
    }

    /// The named shot was charged to `tier`: it is drawn as that tier.
    pub fn charged(mut self, tier: u8) -> Self {
        if let ProjectilePresentation::NamedKind(kind) = self.presentation {
            self.presentation = ProjectilePresentation::NamedCharged(kind, tier);
        }
        self
    }

    /// Set the use of the move that made this shot.
    ///
    /// ⚠ Only the moveset road calls this function. Give it the instance from
    /// the firing event. Do not read the owner's playback for this value.
    #[must_use]
    pub fn fired_by_move(mut self, instance: u32) -> Self {
        self.move_instance = Some(instance);
        self
    }

    /// Give this shot the growing launch its weapon authors.
    #[must_use]
    pub fn with_launch(mut self, launch: Option<ambition_projectiles::ProjectileLaunch>) -> Self {
        self.launch = launch;
        self
    }

    /// Mark this shot as one the owner's ranged weapon fired.
    #[must_use]
    pub fn from_weapon(mut self) -> Self {
        self.weapon_shot = true;
        self
    }

    /// Set the use of the move that made this shot, when the caller has one.
    ///
    /// `None` stays `None`: a technique a brain pressed directly has no move.
    /// Only the caller that read the request can tell the two cases apart.
    ///
    /// Pass the value the `ActorActionMessage` carried. Do not read the owner's
    /// playback; see [`Self::fired_by_move`].
    #[must_use]
    pub fn fired_by_move_if_any(mut self, instance: Option<u32>) -> Self {
        self.move_instance = instance;
        self
    }
}

mod emission;
pub mod extension;
mod materialize;
pub use emission::{ProjectileEmission, ProjectileWriter};
pub use materialize::{
    materialize_projectiles_for_next_tick, materialize_projectiles_for_this_tick,
};

mod pending;
pub use pending::PendingProjectileEmissions;
