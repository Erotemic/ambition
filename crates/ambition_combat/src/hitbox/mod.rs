//! Hitbox-entity lifecycle: spawn, resolve overlaps, then despawn.
//!
//! `HitboxHits` deduplicates victims across a multi-tick active window. `HitboxAnchor::FollowOwner`
//! resolves world geometry from the owner's current position each tick.

use bevy::ecs::query::QueryData;
use bevy::prelude::{Commands, Entity, Has, Message, MessageWriter, Query, Res, With};

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;

use super::components::ActorAggression;
use ambition_characters::actor::ActorFaction;
use super::events::{HitEvent, HitKnockback, HitKnockbackMagnitude, HitMode, HitSource, HitTarget};
use super::targeting::effective_faction;
use super::util::midpoint;
use crate::actor_faction_from_hit_side;
use ambition_time::WorldTime;
use ambition_vfx::vfx::VfxMessage;
use ambition_vfx::vfx::{VfxInRoom, VfxWriter};

// Public hitbox vocabulary remains available beside the resolution systems.
pub use crate::strike::{
    HitSide, Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback, HitboxLifetime,
};

/// One authoritative body contact resolved by a live strike.
///
/// [`apply_hitbox_damage`] is the one system that knows all of the facts needed
/// to say a body strike connected: attacker identity, victim identity, combat
/// relationship, published hurtbox geometry, self-exclusion, and per-strike
/// deduplication. Systems that care about a connect (move confirms, pogo,
/// lifesteal, status effects) consume this fact instead of independently
/// re-running overlap or relationship logic.
#[derive(Message, Clone, Debug)]
pub struct LandedBodyHit {
    /// The live strike entity that produced the contact.
    pub hitbox: Entity,
    /// Body that owns the strike.
    pub attacker: Entity,
    /// Concrete body selected by the shared victim resolver.
    pub victim: Entity,
    pub volume: ae::CombatVolume,
    /// Representative world-space contact point for effects/presentation.
    pub contact: ae::Vec2,
}

/// A hit that RESOLVED — damage applied, defences asked, reaction chosen, and
/// the hitlag it earned known.
///
/// ⭐⭐ [`LandedBodyHit`] MEANS OVERLAP; THIS MEANS CONNECT. They looked like the
/// same fact for as long as the only consumer was presentation, and they stopped
/// being one the moment a consumer needed the RESULT of the hit rather than its
/// geometry. The match freeze was that consumer: it read the victim's
/// `hitstop_timer` in `CombatSet::Settle`, which is written that frame for an
/// actor victim and NEXT frame for a player victim (the player resolver drains a
/// rollback FIFO in `PlayerSimulation`). ⛔⛔ SO THE CONNECT THAT STOPPED THE
/// WORLD FOR A CPU DID NOT STOP IT FOR THE HUMAN — measured 2026-08-25 in the
/// real schedule: 0.036s of player hitlag, zero frozen frames.
///
/// ⛔ NOT `BodyHitResolved`, which is the INSPECTOR's message: that one is
/// `#[cfg(feature = "causal")]` behind an optional writer and its own doc
/// guarantees nothing in the simulation reads it. Depending on it would turn an
/// optional instrument into required gameplay authority. This reuses the
/// resolution VOCABULARY and not the channel.
#[derive(bevy::prelude::Message, Clone, Debug, PartialEq)]
pub struct ResolvedBodyHit {
    /// The body that took it.
    pub victim: bevy::prelude::Entity,
    /// WHO LANDED IT, where the road knows.
    ///
    /// ⭐⭐ THIS IS WHAT MAKES THE CHANNEL USABLE BY THE ATTACKER'S OWN MOVE.
    /// `LandedBodyHit` carries an attacker and means OVERLAP; this one meant
    /// CONNECT and named only the victim, so a consumer asking *"did MY move
    /// connect"* had no channel that could answer and read the overlap instead —
    /// which is why an ordinary held guard confirms an `OnHit` cancel today
    /// (queue row D-CANCEL-ONBLOCK).
    ///
    /// `None` for a resolution with no striker: the blast zone, a hazard, a
    /// stage. ⛔ NOT "unknown" — a road that knows its attacker must say so, and
    /// a `None` here is a claim that nothing struck this body.
    pub attacker: Option<bevy::prelude::Entity>,
    /// The hitlag this connect earned, in seconds — the resolver's own answer,
    /// carried rather than re-read off the victim, because WHEN it is readable
    /// differs by road and that is the whole bug.
    pub hitlag_seconds: f32,
    /// HOW MUCH the victim actually took, after every defence had its say.
    ///
    /// ⭐⭐ THE MESSAGE'S OWN DOC ALREADY PROMISED THIS. It says a resolved hit
    /// is one where *"damage applied, defences asked, reaction chosen"* — and it
    /// carried WHO and HOW LONG the hitlag was, and never HOW MUCH. ⇒ A consumer
    /// scaling anything by the hit's size (a Limit meter, a rage curve, a combo
    /// score) had no channel to read it from and would have had to re-derive it
    /// from health deltas, which is a second authority on a number the resolver
    /// already knows.
    ///
    /// ⚠ THE RESOLVED AMOUNT, NOT THE AUTHORED ONE. `HitEvent::damage` is what
    /// the strike PROPOSED; this is what landed, so a blocked or armoured hit
    /// reports what it actually cost.
    pub damage: i32,
    /// WHAT HIT IT.
    ///
    /// ⛔⛔ THIS CHANNEL IS BROADER THAN `LandedBodyHit` AND THE SOURCE IS HOW A
    /// CONSUMER NARROWS IT. `LandedBodyHit` comes off the hitbox sweep, so it is
    /// strikes and shots only; this comes off the RESOLVER, which also serves
    /// contact attrition, hazards and the blast zone. Measured 2026-08-25: a
    /// fighter leaning on another produced SEVENTEEN `Contact` resolutions in
    /// twenty-three ticks, and a match freeze that armed on all of them
    /// alternated frozen/moving forever. A consumer that wants a CONNECT must
    /// say so.
    pub source: crate::HitSource,
    /// WHICH USE of the attacker's move earned this verdict, when a move did.
    ///
    /// ⛔⛔ WITHOUT IT THE CHANNEL CANNOT SAY, and a verdict draining after its
    /// move has been replaced is credited to the successor —
    /// `mark_move_playback_resolved_hits` keys on the attacker ENTITY alone and
    /// one body plays one move after another on that entity. Carried from
    /// `HitEvent::attacker_move_instance`, which is stamped from
    /// `crate::moveset::AttackerMoveInstance` on the volume.
    ///
    /// ⛔ `None` MEANS NO MOVE CLAIMED THIS, never "instance 0": this channel
    /// also carries contact attrition, hazards and the blast zone.
    pub attacker_move_instance: Option<u32>,
}

/// A strike a GUARD consumed.
///
/// ⭐⭐ THE POSITIVE FACT, because the negative one is a timing guess. A block is
/// "overlapped and did not connect", and a consumer could try to derive it from
/// [`LandedBodyHit`] plus the absence of a [`ResolvedBodyHit`] — but the two
/// roads resolve on different frames (an actor victim the same frame the
/// overlap is seen, a player victim the next one), so absence means "blocked"
/// and "not decided yet" and nothing distinguishes them. The resolver knows, so
/// the resolver says.
///
/// ⛔ ONE AUTHORITY. This is written where `BodyHitResolution::Blocked` is
/// decided and nowhere else — a consumer that instead read the victim's shield
/// state would be a second opinion on a question the damage road already
/// answers, which is the class this repo keeps finding.
#[derive(bevy::prelude::Message, Debug, Clone, Copy, PartialEq)]
pub struct BlockedBodyHit {
    /// The body whose guard ate it.
    pub victim: bevy::prelude::Entity,
    /// Whose strike it was, where the road knows. `None` for a resolution with
    /// no striker.
    pub attacker: Option<bevy::prelude::Entity>,
    /// WHICH USE of the attacker's move earned this verdict, when a move did.
    ///
    /// ⛔⛔ WITHOUT IT THE CHANNEL CANNOT SAY, and a verdict draining after its
    /// move has been replaced is credited to the successor —
    /// `mark_move_playback_resolved_hits` keys on the attacker ENTITY alone and
    /// one body plays one move after another on that entity. Carried from
    /// `HitEvent::attacker_move_instance`, which is stamped from
    /// `crate::moveset::AttackerMoveInstance` on the volume.
    ///
    /// ⛔ `None` MEANS NO MOVE CLAIMED THIS, never "instance 0": this channel
    /// also carries contact attrition, hazards and the blast zone.
    pub attacker_move_instance: Option<u32>,
}

/// A strike a PERFECT SHIELD caught: the defence succeeded and the attack was
/// negated rather than absorbed.
///
/// ⭐⭐ THE THIRD ANSWER, and the family is the argument for it.
/// [`LandedBodyHit`] means overlap, [`BlockedBodyHit`] means a guard ate it,
/// and this means the window beat it. A consumer could not previously tell a
/// parry from a block at all — the parry arm negates the strike BEFORE either
/// of the other two is written, so a caught parry published exactly nothing and
/// looked from the outside like a swing that never reached anybody.
///
/// ⛔ ONE AUTHORITY, on the same rule its sibling states: written where the
/// parry is decided and nowhere else. A consumer that instead watched
/// `BodyShieldState` for a `parry_window_timer` that stopped early would be a
/// second opinion on a question this seam already answers, and it could not
/// name the attacker at all.
///
/// ⇒ THE POINT OF PUBLISHING IT is that "a successful defensive interception"
/// is the fact a counter, a Revenge-style gauge and a Witch-Time-style slow all
/// need, and none of them should re-derive it. Those three differ only in what
/// they do NEXT, which is composition rather than three defensive mechanics.
/// See `docs/planning/engine/expressive-move-capabilities.md`.
#[derive(bevy::prelude::Message, Debug, Clone, Copy, PartialEq)]
pub struct ParriedBodyHit {
    /// The body whose parry window caught the strike.
    pub defender: bevy::prelude::Entity,
    /// The body that swung it. Known here, unlike on the block road, because a
    /// parry is resolved at the strike rather than at a damage resolution that
    /// may have lost the striker.
    pub attacker: bevy::prelude::Entity,
    /// The live strike entity that was caught.
    pub hitbox: bevy::prelude::Entity,
    /// Representative world-space contact point, resolved exactly as
    /// [`LandedBodyHit::contact`] is — so a retaliation and an ordinary hit
    /// place their effects by the same rule.
    pub contact: ae::Vec2,
}

