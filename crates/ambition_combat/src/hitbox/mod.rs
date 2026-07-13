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

use bevy::prelude::{Commands, Entity, MessageWriter, Query, Res, With};

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

use super::components::ActorAggression;
use super::components::ActorFaction;
use super::events::{HitEvent, HitKnockback, HitMode, HitSource, HitTarget};
use super::targeting::{damage_lands, effective_faction};
use super::util::midpoint;
use crate::{actor_faction_from_hit_side, hit_side_from_actor_faction};
use ambition_sfx::SfxMessage;
use ambition_time::WorldTime;
use ambition_vfx::vfx::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

// The hitbox COMPONENTS moved to the reusable `ambition_vfx` crate (the
// damage-box primitive). Re-exported here so `combat::hitbox::Hitbox`
// (and `features::Hitbox`) paths are unchanged; the SYSTEMS below (damage
// resolution, melee spawn, lifecycle) stay in the lib.
pub use ambition_vfx::{HitSide, Hitbox, HitboxAnchor, HitboxHits, HitboxLifetime};

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
    owner_kin: Query<&ambition_engine_core::BodyKinematics>,
    // Friendly-fire policy (the DAMAGE side; targeting is `FactionRelations`).
    // Optional so minimal headless tests that don't stand up the plugin still run
    // (fall back to the default: friendly fire OFF — same-faction allies safe).
    friendly_fire: Option<Res<crate::targeting::FriendlyFire>>,
    // Non-player actor victims for the actor-vs-actor melee path: an Enemy/Boss
    // swing damages any DIFFERENT-faction actor it overlaps (e.g. a Boss vs an
    // Enemy in a duel); same-faction allies are spared unless friendly fire is on.
    // `Option<&Brain>`: a possessed victim (carrying `Brain::Player`) is a
    // Player-EFFECTIVE body, so a former ally's Enemy swing lands on it — via
    // effective allegiance, without its authored faction being mutated.
    // ONE victim query for every body with a published footprint (fable review
    // 2026-07-02 §A3 — this system used to run separate actor and player
    // victim loops whose faction rules and hurtboxes had drifted). Every body
    // carries the three vulnerability clusters now — bosses too, since §A1
    // slice 3 gave them the inert defaults — so the tuple is no longer `Option`.
    // Since §A2 they're read only to MUTE feedback (i-frames are consumed by
    // `resolve_body_hit` on the victim side, never decided here).
    victims: Query<(
        Entity,
        &super::components::CenteredAabb,
        &ActorFaction,
        Option<&ambition_characters::brain::Brain>,
        (
            &ambition_engine_core::BodyOffense,
            &ambition_engine_core::BodyDodgeState,
            &ambition_engine_core::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
        ),
        bevy::prelude::Has<ambition_platformer_primitives::markers::PlayerEntity>,
        // CM1 knockback scaling: the victim's accumulated-damage meter and its
        // archetype weight. Both `Option` — the player carries `BodyHealth` but
        // no `CombatTuning` (weight → reference `1.0`); a headless test body may
        // carry neither (damage_taken → 0). Growth is inert unless the striking
        // volume authored `kb_growth`, so this is parity-free by construction.
        // Reads the combat-owned `CombatTuning`, never the sim-heart `ActorConfig`
        // (E2 verdict b).
        Option<&ambition_characters::actor::BodyHealth>,
        Option<&super::components::CombatTuning>,
    )>,
    // The attacker's grudge, looked up from the swing owner — the DAMAGE-side
    // per-entity override. Lets a hit land on a same-faction body the owner has a
    // personal grudge against (two `Npc` duelists), without re-tagging factions.
    // Read-only, so it may overlap the other actor queries.
    attacker_aggression: Query<&ActorAggression>,
    // The owner's melee swing, so a Player-faction FollowOwner strike (the player's
    // slash — and a possessed actor's) reads the per-swing `hit_targets` for
    // one-hit-per-target dedup and emits only while the swing is live.
    melee_owners: Query<&super::components::BodyMelee>,
    // Iterate every player so a multi-player build hits each
    // overlapping player independently. Single-player behavior is
    // preserved because the iterator has exactly one entity today.
    // The victim's per-tick resolved frame (ADR 0024), for the local-frame
    // knockback side (§B11). Looked up by victim entity; a bare test hurtbox
    // without a body frame falls back to the engine default down.
    victim_frames: Query<&ambition_platformer_primitives::frame_env::ResolvedMotionFrame>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut hit_events: MessageWriter<HitEvent>,
) {
    let friendly_fire = friendly_fire.map(|r| *r).unwrap_or_default();
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
                for (
                    victim_entity,
                    victim_aabb,
                    victim_faction,
                    victim_brain,
                    vuln,
                    is_player,
                    victim_health,
                    victim_tuning,
                ) in &victims
                {
                    if victim_entity == hitbox.owner {
                        continue;
                    }
                    let victim_faction = effective_faction(*victim_faction, victim_brain);
                    if !damage_lands(
                        source_faction,
                        victim_faction,
                        friendly_fire,
                        owner_grudge,
                        victim_entity,
                    ) {
                        continue;
                    }
                    if hits.hit.contains(&victim_entity) {
                        continue;
                    }
                    let victim_body = victim_aabb.aabb();
                    if !world_volume.intersects_aabb(victim_body) {
                        continue;
                    }
                    // §A2: the EVENT always flows — i-frames resolve at CONSUME
                    // time in `resolve_body_hit`, the same for every body (this
                    // was the last emit/consume asymmetry). The vulnerability
                    // read below is FEEDBACK policy only: don't play the
                    // hit-landed sfx/burst for a hit the consumer will ignore
                    // (dodge roll, parry, i-frame window).
                    let (offense, dodge, shield, combat) = vuln;
                    let feedback =
                        !is_player || crate::util::body_vulnerable(offense, dodge, shield, combat);
                    let impact = midpoint(victim_aabb.center, world_volume.center());
                    if feedback {
                        vfx.write(VfxMessage::Impact { pos: impact });
                    }
                    if is_player && feedback {
                        sfx.write(SfxMessage::Play {
                            id: ambition_sfx::ids::PLAYER_DAMAGE,
                            pos: impact,
                        });
                        vfx.write(VfxMessage::Burst {
                            pos: impact,
                            count: 14,
                            speed: 300.0,
                            color: [1.0, 0.34, 0.28, 0.88],
                            kind: ParticleKind::Shard,
                        });
                        debris.write(DebrisBurstMessage {
                            pos: impact,
                            cue: PhysicsDebrisCue::Impact,
                        });
                    }
                    // Knockback side in the victim's LOCAL frame (§B11): under
                    // sideways gravity the attacker and victim separate along
                    // world-Y, exactly when a screen-X comparison degenerates.
                    // The consumer's gravity-relative resolution keeps this as
                    // its fallback, so the stored side must be frame-correct
                    // too. Attached for EVERY victim (§A2 step 6): an actor
                    // victim rides the same resolved knockback the player does.
                    let side = victim_frames
                        .get(victim_entity)
                        .map(|frame| frame.basis())
                        .unwrap_or(ae::AccelerationFrame::new(ae::DEFAULT_GRAVITY_DIR))
                        .side;
                    let dir = if (victim_body.center() - owner_pos).dot(side) >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    };
                    // CM1: fold the smash-percent growth term onto the flat
                    // knockback using THIS victim's accumulated damage + weight.
                    // `growth == 0` (every un-authored volume) returns the flat
                    // strength unchanged — parity by construction.
                    let victim_damage_taken = victim_health.map(|h| h.damage_taken()).unwrap_or(0);
                    let victim_weight = victim_tuning.map(|ct| ct.weight).unwrap_or(1.0);
                    let strength = crate::util::scaled_knockback(
                        hitbox.knockback_strength,
                        hitbox.knockback_growth,
                        victim_damage_taken,
                        victim_weight,
                    )
                    .max(0.0);
                    let knockback = Some(HitKnockback {
                        dir,
                        strength,
                        source_pos: owner_pos,
                        impact_pos: impact,
                        // CM1: the authored launch angle rides through to the
                        // victim-side resolver.
                        launch_dir: hitbox.launch_dir,
                    });
                    hit_events.write(HitEvent {
                        volume: world_volume.clone(),
                        damage: hitbox.damage.max(1),
                        source: source_kind.clone(),
                        // The entity that spawned the hitbox is the attacker —
                        // read on the victim side to attribute hitstun / the
                        // death cause to the right body.
                        attacker: Some(hitbox.owner),
                        // Stamp the victim so the right consumer lands the hit.
                        target: if is_player {
                            HitTarget::Player(victim_entity)
                        } else {
                            HitTarget::Actor(victim_entity)
                        },
                        mode: HitMode::Knockback,
                        knockback,
                        ignored_targets: Vec::new(),
                    });
                    hits.hit.insert(victim_entity);
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
                // resolver folds landed keys back in). The slash's signed `knock_x`
                // rides the strike. No swing armed ⇒ no strike.
                HitboxAnchor::FollowOwner { .. } => {
                    let Some(swing) = melee_owners
                        .get(hitbox.owner)
                        .ok()
                        .and_then(|m| m.swing.as_ref())
                    else {
                        continue;
                    };
                    hit_events.write(HitEvent {
                        volume: world_volume.clone(),
                        damage: hitbox.damage.max(1),
                        source: HitSource::PlayerSlash {
                            knock_x: hitbox.knock_x,
                        },
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
                            volume: world_volume.clone(),
                            damage: hitbox.damage.max(1),
                            source: HitSource::PlayerSlash {
                                knock_x: hitbox.knock_x,
                            },
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

/// Spawn helper: emit a fresh hitbox entity for a melee strike. The
/// caller picks the local offset / half-extent / damage / faction
/// based on the strike's archetype + facing. `knock_x` is the signed
/// horizontal slash impulse for Player-faction strikes (0 for aggressor
/// strikes, which knock via position-derived `knockback_strength`).
#[allow(clippy::too_many_arguments)]
pub fn spawn_melee_hitbox(
    commands: &mut Commands,
    owner: Entity,
    source: ActorFaction,
    local_offset: ae::Vec2,
    half_extent: ae::Vec2,
    damage: i32,
    knockback_strength: f32,
    knock_x: f32,
    active_s: f32,
    frame_down: ae::Vec2,
) -> Entity {
    commands
        .spawn((
            Hitbox {
                owner,
                source: hit_side_from_actor_faction(source),
                anchor: HitboxAnchor::FollowOwner { local_offset },
                half_extent,
                shape: None,
                facing: 1.0,
                damage,
                knockback_strength,
                // Aggressor/player melee strikes are flat-knockback; percent
                // growth is authored only on moveset volumes (CM1).
                knockback_growth: 0.0,
                launch_dir: None,
                knock_x,
                frame_down,
            },
            HitboxLifetime {
                remaining_s: active_s.max(0.0),
            },
            HitboxHits::default(),
        ))
        .id()
}

/// THE ONE melee strike spawn — every body (player AND brain-driven actor) turns
/// its active-frame swing into a damage hitbox AND its slash VFX through this one
/// function, from a SINGLE gravity-resolved `world_box`. Because the hitbox and
/// the slash are both derived from that one box, they can NEVER point in different
/// directions (the bug where the player's screen-axis hitbox diverged from its
/// gravity-rotated slash under rotated gravity). Debug the box once, here, and it
/// is right for every character under every gravity.
///
/// `world_box` is the strike's damage AABB already rotated into the body's world
/// frame (so it lands toward "forward" under any gravity). `knock_x` is the signed
/// slash impulse (Player strikes); `knockback_strength` the aggressor push.
#[allow(clippy::too_many_arguments)]
pub fn spawn_melee_strike(
    commands: &mut Commands,
    vfx: &mut MessageWriter<VfxMessage>,
    owner: Entity,
    source: ActorFaction,
    body_pos: ae::Vec2,
    world_box: ae::Aabb,
    damage: i32,
    knockback_strength: f32,
    knock_x: f32,
    active_s: f32,
    slash_kind: ambition_vfx::vfx::SlashKind,
    frame_down: ae::Vec2,
) -> Entity {
    let local_offset = world_box.center() - body_pos;
    let entity = spawn_melee_hitbox(
        commands,
        owner,
        source,
        local_offset,
        world_box.half_size(),
        damage,
        knockback_strength,
        knock_x,
        active_s,
        frame_down,
    );
    crate::util::emit_melee_slash(
        vfx,
        world_box.center(),
        world_box.half_size(),
        slash_kind,
        local_offset,
    );
    entity
}

#[cfg(test)]
mod tests;
