//! EFFECTS-stage consumers for `ActorActionMessage`.
//!
//! Hitboxes, projectiles, SFX, VFX, and recoil are driven from resolved
//! action messages rather than from per-actor integration loops.
//!
//! This module owns the consumer Bevy systems that read
//! `MessageReader<ActorActionMessage>` and produce effects. Each
//! system is one variant of `ActionRequest`; the upstream
//! `emit_brain_action_messages` resolver translates the actor's
//! `ActorControl` frame + `ActionSet` into the per-request stream
//! these systems consume.
//!
//! Schedule:
//! - `emit_brain_action_messages` runs first
//! - these systems run after, reading the same message stream
//! - the `BrainActionCounter` observer is unaffected (it counts but
//!   doesn't consume)

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_characters::brain::{
    action_set::{ActionRequest, ProjectileFlight, RangedCommitment},
    ActorActionMessage,
};
#[cfg(test)]
use ambition_combat::feel::Platformer2dFeelTuningMonolith;
use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};
use ambition_sfx::{SfxMessage, SfxWriter};

/// Recoil applied to the firing enemy along the negative fire
/// direction. Per-archetype because PirateOnShark visibly knocks
/// back the rider+shark combo.
const RANGED_RECOIL_PIRATE: f32 = 380.0;
const RANGED_RECOIL_DEFAULT: f32 = 60.0;

/// Projectile envelope shared by every ranged enemy. Future
/// per-archetype overrides (slower arrows, gravity-arc rocks)
/// will move this into an `ActionSet`-derived parameter.
const PROJECTILE_HALF_EXTENT: ae::Vec2 = ae::Vec2::new(10.0, 8.0);
const PROJECTILE_MAX_LIFETIME: f32 = 2.4;

/// How long the actor's post-fire Shoot overlay pose holds — matches the player's
/// `SHOOT_ANIM_HOLD_SECS` (`projectile::systems`) so a possessed body and an
/// autonomous one pulse the same, short enough that rapid fire stutters
/// Shoot↔locomotion rather than locking the read (§A9 follow-up).
const SHOOT_ANIM_HOLD_SECS: f32 = 0.18;