/// Resolve a live hitbox's unit-bearing payload for one victim.
///
/// Feel multipliers pass through unchanged. Launch-speed growth is resolved against the victim's
/// accumulated damage and weight; authored nonzero growth overrides the ruleset fraction of base
/// launch speed.
/// The least damage a hit may be reduced to: one for a volume that authors
/// damage, none for one that does not.
///
/// ⭐ the distinction the unconditional `max(1)` could not make. Staling and
/// difficulty may wear a real hit down but must never wear it to nothing — and
/// a WINDBOX authors no damage on purpose, so flooring it to one turns a push
/// into a hit and makes the whole category unauthorable.
fn damage_floor(authored: i32) -> i32 {
    if authored > 0 {
        1
    } else {
        0
    }
}

/// The damage a use deals after staling: the authored damage times the stale
/// scale, rounded, and never below [`damage_floor`]. The hitbox road and the
/// throw road use this one law.
pub(crate) fn staled_damage(authored: i32, stale: f32) -> i32 {
    ((authored as f32 * stale).round() as i32).max(damage_floor(authored))
}

/// THE LAUNCH A SHOT DEALS, by the strike law a hitbox's goes through: a
/// growing `base + growth` resolved against the victim's percent and weight
/// and its room's rules (`rules`). A shot is not in the staling queue and its
/// firer's rage is not read here, so both are `1.0`.
pub fn projectile_launch_magnitude(
    base: f32,
    growth: f32,
    victim_damage_taken: i32,
    victim_weight: f32,
    rules: &crate::rules::ResolvedCombatTuning,
) -> HitKnockbackMagnitude {
    resolved_hitbox_knockback_magnitude(
        HitboxKnockback::LaunchSpeed {
            base,
            growth: Some(growth),
        },
        victim_damage_taken,
        victim_weight,
        rules.knockback_growth,
        rules.victim_percent_knockback_scale,
        rules.growth_base,
        rules.base_knockback_scale,
        1.0,
    )
}

fn resolved_hitbox_knockback_magnitude(
    knockback: HitboxKnockback,
    victim_damage_taken: i32,
    victim_weight: f32,
    ruleset_growth: f32,
    growth_scale: f32,
    // ⭐ THE VOLUME'S OWN `base` STEEPENS ITS PERCENT CURVE, and this is the
    // curve that says by how much. `GrowthBaseCurve::IDENTITY` is the law
    // exactly as first written, which is what every undeclared world passes.
    //
    // ⛔ THE CURVE RATHER THAN A PRECOMPUTED FACTOR, because `base` is bound
    // only inside the match arm below — a caller holding just a
    // `HitboxKnockback` cannot compute the factor to hand in.
    growth_base: crate::rules::GrowthBaseCurve,
    // The ruleset's scale on a growing launch's base (`1.0` undeclared).
    base_scale: f32,
    // ⭐ RAGE ARRIVES HERE RATHER THAN AT THE CALLER because this is the only
    // place that knows whether the launch GROWS. The two authoring roads for
    // growth are collapsed below, and a set launch must decline rage — see
    // `ambition_entity_catalog::launch::launch_speed`. A caller holding a bare
    // `HitboxKnockback` cannot make that call without re-deriving the collapse.
    rage: f32,
) -> HitKnockbackMagnitude {
    match knockback {
        // ⛔ A FEEL SCALE TAKES NO GROWTH SCALE, because it has no percent term
        // to scale. It is a dimensionless multiplier over the victim's own feel
        // tuning — there is no `base + growth * percent` to decompose — so the
        // ruleset's percent knob has nothing to act on here. Its staling is
        // applied whole by the caller, which is what it has always been.
        HitboxKnockback::FeelScale(scale) => HitKnockbackMagnitude::FeelScale(scale.max(0.0)),
        HitboxKnockback::LaunchSpeed { base, growth } => {
            // Preserve authored growth until the shared launch law resolves it.
            // `Some(0.0)` means fixed knockback; `None` selects the ruleset
            // growth fallback. The shared law in `ambition_entity_catalog`
            // serves combat and fighter evaluation with the same result.
            // Apply growth scaling to both authored and fallback growth,
            // without changing the fixed-knockback case.
            HitKnockbackMagnitude::LaunchSpeed(ambition_entity_catalog::launch::launch_speed(
                base,
                growth,
                ambition_entity_catalog::launch::LaunchConditions {
                    victim_damage: victim_damage_taken,
                    victim_weight,
                    growth_scale,
                    growth_base,
                    ruleset_growth,
                    rage,
                    base_scale,
                },
            ))
        }
    }
}

/// Does this strike reach the volumes the victim actually published?
///
/// The one victim-geometry rule, shared by every family. A body that
/// publishes [`DamageableVolumes`](super::components::DamageableVolumes) is hit on
/// exactly those volumes; a body that publishes none falls back to its coarse
/// box.
///
/// Component absence falls back to the coarse body box. A present component with no volumes means
/// intangible and must not fall back.
/// [`strike_reaches_victim`] for a hit: by its pieces where a portal pair cut
/// it ([`crate::events::HitEvent::meets`]).
pub fn hit_reaches_victim(
    event: &crate::events::HitEvent,
    victim_damageable: Option<&super::components::DamageableVolumes>,
    victim_aabb: &super::components::CenteredAabb,
) -> bool {
    if event.pieces.is_empty() {
        return strike_reaches_victim(&event.volume, victim_damageable, victim_aabb);
    }
    match victim_damageable {
        Some(published) if published.intangible() => false,
        Some(published) if published.published() => published.volumes.iter().any(|part| event.meets(part)),
        _ => event.meets_aabb(victim_aabb.aabb()),
    }
}

/// [`hit_reaches_victim`] for a victim a portal pair may cut: a body half
/// through a pair, or a limb a conductor holds in one, is reached where each
/// piece of it is (`docs/planning/engine/portals-a-body-in-two-places.md`),
/// as [`apply_hitbox_damage`] reaches it. A victim no pair cuts is
/// [`hit_reaches_victim`].
pub fn hit_reaches_victim_through(
    event: &crate::events::HitEvent,
    victim_damageable: Option<&super::components::DamageableVolumes>,
    victim_aabb: &super::components::CenteredAabb,
    victim: Entity,
    passages: Option<&ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
) -> bool {
    let Some(passages) = passages.filter(|p| p.cuts_in(room, Some(victim))) else {
        return hit_reaches_victim(event, victim_damageable, victim_aabb);
    };
    let parts = match victim_damageable {
        Some(published) if published.intangible() => return false,
        Some(published) if published.published() => published.volumes.clone(),
        _ => vec![ambition_platformer2d_core::CombatVolume::aabb(victim_aabb.aabb())],
    };
    parts
        .iter()
        .flat_map(|part| passages.pieces(part, Some(victim), victim_aabb.center, room))
        .any(|piece| event.meets(&piece.volume))
}

pub fn strike_reaches_victim(
    world_volume: &ambition_platformer2d_core::CombatVolume,
    victim_damageable: Option<&super::components::DamageableVolumes>,
    victim_aabb: &super::components::CenteredAabb,
) -> bool {
    match victim_damageable {
        // Intangible: published, and published nothing. Spelled as its own arm
        // because it is the one answer every damage family owes whatever its
        // geometry — see [`DamageableVolumes::intangible`], which
        // `step_projectiles` asks directly while it is still a coarse-box
        // consumer.
        Some(published) if published.intangible() => false,
        // Shape against shape: a published part may be a hull (an arm, a wing),
        // and testing the strike against its bounding box instead would let a
        // blade land in the dead corner of a rectangle nobody authored. The
        // box-vs-box fast path still applies whenever both sides are boxes —
        // `CombatVolume::intersects` reaches Parry only for a genuinely shaped
        // pair, after a bounds reject.
        Some(published) if published.published() => published
            .volumes
            .iter()
            .any(|part| world_volume.intersects(part)),
        // No component, or one no publisher has spoken for yet: the coarse box is
        // the best available answer, and it is what every consumer used before
        // published silhouettes existed.
        _ => world_volume.intersects_aabb(victim_aabb.aabb()),
    }
}

