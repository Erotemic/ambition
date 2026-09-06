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
/// WHERE A SHOT IS BORN, for every muzzle a ranged action can name.
///
/// ⭐ EXTRACTED SO IT CAN BE ASKED. This was inline in the fire system, which
/// meant the only way to check that an authored muzzle actually moved the spawn
/// point was to stand up an app, a body, a weapon and a fired shot — so nothing
/// checked it, and `Muzzle::Offset` could have resolved to the body origin
/// forever without a test noticing.
///
/// `origin` is the body's spawn origin; `body_pos` and `facing` come from the
/// same kinematics, and `height` scales the normalized offsets.
pub fn muzzle_world_pos(
    muzzle: ambition_characters::brain::action_set::Muzzle,
    origin: ae::Vec2,
    body_pos: ae::Vec2,
    facing: f32,
    height: f32,
    world_dir: ae::Vec2,
    gravity_dir: ae::Vec2,
    frame: ae::AccelerationFrame,
) -> ae::Vec2 {
    match muzzle {
        // A drawn weapon fires from the hand whether the pirate is still
        // mounted or has fallen off the shark.
        ambition_characters::brain::action_set::Muzzle::Hand { ahead } => {
            let hand =
                ambition_mount::rider_hand_world_pos_in_frame(body_pos, facing, height, gravity_dir);
            hand + world_dir * ahead
        }
        ambition_characters::brain::action_set::Muzzle::BodyOrigin => {
            origin + frame.to_world(ae::Vec2::new(0.0, -8.0))
        }
        // ⭐ THE ACTION'S OWN MUZZLE, resolved exactly the way the hand is:
        // scaled by body height so one authored value fits every body and every
        // sprite tier, flipped by facing, then taken through the acceleration
        // frame so sideways gravity moves the muzzle with the fighter rather
        // than leaving it pointing at the screen's up.
        ambition_characters::brain::action_set::Muzzle::Offset { x, y } => {
            let facing_sign = if facing >= 0.0 { 1.0 } else { -1.0 };
            origin + frame.to_world(ae::Vec2::new(x * height * facing_sign, y * height))
        }
    }
}

