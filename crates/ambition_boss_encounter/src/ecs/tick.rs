//! The per-frame boss tick: encounter-phase sync + brain ticking + the main
//! ECS boss update (`update_ecs_bosses`).

// Named imports, not a glob, so the dependencies are visible to grep.
use ambition_combat::components::{BossDeathAnimation, BossPhase};
use ambition_platformer2d_core as ae;
use ambition_time::WorldTime;
use bevy::prelude::{Query, Res, With, Without};

use ambition_characters::brain::{BossAttackIntent, BossAttackState, Brain, StateMachineCfg};
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use bevy::prelude::{Commands, Entity, Has};

/// Resolve a possessing controller's attack input into the boss's fire
/// intent: the controller→verb→move map.
///
/// A melee press reduces the controller's body-local aim to a discrete
/// [`AttackDir`](ambition_entity_catalog::AttackDir) (`attack_dir_from_axis`,
/// the same reduction the actor moveset trigger uses) and walks the shared
/// [`directional_verb_chain`](ambition_entity_catalog::directional_verb_chain)
/// (`attack_down` → `attack`; a boss is a free-mover, so there is no
/// grounded/air split) over the profile's authored `possessed_verbs`. The
/// special button resolves the `"special"` verb. The winning move key becomes
/// the intent profile via [`BossAttackProfile::from_move_id`], the same id
/// `limb_routing` keys on, so aboard a limb-rigged mount the verb reaches the
/// giant's hands.
///
/// A boss with no verbs uses the fixed mapping: melee → primary authored
/// strike (`slot(0)`), special → signature content special (else `slot(1)`).
/// Guarded by `possession_verb_map_tests`.
fn possessed_attack_choice(
    frame: &ambition_characters::actor::control::ActorControlFrame,
    behavior: &crate::pattern::profile::BossBehaviorProfile,
    capability: Option<&ambition_characters::brain::BossCapability>,
    facing: f32,
) -> Option<ambition_characters::brain::BossAttackProfile> {
    use ambition_characters::brain::BossAttackProfile;
    if frame.melee_pressed || frame.pogo_pressed {
        // A dedicated pogo press aims Down (mirrors `trigger_moveset_moves`);
        // a plain melee press resolves by the body-local aim axis.
        let dir = if frame.pogo_pressed && !frame.melee_pressed {
            ambition_entity_catalog::AttackDir::Down
        } else {
            ambition_combat::moveset::attack_dir_from_axis(frame.attack_axis, facing)
        };
        let authored = ambition_entity_catalog::directional_verb_chain(
            ambition_combat::moveset::ATTACK_VERB,
            dir,
            /* grounded: a boss floats — its verb map authors no air variants */
            true,
        )
        .into_iter()
        .find_map(|verb| possessed_verb_move(behavior, &verb));
        if let Some(move_key) = authored {
            return Some(BossAttackProfile::from_move_id(move_key));
        }
        return capability.and_then(|c| c.slot(0)).map(|(p, _)| p.clone());
    }
    if frame.special_pressed || frame.projectile_pressed {
        return possessed_special(behavior, capability);
    }
    None
}

/// The move key that the profile's `possessed_verbs` map gives to `verb`.
fn possessed_verb_move<'a>(
    behavior: &'a crate::pattern::profile::BossBehaviorProfile,
    verb: &str,
) -> Option<&'a String> {
    behavior
        .possessed_verbs
        .iter()
        .find(|(v, _)| v == verb)
        .map(|(_, move_key)| move_key)
}

/// What the Special button fires on a possessed boss: the `"special"` verb,
/// else the signature content special, else `slot(1)`.
fn possessed_special(
    behavior: &crate::pattern::profile::BossBehaviorProfile,
    capability: Option<&ambition_characters::brain::BossCapability>,
) -> Option<ambition_characters::brain::BossAttackProfile> {
    if let Some(move_key) = possessed_verb_move(behavior, "special") {
        return Some(ambition_characters::brain::BossAttackProfile::from_move_id(
            move_key,
        ));
    }
    capability
        .and_then(|c| c.signature_special().or_else(|| c.slot(1)))
        .map(|(p, _)| p.clone())
}