/// The earliest normalized time in `[0, 1]` at which a projectile's box, swept
/// from `start` by `delta`, reaches the geometry this body published.
///
/// ⛔⛔ **THE ENDPOINT TEST IS NOT A CONTACT TEST FOR A MOVING SHOT.**
/// [`strike_reaches_victim`] asks whether a strike volume overlaps the victim
/// NOW, which is the right question for a melee hitbox that exists for a window
/// of frames and the wrong one for a projectile that crosses the whole victim
/// between two samples. A fast shot's endpoint lands past a thin body, the
/// overlap test says no, and the shot flies on: the target is never touched.
///
/// ⚠ **SWEPT FOR BOXES, ENDPOINT-ONLY FOR A SHAPED PART, AND THAT LIMIT IS
/// DELIBERATE.** Every publisher in the tree emits `CombatVolume::Aabb` today,
/// so the swept answer is exact for all of them. A rotated box, a circle or a
/// convex hull has no swept primitive here, and answering from its BOUNDS would
/// reintroduce exactly what `strike_reaches_victim` refuses — a blade landing in
/// the dead corner of a rectangle nobody authored. Such a part falls back to the
/// endpoint shape test, so it is never over-reported, only under-swept. That is
/// the sampled-target model's stated edge, not full CCD.
///
/// ⚠ THE TARGET IS SAMPLED, NOT SWEPT. A victim moving across a stationary shot
/// between samples is a separate acceptance case and is not covered here.
pub fn swept_strike_reaches_victim(
    start: ambition_platformer2d_core::Vec2,
    half: ambition_platformer2d_core::Vec2,
    delta: ambition_platformer2d_core::Vec2,
    victim_damageable: Option<&super::components::DamageableVolumes>,
    victim_aabb: &super::components::CenteredAabb,
) -> Option<f32> {
    use ambition_platformer2d_core::{AabbExt, CombatVolume};

    let start_box = ae::Aabb::new(start, half);
    let end_box = ae::Aabb::new(start + delta, half);
    let swept_against = |target: ae::Aabb| -> Option<f32> {
        // PARITY with the discrete test for the already-overlapping case, then
        // the path on top: a victim the old test already found answers `0.0`,
        // and the only new answers are genuine tunnels.
        if start_box.strict_intersects(target) {
            return Some(0.0);
        }
        start_box
            .sweep_hit(delta, target)
            .map(|hit| hit.time_of_impact)
    };
    match victim_damageable {
        Some(published) if published.intangible() => None,
        Some(published) if published.published() => published
            .volumes
            .iter()
            .filter_map(|part| match part {
                CombatVolume::Aabb(target) => swept_against(*target),
                // Shaped: no swept primitive, so this is the endpoint answer the
                // caller had before, expressed as "reached at the end of the leg".
                shaped => shaped
                    .intersects(&CombatVolume::aabb(end_box))
                    .then_some(1.0),
            })
            .min_by(f32::total_cmp),
        _ => swept_against(victim_aabb.aabb()),
    }
}

/// Shared query contract for a body that may receive a strike.
///
/// `aabb` and `faction` are required. Optional components each have a defined absence semantic;
/// callers that require a narrower victim population must express that with query filters.
#[derive(QueryData)]
pub struct StrikeVictim {
    pub entity: Entity,
    /// The coarse collision box. The framing fallback when nothing finer is
    /// published, and the impact/knockback geometry in every case.
    pub aabb: &'static super::components::CenteredAabb,
    /// Authored allegiance. Run it through [`StrikeVictimItem::effective_faction`]
    /// rather than reading it raw — a possessed body fights as its driver's side.
    pub faction: &'static ActorFaction,
    /// Who drives this body, if anybody.
    pub driver: Option<&'static ambition_characters::control::DrivingParticipant>,
    /// The published silhouette, when this body publishes one. See
    /// [`strike_reaches_victim`] for why absent and empty mean opposite things.
    pub volumes: Option<&'static super::components::DamageableVolumes>,
    pub health: Option<&'static ambition_characters::actor::BodyHealth>,
    /// The world's hands are off this body — see [`crate::death_rules::OutOfPlay`].
    ///
    /// ⛔ IT IS NOT REDUNDANT WITH `health`. A fighter waiting out its death beat
    /// has had `health.reset()` called on it already, so it reads ALIVE for the
    /// whole interlude while being explicitly out of the fight.
    pub out_of_play: bevy::prelude::Has<crate::death_rules::OutOfPlay>,
    /// A backdrop body is out of reach (`ae::DepthPlane`).
    pub plane: Option<&'static ambition_platformer2d_core::DepthPlane>,
    /// Knockback weight (CM1). Absent  the reference weight `1.0`.
    pub tuning: Option<&'static super::components::CombatTuning>,
    /// Outranks faction for "may this land": two humans share a faction, so a
    /// match could not otherwise let them hit each other.
    pub team: Option<&'static crate::targeting::MatchTeam>,
    // THERE IS NO `shield` FIELD, AND ITS ABSENCE IS THE DESIGN.
    //
    // One sat here documented "read for the parry window only", and catching a
    // parry is not a read: it spends nothing but it PUBLISHES
    // (`BodyShieldState::catch_parry`), and a decision that has to be followed
    // by a write somewhere else is the shape this tree keeps paying for. Every
    // seam that resolves a parry now takes the guard mutably by entity —
    // `guards.get_mut(victim.entity)` — so the test and the consequence are one
    // step. Asking for it here as well would be a second access to the same
    // component in the same system, which Bevy refuses.
    /// This body's voice, for the cue its striker emits (the parry clang).
    pub voice: Option<&'static ambition_sfx::BodyPresentationSource>,
    /// The victim's resolved motion frame; knockback direction is interpreted in this frame.
    pub frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    pub is_player: Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    /// The victim's stable identity, for a FIRST-WINS resolver's final
    /// tie-break. Two bodies at one position (a spawn point, a stack) tie on
    /// every geometric key, and `Entity` is not stable across a rewind; a body
    /// without one still gets hit, it just cannot win the tie.
    ///
    /// ⚠ ORDER IT THROUGH [`victim_identity_key`], never by comparing the
    /// `Option`s: the derived ordering puts `None` FIRST and so says the
    /// opposite of the sentence above.
    pub sim_id: Option<&'static ambition_platformer2d_shared_tangle::sim_id::SimId>,
}

/// THE ordering key for a victim's stable identity, read by every resolver that
/// breaks a geometric tie.
///
/// ⛔⛔ **`Option`'s DERIVED ORDERING SAYS THE OPPOSITE OF THE RULE.**
/// [`StrikeVictim::sim_id`] documents it exactly — *"a body without one still
/// gets hit, it just cannot win the tie"* — and the resolvers implemented that
/// as a bare `a.sim_id.cmp(&b.sim_id)`. `None` sorts FIRST, so an unidentified
/// body BEAT every identified one at a full tie, which is the rule inverted.
/// `an_unidentified_victim_does_not_beat_an_identified_one` is that case.
///
/// ⇒ Absent identity sorts LAST, and the rank is a separate leading field rather
/// than a sentinel string: an authored id could otherwise collide with whatever
/// stand-in was chosen.
///
/// ⚠ **TWO ABSENT IDENTITIES STILL COMPARE EQUAL, AND THAT IS NOT A TIE-BREAK
/// THIS FUNCTION CAN INVENT.** Two coincident bodies that both lack a `SimId`
/// have no stable discriminator on `StrikeVictim` at all — `Entity` does not
/// survive a rewind, and every geometric key is already spent by the time this
/// is reached. The protocol calls missing required target identity a
/// construction failure rather than a sort fallback, so the answer lives where
/// bodies are BUILT, not here. Callers that must not be ambiguous say so at
/// their own seam.
pub fn victim_identity_key(sim_id: Option<&str>) -> (u8, &str) {
    match sim_id {
        Some(id) => (0, id),
        None => (1, ""),
    }
}

impl StrikeVictimItem<'_, '_> {
    /// Allegiance as this body actually fights: a possessed victim carrying
    /// `DrivingParticipant` is a Player-effective body, without its authored
    /// faction being mutated.
    pub fn effective_faction(&self) -> ActorFaction {
        effective_faction(*self.faction, self.driver)
    }

    /// A dead body, or one the world has its hands off, is intangible — the
    /// strike passes through it.
    pub fn is_corpse(&self) -> bool {
        crate::util::body_is_untouchable(self.health, self.out_of_play, self.plane)
    }

    /// This body published NO hurtbox: nothing can reach it.
    ///
    /// For a family that has not adopted [`Self::reached_by`] — a consumer whose
    /// strike geometry is still its own — this is the part of the shared rule it
    /// owes anyway. A caller that asks [`Self::reached_by`] gets it for free and
    /// must not ask twice.
    pub fn is_intangible(&self) -> bool {
        self.volumes
            .is_some_and(super::components::DamageableVolumes::intangible)
    }

    /// The volumes this body is hit on, each cut where it is when a portal
    /// pair cuts it (`docs/planning/engine/portals-a-body-in-two-places.md`):
    /// its published parts, else its coarse box, and none when intangible.
    /// The rule of [`strike_reaches_victim`], in pieces.
    pub fn hurt_pieces(
        &self,
        passages: Option<&ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>,
        room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    ) -> Vec<ambition_platformer2d_core::frame::thread::VolumePiece> {
        use ambition_platformer2d_core::frame::thread::VolumePiece;
        let parts: Vec<ambition_platformer2d_core::CombatVolume> = match self.volumes {
            Some(published) if published.intangible() => return Vec::new(),
            Some(published) if published.published() => published.volumes.clone(),
            _ => vec![ambition_platformer2d_core::CombatVolume::aabb(self.aabb.aabb())],
        };
        match passages {
            Some(passages) => hurt_pieces_of(self.entity, self.aabb.center, parts, Some(passages), room),
            None => parts.into_iter().map(VolumePiece::whole).collect(),
        }
    }

    /// Does `world_volume` reach the geometry this body actually published?
    ///
    /// The single victim-geometry rule, applied to the victim that owns it.
    pub fn reached_by(&self, world_volume: &ambition_platformer2d_core::CombatVolume) -> bool {
        strike_reaches_victim(world_volume, self.volumes, self.aabb)
    }