pub fn spawn_projectiles_from_brain_actions(
    mut messages: MessageReader<ActorActionMessage>,
    mut projectiles: MessageWriter<ProjectileSpawnRequest>,
    mut sfx: SfxWriter,
    mut actors: Query<(
        &mut ae::BodyKinematics,
        &mut ambition_combat::BodyMelee,
        Option<&super::ActorSurfaceState>,
        Option<&ambition_combat::actor_tuning::ActorConfig>,
        Option<&ambition_characters::actor::BodyHealth>,
    )>,
    // Disjoint from `actors` — `ActorClusterQueryData` carries no `BodyAnimFacts`,
    // so this second view borrows the firing body's overlay-pose facts without
    // aliasing. Arms the Shoot pose on the frame the body accepts a shot.
    mut anim_facts: Query<&mut ambition_characters::actor::BodyAnimFacts>,
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
        // ⛔⛔ THE WORLD'S HANDS ARE OFF IT, AND HEALTH DOES NOT SAY SO. D201's
        // stock loss calls `health.reset()` the instant the stock is spent, so a
        // fighter waiting out its death beat reads FULL HEALTH while lying
        // untouchable at the blast line. This scan asked `health.alive()` and
        // therefore bent an assisted shot toward a body nothing can hit — the
        // same defect `select_actor_targets` already carries the lesson for, one
        // authority over.
        Has<ambition_combat::death_rules::OutOfPlay>,
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
        // ⭐⭐ HOW THIS SHOT LEAVES THE WEAPON, ASKED OF THE WEAPON. These four
        // choices — the look, the muzzle, the cue, the kick — were decided here
        // by `held_item_id == Some("gun_sword")`, so the Pirate Admiral's side-B
        // got none of them despite drawing a weapon whose own row says *"same
        // art, same discharge, same hand"*. See
        // `ambition_characters::brain::action_set::Discharge`; this site no
        // longer knows any weapon's name.
        let discharge = spec.discharge.clone().unwrap_or_default();
        // The projectile's APPEARANCE is an OPEN visual id the render layer
        // resolves through the content catalog — never the owner-id string.
        // Precedence: the ACTION's own authored visual (a drawn weapon and an
        // equipment-granted verb both bring their look with them), else the
        // archetype's default ranged look.
        let visual_id = if let Some(authored) = spec.visual.clone() {
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
            splash_half_extent: 0.0,
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
                        .filter(
                            move |(
                                candidate,
                                _,
                                candidate_faction,
                                health,
                                candidate_team,
                                candidate_driver,
                                out_of_play,
                                _,
                            )| {
                                *candidate != msg.actor
                                // The shared gate, not a second health reading.
                                && !ambition_combat::util::body_is_untouchable(
                                    Some(health),
                                    *out_of_play,
                                )
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
                            },
                        )
                        .map(|(candidate, aabb, _, _, _, _, _, sim_id)| {
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
        let spawn_origin = muzzle_world_pos(
            discharge.muzzle,
            origin,
            kin.pos,
            kin.facing,
            kin.size.y,
            world_dir,
            gravity_dir,
            frame,
        );
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
            splash_half_extent: flight.splash_half_extent,
            boomerang_return_s: flight.boomerang_return_s,
        };
        if let Some(cue) = discharge.fire_sfx.as_deref() {
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::SfxId::new(cue),
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
        let kick = world_dir * -discharge.recoil;
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

#[cfg(test)]
mod muzzle_tests {
    use super::muzzle_world_pos;
    use ambition_characters::brain::action_set::Muzzle;
    use ambition_platformer2d_core as ae;

    const HEIGHT: f32 = 48.0;
    const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

    fn at(muzzle: Muzzle, facing: f32) -> ae::Vec2 {
        muzzle_world_pos(
            muzzle,
            ae::Vec2::ZERO,
            ae::Vec2::ZERO,
            facing,
            HEIGHT,
            ae::Vec2::new(1.0, 0.0),
            DOWN,
            ae::AccelerationFrame::new(DOWN),
        )
    }

    /// An authored muzzle actually MOVES the spawn point.
    ///
    /// ⛔ THE FAILURE THIS EXISTS FOR IS SILENT. A `Muzzle::Offset` arm that
    /// fell through to the body origin would spawn every shot eight pixels above
    /// the midriff — exactly what the fighter did before the variant existed —
    /// and the only symptom is art that looks slightly wrong to somebody who
    /// never saw it right.
    #[test]
    fn an_authored_muzzle_is_not_the_body_origin() {
        let authored = at(Muzzle::Offset { x: 0.22, y: -0.34 }, 1.0);
        let default = at(Muzzle::BodyOrigin, 1.0);
        assert!(
            authored.distance(default) > 1.0,
            "an authored muzzle resolved to the body origin ({authored:?} vs \
             {default:?}), so the action stated where its cannon is and the \
             engine fired from the stomach anyway"
        );
        assert!(
            authored.y < default.y,
            "the cannon did not resolve ABOVE the default muzzle: {authored:?} \
             against {default:?} — up is negative"
        );
    }

    /// The offset is scaled by the BODY, not pasted in pixels.
    ///
    /// ⭐ THE REASON THE FIELD IS NORMALIZED. Two fighters sharing one action
    /// must put the muzzle at the same place on their own silhouettes; a pixel
    /// offset would put a small fighter's cannon outside its own body.
    #[test]
    fn the_offset_scales_with_the_body() {
        let muzzle = Muzzle::Offset { x: 0.22, y: -0.34 };
        let small = muzzle_world_pos(
            muzzle,
            ae::Vec2::ZERO,
            ae::Vec2::ZERO,
            1.0,
            HEIGHT / 2.0,
            ae::Vec2::new(1.0, 0.0),
            DOWN,
            ae::AccelerationFrame::new(DOWN),
        );
        let big = at(muzzle, 1.0);
        assert!(
            small.length() < big.length(),
            "halving the body height did not move the muzzle in ({small:?} vs \
             {big:?}), so the offset is being read as pixels"
        );
    }

    /// Facing flips the muzzle to the side the fighter is looking.
    ///
    /// ⛔ A CANNON THAT STAYS ON ONE SIDE fires out of the back of the head when
    /// the fighter turns, and the shot is born behind its own barrel.
    #[test]
    fn the_muzzle_follows_the_facing() {
        let right = at(Muzzle::Offset { x: 0.22, y: -0.34 }, 1.0);
        let left = at(Muzzle::Offset { x: 0.22, y: -0.34 }, -1.0);
        assert!(
            right.x > 0.0 && left.x < 0.0,
            "the muzzle did not flip with facing: right={right:?} left={left:?}"
        );
        assert_eq!(
            right.y, left.y,
            "turning around moved the cannon vertically, so facing is being \
             applied to the wrong axis"
        );
    }
}
