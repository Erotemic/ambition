//! **An empowerment** — a body holding a SET of super-state traits, for a while
//! or for as long as whoever granted them keeps them.
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
//! about. Traits compose instead: Sanic's super form asks for the same two and
//! needs no new engine code, a future invincibility ring asks for `UNTOUCHABLE`
//! alone, and a "heavy" that flattens things without being safe itself asks for
//! `HARMS_ON_CONTACT` alone. None of them is a case in here.
//!
//! ## Each trait delegates to a seam that already exists
//!
//! Nothing here implements invulnerability or damage resolution. `UNTOUCHABLE`
//! takes the `EMPOWERED` reason in the body's [`Invulnerability`] set — the same
//! set a transformation beat takes `TRANSFORMING` in — so the two overlap
//! without either cancelling the other. `HARMS_ON_CONTACT` writes an ordinary
//! [`HitEvent`], so the struck body's own victim consumer applies it: i-frames,
//! knockback, hurt feedback, death, all of it, none of it re-implemented.
//!
//! ## Why this is NOT a `Hitbox`
//!
//! "My body harms what it touches" is body-vs-body contact, which the engine
//! already models for the other direction (`apply_actor_contact_damage`, an
//! enemy's body hurting what it walks into). This is that rule, pointed outward,
//! and it is the same rule Sanic's badniks were resolving by hand against a
//! character id. A transient attack volume would encode the wrong primitive even
//! though a live `Hitbox` is now correctly sufficient authority to deal damage.
//!
//! Historically this distinction was accidentally enforced by a bad gameplay
//! dependency: Player `FollowOwner` hitboxes were dropped unless the owner's
//! `BodyMelee.swing` read-model was populated. That made correctly materialized
//! strike geometry silently inert and contradicted the moveset contract that
//! `BodyMelee` is presentation/read-model only. The damage resolver no longer
//! consults that projection; contact harm stays here because contact is what the
//! mechanic MEANS.
//!
//! ## Hitting once is the VICTIM's rule, not ours
//!
//! There is no hit-memory here and deliberately none: a body struck this tick
//! takes i-frames from its own consumer, and running through it keeps producing
//! events that its i-frames eat. Keeping a per-empowerment `hit` set would be a
//! second dedup authority that rollback would have to carry, disagreeing with
//! the first the moment one of them rewound.

use bevy::prelude::*;

use ambition_characters::actor::{BodyCombat, BodyHealth, Invulnerability};
use ambition_combat::components::{ActorFaction, CenteredAabb};
use ambition_combat::events::{
    HitEvent, HitKnockback, HitKnockbackMagnitude, HitMode, HitSource, HitTarget,
};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;

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

    /// The raw set, for a checksum. Not for logic — ask [`Self::holds`].
    pub fn bits(self) -> u32 {
        self.0
    }
}

/// **A body currently empowered**, and for how much longer — if that is even a
/// question.
///
/// Snapshot state by the strictest reading: it gates whether hits land AND
/// whether this body deals them, so a rollback that dropped it would disagree
/// with itself about damage. Games register it with their own rollback rows.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Empowered {
    /// Seconds left, or `None` for an empowerment HELD by whatever granted it.
    ///
    /// The two are genuinely different states, not one with a sentinel. A
    /// pickup starts a clock and then has nothing more to do with it (Jon: *"an
    /// item should just be triggering the start of the invincible state. It
    /// shouldn't be bound to the lifetime of that item"*). A form — Sanic's
    /// super identity — is true for exactly as long as the body wears it, and
    /// expressing that as a very large number would be a lie that eventually
    /// comes due.
    pub remaining: Option<f32>,
    pub traits: Empowerment,
}

impl Empowered {
    /// Grant `traits` for `seconds`, after which this leaves the body.
    pub fn for_seconds(traits: Empowerment, seconds: f32) -> Self {
        Self {
            remaining: Some(seconds),
            traits,
        }
    }

