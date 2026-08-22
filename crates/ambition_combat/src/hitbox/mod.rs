//! Hitbox-entity lifecycle: spawn, resolve overlaps, then despawn.
//!
//! `HitboxHits` deduplicates victims across a multi-tick active window. `HitboxAnchor::FollowOwner`
//! resolves world geometry from the owner's current position each tick.

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

/// Resolve a live hitbox's unit-bearing payload for one victim.
///
/// Feel multipliers pass through unchanged. Launch-speed growth is resolved against the victim's
/// accumulated damage and weight; authored nonzero growth overrides the ruleset fraction of base
/// launch speed.
fn resolved_hitbox_knockback_magnitude(
    knockback: HitboxKnockback,
    victim_damage_taken: i32,
    victim_weight: f32,
    ruleset_growth: f32,
) -> HitKnockbackMagnitude {
    match knockback {
        HitboxKnockback::FeelScale(scale) => HitKnockbackMagnitude::FeelScale(scale.max(0.0)),
        HitboxKnockback::LaunchSpeed { base, growth } => {
            let growth = if growth > 0.0 {
                growth
            } else {
                base * ruleset_growth.max(0.0)
            };
            let launch_speed =
                crate::util::scaled_knockback(base, growth, victim_damage_taken, victim_weight)
                    .max(0.0);
            HitKnockbackMagnitude::LaunchSpeed(launch_speed)
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
    /// Knockback weight (CM1). Absent  the reference weight `1.0`.
    pub tuning: Option<&'static super::components::CombatTuning>,
    /// Outranks faction for "may this land": two humans share a faction, so a
    /// match could not otherwise let them hit each other.
    pub team: Option<&'static crate::targeting::MatchTeam>,
    /// Read for the parry window only. A body without one simply cannot parry.
    pub shield: Option<&'static ambition_platformer2d_core::BodyShieldState>,
    /// This body's voice, for the cue its striker emits (the parry clang).
    pub voice: Option<&'static ambition_sfx::BodyPresentationSource>,
    /// The victim's resolved motion frame; knockback direction is interpreted in this frame.
    pub frame: Option<&'static ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    pub is_player: Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
}

impl StrikeVictimItem<'_, '_> {
    /// Allegiance as this body actually fights: a possessed victim carrying
    /// `DrivingParticipant` is a Player-effective body, without its authored
    /// faction being mutated.
    pub fn effective_faction(&self) -> ActorFaction {
        effective_faction(*self.faction, self.driver)
    }

    /// A dead body is an intangible corpse — the strike passes through it.
    pub fn is_corpse(&self) -> bool {
        crate::util::body_is_corpse(self.health)
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
    // These components narrow hostile melee to complete combat bodies; victim-side resolution
    // owns their actual semantics.
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
    // The swing owner's own accumulated damage, for RAGE. Looked up exactly like
    // its grudge and its team, and read-only for the same reason.
    attacker_health: Query<&ambition_characters::actor::BodyHealth>,
    // The swing owner's stale queue and the move it is executing, so a repeated
    // answer is worth less. Read-only, looked up by owner, like the three above.
    attacker_stale: Query<&crate::stale::BodyStaleMoves>,
    attacker_playback: Query<&crate::moveset::MovePlayback>,
    // The attacker's own move state, read for ONE thing: the per-strike dedup accumulator that
    // keeps a multi-tick Active window from re-smashing the same breakable every frame. A body with
    // no playback answers with an empty list and still strikes.
    attacker_moves: Query<&crate::moveset::MovePlayback>,
    // Live Hitbox state is authoritative for damage; presentation projections must not gate it.
    // Victim-side `emit_hit_feedback` owns melee feedback. This writer is only for wielded-AOE VFX.
    mut vfx: MessageWriter<VfxMessage>,
    mut hit_events: MessageWriter<HitEvent>,
    mut landed_hits: MessageWriter<LandedBodyHit>,
) {
    // Both rule reads take the resource by reference: the growth term is read
    // per victim below, and moving it here left that read with nothing.
    let ruleset_growth = tuning
        .as_deref()
        .map(|t| t.knockback_growth)
        .unwrap_or_default();
    // RAGE is read here for the same reason growth is read per victim below:
    // the resource is borrowed once and each rule takes what it needs.
    let rules = tuning.as_deref().copied().unwrap_or_default();
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
                // Structural tangibility gate: a dead body is
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
                    ruleset_growth,
                );
                // RAGE, and it is the mirror of the percent mechanic. The
                // victim's damage already scaled that launch; without this the
                // fighter behind is punished twice — easier to launch and no
                // harder to launch with. `1.0` in a game that declares no rage.
                // RAGE and STALING are one multiplier, applied once. They
                // pull opposite ways on purpose — a hurt fighter hits harder, a
                // repeated move hits softer — and a game that declares neither
                // gets exactly `1.0` from both.
                let rage = rules.rage_scale(
                    attacker_health
                        .get(hitbox.owner)
                        .map(|h| h.damage_taken())
                        .unwrap_or(0),
                );
                let stale = rules.stale_scale(
                    match (
                        attacker_playback.get(hitbox.owner),
                        attacker_stale.get(hitbox.owner),
                    ) {
                        (Ok(playback), Ok(queue)) => {
                            queue.occurrences(crate::stale::stale_move_hash(&playback.spec.id))
                        }
                        // A body with no live move or no queue has thrown nothing
                        // to wear out.
                        _ => 0,
                    },
                );
                let magnitude = magnitude.scaled(rage * stale);
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
                    // the DAMAGE stales with the launch. A move worn out that
                    // still filled the percent meter at full rate would be half a
                    // mechanic — and `max(1)` keeps a fully stale hit a hit.
                    damage: ((hitbox.damage as f32 * stale).round() as i32).max(1),
                    source: source_kind.clone(),
                    attacker: Some(hitbox.owner),
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

            // Publish the unresolved feature half of the strike after body targets
            // have been resolved. Feature consumers may scan bosses/breakables but
            // must not damage bodies again. Per-strike dedup is authoritative in
            // `MovePlayback.hit_targets`, not the `BodyMelee` read model.
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
            // Keep its broadcast semantics until that separate primitive is refactored: fire
            // once per strike via the owner sentinel and let the feature resolver fan out the
            // volume.
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
