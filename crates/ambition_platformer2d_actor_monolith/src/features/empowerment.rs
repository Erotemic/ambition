//! **A timed empowerment** — a body holding a SET of super-state traits for a
//! while.
//!
//! Jon: *"There should be an elegant way to represent the idea of I'm invincible
//! and I hurt everything I touch and compose those together for the
//! COSMIC_QUASAR_SUPER_STATE."*
//!
//! So there is no "super mode" here, and deliberately no Mary-O in it. There are
//! independent TRAITS ([`Empowerment`]) and a duration, and a game composes the
//! ones it wants:
//!
//! ```ignore
//! const COSMIC_QUASAR: Empowerment =
//!     Empowerment::UNTOUCHABLE.with(Empowerment::HARMS_ON_CONTACT);
//! ```
//!
//! A super mode expressed as an enum grows a variant per character, and every
//! system that reads it grows a match arm — that is the monolith Jon is warning
//! about. Traits compose instead: Sanic's super form asks for `UNTOUCHABLE`
//! alone and needs no new engine code, and a future "heavy" that harms on
//! contact without being safe asks for `HARMS_ON_CONTACT` alone.
//!
//! ## Each trait delegates to a seam that already exists
//!
//! Nothing here implements invulnerability or damage. `UNTOUCHABLE` takes the
//! `EMPOWERED` reason in the body's [`Invulnerability`] set — the same set a
//! transformation beat takes `TRANSFORMING` in — so the two overlap without
//! either cancelling the other. `HARMS_ON_CONTACT` publishes an ordinary
//! [`Hitbox`] that follows the body, so contact damage resolves through the
//! exact path a sword swing does: hit-once memory, knockback, strike sound,
//! rollback registration, all of it, none of it re-implemented.
//!
//! ## The hitbox is found, not remembered
//!
//! The body does NOT store the hitbox's `Entity`. A back-reference to an entity
//! that can die independently has to be invalidated when it does, and forgetting
//! that is what left Mary-O's quasar overlay permanently dark earlier today. The
//! hitbox carries a [`ContactHitbox`] marker naming its owner instead, so "does
//! this body already have one" is a QUERY. There is no cache, so there is
//! nothing to invalidate.

use bevy::prelude::*;

use ambition_characters::actor::{BodyHealth, Invulnerability};
use ambition_platformer2d_core as ae;
use ambition_vfx::{Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback, HitboxLifetime, HitSide};

/// The traits an empowerment can grant, as a set.
///
/// A set rather than an enum because they are INDEPENDENT: being unhittable and
/// hurting what you touch are different claims, either is useful alone, and a
/// game that wants both should say both rather than name a third thing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Empowerment(u32);

impl Empowerment {
    /// Nothing can hurt this body. Delegates to the [`Invulnerability::EMPOWERED`]
    /// reason, so it coexists with every other reason rather than replacing them.
    pub const UNTOUCHABLE: Self = Self(1 << 0);
    /// This body's own footprint damages what it overlaps — a star-powered
    /// runner flattening what it touches.
    pub const HARMS_ON_CONTACT: Self = Self(1 << 1);

    /// Nothing granted.
    pub const fn none() -> Self {
        Self(0)
    }

    /// Both of these, and whatever else is added later.
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Is this trait granted?
    pub fn holds(self, trait_: Self) -> bool {
        self.0 & trait_.0 != 0
    }
}

/// **A body currently empowered**, and for how much longer.
///
/// Snapshot state by the strictest reading: it gates whether hits land AND
/// whether this body deals them, so a rollback that dropped it would disagree
/// with itself about damage. Games register it with their own rollback rows.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Empowered {
    pub remaining: f32,
    pub traits: Empowerment,
}

impl Empowered {
    /// Grant `traits` for `seconds`.
    pub fn new(traits: Empowerment, seconds: f32) -> Self {
        Self {
            remaining: seconds,
            traits,
        }
    }
}

/// How hard a contact-harming body hits, and how far it throws what it hits.
///
/// Engine defaults rather than per-game numbers: a game that wants its own
/// authors them on the component. Deliberately lethal to a one-HP walker, which
/// is what "I flatten everything I run through" means in a platformer.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ContactHarm {
    pub damage: i32,
    pub knockback: HitboxKnockback,
}

impl Default for ContactHarm {
    fn default() -> Self {
        Self {
            damage: 100,
            knockback: HitboxKnockback::LaunchSpeed {
                base: 420.0,
                growth: 0.0,
            },
        }
    }
}