    /// When along a projectile's travel this body is first reached, if at all.
    ///
    /// The swept sibling of [`Self::reached_by`], for a caller that owns a
    /// segment rather than a single sample — see
    /// [`swept_strike_reaches_victim`] for what it sweeps and what it does not.
    pub fn reached_along(
        &self,
        start: ambition_platformer2d_core::Vec2,
        half: ambition_platformer2d_core::Vec2,
        delta: ambition_platformer2d_core::Vec2,
    ) -> Option<f32> {
        swept_strike_reaches_victim(start, half, delta, self.volumes, self.aabb)
    }

    /// The "away" axis in the VICTIM's local frame (§B11).
    ///
    /// Not the world axis: under sideways gravity the pair separates along
    /// world-Y, which is exactly when a screen-X comparison degenerates.
    pub fn knockback_side(&self) -> ae::Vec2 {
        self.frame
            .map(|frame| frame.basis())
            .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR))
            .side
    }

    /// Accumulated damage and weight, the two CM1 knockback-growth inputs.
    /// Absent components answer with the inert defaults rather than skipping
    /// the body.
    pub fn knockback_growth_inputs(&self) -> (i32, f32) {
        (
            self.health.map(|h| h.damage_taken()).unwrap_or(0),
            self.tuning.map(|ct| ct.weight).unwrap_or(1.0),
        )
    }
}

/// Apply each live hitbox's damage to the bodies it reaches.
///
/// Body-owned melee (`FollowOwner`) resolves every attacker side through ONE
/// victim loop: self exclusion, combat relationship, overlap, per-hitbox dedup,
/// and victim-specific knockback are identical for human- and brain-controlled
/// bodies. The remaining `Player` + `World` branch is deliberately not melee: it
/// is the legacy wielded world-AOE primitive consumed as a broadcast volume.
/// Everything this sweep knows about the body that SWUNG, keyed by that body.
///
/// ⭐⭐ FIVE QUERIES THAT WERE ALWAYS ASKED THE SAME QUESTION. Each of these was
/// a separate top-level parameter, and every single call site read it with
/// `.get(hitbox.owner)` — so the entity was repeated at each use and the fact
/// that they are one subject was expressible only by a shared `attacker_` name
/// prefix. The accessors take the owner once and return the FACT, so a call
/// site asks for the attacker's rage rather than for a health component it then
/// has to reduce.
///
/// ⛔ AND THERE WERE SIX, BECAUSE TWO OF THEM WERE THE SAME QUERY.
/// `attacker_playback` and `attacker_moves` were both
/// `Query<&MovePlayback>` on the swing owner — one read for the stale-queue
/// hash, one for the already-hit list — so the system carried two independent
/// borrows of one component to answer two questions about it. Collapsing them
/// is a removal, not a rename.
#[derive(bevy::ecs::system::SystemParam)]
pub struct AttackerFacts<'w, 's> {
    aggression: Query<'w, 's, &'static ActorAggression>,
    team: Query<'w, 's, &'static crate::targeting::MatchTeam>,
    health: Query<'w, 's, &'static ambition_characters::actor::BodyHealth>,
    stale: Query<'w, 's, &'static crate::stale::BodyStaleMoves>,
    playback: Query<'w, 's, &'static crate::moveset::MovePlayback>,
}

impl AttackerFacts<'_, '_> {
    /// The DAMAGE-side per-entity override: a specific body this one has decided
    /// to oppose, which lets a hit land on a same-faction target without
    /// re-tagging factions.
    fn grudge(&self, owner: Entity) -> Option<crate::components::Grudge> {
        self.aggression.get(owner).ok().and_then(|a| a.grudge)
    }

    fn team(&self, owner: Entity) -> Option<&crate::targeting::MatchTeam> {
        self.team.get(owner).ok()
    }

    /// Accumulated damage, for RAGE. A body with no health pool has taken none.
    fn damage_taken(&self, owner: Entity) -> i32 {
        self.health.get(owner).map(|h| h.damage_taken()).unwrap_or(0)
    }

    /// How many times the move being swung has already been thrown. A body with
    /// no live move or no queue has thrown nothing, and answers zero.
    fn staleness(&self, owner: Entity) -> u32 {
        match (self.playback.get(owner), self.stale.get(owner)) {
            (Ok(playback), Ok(queue)) => {
                queue.occurrences(crate::stale::stale_move_hash(&playback.spec.id))
            }
            _ => 0,
        }
    }

    /// The per-strike dedup accumulator that keeps a multi-tick Active window
    /// from re-smashing the same breakable every frame. A body with no playback
    /// answers with an empty list and still strikes.
    fn already_hit(&self, owner: Entity) -> Vec<String> {
        self.playback
            .get(owner)
            .map(|pb| pb.hit_targets.clone())
            .unwrap_or_default()
    }
}

/// Register every message [`StrikeOutcomeWriters`] can write.
///
/// ⛔⛔ THE LIST AND THE STRUCT MUST CHANGE TOGETHER, AND THIS EXISTS BECAUSE
/// THEY ONCE DID NOT. Adding `ParriedBodyHit` to the writer struct broke nine
/// tests in another crate, in a way no compiler could see: those apps
/// hand-listed `HitEvent`, `LandedBodyHit` and `VfxMessage` — precisely the set
/// the system needed at the time — and a `MessageWriter` for an unregistered
/// message panics inside `Main::run_main` with the underlying message swallowed,
/// so the whole diagnostic was "a system panicked".
///
/// ⇒ A hand-listed set of dependencies is a POPULATION, and adding to the source
/// does not update the lists. Every app that runs [`apply_hitbox_damage`] calls
/// this instead of enumerating, so the next output added to the struct reaches
/// all of them at once.
pub fn register_strike_outcome_messages(app: &mut bevy::prelude::App) {
    app.add_message::<VfxInRoom>();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<ParriedBodyHit>();
    app.add_message::<crate::guard::DeflectedBodyHit>();
}

/// Everything the strike sweep PUBLISHES, in one parameter.
///
/// These four are the system's OUTPUTS, which is a group a reader can name, so
/// the next publication belongs here rather than in another top-level slot.
///
/// ⓘ Bevy's system-parameter limit is 16 and this system was at it, which is
/// what prompted the grouping — but the limit is worth knowing about for how it
/// FAILS, not as a justification. A seventeenth parameter does not report "too
/// many parameters": the function silently stops satisfying `IntoSystem`, and
/// every `.chain()` mentioning it instead reports that a tuple of function types
/// does not implement `Curve` — screens of `bevy_math::curve` diagnostics for a
/// combat change that touched no curve. The error names nothing that appears in
/// the edit that caused it.
#[derive(bevy::ecs::system::SystemParam)]
pub struct StrikeOutcomeWriters<'w> {
    /// Wielded-AOE VFX only; victim-side `emit_hit_feedback` owns melee feedback.
    pub vfx: VfxWriter<'w>,
    pub hit_events: MessageWriter<'w, HitEvent>,
    /// Overlap.
    pub landed: MessageWriter<'w, LandedBodyHit>,
    /// A perfect shield caught it.
    pub parried: MessageWriter<'w, ParriedBodyHit>,
    /// A guard turned it aside (`crate::guard`).
    pub deflected: MessageWriter<'w, crate::guard::DeflectedBodyHit>,
}

/// Where a victim can be reached this tick: the box around its hurt parts
/// and its guards; anywhere, when a pair cuts it; nowhere, when it has
/// neither.
#[derive(Clone, Copy, Debug)]
enum VictimReach {
    Anywhere,
    Within(ae::Aabb),
    Nowhere,
}

/// Whether two boxes touch, edges included, with a pixel to spare: the broad
/// phase must never reject a pair the exact test accepts.
fn boxes_touch(a: ae::Aabb, b: ae::Aabb) -> bool {
    const SPARE: f32 = 1.0;
    a.min.x <= b.max.x + SPARE && b.min.x <= a.max.x + SPARE && a.min.y <= b.max.y + SPARE && b.min.y <= a.max.y + SPARE
}

/// A volume of a ranked move, placed once a tick: who owns it, its rank, its
/// pieces, where it strikes from, and whether a guard can turn it.
struct RankedStrike {
    entity: Entity,
    owner: Entity,
    rank: crate::moveset::StrikeRank,
    pieces: Vec<ae::frame::thread::VolumePiece>,
    source: ae::Vec2,
    meets_guard: bool,
}

/// A guard of a body and its pieces, cut by the body's own room and transit.
type CutGuard = (crate::guard::GuardPiece, Vec<ae::frame::thread::VolumePiece>);

/// The guard that turns aside a strike of `strike` pieces from `source`, and
/// where they touch. A guard turns a strike that meets it from a side it
/// covers.
fn turned_aside(
    strike: &[ae::frame::thread::VolumePiece],
    source: ae::Vec2,
    guards: &[CutGuard],
    down: ae::Vec2,
) -> Option<(u8, ae::Vec2)> {
    guards.iter().find_map(|(guard, guard_pieces)| {
        strike.iter().find_map(|sp| {
            let gp = guard_pieces.iter().find(|gp| sp.meets(gp))?;
            // Where the strike comes from, in the guard's chart.
            let source = gp.uncarry(sp.carry(source));
            guard
                .covers(source, down)
                .then(|| (guard.guard, midpoint(sp.center(), gp.center())))
        })
    })
}

