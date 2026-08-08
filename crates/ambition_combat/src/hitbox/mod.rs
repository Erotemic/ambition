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
use bevy::prelude::{Commands, Entity, Has, MessageWriter, Query, Res, With};

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
pub use ambition_vfx::{
    HitSide, Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback, HitboxLifetime,
};

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

/// Apply each live hitbox's damage to the right faction's targets.
///
/// Enemy / Boss hitboxes hit the player and emit `HitEvent` with a
/// victim-side `HitSource`. Player / Npc / Neutral hitboxes are
/// routed through other paths (player slash flows as
/// `HitSource::PlayerSlash`); this system is the catch-all for
/// hostile melee.
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
    // The owner's melee swing, so a Player-faction FollowOwner strike (the player's
    // slash — and a possessed actor's) reads the per-swing `hit_targets` for
    // one-hit-per-target dedup and emits only while the swing is live.
    melee_owners: Query<&super::components::BodyMelee>,
    // CM8: hostile melee overlap no longer emits hit feedback here — the ONE
    // victim-side reaction (`emit_hit_feedback`) owns sfx/spray/debris now, so
    // this system only needs the `VfxMessage` writer for the wielded-AOE landing
    // cue (a Volume strike, not a per-victim reaction).
    mut vfx: MessageWriter<VfxMessage>,
    mut hit_events: MessageWriter<HitEvent>,
) {
    let friendly_fire = tuning.map(|t| t.friendly_fire()).unwrap_or_default();
    for (_hitbox_entity, hitbox, mut hits) in &mut hitboxes {
        // Resolve the owner's collision-box center for FollowOwner tracking.
        // Actors carry `CenteredAabb`; the PLAYER (a melee strike owner now) does
        // NOT — it carries `BodyKinematics` (pos = box center). Try the actor box,
        // then fall back to body kinematics, so a player-owned strike tracks too.
        // If neither resolves (owner despawned), leave the hitbox a harmless ghost
        // for `tick_and_despawn_hitboxes` — an owner-less hitbox has no source pos.
        let owner_pos = if let Ok(aabb) = owners.get(hitbox.owner) {
            aabb.center
        } else if let Ok(kin) = owner_kin.get(hitbox.owner) {
            kin.pos
        } else {
            continue;
        };
        let world_volume = hitbox.world_volume(owner_pos);

        let source_faction = actor_faction_from_hit_side(hitbox.source);
        match hitbox.source {
            // Aggressor melee: Enemy, Boss, OR a PROVOKED Npc (a peaceful NPC turned
            // hostile keeps its Npc faction but fights like any aggressor). All three
            // damage different-faction actors + an overlapping player under the
            // physical rule; same-faction allies are spared via `can_damage`. (A
            // PEACEFUL NPC never reaches here — with no combat target it spawns no
            // hitbox.) Only `Neutral` is truly inert.
            HitSide::Enemy | HitSide::Boss | HitSide::Npc => {
                let source_kind = if matches!(hitbox.source, HitSide::Boss) {
                    HitSource::BossAttack
                } else {
                    HitSource::EnemyAttack
                };
                // Actor-vs-actor: a swing damages any DIFFERENT-faction actor it
                // overlaps, OR a same-faction actor the owner holds a personal grudge
                // against (two `Npc` duelists feuding). Same-faction non-grudged allies
                // are spared unless friendly fire is on; the attacker never hits itself
                // (owner check). Stamped `HitTarget::Actor` so the actor-damage consumer
                // applies it to exactly that body.
                let owner_grudge = attacker_aggression
                    .get(hitbox.owner)
                    .ok()
                    .and_then(|a| a.grudge);
                // ONE victim loop (§A3): every body with a published footprint —
                // player, actor, boss, possessed anything — resolves through the
                // same relational rule (`damage_lands` = different-faction ||
                // personal grudge; `can_damage` for a Player victim is the same
                // predicate since a player is never the aggressor's faction) and
                // the same published hurtbox. i-frames resolve at CONSUME time
                // for every body (`resolve_body_hit`, §A2). Victim KIND picks
                // only policy: a player victim gets the knockback payload and
                // the richer feedback.
                for victim in &victims {
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
                    // §A2: the EVENT always flows — i-frames resolve at CONSUME
                    // time in `resolve_body_hit`, the same for every body.
                    // CM8: hit FEEDBACK (the sfx/spray/debris) is no longer
                    // emitted here. It moved to the ONE victim-side reaction
                    // (`emit_hit_feedback`), which fires only when the consumer's
                    // `resolve_body_hit` reports the hit LANDED — so a dodged /
                    // parried / i-framed hit is muted for free, without this side
                    // recomputing vulnerability, and an enemy struck by another
                    // enemy uses its OWN `HurtFeedback` instead of the player's
                    // red "you got hurt" burst.
                    let impact = midpoint(victim.aabb.center, world_volume.center());
                    // Knockback side in the victim's LOCAL frame (§B11): under
                    // sideways gravity the attacker and victim separate along
                    // world-Y, exactly when a screen-X comparison degenerates.
                    // The consumer's gravity-relative resolution keeps this as
                    // its fallback, so the stored side must be frame-correct
                    // too. Attached for EVERY victim (§A2 step 6): an actor
                    // victim rides the same resolved knockback the player does.
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
                        // CM1: the authored launch angle rides through to the
                        // victim-side resolver.
                        launch_dir: hitbox.launch_dir,
                    });
                    hit_events.write(HitEvent {
                        strike_sfx: hitbox.strike_sfx,
                        volume: world_volume.clone(),
                        damage: hitbox.damage.max(1),
                        source: source_kind.clone(),
                        // The entity that spawned the hitbox is the attacker —
                        // read on the victim side to attribute hitstun / the
                        // death cause to the right body.
                        attacker: Some(hitbox.owner),
                        // Stamp the victim so the right consumer lands the hit.
                        target: if victim.is_player {
                            HitTarget::Player(victim.entity)
                        } else {
                            HitTarget::Actor(victim.entity)
                        },
                        mode: HitMode::Knockback,
                        knockback,
                        ignored_targets: Vec::new(),
                    });
                    hits.hit.insert(victim.entity);
                }
            }
            // Player-faction hitbox (a wielded boss-style AOE — see
            // `crate::abilities::ranged::shockwave`): damage the enemies/bosses it overlaps by
            // emitting ONE attacker-side Volume `HitEvent` that
            // `apply_feature_hit_events` resolves against every overlapping
            // actor + boss. This is the player end of the same primitive a boss
            // AOE uses through the Enemy/Boss branch above — the faction is the
            // only difference. Fires once per strike (the owner doubles as a
            // "already fired" sentinel in `HitboxHits`, harmless since a hitbox
            // never targets its own owner).
            HitSide::Player => match hitbox.anchor {
                // A FollowOwner Player strike is a MELEE SWING (the player's slash,
                // or a possessed actor's) — the unified counterpart of the old
                // per-frame `advance_attack` Volume emit. Emit the Volume `HitEvent`
                // every active tick (the hitbox tracks the owner, so it connects on
                // whatever frame it reaches the target), deduped per-swing via the
                // owner's accumulating `MeleeSwing.hit_targets` (the universal
                // resolver folds landed keys back in). Melee knockback rides the
                // moveset volume's authored launch speed / `launch_dir`, so the slash
                // event carries no signed impulse. No swing armed ⇒ no strike.
                HitboxAnchor::FollowOwner { .. } => {
                    let Some(swing) = melee_owners
                        .get(hitbox.owner)
                        .ok()
                        .and_then(|m| m.swing.as_ref())
                    else {
                        continue;
                    };
                    hit_events.write(HitEvent {
                        strike_sfx: hitbox.strike_sfx,
                        volume: world_volume.clone(),
                        damage: hitbox.damage.max(1),
                        source: HitSource::PlayerSlash { knock_x: 0.0 },
                        attacker: Some(hitbox.owner),
                        target: HitTarget::Volume,
                        mode: HitMode::Knockback,
                        knockback: None,
                        ignored_targets: swing.hit_targets.clone(),
                    });
                }
                // A World-anchored Player strike is a fixed AOE (the wielded boss-
                // style shockwave). Fire ONCE per strike via the owner sentinel.
                HitboxAnchor::World { .. } => {
                    if hits.hit.insert(hitbox.owner) {
                        vfx.write(VfxMessage::Impact {
                            pos: world_volume.center(),
                        });
                        hit_events.write(HitEvent {
                            strike_sfx: hitbox.strike_sfx,
                            volume: world_volume.clone(),
                            damage: hitbox.damage.max(1),
                            source: HitSource::PlayerSlash { knock_x: 0.0 },
                            attacker: Some(hitbox.owner),
                            target: HitTarget::Volume,
                            mode: HitMode::Knockback,
                            knockback: None,
                            ignored_targets: Vec::new(),
                        });
                    }
                }
            },
            // Neutral never spawns a damaging hitbox (a provoked Npc is handled by
            // the aggressor branch above with its real faction).
            HitSide::Neutral => {}
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