/// The Attack and Special actions a possessed boss owns, for its action scheme.
///
/// `possessed_attack_choice` reads the raw press in the boss tick, which runs
/// before the control gate. The boss's `ActionSet` and its profile-keyed moveset
/// declare no `attack` or `special` verb, so without these the scheme has no
/// Attack or Special slot: the prompt shows neither, and the gate treats the
/// press as a verb the body does not own.
///
/// Each action is a `Technique` gate, so the gate clears the raw press after the
/// boss tick has read it. A `Move` gate would keep the press alive for
/// `trigger_moveset_moves`, which would start a second move for one press. Each
/// id is the move that the neutral press resolves to (`attack` or slot 0, and
/// the Special resolution above), so the prompt names what the button does.
pub fn possessed_boss_techniques(
    behavior: &crate::pattern::profile::BossBehaviorProfile,
    capability: &ambition_characters::brain::BossCapability,
) -> Vec<ambition_entity_catalog::action_scheme::ActionSpec> {
    use ambition_entity_catalog::action_scheme::{ActionGate, ActionId, ActionSpec, ControlSlot};
    let attack = possessed_verb_move(behavior, ambition_combat::moveset::ATTACK_VERB)
        .cloned()
        .or_else(|| capability.slot(0).map(|(p, _)| p.move_id()));
    let special = possessed_special(behavior, Some(capability)).map(|p| p.move_id());
    [(ControlSlot::Attack, attack), (ControlSlot::Special, special)]
        .into_iter()
        .filter_map(|(slot, move_id)| {
            let move_id = move_id?;
            Some(ActionSpec {
                id: ActionId::new(&move_id),
                slot,
                display_name: None,
                visual: None,
                gate: ActionGate::Technique(move_id),
            })
        })
        .collect()
}