/// Where a hit comes from: a placed volume's own centre, a carried one's
/// owner. The knockback direction and the portal cut start there.
fn spatial_source_of(hitbox: &Hitbox, owner_pos: ae::Vec2) -> ae::Vec2 {
    match hitbox.anchor {
        HitboxAnchor::World { center } => center,
        HitboxAnchor::FollowOwner { .. } => owner_pos,
    }
}

/// Where a strike's volume is in live room `room`: its pieces, as the portal
/// pairs cut it, or its whole volume. The one rule [`apply_hitbox_damage`]
/// resolves by and the combat read model shows. A volume the owner carries
/// (it follows the owner) reaches from it: through an opening it goes in the
/// wedge from the owner through the opening's ends, and what it swept beside
/// the opening met the host (its drawing is cut the same way,
/// `ambition_portal2d_presentation::carry`). One placed in the world is cut
/// square on by what it reaches through.
pub fn strike_pieces(
    passages: Option<&ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>,
    hitbox: &Hitbox,
    owner_pos: ae::Vec2,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
) -> Vec<ambition_platformer2d_core::frame::thread::VolumePiece> {
    let carrier = matches!(hitbox.anchor, HitboxAnchor::FollowOwner { .. }).then_some(hitbox.owner);
    let volume = hitbox.world_volume(owner_pos);
    match passages.filter(|p| p.cuts_in(room, carrier)) {
        Some(p) if carrier.is_some() => p.pieces_reaching(&volume, carrier, owner_pos, room),
        Some(p) => p.pieces(&volume, carrier, spatial_source_of(hitbox, owner_pos), room),
        None => vec![ambition_platformer2d_core::frame::thread::VolumePiece::whole(volume)],
    }
}

/// Where a body's hurt volumes `parts` are in live room `room`: their pieces,
/// as the portal pairs cut them (a body half through a pair is hurt at both
/// ends), or whole. The rule [`apply_hitbox_damage`] reaches a victim by.
pub fn hurt_pieces_of(
    body: Entity,
    center: ae::Vec2,
    parts: Vec<ae::CombatVolume>,
    passages: Option<&ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
) -> Vec<ambition_platformer2d_core::frame::thread::VolumePiece> {
    match passages.filter(|p| p.cuts_in(room, Some(body))) {
        Some(passages) => parts.iter().flat_map(|part| passages.pieces(part, Some(body), center, room)).collect(),
        None => parts.into_iter().map(ambition_platformer2d_core::frame::thread::VolumePiece::whole).collect(),
    }
}

