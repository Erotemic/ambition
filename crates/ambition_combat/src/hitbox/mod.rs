//! Hitbox-entity lifecycle: spawn → overlap-check → despawn.
//!
//! Per the actor/brain follow-up plan
//! (`dev/journals/actor-brain-migration-followups-plan.md`, Task A):
//! enemy melee strikes were resolved by per-tick polling inside
//! `update_ecs_actors` (calling `enemy.player_damage(player_body)`
//! every frame the attack_timer was hot). That bypass made melee
//! the only attack family that didn't flow through the actor/brain
//! → ActorActionMessage → EFFECTS-consumer seam.
//!
//! This module replaces the poll with explicit entities:
//!
//! - `update_ecs_actors` detects the windup → active edge and
//!   spawns one `(Hitbox, HitboxLifetime, HitboxHits)` entity per
//!   strike using the strike's per-archetype AABB.
//! - `apply_hitbox_damage` (this module) tests overlap against the
//!   target faction's hurtboxes each tick, emits the matching
//!   damage event, and inserts hit targets into `HitboxHits` so a
//!   long active window can't double-hit the same target.
//! - `tick_and_despawn_hitboxes` (this module) advances every
//!   hitbox's lifetime and despawns expired ones.
//!
//! `HitboxAnchor::FollowOwner` re-resolves the hitbox AABB each
//! tick from the owner entity's position, so a moving attacker's
//! swing tracks the actor without a per-frame component update.
//! `HitboxAnchor::World` (Task B groundwork) is a fixed
//! world-space rectangle for hazards / boss specials.

use bevy::ecs::query::QueryData;
use bevy::prelude::{Commands, Entity, Has, Message, MessageWriter, Query, Res, With};

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;

use super::components::ActorAggression;
use super::components::ActorFaction;
use super::events::{HitEvent, HitKnockback, HitKnockbackMagnitude, HitMode, HitSource, HitTarget};
use super::targeting::effective_faction;
use super::util::midpoint;
use crate::actor_faction_from_hit_side;
use ambition_time::WorldTime;
use ambition_vfx::vfx::VfxMessage;

// The hitbox COMPONENTS moved to the reusable `ambition_vfx` crate (the
// damage-box primitive). Re-exported here so `combat::hitbox::Hitbox`
// (and `features::Hitbox`) paths are unchanged; the SYSTEMS below (damage
// resolution, melee spawn, lifecycle) stay in the lib.
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
    /// Exact world-space strike geometry used to resolve the contact.
    pub volume: ae::CombatVolume,
    /// Representative world-space contact point for effects/presentation.
    pub contact: ae::Vec2,
}

/// Resolve a live hitbox's unit-bearing payload for one victim.
///
/// Feel multipliers pass through unchanged. Authored melee speed growth is
/// evaluated here because it depends on the struck body's accumulated damage
/// and weight; the resulting event no longer carries unresolved growth.
fn resolved_hitbox_knockback_magnitude(
    knockback: HitboxKnockback,
    victim_damage_taken: i32,
    victim_weight: f32,
) -> HitKnockbackMagnitude {
    match knockback {
        HitboxKnockback::FeelScale(scale) => HitKnockbackMagnitude::FeelScale(scale.max(0.0)),
        HitboxKnockback::LaunchSpeed { base, growth } => {
            let launch_speed =
                crate::util::scaled_knockback(base, growth, victim_damage_taken, victim_weight)
                    .max(0.0);
            HitKnockbackMagnitude::LaunchSpeed(launch_speed)
        }
    }
}

/// Does this strike reach the volumes the victim actually published?
///
/// **The one victim-geometry rule**, shared by every family. A body that
/// publishes [`DamageableVolumes`](super::components::DamageableVolumes) is hit on
/// exactly those volumes; a body that publishes none falls back to its coarse
/// box.
///
/// The distinction that matters: a body carrying the component with an EMPTY list
/// is *intangible* — the authored answer for an invulnerable window, or a corpse
/// the publisher cleared — so it is a MISS, not a reason to consult the coarse
/// box. Collapsing those two cases is how an authored invulnerability silently
/// stops working, and it is why the fallback is keyed on the component's absence
/// rather than on the list being empty.
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