/// Start the moveset entry named by the boss's current attack intent.
///
/// Geometry and special strikes both use the shared moveset runtime: geometry
/// moves publish hit volumes, while sustained effect windows dispatch content
/// techniques. `Without<MovePlayback>` prevents retrigger during the authored
/// window. Possessed bosses choose from the same body-owned repertoire; emitted
/// attacks use the possessor's effective faction.
pub fn trigger_boss_attack_moves(
    mut commands: Commands,
    // A separate query, not a column in `bosses` (like
    // `trigger_moveset_moves`): `MoveOccurrence` is in no other query here, so
    // a read-only lookup cannot alias, and `bosses` is near the engine's tuple
    // width limit.
    occurrences: Query<&ambition_combat::moveset::MoveOccurrence>,
    mut bosses: Query<
        (
            Entity,
            &BossAttackIntent,
            &ambition_combat::moveset::ActorMoveset,
            (
                &ambition_platformer2d_core::BodyKinematics,
                Has<ambition_platformer2d_core::Unmirrored>,
            ),
            // Mutable, so an interrupted windup goes through the one teardown
            // path below and "cancel this move" has one meaning.
            Option<&mut ambition_combat::moveset::MovePlayback>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    use ambition_characters::brain::BossAttackProfile;
    use ambition_entity_catalog::WindowTag;
    let active_start = |spec: &ambition_entity_catalog::MoveSpec| -> f32 {
        spec.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .map(|w| w.start_s)
            .unwrap_or(0.0)
    };
    for (entity, attack_intent, moveset, (kin, unmirrored), playback) in &mut bosses {
        // This frame's intent, written by the boss pattern or possession
        // before the combat phase. A Telegraph step starts the move at its
        // windup (`t0 = 0`). A Strike or possession step with no telegraph
        // starts at the strike (`t0 = tel`), so possession hits instantly.
        let intent: Option<(&BossAttackProfile, bool)> = attack_intent
            .telegraph_profile
            .as_ref()
            .map(|p| (p, true))
            .or_else(|| attack_intent.active_profile.as_ref().map(|p| (p, false)));

        // An interrupted windup must not strike. A move already in its Active
        // window is committed and runs to completion.
        if let Some(mut pb) = playback {
            let move_profile = BossAttackProfile::from_move_id(&pb.spec.id);
            let in_windup = pb.t < active_start(&pb.spec);
            let intent_wants_this = intent.is_some_and(|(p, _)| *p == move_profile);
            if in_windup && !intent_wants_this {
                ambition_combat::moveset::cancel_move_playback(
                    &mut commands,
                    entity,
                    &mut pb,
                    ambition_combat::moveset::MoveEnd::Interrupted,
                );
            }
            continue;
        }

        let Some((profile, from_telegraph)) = intent else {
            continue;
        };
        // A possessed boss's geometry strike fires like any other: possession
        // grants the full kit, and the hitbox carries the possessor's
        // effective faction (stamped in `advance_move_playback`), so it hits
        // the boss's former allies, not the controlling player.
        if let Some(spec) = moveset.0.move_by_id(&profile.move_id()) {
            // Telegraph edge: `t0 = 0` plays the windup through the move, so
            // the projected telegraph read-model and a bound clip follow one
            // timeline. Strike or possession edge: `t0 = tel` starts at the
            // strike, so the hitbox is live the same frame.
            let t0 = if from_telegraph {
                0.0
            } else {
                active_start(spec)
            };
            // Mint the occurrence from the body's counter.
            // `MovePlayback::new_at` leaves `instance` at 0, so without this
            // every boss move would reuse occurrence 0.
            //
            // This does not go through `start_move`, which is the player
            // acceptance authority (action buffer, affordability, gesture,
            // recovery and meter rules). Only the body's counter is shared.
            let occurrence =
                ambition_combat::moveset::MoveOccurrence::next(occurrences.get(entity).ok());
            commands
                .entity(entity)
                .insert(ambition_combat::moveset::MoveOccurrence(occurrence));
            commands.entity(entity).insert(
                // The move's volumes mirror with the drawn body, so by its side.
                ambition_combat::moveset::MovePlayback::new_at(
                    spec.clone(),
                    ambition_platformer2d_core::mirror_side(kin.facing, unmirrored),
                    t0,
                )
                    .at_occurrence(occurrence),
            );
        }
    }
}

/// Project [`BossAttackState`] from the live boss [`MovePlayback`].
/// `BossAttackState` is the boss telegraph/strike read-model
/// (`telegraph_profile` / `active_profile` and their remaining/elapsed), and
/// this projection is its only writer. While a boss move plays, the
/// read-model is derived from the move (the shared move runtime is the
/// authority); with no move playing, it is cleared. The boss brain publishes
/// a `BossAttackIntent`, the trigger starts a move from it, and this projects
/// that move.
///
/// The move is the whole telegraph→strike timeline: clock `t` in `[0, tel)`
/// is the windup and `[tel, tel+strike)` the strike, so
/// `telegraph_elapsed == t` and `active_elapsed == t`. A resting boss (no
/// `MovePlayback`) or a boss with no `ActorMoveset` has no move, so the state
/// is cleared. Runs after `advance_move_playback` so `t` is current, and
/// before the hurtbox/damage consumers (`apply_feature_hit_events`) so they
/// read this frame's value.
pub fn project_boss_attack_state_from_move(
    mut bosses: Query<
        (
            Option<&ambition_combat::moveset::MovePlayback>,
            &mut BossAttackState,
        ),
        With<FeatureSimEntity>,
    >,
) {
    use ambition_characters::brain::BossAttackProfile;
    for (playback, mut attack_state) in &mut bosses {
        let Some(playback) = playback else {
            attack_state.clear();
            continue;
        };
        let t = playback.t;
        let Some(active) = playback
            .spec
            .windows
            .iter()
            .find(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
        else {
            // A move with no Active window projects no strike state.
            attack_state.clear();
            continue;
        };
        let profile = BossAttackProfile::from_move_id(&playback.spec.id);
        if t < active.start_s {
            // Windup: the move is playing its telegraph (no hitbox yet).
            attack_state.telegraph_profile = Some(profile);
            attack_state.telegraph_remaining = (active.start_s - t).max(0.0);
            attack_state.telegraph_elapsed = t;
            attack_state.active_profile = None;
            attack_state.active_remaining = 0.0;
            attack_state.active_elapsed = 0.0;
        } else if t < active.end_s {
            // Strike: the hitbox is live; active_elapsed folds in the telegraph.
            attack_state.telegraph_profile = None;
            attack_state.telegraph_remaining = 0.0;
            attack_state.telegraph_elapsed = 0.0;
            attack_state.active_profile = Some(profile);
            attack_state.active_remaining = (active.end_s - t).max(0.0);
            attack_state.active_elapsed = t;
        } else {
            // Spent tail (t >= end; the move is about to be removed): no live strike.
            attack_state.clear();
        }
    }
}

/// Drive each boss's animation frame and publish the per-frame
/// [`crate::attack_geometry::BossAnimationFrameSample`] that boss geometry
/// reads. The simulation owns the cursor: it picks the anim from the
/// projected `BossAttackState`, advances the frame, and writes the sample. The
/// renderer mirrors that cursor into its draw-only
/// [`BossAnimator`](crate::sprites::BossAnimator).
pub fn drive_boss_animators(
    mut commands: Commands,
    boss_catalog: Res<crate::BossCatalog>,
    world_time: Res<WorldTime>,
    ecs_bosses: Query<(
        Entity,
        &ambition_combat::components::FeatureId,
        crate::BossClusterRef,
        &ambition_characters::actor::BodyHealth,
        &BossAttackState,
        &Brain,
    )>,
    mut frames: Query<(
        Entity,
        &ambition_combat::components::FeatureId,
        &mut crate::sprites::BossAnimFrame,
        Option<&ambition_time::ProperTimeScale>,
    )>,
) {
    for (entity, feature_id, mut frame, scale) in &mut frames {
        let dt = world_time.entity_dt(ambition_time::ProperTimeScale::or_default(scale));
        // Both helpers belong to `crate::anim`; call them there.
        let Some((_, state)) =
            crate::anim::ecs_boss_anim_state_and_entity(feature_id.as_str(), &ecs_bosses)
        else {
            continue;
        };
        let anim = crate::sprites::pick_boss_anim(state);
        frame.request_for_phase(anim, state.drive_phase());
        frame.tick(dt);
        match crate::anim::ecs_boss_animation_frame_sample(
            &boss_catalog,
            feature_id.as_str(),
            &ecs_bosses,
            anim,
            frame.frame,
        ) {
            Some((sample_entity, sample)) => {
                commands.entity(sample_entity).insert(sample);
            }
            None => {
                commands
                    .entity(entity)
                    .remove::<crate::attack_geometry::BossAnimationFrameSample>();
            }
        }
    }
}

/// Tick every boss's `BossPattern` brain: advance the cursor, emit
/// `ActorControlFrame` intent (movement and melee/special edges), and publish
/// the per-frame attack intent (`BossAttackIntent`) the moveset trigger reads.
/// This tick does not write `BossAttackState`; that read-model is projected
/// only from the live `MovePlayback` by `project_boss_attack_state_from_move`.
///
/// The autonomous arm builds `BossPatternContext` directly from its selected
/// target. Player-possessed bosses use the generic player-brain snapshot,
/// because controller input is the point of that path.
pub fn tick_boss_brains_system(
    world_time: Res<WorldTime>,
    // The composed collision read-API rather than its three ingredients.
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    // A possessed boss carries `DrivingParticipant(slot)` and reads its
    // controller frame from here, through the same control path as every
    // driven body. Which bosses are possessable is decided by the possession
    // target filter, not in this tick.
    slot_controls: Res<ambition_characters::control::SlotControls>,
    mut bosses: Query<
        (
            bevy::ecs::entity::Entity,
            crate::BossClusterRef,
            // The boss's HP authority (§A1) — liveness is `health.alive()`.
            &ambition_characters::actor::BodyHealth,
            &mut Brain,
            // Possession keys on driver authority; the boss brain stays
            // attached and stops deciding while a participant drives.
            Option<&ambition_characters::control::DrivingParticipant>,
            &mut ActorControl,
            // The per-frame attack intent: the driver (pattern or possession)
            // writes the profile it wants to fire; `trigger_boss_attack_moves`
            // reads it.
            &mut ambition_characters::brain::BossAttackIntent,
            &ambition_combat::components::ActorTarget,
            // The boss's authored special repertoire (body capability, kept
            // across a brain swap). Read only by the possession arm. `Option`
            // for test fixtures that spawn a boss without it.
            Option<&ambition_characters::brain::BossCapability>,
            // The projected live-move read-model (last frame's; the
            // projection runs after this tick). The pattern observes its own
            // playing move through it: cycle mode sustains its request through
            // the windup and rests when the move ends. Read-only.
            Option<&BossAttackState>,
            // A rider's mount takes its facing (`ambition_mount`), so a boss
            // riding a mount that mirrors turns by the mount's width.
            Option<&ambition_mount::RidingOn>,
        ),
        With<FeatureSimEntity>,
    >,
    // Any body a boss may aim at or ride: its collision extent, read-only. Not
    // `CenteredAabb`, which for a boss comes from its `BodyEnvelope` render
    // envelope. `BodyKinematics::size` is the box the movement seam sweeps;
    // `integrate_boss_bodies` sets `kin.size` to the authored `combat_size`
    // every tick, the extent `BossPatternCfg::combat_size` gives the boss side.
    //
    // With whether it mirrors: a mount that does not cannot be spun by its rider.
    target_bodies: Query<(
        &ambition_platformer2d_core::BodyKinematics,
        Has<ambition_platformer2d_core::Unmirrored>,
    )>,
) {
    let dt = world_time.sim_dt();
    let Some(feature_world) = collision.solids() else {
        return;
    };
    for (
        _entity,
        feature,
        health,
        mut brain,
        driver,
        mut control,
        mut intent,
        target,
        capability,
        attack_state,
        riding,
    ) in &mut bosses
    {
        let boss = feature.as_boss_ref();
        if !health.alive() {
            // Dead boss: clear the control frame and fire intent so the trigger
            // starts nothing; the projection clears `BossAttackState`.
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            intent.clear();
            continue;
        }

        // Possessed boss: driven from slot input through the player brain,
        // like every controlled body. It steers by `velocity_target` at the
        // shared body run capability and commands its own authored specials
        // through an input→special mapping over `BossCapability`.
        //
        // The scripted pattern is not asked. Its `Brain` stays on the body
        // unchanged, and when the seat leaves, the pattern resumes from the
        // same state.
        if let Some(slot) = driver.map(|driver| driver.0) {
            let mut snapshot = ambition_characters::brain::BrainSnapshot::idle();
            snapshot.actor_pos = boss.kin.pos;
            snapshot.actor_vel = boss.kin.vel;
            snapshot.actor_facing = boss.kin.facing;
            snapshot.actor_aerial = true;
            snapshot.max_run_speed = ae::MAX_RUN_SPEED;
            snapshot.dt = dt;
            snapshot.player_input = Some(slot_controls.get(slot));
            let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
            ambition_characters::brain::tick_player_brain(slot, &snapshot, &mut frame);
            control.0 = frame;

            // Map controller input onto the boss's repertoire and publish it
            // as this frame's fire intent. `trigger_boss_attack_moves` starts
            // the matching move; a live `MovePlayback` blocks re-trigger, so
            // the move's duration is the fire-rate gate. A possessed strike is
            // a real strike (the hitbox carries the possessor's effective
            // faction). When this boss rides a limb-rigged mount, the
            // projected `BossAttackState` drives `route_boss_strikes_to_limbs`
            // as for the pattern: down+attack aboard the giant slams both
            // hands.
            //
            // The mapping is `possessed_attack_choice`.
            intent.clear();
            if let Some(profile) =
                possessed_attack_choice(&frame, &boss.config.behavior, capability, boss.kin.facing)
            {
                intent.active_profile = Some(profile);
            }
            continue;
        }

        // Non-BossPattern brains on a boss (test fixtures) emit no fire
        // intent. The projection clears their `BossAttackState`.
        if !matches!(
            &*brain,
            Brain::StateMachine(StateMachineCfg::BossPattern { .. })
        ) {
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            intent.clear();
            continue;
        }

        // Boss perception is omniscient (`Perception::default()`): a boss
        // knows where its foe is anywhere in its arena, so it reads the global
        // `ActorTarget` that `select_actor_targets` maintains. This is a
        // documented policy: a boss carries no `Perception` component. A boss
        // with bounded senses would carry `Perception::Sighted` and branch
        // here as `tick_actor_brains` does; none do today.
        let target_pos = target.pos;
        // The target's body, not only its position: a contact chase asks
        // whether two bodies touch (see `lateral_body_gap`). Every body has
        // `BodyKinematics`. A target without a body reads as a point.
        let target_body_size = target
            .entity
            .and_then(|entity| target_bodies.get(entity).ok())
            .map_or(ae::Vec2::ZERO, |(kin, _)| kin.size);

        // The front-wall standoff the pattern probes with — read before the brain
        // borrow that `brain.tick` needs.
        let front_wall_standoff = match &*brain {
            Brain::StateMachine(StateMachineCfg::BossPattern { cfg, .. }) => {
                cfg.macro_tuning.front_wall_standoff
            }
            _ => 0.0,
        };
        let front_wall_clearance =
            boss_front_wall_clearance(&feature_world, &boss, target_pos, front_wall_standoff);

        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        let attack_request = match &mut *brain {
            Brain::StateMachine(StateMachineCfg::BossPattern { cfg, state }) => {
                let ctx = ambition_characters::brain::BossPatternContext {
                    encounter_phase: boss.status.encounter_phase(),
                    actor_pos: boss.kin.pos,
                    target_pos,
                    target_body_size,
                    world_size: feature_world.size,
                    front_wall_clearance,
                    dt,
                    // Situation buckets and `OnHitTaken`. The brain remembers
                    // its own last HP, so a hit is a drop in this pool; there
                    // is no per-tick damage channel.
                    actor_facing: boss.kin.facing,
                    // The body that turns: its own box, or its mount's when the
                    // turn MIRRORS the mount. A scholar that turned by his own
                    // 68 px spun a mirroring 440 px mount under him whenever
                    // the player crossed his centre line. An `Unmirrored` mount
                    // (the gnu) shows no turn, so he turns by his own box and
                    // watches a player standing under him.
                    actor_half_width: riding
                        .and_then(|riding| target_bodies.get(riding.mount).ok())
                        .filter(|(_, unmirrored)| !unmirrored)
                        .map_or(boss.kin.size.x, |(mount, _)| mount.size.x.max(boss.kin.size.x))
                        * 0.5,
                    hp_current: health.current(),
                    hp_max: health.max(),
                    // The brain's observation of its own live move, from the
                    // projected read-model (one frame stale): telegraphing is
                    // `striking: false`, striking is `striking: true`, no move
                    // is `None`.
                    live_attack: attack_state.and_then(|s| {
                        if let Some(profile) = &s.active_profile {
                            Some(ambition_characters::brain::LiveBossAttack {
                                profile: profile.clone(),
                                striking: true,
                            })
                        } else {
                            s.telegraph_profile.as_ref().map(|profile| {
                                ambition_characters::brain::LiveBossAttack {
                                    profile: profile.clone(),
                                    striking: false,
                                }
                            })
                        }
                    }),
                };
                let mut attack_intent = core::mem::take(&mut state.attack_intent);
                crate::pattern::tick_boss_pattern(cfg, state, &ctx, &mut frame, &mut attack_intent);
                state.attack_intent = attack_intent;
                &state.attack_intent
            }
            _ => unreachable!("non-BossPattern brains returned above"),
        };
        control.0 = frame;
        // Publish the brain's transient profile request. This component is the
        // move trigger's input; `BossAttackState` remains a separate read-model
        // projected solely from the move that this request starts.
        intent.clone_from(attack_request);

        // Geometry strikes and content-technique specials share this path:
        // the profile request starts one authored move, whose active windows
        // own hit volumes or sustained `Effect{key}` emission.
    }
}

pub(crate) fn boss_front_wall_clearance(
    world: &ae::World,
    boss: &crate::BossRef<'_>,
    target_pos: ae::Vec2,
    standoff: f32,
) -> Option<f32> {
    if standoff <= 0.0 {
        return None;
    }
    let dx = target_pos.x - boss.kin.pos.x;
    if dx.abs() <= 1.0 {
        return None;
    }
    let dir_x = dx.signum();
    let probe_distance = dx.abs().max(standoff + 1.0).min(1_024.0);
    let body = boss.aabb();
    horizontal_front_wall_clearance(world, body, dir_x, probe_distance)
}

pub(crate) fn horizontal_front_wall_clearance(
    world: &ae::World,
    body: ae::Aabb,
    dir_x: f32,
    probe_distance: f32,
) -> Option<f32> {
    if dir_x.abs() <= f32::EPSILON || probe_distance <= 0.0 {
        return None;
    }
    let dir_x = dir_x.signum();
    // Probe the vertical lane the boss body would actually sweep through.
    // Use only a small skin instead of a large percentage inset: low side
    // walls should still stop the behemoth, but a floor tile that merely
    // touches the boss's feet (or overlaps by a pixel due to integration
    // tolerance) must not be misclassified as a front wall.
    let vertical_skin = 4.0_f32.min(body.height() * 0.10);
    let lane_top = body.top() + vertical_skin;
    let lane_bottom = body.bottom() - vertical_skin;
    let (lane_top, lane_bottom) = if lane_top < lane_bottom {
        (lane_top, lane_bottom)
    } else {
        let center_y = body.center().y;
        (
            center_y - body.height() * 0.25,
            center_y + body.height() * 0.25,
        )
    };

    let mut best: Option<f32> = None;
    for block in &world.blocks {
        if !matches!(
            block.kind,
            ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
        ) {
            continue;
        }
        let vertical_overlap =
            lane_bottom.min(block.aabb.bottom()) - lane_top.max(block.aabb.top());
        if vertical_overlap <= 1.0 {
            continue;
        }
        let clearance = if dir_x > 0.0 {
            block.aabb.left() - body.right()
        } else {
            body.left() - block.aabb.right()
        };
        if clearance < -1.0 || clearance > probe_distance {
            continue;
        }
        let clearance = clearance.max(0.0);
        if best.is_none_or(|b| clearance < b) {
            best = Some(clearance);
        }
    }
    best
}

/// Boss presentation: decay the boss's body-generic reaction timers and sync
/// the sprite-animation facts (`BossPhase`, death anim).
///
/// This system moves no body and deals no damage. Movement is
/// [`integrate_boss_bodies`]; strike damage is the moveset's own hitboxes
/// (`trigger_boss_attack_moves` → `advance_move_playback` →
/// `apply_hitbox_damage`); body-contact damage is the shared
/// `apply_actor_contact_damage`. A boss uses the same systems as every actor.
pub fn update_ecs_bosses(
    world_time: Res<WorldTime>,
    mut bosses: Query<
        (
            &ambition_characters::actor::BodyHealth,
            &mut ambition_characters::actor::BodyCombat,
            &mut BossDeathAnimation,
            &mut BossPhase,
        ),
        // The player carries `BodyKinematics`; exclude it so this query is
        // disjoint (boss and player are mutually exclusive archetypes).
        (
            With<FeatureSimEntity>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
        ),
    >,
) {
    // Sim clock: bosses must slow with bullet-time (ADR 0010).
    let dt = world_time.sim_dt();
    for (health, mut boss_combat, mut death_anim, mut phase) in &mut bosses {
        let alive = health.alive();
        // Body-generic reaction timers (hit flash, i-frames, stagger) decay
        // through the same `BodyCombat` method the actor tick uses. The boss
        // is excluded from the actor tick, so it calls it here.
        boss_combat.decay_reaction_timers(dt);
        if alive {
            death_anim.clear();
        } else if phase.is_active() && death_anim.remaining_s <= 0.0 {
            death_anim.start();
        } else {
            death_anim.tick(dt);
        }
        *phase = BossPhase::from_alive(alive);
    }
}

#[cfg(test)]
mod attack_moveset_tests;
#[cfg(test)]
mod possession_verb_map_tests;