/// Marks the strike volume an empowered body publishes, naming its owner so the
/// pair is discoverable by QUERY rather than by a stored `Entity`.
#[derive(Component, Clone, Copy, Debug)]
pub struct ContactHitbox {
    pub owner: Entity,
}

/// Seconds of lifetime the contact hitbox is kept topped up with. Longer than a
/// tick so a dropped frame does not blink it out, short enough that it vanishes
/// promptly if this system ever stops running.
const CONTACT_HITBOX_LEASE_S: f32 = 0.2;

/// **Run every empowerment**: hold its traits while it lasts, release them when
/// it ends.
///
/// Traits are re-asserted every tick rather than set once at the start. That is
/// not defensive coding — an empowerment is a CONTINUOUS claim ("I am
/// untouchable *now*"), and a system that stated it once would have to be
/// consulted by everything that might overwrite it. Re-stating is what makes the
/// claim true independently of who else writes.
pub fn run_empowerments(
    time: Res<ambition_time::WorldTime>,
    mut commands: Commands,
    mut bodies: Query<(
        Entity,
        &mut Empowered,
        &mut BodyHealth,
        &ae::BodyKinematics,
        Option<&ContactHarm>,
    )>,
    hitboxes: Query<(Entity, &ContactHitbox)>,
) {
    let dt = time.scaled_dt;
    for (body, mut empowered, mut health, kin, harm) in &mut bodies {
        empowered.remaining -= dt;
        let live = empowered.remaining > 0.0;

        // ── Untouchable ───────────────────────────────────────────────────
        // Our reason only. A transformation beat overlapping this keeps its own,
        // and neither can strip the other by ending first.
        health
            .health
            .invulnerable
            .set(Invulnerability::EMPOWERED, live && empowered.traits.holds(Empowerment::UNTOUCHABLE));

        // ── Harms on contact ──────────────────────────────────────────────
        let wants_hitbox = live && empowered.traits.holds(Empowerment::HARMS_ON_CONTACT);
        let existing = hitboxes
            .iter()
            .find(|(_, marker)| marker.owner == body)
            .map(|(entity, _)| entity);
        match (wants_hitbox, existing) {
            (true, Some(hitbox)) => {
                // Top up the lease. The volume follows the owner on its own, so
                // there is nothing else to keep in step.
                commands.entity(hitbox).try_insert(HitboxLifetime {
                    remaining_s: CONTACT_HITBOX_LEASE_S,
                });
            }
            (true, None) => {
                let harm = harm.copied().unwrap_or_default();
                commands.spawn((
                    Hitbox {
                        owner: body,
                        // The body's own side: an empowered PLAYER must hurt
                        // enemies, not other players. Read off the body rather
                        // than assumed, so an empowered NPC works too.
                        source: HitSide::Player,
                        // Her whole footprint, centred on her — "everything I
                        // touch" is literally her collision box.
                        anchor: HitboxAnchor::FollowOwner {
                            local_offset: ae::Vec2::ZERO,
                        },
                        half_extent: kin.size * 0.5,
                        shape: None,
                        facing: 1.0,
                        damage: harm.damage,
                        knockback: harm.knockback,
                        launch_dir: None,
                        frame_down: ae::DEFAULT_GRAVITY_DIR,
                        strike_sfx: None,
                    },
                    HitboxLifetime {
                        remaining_s: CONTACT_HITBOX_LEASE_S,
                    },
                    // Hit-once memory, so running THROUGH something hits it once
                    // rather than every tick it overlaps.
                    HitboxHits::default(),
                    ContactHitbox { owner: body },
                    Name::new("Empowered contact volume"),
                ));
            }
            (false, Some(hitbox)) => {
                commands.entity(hitbox).despawn();
            }
            (false, None) => {}
        }

        if !live {
            commands.entity(body).remove::<Empowered>();
        }
    }
}

/// Sweep a contact volume whose owner is gone.
///
/// The mirror of the pairing above, and the half that is always forgotten: the
/// hitbox outlives its owner otherwise, following a despawned entity and hurting
/// whatever happens to be standing where the body used to be.
pub fn despawn_orphaned_contact_hitboxes(
    mut commands: Commands,
    owners: Query<(), With<Empowered>>,
    hitboxes: Query<(Entity, &ContactHitbox)>,
) {
    for (hitbox, marker) in &hitboxes {
        if owners.get(marker.owner).is_err() {
            commands.entity(hitbox).despawn();
        }
    }
}

#[cfg(test)]
mod tests;