/// Read every `ActorActionMessage::Ranged` and spawn the matching projectile.
/// Applies recoil to the firing body's velocity.
///
/// BODY-GENERIC. The query now names only what firing actually needs: kinematics, the body's melee
/// state (which owns the shared refire floor), its surface frame, and an OPTIONAL archetype config
/// for the per-archetype default look. Any body that emits `ActionRequest::Ranged` now fires
/// through this one consumer.
pub fn spawn_projectiles_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut sfx: SfxWriter,
    mut actors: Query<(
        &mut ae::BodyKinematics,
        &mut crate::actor::BodyMelee,
        Option<&super::ActorSurfaceState>,
        Option<&super::ActorConfig>,
        Option<&ambition_characters::actor::BodyHealth>,
    )>,
    // Disjoint from `actors` — `ActorClusterQueryData` carries no `BodyAnimFacts`,
    // so this second view borrows the firing body's overlay-pose facts without
    // aliasing. Arms the Shoot pose on the frame the body accepts a shot.
    mut anim_facts: Query<&mut crate::actor::BodyAnimFacts>,
    held_items: Query<&super::HeldItem>,
    // ── AIM ASSIST ── the three reads that turn "the way I was pointing" into
    // "at the one opponent over there". Read-only and disjoint from `actors`,
    // which borrows kinematics mutably for the recoil.
    relations: Option<Res<ambition_combat::targeting::FactionRelations>>,
    shooters: Query<(
        &ambition_characters::actor::ActorFaction,
        Option<&ambition_combat::targeting::MatchTeam>,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
    candidates: Query<(
        Entity,
        &ae::CenteredAabb,
        &ambition_characters::actor::ActorFaction,
        &ambition_characters::actor::BodyHealth,
        Option<&ambition_combat::targeting::MatchTeam>,
        Option<&ambition_characters::control::DrivingParticipant>,
        // ⛔ THE STABLE IDENTITY, because an exact distance tie decided by
        // `Entity` is a desync: bevy_ggrs destroys and recreates rollback
        // entities, so the raw id a resimulation sees is not the one the
        // confirmed timeline saw.
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    )>,
    // WHO WROTE THIS BODY'S VELOCITY. The causal log answers "what is the
    // velocity" and never "who set it", so a body that moves without asking to
    // costs a survey of all 70 velocity writers to explain — measured, six
    // rebuild-and-print cycles on one 12-tick window. Recoil is one
    // of those writers and the first to say so out loud.
    //
    // `Option`, because the FEATURE and the PLUGIN are two switches: a host
    // may compile the publishers without installing an inspector.
    #[cfg(feature = "causal")] log: Option<ResMut<ambition_causal::CausalRecording>>,
    #[cfg(feature = "causal")] identities: Query<&ambition_combat::components::ActorIdentity>,
    #[cfg(feature = "causal")] tick: Option<Res<ambition_time::SimTick>>,
) {
    #[cfg(feature = "causal")]
    let mut log = log;
    for msg in messages.read() {
        let ActionRequest::Ranged {
            spec,
            origin,
            dir,
            dir_policy,
            commitment,
        } = msg.request.clone()
        else {
            continue;
        };
        let Ok((mut kin, mut melee, surface, config, health)) = actors.get_mut(msg.actor) else {
            // Message references a body that no longer exists
            // (despawned this frame). Skip silently.
            continue;
        };
        // Capability, not AI policy: the body fires because it OWNS a ranged
        // `ActionSet` slot (the upstream resolver only emits `Ranged` for a body
        // whose `ActionSet.ranged.is_some()`). A player possessing a peaceful NPC
        // fires its authored weapon; an autonomous peaceful NPC has no ranged
        // slot, so it emits nothing. Disposition (attack-or-not while autonomous)
        // is the BRAIN's business, not this effect consumer's.
        // A dead body fires nothing. `None` (a headless test body with no health
        // pool) is treated as alive, matching the shared hit resolver.
        if health.is_some_and(|h| !h.alive()) {
            continue;
        }
        // Body-side fire-rate enforcement for an ATTEMPT (invariant I3): a
        // controller emits `fire` every in-band tick and never rate-limits
        // itself, so the body accepts a shot only when the weapon is off
        // cooldown. A blocked attempt spawns nothing, and that is honest — the
        // controller was told nothing and the player was shown nothing.
        //
        // ⭐⭐ A COMMITTED MOVE IS NOT AN ATTEMPT. Its recharge was spent where
        // the move was ACCEPTED (`moveset::start_move`), a quarter of a second
        // and one whole windup animation ago. Asking again here is what made an
        // accepted Charge Shot play its charge and fire nothing — 22 of 28
        // authored ranged events in the duel arena, measured 2026-08-23.
        //
        // ⛔ SO THE FLOOR DID NOT GO AWAY; IT MOVED UPSTREAM, and it is authored
        // now (`RangedActionSpec::refire_s`) rather than being one constant in
        // this file that every character in the game was silently balanced
        // around.
        if commitment == RangedCommitment::Attempt
            && !melee.try_fire_ranged(spec.refire_s).accepted()
        {
            continue;
        }
        // The shot is committed — arm the firing body's Shoot overlay pose (the
        // actor analogue of the player's post-fire pulse in `projectile::systems`).
        // The pick reads `shoot_anim_timer`; the pose shows for whatever body owns
        // a Shoot row, autonomous or possessed (§A9 follow-up).
        if let Ok(mut anim) = anim_facts.get_mut(msg.actor) {
            anim.shoot_anim_timer = SHOOT_ANIM_HOLD_SECS;
        }
        // Held-item muzzle: a gun-sword shot should originate at the actor's
        // hand whether the pirate is still mounted or has fallen off the shark.
        // Future items can extend this routing by id without changing the brain.
        let held_item_id = held_items.get(msg.actor).ok().map(|item| item.id());
        let uses_gun_sword = held_item_id == Some("gun_sword");
        // The projectile's APPEARANCE is chosen by an OPEN visual id, set here at
        // the fire site: a gun-sword discharge is the spinning `"lasersword"`;
        // otherwise the archetype's authored ranged visual (e.g. the PCA's
        // `"glider"`), defaulting to the empty/generic hostile shot. The render
        // layer resolves this id through the content catalog — never the owner-id
        // string. The held-item id → projectile-visual id mapping is game policy;
        // when a second item needs its own discharge look this table can move to
        // a content-owned held-item→projectile registration.
        // Precedence: a held item's discharge, else the ACTION's own authored
        // visual (an equipment-granted verb brings its look with it), else the
        // archetype's default ranged look.
        let visual_id = if uses_gun_sword {
            "lasersword".to_string()
        } else if let Some(authored) = spec.visual.clone() {
            authored
        } else {
            config
                .map(|c| c.tuning.ranged_visual.clone())
                .unwrap_or_default()
        };
        // Flight is the ACTION's to author; the shared envelope is the fallback
        // for every ranged verb that doesn't care.
        let flight = spec.flight.unwrap_or(ProjectileFlight {
            gravity: 0.0,
            bounces: 0,
            bounce_on_world_contact: false,
            max_lifetime: PROJECTILE_MAX_LIFETIME,
            half_extent: PROJECTILE_HALF_EXTENT,
            // The pool's own straight envelope never turns around; a shot that
            // does says so on its authored flight.
            boomerang_return_s: None,
        });
        let gravity_dir = -surface
            .map(|s| s.surface_normal)
            .unwrap_or(ae::Vec2::new(0.0, -1.0))
            .normalize_or(ae::Vec2::new(0.0, -1.0));
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let request = ambition_characters::actor::control::ActorFireRequest {
            dir,
            dir_policy,
            speed: spec.speed(),
        };
        let commanded = request.dir_to_world(frame).normalize_or_zero();
        // ⭐ THE ONE PLACE A SHOT'S DIRECTION BECOMES WORLD-SPACE, which is the
        // only place an assist can be applied once and hold for every path that
        // fires — a move's timed `Ranged` event, a flat brain fire, a possessed
        // body, a replay. Bending it upstream would have to bend a
        // frame-relative vector and would miss half of them.
        let world_dir = match spec.aim_assist {
            None => commanded,
            Some(assist) => {
                let muzzle = kin.pos;
                let matrix = relations.as_deref();
                let foes = shooters.get(msg.actor).ok().map(|(faction, team, driver)| {
                    candidates
                        .iter()
                        .filter(move |(candidate, _, candidate_faction, health, candidate_team, candidate_driver, _)| {
                            *candidate != msg.actor
                                && health.alive()
                                && ambition_combat::targeting::combat_relation(
                                    matrix,
                                    *faction,
                                    driver,
                                    team,
                                    None,
                                    *candidate,
                                    **candidate_faction,
                                    *candidate_driver,
                                    *candidate_team,
                                )
                                .is_target()
                        })
                        .map(|(candidate, aabb, _, _, _, _, sim_id)| {
                            (candidate, sim_id.cloned(), aabb.center)
                        })
                        .collect::<Vec<_>>()
                });
                match foes {
                    Some(foes) => ambition_combat::targeting::assisted_fire_direction(
                        muzzle, commanded, assist, foes,
                    ),
                    None => commanded,
                }
            }
        };
        let spawn_origin = if uses_gun_sword {
            let hand = ambition_mount::rider_hand_world_pos_in_frame(
                kin.pos,
                kin.facing,
                kin.size.y,
                gravity_dir,
            );
            hand + world_dir * 18.0
        } else {
            origin + frame.to_world(ae::Vec2::new(0.0, -8.0))
        };
        let spawn = ProjectileSpawn {
            origin: spawn_origin,
            dir: world_dir,
            speed: spec.speed(),
            damage: spec.damage(),
            max_lifetime: flight.max_lifetime,
            half_extent: flight.half_extent,
            gravity: flight.gravity,
            visual_id,
            bounces: flight.bounces,
            bounce_on_world_contact: flight.bounce_on_world_contact,
            boomerang_return_s: flight.boomerang_return_s,
        };
        if uses_gun_sword {
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::SfxId::from_static("weapon.lasersword.fire"),
                pos: spawn.origin,
            });
        }
        projectiles.write(ProjectileSpawnRequest::open(
            msg.actor,
            spawn,
            ProjectileStart::StepThisTick,
        ));
        // Recoil: push the firing actor backward along the negative
        // fire direction.
        let recoil_strength = if uses_gun_sword {
            RANGED_RECOIL_PIRATE
        } else {
            RANGED_RECOIL_DEFAULT
        };
        let kick = world_dir * -recoil_strength;
        #[cfg(feature = "causal")]
        let before = kin.vel;
        kin.vel += kick;
        // The authorship fact: this site NAMES itself as the writer, with the
        // velocity either side of its own write. An explanation of the tick now
        // says who moved the body instead of only that it moved.
        #[cfg(feature = "causal")]
        if let (Some(log), Ok(identity)) = (log.as_mut(), identities.get(msg.actor)) {
            if log.is_recording() {
                log.record(ambition_causal::velocity_authored(
                    tick.as_deref().map_or(0, |t| t.get()),
                    ambition_causal::SubjectKey::Sim(identity.id.clone()),
                    "ranged_recoil",
                    before.x,
                    kin.vel.x,
                ));
            }
        }
    }
}

// Melee is a moveset move for EVERY body — there is no actor-specific (or player-specific)
// melee driver. A body's melee capability (`ActionSet.melee`) is folded into a `"attack"`-verb
// move at spawn (`build_actor_moveset`); the brain's `melee_pressed` edge starts it via
// `combat::moveset::trigger_moveset_moves` and `advance_move_playback` spawns the active-window
// strike.

/// Helper: combat-tuning lookup. Lives on the test side to make
/// the helper available to the unit tests below without leaking
/// `Platformer2dFeelTuningMonolith` through the public API.
#[cfg(test)]
fn default_combat_tuning() -> ambition_combat::events::FeatureCombatTuning {
    Platformer2dFeelTuningMonolith::default().feature_combat_tuning()
}

#[cfg(test)]
mod tests;