/// **The body a strike lands on** — one entity role, named once.
///
/// Every damage family asks the same questions of the thing it hit: where is it,
/// whose side is it on, may it be struck at all, what silhouette does it present,
/// and which way does *its* frame call "away". Before this type each family spelled
/// that role as its own positional tuple, and the tuples had already drifted:
///
/// * [`apply_hitbox_damage`] (melee) carried the published silhouette inline;
/// * `apply_feature_hit_events` could not — its tuple was at the ARITY ceiling, so
///   the silhouette rode a second `Query<&DamageableVolumes>` beside it;
/// * `step_projectiles` had neither, and its comment nonetheless claimed "the SAME
///   published hurtbox" as melee. It tests the coarse box. **The prose was the only
///   place the parity existed.**
///
/// That drift is the argument for the type. A role spelled three ways cannot be
/// diffed; a role spelled once has one answer, and [`Self::reached_by`] is it.
///
/// # Required versus optional is deliberate, and it is a FILTER decision
///
/// Only `aabb` and `faction` are required — the two facts without which "a strike
/// hit this" is not a sentence. Everything else is `Option`, because a body that
/// lacks it has a correct answer rather than a missing one: no `DamageableVolumes`
/// means fall back to the coarse box, no `BodyHealth` means it cannot be a corpse,
/// no `BodyShieldState` means it cannot parry.
///
/// ⛔ **Do not add a required field to widen a caller's victim set.** Requiring a
/// component silently DROPS every body without it from the query, which is the
/// "my hit does nothing" bug. A caller that genuinely wants a narrower set says so
/// in its own [`With`] filter, where the narrowing is visible at the call site —
/// that is exactly how melee keeps its combat-body-only victim set below.
#[derive(QueryData)]
pub struct StrikeVictim {
    pub entity: Entity,
    /// The coarse collision box. The framing fallback when nothing finer is
    /// published, and the impact/knockback geometry in every case.
    pub aabb: &'static super::components::CenteredAabb,
    /// Authored allegiance. Run it through [`StrikeVictimItem::effective_faction`]
    /// rather than reading it raw — a possessed body fights as its driver's side.
    pub faction: &'static ActorFaction,
    pub brain: Option<&'static ambition_characters::brain::Brain>,
    /// The published silhouette, when this body publishes one. See
    /// [`strike_reaches_victim`] for why absent and empty mean opposite things.
    pub volumes: Option<&'static super::components::DamageableVolumes>,
    pub health: Option<&'static ambition_characters::actor::BodyHealth>,
    /// Knockback weight (CM1). Absent ⇒ the reference weight `1.0`.
    pub tuning: Option<&'static super::components::CombatTuning>,
    /// Outranks faction for "may this land": two humans share a faction, so a
    /// match could not otherwise let them hit each other.
    pub team: Option<&'static crate::targeting::MatchTeam>,
    /// Read for the parry window only. A body without one simply cannot parry.
    pub shield: Option<&'static ambition_platformer2d_core::BodyShieldState>,
    /// This body's voice, for the cue its striker emits (the parry clang).
    pub voice: Option<&'static ambition_sfx::BodyPresentationSource>,
    /// The victim's own resolved motion frame (ADR 0024).
    ///
    /// **A field, not a `Query<&ResolvedMotionFrame>` beside the victim query.**
    /// Both damage families looked this up by victim entity, through byte-identical
    /// `.map(|f| f.basis()).unwrap_or(default)` ladders — a per-victim component
    /// reached by a second lookup only because the victim tuple had run out of
    /// room. It belongs to the victim, so it rides with the victim, and the ladder
    /// is [`StrikeVictimItem::knockback_side`] once.
    pub frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    pub is_player: Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
}

impl StrikeVictimItem<'_, '_> {
    /// Allegiance as this body actually fights: a possessed victim carrying
    /// `Brain::Player` is a Player-effective body, without its authored faction
    /// being mutated.
    pub fn effective_faction(&self) -> ActorFaction {
        effective_faction(*self.faction, self.brain)
    }

    /// A dead body is an intangible corpse — the strike passes through it.
    pub fn is_corpse(&self) -> bool {
        crate::util::body_is_corpse(self.health)
    }

    /// **This body published NO hurtbox: nothing can reach it.**
    ///
    /// For a family that has not adopted [`Self::reached_by`] — a consumer whose
    /// strike geometry is still its own — this is the part of the shared rule it
    /// owes anyway. A caller that asks [`Self::reached_by`] gets it for free and
    /// must not ask twice.
    pub fn is_intangible(&self) -> bool {
        self.volumes
            .is_some_and(super::components::DamageableVolumes::intangible)
    }