pub fn apply_hitbox_damage(
    // ⭐ THE MOVE INSTANCE RIDES ALONG, `Option` because this system resolves
    // volumes that no move spawned (an ability's own box, a hazard). See
    // `crate::moveset::AttackerMoveInstance`: `None` here means no move claims
    // the verdict, which is not the same as instance 0.
    mut hitboxes: Query<(
        Entity,
        &Hitbox,
        &mut HitboxHits,
        Option<&crate::moveset::AttackerMoveInstance>,
    )>,
    owners: Query<&super::components::CenteredAabb>,
    // Owner-position fallback when the owner carries no `CenteredAabb`
    // (bare test bodies); every real body — player included — publishes one.
    owner_kin: Query<&ambition_platformer2d_core::BodyKinematics>,
    // Friendly-fire policy (the DAMAGE side; targeting is `FactionRelations`).
    // Optional so minimal headless tests that don't stand up the plugin still run
    // (fall back to the default: friendly fire OFF — same-faction allies safe).
    // AE6: resolved match rules, not the world's baseline toggle, of the
    // strike's own live room.
    tuning: crate::rules::CombatTuningOf,
    // These components narrow hostile melee to complete combat bodies; victim-side resolution
    // owns their actual semantics.
    victims: Query<
        StrikeVictim,
        (
            With<ambition_platformer2d_core::BodyMotionFacts>,
            With<ambition_platformer2d_core::BodyShieldState>,
            With<ambition_characters::actor::BodyCombat>,
        ),
    >,
    // EVERYTHING KNOWN ABOUT THE BODY THAT SWUNG — grudge, team, rage,
    // staleness and its already-hit list. All read-only, so they may overlap the
    // victim query, and all keyed by the swing owner, which is why they are one
    // parameter rather than five. See `AttackerFacts`.
    attacker: AttackerFacts,
    // THE SIBLING VOLUMES of whatever move is swinging — read-only, so it
    // overlaps the `hitboxes` query above on `Hitbox` without conflicting
    // (only `HitboxHits` is taken mutably there).
    //
    // This is the whole of the sweetspot rule's input: which other volumes one
    // move has live right now, and which of them the author wrote first.
    strike_ranks: Query<(Entity, &Hitbox, &crate::moveset::StrikeRank)>,
    // EVERY VICTIM'S GUARD, taken mutably and looked up by entity like the
    // attacker rows below. This is where a perfect shield CATCHES a strike, so
    // the guard is written, not read — see the parry arm in the victim loop.
    // `BodyShieldState` is deliberately absent from `StrikeVictim` so this is
    // the system's only access to it.
    mut guards: Query<&mut ambition_platformer2d_core::BodyShieldState>,
    // The parts of each body that turn strikes aside, where they are this
    // tick (`crate::guard`): asked before the body.
    turning: Query<&crate::guard::GuardVolumes>,
    // Live Hitbox state is authoritative for damage; presentation projections must
    // not gate it. One parameter rather than four — see `StrikeOutcomeWriters`.
    mut out: StrikeOutcomeWriters,
    // A strike reaches only the bodies of its own live room (OW1 cut 4).
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    // ⭐ WHERE A VOLUME IS when a portal pair cuts it: a strike that reaches
    // through a pair hurts at its far end, and a body half through is hit at
    // both (`docs/planning/engine/portals-a-body-in-two-places.md`). Absent in
    // a composition with no portals: every volume is whole.
    passages: Option<Res<ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>>,
) {
    use ambition_platformer2d_core::frame::thread::VolumePiece;
    // Each victim's pieces, cut once a tick.
    let mut victim_pieces: std::collections::BTreeMap<Entity, Vec<VolumePiece>> = std::collections::BTreeMap::new();
    // Each victim's guards, cut once a tick.
    let mut victim_guards: std::collections::BTreeMap<Entity, Vec<CutGuard>> = std::collections::BTreeMap::new();
    // ⭐⭐ THE PULSE'S LEDGER, collected here and written to every sibling after
    // the sweep. `(striker, owner, victim)`.
    //
    // ⛔⛔ "ONE SWING LANDS ONCE" WAS NOT WHAT THE ARBITRATION ABOVE
    // IMPLEMENTED. It resolves a TIE — which of two overlapping volumes takes a
    // body reached by both, on one tick — and the ledger it deduplicates
    // against belonged to ONE hitbox. So a victim struck by the sourspot on one
    // tick, stepping into the sweetspot on the next, was unknown to the
    // sweetspot's own empty ledger and was hit AGAIN by the same swing. Both
    // orders, and no test could see it: they placed the victim where both
    // volumes overlapped AT ONCE, which is the tie and not the invariant.
    //
    // ⚠ A DESIGN COLLISION, not an oversight. The arbitration's own note
    // defended "a sourspot whose sweetspot window has closed is free again on
    // the very next tick, which is what a lingering late hit should be" — and
    // that sentence and "one swing lands once" cannot both be true of
    // independent per-volume ledgers. The pulse is the level that owns both: a
    // late sourspot is the SAME pulse and does not re-hit, while a genuine
    // multi-hit puts a GAP in Active time and earns its second hit.
    let mut pulse_records: Vec<(Entity, Entity, Entity)> = Vec::new();
    // Resolve the owner's collision-box center for FollowOwner tracking.
    // Actors carry `CenteredAabb`; bare fixtures may carry only
    // `BodyKinematics`. `None` (owner despawned) leaves the hitbox a harmless
    // ghost for `tick_and_despawn_hitboxes`.
    let owner_position = |hitbox: &Hitbox| {
        let resolved_owner = owners
            .get(hitbox.owner)
            .map(|aabb| aabb.center)
            .ok()
            .or_else(|| owner_kin.get(hitbox.owner).map(|kin| kin.pos).ok());
        Some(match (resolved_owner, hitbox.anchor) {
            (Some(pos), _) => pos,
            // ⛔⛔ A WORLD-ANCHORED BOX DOES NOT NEED AN OWNER THAT STILL EXISTS,
            // AND REQUIRING ONE MADE EVERY ITEM BLAST A GHOST. `world_volume`
            // reads `HitboxAnchor::World { center }` and ignores `owner_pos`
            // entirely — but this resolution ran FIRST and `continue`d before any
            // anchor was consulted. ⇒ A bomb or mine that names the exploding
            // `GroundItem` as its owner despawns as it emits the effect, so the
            // blast it just spawned was skipped on the very next tick, every
            // time, with nothing anywhere saying so.
            //
            // ⭐ THE BOX'S OWN CENTRE IS THE HONEST SUBSTITUTE, not a zero: the
            // value is still used for the launch direction and for `source_pos`,
            // and a blast's knockback radiates FROM THE BLAST. A placeholder
            // would have thrown every victim toward the world origin.
            //
            // ⚠ ONLY WHEN THE OWNER CANNOT BE RESOLVED. A World box whose owner
            // is alive keeps using the owner's position exactly as before, so
            // this repairs the ghost without changing a single case that already
            // worked.
            (None, HitboxAnchor::World { center }) => center,
            // A FollowOwner box genuinely cannot be placed without one. Leave it
            // a harmless ghost for `tick_and_despawn_hitboxes`.
            (None, HitboxAnchor::FollowOwner { .. }) => return None,
        })
    };
    // Where a strike is: its live room and its pieces. Each volume is placed
    // by its own anchor, carrier and room, also when another volume of the
    // same move asks about it.
    let place = |hitbox_entity: Entity, hitbox: &Hitbox, owner_pos: ae::Vec2| {
        // The strike's live room: the volume's own, else its owner's. Two
        // live rooms share one local frame, so a body at the same place
        // in another room is not in reach.
        let room = rooms.stamped(hitbox_entity).or_else(|| rooms.of(hitbox.owner));
        (room, strike_pieces(passages.as_deref(), hitbox, owner_pos, room))
    };
    // The victims in a stable order (see the victim loop), sorted once a
    // tick: the order is the same for every strike.
    let mut ordered: Vec<_> = victims.iter().collect();
    ordered.sort_by(|a, b| {
        victim_identity_key(a.sim_id.map(|id| id.as_str())).cmp(&victim_identity_key(b.sim_id.map(|id| id.as_str())))
    });
    // Where each victim can be reached this tick (`VictimReach`), worked out
    // once.
    let mut victim_reach: std::collections::BTreeMap<Entity, VictimReach> = std::collections::BTreeMap::new();
    // The ranked volumes live this tick, each placed once: a victim loop asks
    // about each sibling for each body.
    let ranked: Vec<RankedStrike> = strike_ranks
        .iter()
        .filter_map(|(entity, hitbox, rank)| {
            let owner_pos = owner_position(hitbox)?;
            Some(RankedStrike {
                entity,
                owner: hitbox.owner,
                rank: *rank,
                pieces: place(entity, hitbox, owner_pos).1,
                source: spatial_source_of(hitbox, owner_pos),
                meets_guard: hitbox.meets_guard(),
            })
        })
        .collect();
    for (hitbox_entity, hitbox, mut hits, move_instance) in &mut hitboxes {
        let Some(owner_pos) = owner_position(hitbox) else {
            continue;
        };
        let world_volume = hitbox.world_volume(owner_pos);
        // ⭐⭐ WHERE THE HIT CAME FROM IS THE ANCHOR'S QUESTION, NOT THE OWNER'S.
        // `world_volume` already ignores `owner_pos` for a `World` anchor, but
        // the knockback DIRECTION and `source_pos` still read it — so the same
        // explosion threw a victim differently depending on where the entity
        // CREDITED with it happened to be standing. ⇒ A steered bolt centres its
        // blast at the bolt while its owner is the caster, so a curved shot
        // struck from one side and launched away from the caster on the other.
        //
        // ⛔ ATTRIBUTION IS A DIFFERENT QUESTION AND KEEPS ITS OWN FIELD.
        // `hitbox.owner` still answers "who is credited" for kills, grudges and
        // staleness; this answers "what did the victim get hit BY", and a blast
        // is hit by the blast.
        let spatial_source = spatial_source_of(hitbox, owner_pos);
        let source_faction = actor_faction_from_hit_side(hitbox.source);

        // ONE BODY, ONE PATH: every body-owned melee strike resolves contacts
        // here. `HitSide` selects descriptive source vocabulary only; it does not
        // select a different overlap/dedup/knockback algorithm.
        let melee_source = match (hitbox.source, hitbox.anchor) {
            (HitSide::Player, HitboxAnchor::FollowOwner { .. }) => Some(HitSource::Melee),
            (HitSide::Enemy | HitSide::Npc, _) => Some(HitSource::Melee),
            (HitSide::Boss, _) => Some(HitSource::Melee),
            (HitSide::Player, HitboxAnchor::World { .. }) | (HitSide::Neutral, _) => None,
            // ⭐⭐ A HAZARD TAKES THE BODY PATH, WHICH IS THE WHOLE POINT. A
            // thrown bomb, a placed mine and a steered bolt all authored
            // `Neutral` and therefore hurt NOBODY — the resolver is explicit
            // that Neutral never spawns a damaging hitbox, and three shipped
            // moves believed the opposite. This is the side that means "hurts
            // everyone", and everyone is reached through the melee path.
            (HitSide::Environment, _) => Some(HitSource::Melee),
        };

        let (strike_room, pieces) = place(hitbox_entity, hitbox, owner_pos);
        // What a feature (a boss, a breakable) is hit by: the pieces, when a
        // pair cut the strike; else the volume, whole.
        let cut = pieces.len() != 1 || !pieces[0].bounds.is_empty();
        let feature_pieces = || if cut { pieces.clone() } else { Vec::new() };
        if let Some(source_kind) = melee_source {
            let owner_grudge = attacker.grudge(hitbox.owner);

            // ⛔⛤ IN A STABLE ORDER, NOT QUERY ORDER. This was
            // `for victim in &victims` — Bevy archetype order — and a rewind
            // need not reproduce it. One strike reaching TWO victims on one tick
            // then damaged them in an order the resimulation could disagree
            // about, and the damage road writes shared state on the way through
            // (hitstop, the attacker's own contact facts, a spent ledger), so
            // the order is not private to each victim.
            //
            // ⭐ THE PROJECTILE ROAD ALREADY HAD THIS AUTHORITY and the melee
            // road never adopted it: `victim_identity_key` is used twice in
            // `projectile/systems.rs` and nowhere else. One rule, one key, both
            // roads — rather than two answers to "which victim first".
            //
            // ⚠ TWO VICTIMS THAT BOTH LACK A `SimId` STILL COMPARE EQUAL, and
            // `victim_identity_key`'s own doc says why that cannot be repaired
            // here: `Entity` does not survive a rewind and every geometric key
            // is already spent. That residue is a CONSTRUCTION question — a
            // damageable rollback-authoritative body should carry identity by
            // the time it reaches a resolver — and it is not made worse by
            // ordering the rest.
            // The rules of the strike's room: two live rooms can play under
            // two games' rules.
            let room_rules = tuning.in_room(strike_room);
            let ruleset_growth = room_rules.map(|t| t.knockback_growth).unwrap_or_default();
            let rules = room_rules.unwrap_or_default();
            let friendly_fire = room_rules.map(|t| t.friendly_fire()).unwrap_or_default();
            // What the strike can touch at most, when no pair cuts it: the
            // broad phase below rejects a body none of whose parts it reaches.
            let strike_bounds = (!cut).then(|| world_volume.bounds());
            for victim in ordered.iter() {
                // Identity beats every relationship rule. Friendly fire, match
                // teams, and grudges can decide whether TWO bodies may fight;
                // none of them can make one body become its own victim.
                // ⛔⛔ …EXCEPT A HAZARD, WHICH HAS NO SELF TO EXCLUDE. This is
                // the collision Jon's four questions name: `hitbox.owner` was
                // doing two jobs — WHO IS CREDITED and WHO IS IMMUNE — and the
                // genre wants them to disagree. Your own bomb hurts you, and you
                // still placed it.
                //
                // ⭐ Splitting them here rather than at attribution is what lets
                // the owner stay the placing FIGHTER (so a kill is credited, a
                // grudge is tracked, and a rollback names the same body) while
                // the blast still catches them. `DamageTeam::Environment` says
                // the same thing one layer down: it damages Player, Enemy and
                // Neutral, with no exception carved for whoever spawned it.
                if victim.entity == hitbox.owner && hitbox.source != HitSide::Environment {
                    continue;
                }
                // THE BROAD PHASE: a strike that reaches none of a body's
                // parts and none of its guards does nothing to it, and the
                // rules below cost more than this test. Not for a strike or a
                // body that a pair cuts: their pieces are elsewhere.
                let reach = *victim_reach.entry(victim.entity).or_insert_with(|| {
                    let room = rooms.of(victim.entity);
                    if passages.as_deref().is_some_and(|p| p.cuts_in(room, Some(victim.entity))) {
                        return VictimReach::Anywhere;
                    }
                    let hurt = victim.hurt_pieces(None, room).into_iter().map(|piece| piece.volume.bounds());
                    let guarded = turning
                        .get(victim.entity)
                        .into_iter()
                        .flat_map(|turning| turning.pieces.iter().map(|guard| guard.volume));
                    hurt.chain(guarded)
                        .reduce(|a, b| ae::Aabb { min: a.min.min(b.min), max: a.max.max(b.max) })
                        .map_or(VictimReach::Nowhere, VictimReach::Within)
                });
                match (strike_bounds, reach) {
                    (_, VictimReach::Nowhere) => continue,
                    (Some(strike), VictimReach::Within(within)) if !boxes_touch(strike, within) => continue,
                    _ => {}
                }
                // Structural tangibility gate: a dead body is
                // an intangible corpse — the swing passes through it. Skipping
                // here means NO event and NO impact VFX are produced at the
                // corpse, so a dead thing neither interacts nor presents. (The
                // consume-time `resolve_body_hit` alive check stays as defense.)
                if victim.is_corpse() {
                    continue;
                }
                if rooms.of(victim.entity) != strike_room {
                    continue;
                }
                // ⛔ A HAZARD ASKS NO RELATIONSHIP QUESTION EITHER, and routing
                // it through one would be worse than pointless: the faction
                // projection of `Environment` is `Neutral`, and a same-faction
                // rule would then make a bomb harmless to any Neutral body while
                // `DamageTeam::Environment::can_damage(Neutral)` says the
                // opposite. ⇒ One relationship, stated once, in the vocabulary
                // that already had it.
                if hitbox.source != HitSide::Environment
                    && !crate::targeting::damage_lands_between(
                        source_faction,
                        victim.effective_faction(),
                        attacker.team(hitbox.owner),
                        victim.team,
                        friendly_fire,
                        owner_grudge,
                        victim.entity,
                    )
                {
                    continue;
                }
                if hits.hit.contains(&victim.entity) {
                    continue;
                }
                let victim_room = rooms.of(victim.entity);
                // Cut by the victim's own room and transit, not the strike's:
                // the pieces are kept for each strike of the tick.
                let hurt = victim_pieces.entry(victim.entity).or_insert_with(|| {
                    let threaded = passages.as_deref().filter(|p| p.cuts_in(victim_room, Some(victim.entity)));
                    victim.hurt_pieces(threaded, victim_room)
                });
                let guards_here: &[CutGuard] = victim_guards.entry(victim.entity).or_insert_with(|| {
                    let threaded = passages.as_deref().filter(|p| p.cuts_in(victim_room, Some(victim.entity)));
                    turning.get(victim.entity).map_or_else(
                        |_| Vec::new(),
                        |turning| {
                            turning
                                .pieces
                                .iter()
                                .map(|guard| {
                                    let volume = ae::CombatVolume::aabb(guard.volume);
                                    let cut = match threaded {
                                        Some(passages) => passages.pieces(&volume, Some(victim.entity), victim.aabb.center, victim_room),
                                        None => vec![VolumePiece::whole(volume)],
                                    };
                                    (*guard, cut)
                                })
                                .collect()
                        },
                    )
                });
                let down = victim.frame.map_or(ae::DEFAULT_GRAVITY_DIR, |frame| frame.down());
                // ⭐ A GUARD IS ASKED BEFORE THE BODY. A strike that meets a
                // guard from a side it covers is turned aside and reaches
                // nothing behind it, whether or not it also reaches the body:
                // the shield is in front. Like a parry it is a negation (no
                // damage, no `LandedBodyHit`), and the strike is recorded as
                // having reached this body so it cannot reach it again.
                //
                // An unblockable strike and a windbox pass every guard: the
                // first breaks through a guard by its contract, and the second
                // only pushes.
                let turned = if hitbox.meets_guard() {
                    turned_aside(&pieces, spatial_source, guards_here, down)
                } else {
                    None
                };
                // The piece of the strike that reaches a piece of the victim.
                let reach = pieces
                    .iter()
                    .find_map(|sp| hurt.iter().find(|vp| sp.meets(vp)).map(|vp| (sp, vp.clone())));
                if turned.is_none() && reach.is_none() {
                    continue;
                }
                // ⭐ THE SWEETSPOT WINS. A move that authors a tip and a base
                // has both live at once, and a body standing between them is
                // reached by both — so one swing landed twice, for two
                // different damages and two knockbacks, until this.
                //
                // The victim takes the FIRST-AUTHORED volume that reaches it
                // and no other: the loser stands down whenever a lower-ranked
                // sibling of the same move also reaches this body. Asked from
                // the LOSER's side deliberately — it needs no shared ledger and
                // no write, so there is no order for two writers to disagree
                // about, and a sourspot whose sweetspot window has closed is
                // free again on the very next tick, which is what a lingering
                // late hit should be.
                //
                // ⛔ A GUARD IS PART OF WHAT A VOLUME REACHES. The guard used to
                // be asked before this rule, so a sourspot that met the shield
                // was turned aside while the sweetspot landed on the body: one
                // swing deflected and landed. A sibling that the guard turns
                // aside has reached this body as surely as one that hits it.
                if let Some(me) = ranked.iter().find(|ranked| ranked.entity == hitbox_entity) {
                    let outranked = ranked.iter().any(|sibling| {
                        sibling.owner == hitbox.owner
                            && sibling.rank < me.rank
                            && (sibling.pieces.iter().any(|sp| hurt.iter().any(|vp| sp.meets(vp)))
                                || (sibling.meets_guard
                                    && turned_aside(&sibling.pieces, sibling.source, guards_here, down).is_some()))
                    });
                    if outranked {
                        continue;
                    }
                }
                if let Some((guard, contact)) = turned {
                    hits.hit.insert(victim.entity);
                    pulse_records.push((hitbox_entity, hitbox.owner, victim.entity));
                    out.deflected.write(crate::guard::DeflectedBodyHit {
                        defender: victim.entity,
                        attacker: hitbox.owner,
                        hitbox: hitbox_entity,
                        guard,
                        contact,
                        damage: hitbox.damage,
                    });
                    continue;
                }
                let Some((strike_piece, victim_piece)) = reach else {
                    continue;
                };
                // The strike's own chart into the victim's own: through the pair
                // the strike piece was carried by, back through the victim's.
                let to_victim = |p: ae::Vec2| victim_piece.uncarry(strike_piece.carry(p));
                let to_victim_dir = |v: ae::Vec2| victim_piece.uncarry_dir(strike_piece.carry_dir(v));
                let from_victim = |p: ae::Vec2| strike_piece.uncarry(victim_piece.carry(p));
                let through = strike_piece.is_carried() || victim_piece.is_carried();
                // Where they touch: in the chart they meet in.
                let impact = if through {
                    midpoint(strike_piece.center(), victim_piece.center())
                } else {
                    midpoint(victim.aabb.center, world_volume.center())
                };
                let source_here = to_victim(spatial_source);
                // A PERFECT SHIELD CATCHES THE STRIKE, and it is resolved HERE
                // because this is the one seam every body-strike passes through.
                // The two damage roads below it did not agree: the primary
                // player's refused a parried hit inside `body_vulnerable`, and
                // the actor road — which is the road every match fighter takes —
                // never asked at all, so a fighter's parry was an ordinary block
                // that spent guard and paid shieldstun.
                //
                // A caught parry is a NEGATION, not a block. No damage event, and
                // no `LandedBodyHit`: the attacker's move did not connect, so it
                // neither confirms an on-hit cancel nor wears itself out on the
                // stale queue. The strike is still recorded as having reached
                // this victim so it cannot re-reach it while the window is open.
                // ⛔⛔ A WINDBOX CANNOT BE PARRIED. Its authored contract is that
                // it PUSHES and does nothing else — no damage, no hitstun, NO
                // SHIELD — and a parry is the strongest form of the thing it
                // says it does not interact with. Caught here, a gust produced
                // no push at all, so a defender could simply parry the wind.
                //
                // ⭐ THE FACT IS ALREADY ON THE HITBOX (`reaction`), which is
                // why this reads as one condition rather than a new channel.
                //
                // ⭐⭐ AND THE OTHER HALF IS FIXED TOO — this paragraph used to
                // say it was not. Ordinary guard resolution happens later, and
                // `GuardUnderFire::offered_to` returns `None` for a flinchless
                // knockback, so a gust spends no shield integrity, charges no
                // shieldstun and applies no pushback. The fact rode the
                // knockback that was already going there; no channel was added.
                // An unblockable strike is not parried either: a parry is a
                // guard (S0.3).
                if hitbox.meets_guard() {
                    if let Ok(mut shield) = guards.get_mut(victim.entity) {
                        if shield.parrying() {
                            shield.catch_parry();
                            hits.hit.insert(victim.entity);
                            pulse_records.push((hitbox_entity, hitbox.owner, victim.entity));
                            // ⭐ THE SUCCESSFUL DEFENCE, SAID OUT LOUD. Everything
                            // above this line negates the strike; this is the only
                            // place that reports the negation happened, and it is
                            // written before the `continue` that ends the victim's
                            // participation in the ordinary damage road.
                            out.parried.write(ParriedBodyHit {
                                defender: victim.entity,
                                attacker: hitbox.owner,
                                hitbox: hitbox_entity,
                                contact: impact,
                            });
                            continue;
                        }
                    }
                }

                let victim_body = victim.aabb.aabb();
                // Knockback side in the victim's LOCAL frame (§B11): under
                // sideways gravity the attacker and victim separate along
                // world-Y, exactly when a screen-X comparison degenerates.
                let side = victim.knockback_side();
                let dir = if (victim_body.center() - source_here).dot(side) >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let (victim_damage_taken, victim_weight) = victim.knockback_growth_inputs();
                // RAGE, and it is the mirror of the percent mechanic. The
                // victim's damage already scaled that launch; without this the
                // fighter behind is punished twice — easier to launch and no
                // harder to launch with. `1.0` in a game that declares no rage.
                let rage = rules.rage_scale(attacker.damage_taken(hitbox.owner));
                // STALING, which is ONE number with TWO jobs and used to be
                // applied as though it had one. It still scales the DAMAGE this
                // hit deals, further down this same loop, unchanged.
                let stale = rules.stale_scale(attacker.staleness(hitbox.owner));
                // Staling scales percent-based launch growth, not its base.
                // Rage scales the entire launch. Preserve this split when
                // resolving the shared launch law for combat and AI.
                let growth_scale =
                    rules.victim_percent_knockback_scale * rules.knockback_stale_scale(stale);
                let magnitude = resolved_hitbox_knockback_magnitude(
                    hitbox.knockback,
                    victim_damage_taken,
                    victim_weight,
                    ruleset_growth,
                    growth_scale,
                    rules.growth_base,
                    rules.base_knockback_scale,
                    rage,
                );
                // ⛔ A FEEL SCALE KEEPS TAKING STALING WHOLE. It carries no
                // `base + growth` to split, so there is no percent term for the
                // influence above to ride — and every hazard, dive and
                // projectile in Ambition's PvE is a `FeelScale`. Dropping the
                // stale multiply for them would have been a silent PvE change
                // smuggled in behind a platform-fighter repair.
                let magnitude = match magnitude {
                    HitKnockbackMagnitude::FeelScale(_) => magnitude.scaled(rage * stale),
                    // ⭐ RAGE ALREADY RODE THIS ARM, inside the resolver, which
                    // is the only place that can see whether the launch grows
                    // with percent — a SET launch declines rage and this line
                    // could not tell one from a weak one.
                    HitKnockbackMagnitude::LaunchSpeed(_) => magnitude,
                };
                let knockback = Some(HitKnockback {
                    // ⭐ A GUST IS THROWN THE SAME WAY A PUNCH IS — the strength
                    // and direction above are the volume's ordinary authoring —
                    // and what it declines is the INJURY, not the physics. So
                    // the authored KIND is carried through rather than a second
                    // push vector on the volume…
                    //
                    // ⛔⛔ …AND IT IS THE KIND, NOT A BOOL. This lowered to
                    // `flinchless: bool`, which named ONE of the four things a
                    // windbox declines; the other three (the air-dodge refund,
                    // the helplessness clear, the hitlag) went on happening
                    // because nothing downstream could still tell a gust from a
                    // weak punch. See `HitReaction`.
                    reaction: if hitbox.windbox().is_some() {
                        ae::hit_response::HitReaction::Windbox
                    } else {
                        ae::hit_response::HitReaction::Strike
                    },
                    guard: hitbox.hit_guard(),
                    dir,
                    magnitude,
                    source_pos: source_here,
                    impact_pos: impact,
                    // LOWERED HERE, as the autolink anchor below is: the
                    // volume has the facing and the frame of the body that
                    // threw it, and the reaction of the victim has neither.
                    launch_dir: hitbox
                        .launch_world(spatial_source, from_victim(victim_body.center()))
                        .map(|launch| launch.mapped(to_victim_dir)),
                    // AUTOLINK, and the ATTACKER'S VELOCITY is sampled HERE.
                    // The reaction holds a victim and no attacker entity, and
                    // the velocity is a fact about this pulse rather than about
                    // the move — so the producer, which has the owner, is the
                    // only place that can honestly answer it. ⭐ this is what
                    // makes a RISING multi-hit work: the correction only closes
                    // a gap, and a fighter climbing fast outruns any gap-closing
                    // term. A body with no kinematics contributes nothing, which
                    // reads as a stationary attacker — the correct answer for a
                    // volume nothing is carrying.
                    follow: hitbox
                        .autolink()
                        .map(|link| ae::hit_response::AutolinkFollow {
                            // ⭐ RESOLVED HERE, against the ATTACKER'S facing and
                            // frame — both of which the hitbox carries from the
                            // body that spawned it. The victim side used to
                            // rebuild this point from `dir` (the victim's
                            // away-side, NOT the attacker's facing) and the
                            // victim's gravity (not the attacker's), so a hit
                            // catching somebody BEHIND the attacker mirrored the
                            // anchor toward them, and two bodies in different
                            // frames disagreed about where "toward the feet" was.
                            anchor_world: to_victim(ae::hit_response::autolink_anchor_world(
                                ae::Vec2::new(link.anchor.0, link.anchor.1),
                                owner_pos,
                                hitbox.facing,
                                hitbox.frame_down,
                            )),
                            carry: link.carry,
                            pull: link.pull,
                            max_speed: link.max_speed,
                            source_vel: to_victim_dir(
                                owner_kin.get(hitbox.owner).map(|kin| kin.vel).unwrap_or(ae::Vec2::ZERO),
                            ),
                        }),
                });
                out.hit_events.write(HitEvent {
                    pieces: Vec::new(),
                    strike_sfx: hitbox.strike_sfx,
                    volume: world_volume.clone(),
                    // the DAMAGE stales with the launch. A move worn out that
                    // still filled the percent meter at full rate would be half a
                    // mechanic — and the floor keeps a fully stale hit a hit.
                    //
                    // ⭐ THE FLOOR IS ONE, OR NONE, and which depends on what the
                    // author wrote. A volume that authors damage keeps at least a
                    // point of it however stale it gets; a volume that authors
                    // NO damage is a WINDBOX, and flooring it to one turned a
                    // push into a hit. The floor was unconditional, so a
                    // damageless volume was unauthorable: you could write
                    // `damage: 0` and the runtime dealt one.
                    damage: staled_damage(hitbox.damage, stale),
                    source: source_kind.clone(),
                    attacker: Some(hitbox.owner),
                    room: None,
                    target: HitTarget::Body(victim.entity),
                    mode: HitMode::Knockback,
                    knockback,
                    ignored_targets: Vec::new(),
                                    attacker_move_instance: move_instance.map(|i| i.0),
                });
                out.landed.write(LandedBodyHit {
                    hitbox: hitbox_entity,
                    attacker: hitbox.owner,
                    victim: victim.entity,
                    volume: world_volume.clone(),
                    contact: impact,
                });
                // ⭐⭐ A SUSTAINED GUST DOES NOT SPEND ITS HIT-ONCE SLOT. The set
                // exists so a long active window cannot re-hit a stationary
                // target every frame — exactly right for a strike, and exactly
                // wrong for a wind, which is supposed to keep pushing for as
                // long as you stand in it.
                //
                // ⛔ AUTHORED, NOT ASSUMED. A one-shot shove is a windbox too,
                // and it wants the ordinary once-only rule; only a volume that
                // says `repeating` opts out.
                if !hitbox
                    .windbox()
                    .is_some_and(|gust: ambition_entity_catalog::WindboxVolume| gust.repeating)
                {
                    hits.hit.insert(victim.entity);
                }
                pulse_records.push((hitbox_entity, hitbox.owner, victim.entity));
            }

            // Publish the unresolved feature half of the strike after body targets
            // have been resolved. Feature consumers may scan bosses/breakables but
            // must not damage bodies again. Per-strike dedup is authoritative in
            // `MovePlayback.hit_targets`, which is the only record.
            {
                out.hit_events.write(HitEvent {
                    pieces: feature_pieces(),
                    strike_sfx: hitbox.strike_sfx,
                    volume: world_volume.clone(),
                    damage: hitbox.damage.max(damage_floor(hitbox.damage)),
                    source: source_kind,
                    attacker: Some(hitbox.owner),
                    room: None,
                    target: HitTarget::UnresolvedFeatures,
                    mode: HitMode::Knockback,
                    knockback: None,
                    ignored_targets: attacker.already_hit(hitbox.owner),
                                    attacker_move_instance: move_instance.map(|i| i.0),
                });
            }
            continue;
        }

        match hitbox.source {
            // Keep its broadcast semantics until that separate primitive is refactored: fire
            // once per strike via the owner sentinel and let the feature resolver fan out the
            // volume.
            HitSide::Player => {
                debug_assert!(matches!(hitbox.anchor, HitboxAnchor::World { .. }));
                if hits.hit.insert(hitbox.owner) {
                    out.vfx.write_in(strike_room, VfxMessage::Impact {
                        pos: world_volume.center(),
                        blow: ambition_vfx::ImpactBlow::PLAIN,
                    });
                    out.hit_events.write(HitEvent {
                        pieces: feature_pieces(),
                        strike_sfx: hitbox.strike_sfx,
                        volume: world_volume.clone(),
                        damage: hitbox.damage.max(damage_floor(hitbox.damage)),
                        source: HitSource::Melee,
                        attacker: Some(hitbox.owner),
                        room: None,
                        target: HitTarget::Volume,
                        mode: HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                                            attacker_move_instance: move_instance.map(|i| i.0),
                    });
                }
            }
            // Neutral never spawns a damaging hitbox (a provoked Npc is handled
            // by the direct melee path above with its real faction).
            HitSide::Neutral => {}
            // These sides were consumed by `melee_source` and continued above.
            HitSide::Enemy | HitSide::Boss | HitSide::Npc | HitSide::Environment => {
                unreachable!()
            }
        }
    }

    // THE PULSE RECORDS ITS VICTIMS ON EVERY SIBLING, so a swing that lands
    // with one of its volumes cannot land again with another.
    //
    // ⛔ written AFTER the sweep, not during it: the sweep holds `hits`
    // mutably for the striking box, and the siblings are rows of the same
    // query. Deferring is also what keeps the order irrelevant — a set union
    // is the same however it is built.
    //
    // ⚠ MOVE VOLUMES ONLY. A projectile or a stage hazard has no pulse and no
    // siblings, so it keeps the per-hitbox ledger it always had; membership of
    // a pulse is exactly `StrikeRank`, which only `advance_move_playback`
    // grants.
    let siblings: Vec<(Entity, Entity)> = strike_ranks
        .iter()
        .map(|(entity, hitbox, _)| (entity, hitbox.owner))
        .collect();
    for (striker, owner, victim) in pulse_records {
        for (sibling, sibling_owner) in &siblings {
            if *sibling_owner != owner || *sibling == striker {
                continue;
            }
            if let Ok((_, _, mut hits, _)) = hitboxes.get_mut(*sibling) {
                hits.hit.insert(victim);
            }
        }
    }
}

pub fn tick_and_despawn_hitboxes(
    mut commands: Commands,
    world_time: Res<WorldTime>,
    mut hitboxes: Query<(Entity, &mut HitboxLifetime), With<Hitbox>>,
) {
    let dt = world_time.sim_dt();
    for (entity, mut lifetime) in &mut hitboxes {
        lifetime.remaining_s -= dt;
        if lifetime.remaining_s <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

// The former flat melee strike spawns (`spawn_melee_strike` = hitbox + slash from
// one gravity-resolved box, and its `spawn_melee_hitbox` primitive) are deleted:
// melee is a moveset move now, and `advance_move_playback` spawns its own richer
// window-scoped strike (convex authored blades, per-window multi-volumes, charge
// scaling, on-hit techniques) inline — the sole melee strike spawn.

#[cfg(test)]
mod tests;
