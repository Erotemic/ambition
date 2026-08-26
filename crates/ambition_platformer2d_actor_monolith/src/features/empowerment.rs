//! Composable temporary body empowerment traits.
//!
//! [`Empowerment::UNTOUCHABLE`] delegates to the body's invulnerability-reason set.
//! [`Empowerment::HARMS_ON_CONTACT`] emits ordinary [`HitEvent`]s for body overlap, so
//! victim-side i-frames, knockback, feedback, and death remain under the normal combat
//! consumer. Contact harm intentionally has no separate hit-memory authority.

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

/// A body currently empowered, and for how much longer — if that is even a
/// question.
///
/// Snapshot state by the strictest reading: it gates whether hits land AND
/// whether this body deals them, so a rollback that dropped it would disagree
/// with itself about damage. Games register it with their own rollback rows.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Empowered {
    /// Seconds left, or `None` for an empowerment HELD by whatever granted it.
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

/// Run every empowerment: hold its traits while it lasts, release them when
/// it ends.
///
/// Traits are re-asserted every tick rather than set once at the start. That is
/// not defensive coding — an empowerment is a CONTINUOUS claim ("I am
/// untouchable *now*"), and a system that stated it once would have to be
/// consulted by everything that might overwrite it. Re-stating is what makes the
/// claim true independently of who else writes.
///
/// do not add this to a game's schedule. [`EmpowermentLifecyclePlugin`]
/// installs it in [`EmpowermentExpiry`], and a second registration would tick
/// `remaining` twice per frame — a two-second grant lasting one. Order against
/// the SET instead.
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

/// Everything an empowered body touches takes the hit.
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
/// a corpse is skipped, and so is a body its own vulnerability rule protects (i-frames, a dodge
/// roll, a parry).
///
/// A game that wants it and wants the frame an empowerment ends not to also be a frame it flattens
/// something orders it `.after(EmpowermentExpiry)`.
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
        Option<&ambition_combat::targeting::MatchTeam>,
        // The world's hands are off this body — a contact-harm empowerment does
        // not reach it either.
        bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
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
            victim_out_of_play,
        ) in &victims
        {
            if victim == striker {
                continue;
            }
            if ambition_combat::util::body_is_untouchable(Some(victim_health), victim_out_of_play)
            {
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
            if !ambition_combat::util::body_vulnerable(
                victim_health.health.invulnerable,
                facts.evading(),
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
                source: HitSource::Contact,
                attacker: Some(striker),
                target: HitTarget::Body(victim),
                mode: HitMode::Knockback,
                knockback: Some(HitKnockback {
                    // An ordinary hit: it stuns.
                    reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
                    dir,
                    magnitude: HitKnockbackMagnitude::FeelScale(harm.knockback),
                    source_pos: kin.pos,
                    impact_pos: victim_body.center(),
                    launch_dir: None,
                    follow: None,
                }),
                ignored_targets: Vec::new(),
            });
        }
    }
}

#[cfg(test)]
mod tests;

/// Schedule slot for [`run_empowerments`]. The engine owns expiry semantics; games order grants
/// before this set and effects that must observe expiry after it. It runs in `GameplayEffects`,
/// after grant sites and with the same one-frame latency to combat consumers.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct EmpowermentExpiry;

/// Installs empowerment ticking and removal cleanup. Removing [`Empowered`] clears the
/// `Invulnerability::EMPOWERED` projection regardless of whether removal came from expiry, a
/// granter, despawn, or rollback restoration.
pub struct EmpowermentLifecyclePlugin;

impl Plugin for EmpowermentLifecyclePlugin {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d_shared_tangle::schedule::{
            Platformer2dSimulationPhaseMonolith, SimScheduleExt,
        };
        app.add_observer(release_empowerment_projection);
        let sim = app.sim_schedule();
        app.configure_sets(
            sim,
            EmpowermentExpiry.in_set(Platformer2dSimulationPhaseMonolith::GameplayEffects),
        );
        app.add_systems(sim, run_empowerments.in_set(EmpowermentExpiry));
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