    /// Grant `traits` until whoever granted them takes them back. Nothing here
    /// expires it — the granting authority owns that, and usually already owns
    /// the state it derives from.
    pub fn held(traits: Empowerment) -> Self {
        Self {
            remaining: None,
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
    /// Feel scale for the separating launch, in the same units every other
    /// knockback in the game is expressed in.
    pub knockback: f32,
}

impl Default for ContactHarm {
    fn default() -> Self {
        Self {
            damage: 100,
            knockback: 1.0,
        }
    }
}

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
    mut bodies: Query<(Entity, &mut Empowered, &mut BodyHealth)>,
) {
    let dt = time.scaled_dt;
    for (body, mut empowered, mut health) in &mut bodies {
        // A HELD empowerment has no clock to run: it is live until its granter
        // removes it, and counting down toward an expiry that must never arrive
        // is how "indefinite" quietly becomes "about five minutes".
        let live = match empowered.remaining {
            Some(remaining) => {
                let next = remaining - dt;
                empowered.remaining = Some(next);
                next > 0.0
            }
            None => true,
        };

        // ── Untouchable ───────────────────────────────────────────────────
        // Our reason only. A transformation beat overlapping this keeps its own,
        // and neither can strip the other by ending first.
        health.health.invulnerable.set(
            Invulnerability::EMPOWERED,
            live && empowered.traits.holds(Empowerment::UNTOUCHABLE),
        );

        if !live {
            commands.entity(body).remove::<Empowered>();
        }
    }
}

/// **Everything an empowered body touches takes the hit.**
///
/// The `HARMS_ON_CONTACT` half, and the mirror of `apply_actor_contact_damage`:
/// that one is an actor's body hurting what it walks into, this is a body
/// hurting what it runs THROUGH. Same primitive, opposite direction, and both
/// end in a [`HitEvent`] the victim's own consumer resolves.
///
/// Who may be hit is decided relationally, by the shared
/// [`damage_lands_between`](ambition_combat::targeting::damage_lands_between) —
/// the same predicate every swing in the game asks. That is what makes this
/// actor-generic rather than a player rule: an empowered NPC flattens what an
/// NPC may flatten, without this system knowing what a player is.
///
/// ⚠ a corpse is skipped, and so is a body its own vulnerability rule protects
/// (i-frames, a dodge roll, a parry). Both checks are the SHARED ones — a second
/// opinion about who is hittable is the bug this file was rewritten to remove.
pub fn apply_contact_harm(
    mut hit_events: MessageWriter<HitEvent>,
    empowered: Query<(
        Entity,
        &Empowered,
        &ae::BodyKinematics,
        &ActorFaction,
        Option<&ContactHarm>,
        Option<&ambition_combat::targeting::MatchTeam>,
    )>,
    victims: Query<(
        Entity,
        &CenteredAabb,
        &ActorFaction,
        &BodyHealth,
        &ae::BodyMotionFacts,
        &crate::actor::BodyShieldState,
        &BodyCombat,
        // ⛔ a `Has<PlayerEntity>` column used to ride here to pick between two
        // target variants. There is one, so the column is gone with the fork.
        Option<&ambition_combat::targeting::MatchTeam>,
    )>,
    tuning: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
) {
    let friendly_fire = tuning.map(|t| t.friendly_fire()).unwrap_or_default();
    for (striker, empowerment, kin, striker_faction, harm, striker_team) in &empowered {
        if !empowerment.traits.holds(Empowerment::HARMS_ON_CONTACT) {
            continue;
        }
        let harm = harm.copied().unwrap_or_default();
        // "Everything I touch" is literally this body's collision box.
        let volume = kin.aabb();
        for (
            victim,
            victim_aabb,
            victim_faction,
            victim_health,
            facts,
            shield,
            combat,
            victim_team,
        ) in &victims
        {
            if victim == striker {
                continue;
            }
            if crate::combat::util::body_is_corpse(Some(victim_health)) {
                continue;
            }
            if !ambition_combat::targeting::damage_lands_between(
                *striker_faction,
                *victim_faction,
                striker_team,
                victim_team,
                friendly_fire,
                None,
                victim,
            ) {
                continue;
            }
            if !crate::combat::util::body_vulnerable(
                victim_health.health.invulnerable,
                facts.dodge_rolling,
                shield,
                combat,
            ) {
                continue;
            }
            let victim_body = victim_aabb.aabb();
            if !volume.strict_intersects(victim_body) {
                continue;
            }
            // Thrown away from the striker along the world side axis. A contact
            // that produced no separation would leave the victim inside the
            // volume, taking the hit again the moment its i-frames lapse.
            let dir = if victim_body.center().x >= kin.pos.x {
                1.0
            } else {
                -1.0
            };
            hit_events.write(HitEvent {
                strike_sfx: None,
                volume: volume.into(),
                damage: harm.damage,
                source: HitSource::ContactHarm,
                attacker: Some(striker),
                target: HitTarget::Body(victim),
                mode: HitMode::Knockback,
                knockback: Some(HitKnockback {
                    dir,
                    magnitude: HitKnockbackMagnitude::FeelScale(harm.knockback),
                    source_pos: kin.pos,
                    impact_pos: victim_body.center(),
                    launch_dir: None,
                }),
                ignored_targets: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod tests;

/// **Removing the empowerment releases the trait it was projecting.**
///
/// ⛔ **this existed as a ritual, and the ritual was already being performed by
/// hand.** `run_empowerments` writes `Invulnerability::EMPOWERED` from the
/// component, and it can only write it for bodies that still HAVE the component
/// — so a granter that takes the empowerment back (Sanic dropping its super
/// form) left the reason set forever, and Sanic worked around it with a second
/// call beside the removal:
///
/// ```ignore
/// commands.entity(body).remove::<Empowered>();
/// health.health.invulnerable.set(Invulnerability::EMPOWERED, false);
/// ```
///
/// Two steps, and the second is the one people forget — the shape this
/// repository has a standing rule about. An observer makes removal complete
/// however it happens: expiry, a granter taking it back, a despawn, or a
/// rollback restoring a frame where the body was never empowered.
///
/// ⚠ **the SCHEDULING deliberately stays with the games.** The review that found
/// this asked for one engine-owned installation point for the whole feature, and
/// that would take away a choice each game makes on purpose: Sanic installs
/// `run_empowerments` in `GameplayEffects` and deliberately does NOT install
/// `apply_contact_harm` (`defeat_badniks` already owns the destroy-on-touch
/// reaction, and two authorities killing one badnik is the bug it was avoiding),
/// while Mary-O installs both in `FeatureInteraction` with contact harm ordered
/// after expiry. What is engine-owned is the INVARIANT, not the order.
pub struct EmpowermentProjectionPlugin;

impl Plugin for EmpowermentProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(release_empowerment_projection);
    }
}

fn release_empowerment_projection(
    removal: On<bevy::ecs::lifecycle::Remove, Empowered>,
    mut bodies: Query<&mut BodyHealth>,
) {
    if let Ok(mut health) = bodies.get_mut(removal.entity) {
        health
            .health
            .invulnerable
            .set(Invulnerability::EMPOWERED, false);
    }
}