    /// Does `world_volume` reach the geometry this body actually published?
    ///
    /// The single victim-geometry rule, applied to the victim that owns it.
    pub fn reached_by(&self, world_volume: &ambition_platformer2d_core::CombatVolume) -> bool {
        strike_reaches_victim(world_volume, self.volumes, self.aabb)
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
pub fn apply_hitbox_damage(
    mut hitboxes: Query<(Entity, &Hitbox, &mut HitboxHits)>,
    owners: Query<&super::components::CenteredAabb>,
    // Owner-position fallback when the owner carries no `CenteredAabb`
    // (bare test bodies); every real body — player included — publishes one.
    owner_kin: Query<&ambition_platformer2d_core::BodyKinematics>,
    // Friendly-fire policy (the DAMAGE side; targeting is `FactionRelations`).
    // Optional so minimal headless tests that don't stand up the plugin still run
    // (fall back to the default: friendly fire OFF — same-faction allies safe).
    // AE6: resolved match rules, not the world's baseline toggle.
    tuning: Option<Res<crate::rules::ResolvedCombatTuning>>,
    // ONE victim query for every body with a published footprint (fable review
    // 2026-07-02 §A3 — this system used to run separate actor and player victim
    // loops whose faction rules and hurtboxes had drifted), named as the role it
    // is: [`StrikeVictim`].
    //
    // ⭐ **The vulnerability cluster is a FILTER here, and it is now spelled as
    // one.** It was four REQUIRED members of the data tuple bound to `_vuln` and
    // never read — since §A2 i-frames are consumed by `resolve_body_hit` on the
    // victim side, never decided here. Its remaining job is to say "only real
    // combat bodies are victims of hostile melee", which is a `With` claim. As
    // data it read like an input; as a filter it reads like the narrowing it is,
    // and the victim SET is byte-identical either way (`With<T>` and `&T` match
    // the same archetypes).
    //
    // ⚠ this filter is why melee's victim set is narrower than the projectile
    // path's, which carries no such `With`. That difference used to be invisible,
    // buried in whether one tuple wrote `Option<(..)>` and the other did not.
    victims: Query<
        StrikeVictim,
        (
            With<ambition_platformer2d_core::BodyOffense>,
            With<ambition_platformer2d_core::BodyMotionFacts>,
            With<ambition_platformer2d_core::BodyShieldState>,
            With<ambition_characters::actor::BodyCombat>,
        ),
    >,
    // The attacker's grudge, looked up from the swing owner — the DAMAGE-side
    // per-entity override. Lets a hit land on a same-faction body the owner has a
    // personal grudge against (two `Npc` duelists), without re-tagging factions.
    // Read-only, so it may overlap the other actor queries.
    attacker_aggression: Query<&ActorAggression>,
    // The swing owner's team, looked up the same way its grudge is. Read-only,
    // so it may overlap the victim query.
    attacker_team: Query<&crate::targeting::MatchTeam>,
    // The attacker's own move state, read for ONE thing: the per-strike dedup
    // accumulator that keeps a multi-tick Active window from re-smashing the same
    // breakable every frame. `MovePlayback` is authoritative move-timeline state
    // (rollback-registered and checksummed), which is why the ignore list is read
    // from here and not from the `BodyMelee.swing` projection that used to gate
    // this emit — the projection is rebuilt every frame and wiped the accumulator.
    // A body with no playback answers with an empty list and still strikes.
    attacker_moves: Query<&crate::moveset::MovePlayback>,
    // A live Hitbox is already authoritative gameplay state. Moveset strikes
    // exist only while their active window exists; `BodyMelee.swing` is a
    // presentation/read-model projection and must never gate whether this
    // geometry can deal damage. Keeping the authority here prevents a visible,
    // correctly placed strike from becoming inert because a secondary projection
    // lagged, was absent, or classified the move differently.
    // CM8: melee overlap no longer emits hit feedback here — the ONE victim-side
    // reaction (`emit_hit_feedback`) owns sfx/spray/debris now, so this system
    // only needs the `VfxMessage` writer for the wielded-AOE landing cue (a World
    // strike, not a body-owned melee contact).
    mut vfx: MessageWriter<VfxMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    mut landed_hits: MessageWriter<LandedBodyHit>,
) {
    let friendly_fire = tuning.map(|t| t.friendly_fire()).unwrap_or_default();
    for (hitbox_entity, hitbox, mut hits) in &mut hitboxes {
        // Resolve the owner's collision-box center for FollowOwner tracking.
        // Actors carry `CenteredAabb`; bare fixtures may carry only
        // `BodyKinematics`. If neither resolves (owner despawned), leave the
        // hitbox a harmless ghost for `tick_and_despawn_hitboxes`.
        let owner_pos = if let Ok(aabb) = owners.get(hitbox.owner) {
            aabb.center
        } else if let Ok(kin) = owner_kin.get(hitbox.owner) {
            kin.pos
        } else {
            continue;
        };
        let world_volume = hitbox.world_volume(owner_pos);
        let source_faction = actor_faction_from_hit_side(hitbox.source);

        // ONE BODY, ONE PATH: every body-owned melee strike resolves contacts
        // here. `HitSide` selects descriptive source vocabulary only; it does not
        // select a different overlap/dedup/knockback algorithm.
        let melee_source = match (hitbox.source, hitbox.anchor) {
            (HitSide::Player, HitboxAnchor::FollowOwner { .. }) => {
                Some(HitSource::Melee)
            }
            (HitSide::Enemy | HitSide::Npc, _) => Some(HitSource::Melee),
            (HitSide::Boss, _) => Some(HitSource::Melee),
            (HitSide::Player, HitboxAnchor::World { .. }) | (HitSide::Neutral, _) => None,
        };

        if let Some(source_kind) = melee_source {
            let owner_grudge = attacker_aggression
                .get(hitbox.owner)
                .ok()
                .and_then(|a| a.grudge);

            for victim in &victims {
                // Identity beats every relationship rule. Friendly fire, match
                // teams, and grudges can decide whether TWO bodies may fight;
                // none of them can make one body become its own victim.
                if victim.entity == hitbox.owner {
                    continue;
                }
                // Structural tangibility gate (Jon 2026-07-22): a dead body is
                // an intangible corpse — the swing passes through it. Skipping
                // here means NO event and NO impact VFX are produced at the
                // corpse, so a dead thing neither interacts nor presents. (The
                // consume-time `resolve_body_hit` alive check stays as defense.)
                if victim.is_corpse() {
                    continue;
                }
                if !crate::targeting::damage_lands_between(
                    source_faction,
                    victim.effective_faction(),
                    attacker_team.get(hitbox.owner).ok(),
                    victim.team,
                    friendly_fire,
                    owner_grudge,
                    victim.entity,
                ) {
                    continue;
                }
                if hits.hit.contains(&victim.entity) {
                    continue;
                }
                if !victim.reached_by(&world_volume) {
                    continue;
                }

                let victim_body = victim.aabb.aabb();
                let impact = midpoint(victim.aabb.center, world_volume.center());
                // Knockback side in the victim's LOCAL frame (§B11): under
                // sideways gravity the attacker and victim separate along
                // world-Y, exactly when a screen-X comparison degenerates.
                let side = victim.knockback_side();
                let dir = if (victim_body.center() - owner_pos).dot(side) >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let (victim_damage_taken, victim_weight) = victim.knockback_growth_inputs();
                let magnitude = resolved_hitbox_knockback_magnitude(
                    hitbox.knockback,
                    victim_damage_taken,
                    victim_weight,
                );
                let knockback = Some(HitKnockback {
                    dir,
                    magnitude,
                    source_pos: owner_pos,
                    impact_pos: impact,
                    launch_dir: hitbox.launch_dir,
                });
                hit_events.write(HitEvent {
                    strike_sfx: hitbox.strike_sfx,
                    volume: world_volume.clone(),
                    damage: hitbox.damage.max(1),
                    source: source_kind.clone(),
                    attacker: Some(hitbox.owner),
                    // The victim, named. ⛔ this used to fork on
                    // `victim.is_player` to pick between two target variants —
                    // a producer classifying its victim for the benefit of a
                    // consumer's routing. The entity says it already.
                    target: HitTarget::Body(victim.entity),
                    mode: HitMode::Knockback,
                    knockback,
                    ignored_targets: Vec::new(),
                });
                landed_hits.write(LandedBodyHit {
                    hitbox: hitbox_entity,
                    attacker: hitbox.owner,
                    victim: victim.entity,
                    volume: world_volume.clone(),
                    contact: impact,
                });
                hits.hit.insert(victim.entity);
            }

            // THE UNRESOLVED HALF OF THE SAME STRIKE.
            //
            // The loop above resolved every real combat body by identity. It could
            // not resolve two things that a swing nevertheless hits: a breakable,
            // and a boss whose HP and phase live on an encounter rather than on a
            // body carrying the combat cluster. Neither matches `StrikeVictim`, so
            // neither has an entity this system may name — and when the player's
            // melee stopped broadcasting, both stopped being hittable at all. That
            // was the regression `boss_contact_iframes` and `rollback_exit_oracle`
            // caught: the swing connected with nothing, all day, in both games.
            //
            // ⛔ so the broadcast is not restored as a second melee path. It is
            // published as what it is — [`HitTarget::UnresolvedFeatures`], the part
            // of this strike whose targets are still unnamed — and the resolver
            // above stays the one authority on bodies. The consumer scans bosses
            // and breakables for it and MUST NOT scan bodies, which have already
            // taken their identified hit.
            //
            // Dedup rides `MovePlayback.hit_targets`, the move's own authoritative
            // per-strike accumulator, NOT the `BodyMelee.swing` projection that
            // used to gate this emit — a read-model must never decide whether a
            // strike can damage, and that projection is rebuilt every frame.
            //
            // ⭐ **EVERY body-owned melee publishes it, not just the player's.**
            // The gate here was `matches!(source_kind, PlayerSlash)`, and that
            // one permission was standing in for a rule nobody had written down:
            // the boss scan applied no relationship policy, so "only the player
            // may broadcast" WAS the boss's who-may-hurt-me rule. With the scan
            // adjudicating properly the permission is free to go, and an enemy's
            // swing smashing a crate or reaching a boss is the body-generic
            // answer rather than a new special case.
            //
            // ⚠ lifting it desynced the rollback suite, and the cause was NOT
            // where I predicted. `stage_player_victim_hit_events` staged this
            // unresolved half into the player-victim FIFO — its fallback arm
            // reads `!seeks_victims()`, and an enemy swing's cause is filed
            // victim-side by the direction words. The per-component localizer
            // named the resource in one run; see
            // `which_component_does_the_lifecycle_reset_divergence_live_in`.
            {
                hit_events.write(HitEvent {
                    strike_sfx: hitbox.strike_sfx,
                    volume: world_volume.clone(),
                    damage: hitbox.damage.max(1),
                    source: source_kind,
                    attacker: Some(hitbox.owner),
                    target: HitTarget::UnresolvedFeatures,
                    mode: HitMode::Knockback,
                    knockback: None,
                    ignored_targets: attacker_moves
                        .get(hitbox.owner)
                        .map(|pb| pb.hit_targets.clone())
                        .unwrap_or_default(),
                });
            }
            continue;
        }

        match hitbox.source {
            // A World-anchored Player strike is a fixed AOE (the wielded boss-
            // style shockwave), not body-owned melee. Keep its broadcast semantics
            // until that separate primitive is refactored: fire once per strike via
            // the owner sentinel and let the feature resolver fan out the volume.
            HitSide::Player => {
                debug_assert!(matches!(hitbox.anchor, HitboxAnchor::World { .. }));
                if hits.hit.insert(hitbox.owner) {
                    vfx.write(VfxMessage::Impact {
                        pos: world_volume.center(),
                    });
                    hit_events.write(HitEvent {
                        strike_sfx: hitbox.strike_sfx,
                        volume: world_volume.clone(),
                        damage: hitbox.damage.max(1),
                        source: HitSource::Melee,
                        attacker: Some(hitbox.owner),
                        target: HitTarget::Volume,
                        mode: HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                    });
                }
            }
            // Neutral never spawns a damaging hitbox (a provoked Npc is handled
            // by the direct melee path above with its real faction).
            HitSide::Neutral => {}
            // These sides were consumed by `melee_source` and continued above.
            HitSide::Enemy | HitSide::Boss | HitSide::Npc => unreachable!(),
        }
    }
}

/// Advance every hitbox's lifetime by `world_time.sim_dt()` and
/// despawn the ones that hit zero. Sim-clock so bullet-time freezes
/// in-flight hitboxes alongside the rest of combat (ADR 0010).
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
