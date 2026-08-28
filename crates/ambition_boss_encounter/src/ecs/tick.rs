//! The per-frame boss tick: encounter-phase sync + brain ticking + the main
//! ECS boss update (`update_ecs_bosses`).

// ⭐ NAMED, NOT GLOBBED. This was `use super::super::*`, a glob over the
// whole `features/ecs` module — a channel a `crate::` grep cannot see, and
// the reason a carve estimate needs more than an import count. Measured by
// deleting it: everything it actually supplied was bevy's prelude and
// `WorldTime`, and NO monolith vocabulary at all.
use ambition_combat::components::{BossDeathAnimation, BossPatternTimer, BossPhase};
use ambition_platformer2d_core as ae;
use ambition_time::WorldTime;
use bevy::prelude::{Query, Res, With, Without};

use ambition_characters::brain::{BossAttackIntent, BossAttackState, Brain, StateMachineCfg};
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use bevy::prelude::{Commands, Entity};

/// G5 (R10.6): resolve a POSSESSING controller's attack input into the boss's
/// fire intent — the controller→verb→move map.
///
/// A melee press reduces the controller's body-local aim to a discrete
/// [`AttackDir`](ambition_entity_catalog::AttackDir) (`attack_dir_from_axis`,
/// the SAME reduction the actor moveset trigger uses) and walks the shared
/// [`directional_verb_chain`](ambition_entity_catalog::directional_verb_chain)
/// (`attack_down` → `attack`; a boss is a free-mover, so there is no
/// grounded/air split in its chain) over the profile's authored
/// `possessed_verbs`; the special/projectile button resolves the `"special"`
/// verb. The winning move key becomes the intent profile via
/// [`BossAttackProfile::from_move_id`] — the same id `limb_routing` keys on, so
/// aboard a limb-rigged mount the verb lands on the giant's hands with no extra
/// plumbing.
///
/// A boss authoring NO verbs keeps the legacy deterministic mapping —
/// melee → primary authored strike (`slot(0)`), special → signature content
/// special (falling back to `slot(1)`) — byte-identical to the pre-G5 arm
/// (pinned by `possession_verb_map_tests`).
fn possessed_attack_choice(
    frame: &ambition_characters::actor::control::ActorControlFrame,
    behavior: &crate::pattern::profile::BossBehaviorProfile,
    capability: Option<&ambition_characters::brain::BossCapability>,
    facing: f32,
) -> Option<ambition_characters::brain::BossAttackProfile> {
    use ambition_characters::brain::BossAttackProfile;
    let verb_move = |verb: &str| -> Option<&String> {
        behavior
            .possessed_verbs
            .iter()
            .find(|(v, _)| v == verb)
            .map(|(_, move_key)| move_key)
    };
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
        .find_map(|verb| verb_move(&verb));
        if let Some(move_key) = authored {
            return Some(BossAttackProfile::from_move_id(move_key));
        }
        return capability.and_then(|c| c.slot(0)).map(|(p, _)| p.clone());
    }
    if frame.special_pressed || frame.projectile_pressed {
        if let Some(move_key) = verb_move("special") {
            return Some(BossAttackProfile::from_move_id(move_key));
        }
        return capability
            .and_then(|c| c.signature_special().or_else(|| c.slot(1)))
            .map(|(p, _)| p.clone());
    }
    None
}

/// Sync each boss's `encounter_phase` mirror from the entity-local
/// [`ActorPhaseState`] copy (`BossEncounter.encounter`). The mirror is a convenience
/// field the brain (`BossPatternContext`) reads; the `BossEncounter.encounter`
/// phase machine — ticked by `update_boss_encounters` — is the source of truth.
/// Keyed per-entity by construction, so two of the same archetype sync
/// independent phases.
///
/// Runs before [`tick_boss_brains_system`] so the brain sees this frame's phase.
pub fn sync_boss_encounter_phase(
    mut bosses: Query<crate::BossClusterQueryData, With<FeatureSimEntity>>,
    mut last_logged: bevy::ecs::system::Local<
        std::collections::HashMap<String, crate::BossEncounterPhase>,
    >,
) {
    for mut feature in &mut bosses {
        let boss_id = feature.config.id.clone();
        let behavior_id = feature.config.behavior.id.clone();
        // Phase comes from the entity-local copy, keyed per-entity by
        // construction, so two of the same archetype sync independent phases.
        let new_phase = feature.status.encounter.as_ref().map(|p| p.phase);
        // Log phase transitions per boss so we can see in the logs
        // when (or if) Dormant → Intro → Phase1 actually fires.
        let prev = last_logged.get(&boss_id).copied();
        if new_phase != prev {
            match new_phase {
                Some(phase) => {
                    bevy::log::info!(
                        target: "ambition_platformer2d::boss_encounter",
                        "sync_phase: boss={} (behavior.id={}) phase {:?} → {:?}",
                        boss_id,
                        behavior_id,
                        prev,
                        phase,
                    );
                    ambition_platformer2d_shared_tangle::world_log::world_event(format_args!(
                        "boss-phase {boss_id} {prev:?} -> {phase:?}"
                    ));
                    last_logged.insert(boss_id.clone(), phase);
                }
                None => {
                    bevy::log::warn!(
                        target: "ambition_platformer2d::boss_encounter",
                        "sync_phase: boss={} behavior.id={} has no entity-local encounter state (boss.encounter_phase stays {:?})",
                        boss_id,
                        behavior_id,
                        feature.status.encounter_phase,
                    );
                    last_logged.insert(boss_id.clone(), feature.status.encounter_phase);
                }
            }
        }
        if let Some(phase) = new_phase {
            feature.status.encounter_phase = phase;
        }
    }
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
    mut bosses: Query<
        (
            Entity,
            &BossAttackIntent,
            &ambition_combat::moveset::ActorMoveset,
            &ambition_platformer2d_core::BodyKinematics,
            // MUTABLE so an interrupted windup can go through the one teardown
            // path below. A windup carries no strike boxes yet, so nothing is
            // leaking today -- but "cancel this move" having one meaning is what
            // keeps the next interrupt from being the one that does.
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
    for (entity, attack_intent, moveset, kin, playback) in &mut bosses {
        // The driver's per-tick INTENT this frame (§A1 split — written by the boss
        // pattern OR possession before the combat phase): a Telegraph step wants the
        // move played from its windup (`t0 = 0`), a Strike/possession step with no
        // telegraph wants it started at the strike (`t0 = tel`, skipping the windup —
        // preserving possession's instant hit).
        let intent: Option<(&BossAttackProfile, bool)> = attack_intent
            .telegraph_profile
            .as_ref()
            .map(|p| (p, true))
            .or_else(|| attack_intent.active_profile.as_ref().map(|p| (p, false)));

        // This is the telegraph-edge trigger's parity with the old strike-edge behavior: an
        // interrupted windup must NOT strike. A move already in its Active window is committed
        // (the Smash convention) and runs to completion.
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
        // A possessed boss's GEOMETRY strike fires like any other (R1.4): possession
        // grants the full kit (invariant I2), and its strike hitbox carries the
        // possessor's EFFECTIVE faction (stamped in `advance_move_playback`), so it
        // hits the boss's former allies, not the controlling player. (This retires
        // the §A1-slice-1b suppression that kept parity with the deleted
        // `sync_boss_strike_hitboxes`, which never struck for a controlled boss.)
        if let Some(spec) = moveset.0.move_by_id(&profile.move_id()) {
            // Telegraph edge → `t0 = 0` plays the windup THROUGH the move (so the
            // projected telegraph read-model + a future bound anim clip slave to the
            // one move timeline). Strike/possession edge → `t0 = tel` starts at the
            // strike, so the hitbox is live the same frame as the pre-Slice-D move.
            let t0 = if from_telegraph {
                0.0
            } else {
                active_start(spec)
            };
            commands
                .entity(entity)
                .insert(ambition_combat::moveset::MovePlayback::new_at(
                    spec.clone(),
                    kin.facing,
                    t0,
                ));
        }
    }
}

/// PROJECT [`BossAttackState`] from the live boss [`MovePlayback`] (E53, §A1 slice 1b).
/// `BossAttackState` is the boss telegraph/strike READ-MODEL — `telegraph_profile` /
/// `active_profile` + their remaining/elapsed — and this projection is now its SOLE
/// writer: while a boss move plays, the read-model is DERIVED from the move (the shared
/// move runtime is the authority, mirroring `project_moveset_melee_to_body_melee`); with
/// NO move playing it is CLEARED. The boss brain no longer writes the component — it
/// publishes a `BossAttackIntent` the trigger consumes, and the move the trigger starts
/// is what this projects.
///
/// The move IS the whole telegraph→strike timeline: its clock `t` in `[0, tel)` is the
/// windup, `[tel, tel+strike)` the strike, so `telegraph_elapsed == t` and
/// `active_elapsed == t` (the latter folds in the telegraph offset the same way the
/// brain's mirror did). A resting boss (no `MovePlayback`), a boss with no `ActorMoveset`
/// (test fixtures / no authored strikes), and a possessed boss whose GEOMETRY strike the
/// trigger suppressed all have no move → cleared (the possessed-geometry pose loss is the
/// §A1 slice 1b BLIND change). Runs AFTER `advance_move_playback` so `t` is current, and
/// BEFORE the hurtbox/damage consumers (`apply_feature_hit_events`) so they read this
/// frame's value.
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
            // WINDUP: the move is playing its telegraph (no hitbox yet).
            attack_state.telegraph_profile = Some(profile);
            attack_state.telegraph_remaining = (active.start_s - t).max(0.0);
            attack_state.telegraph_elapsed = t;
            attack_state.active_profile = None;
            attack_state.active_remaining = 0.0;
            attack_state.active_elapsed = 0.0;
        } else if t < active.end_s {
            // STRIKE: the hitbox is live; active_elapsed folds in the telegraph.
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

/// PHASE (presentation, SIM-side) — drive each boss's animation frame and publish the per-frame
/// [`crate::attack_geometry::BossAnimationFrameSample`] the boss GEOMETRY reads. This retires the
/// render→sim WRITE-BACK: render no longer owns or writes the frame. Now the SIM owns the
/// cursor: it picks the anim from the projected `BossAttackState`, advances the frame, and
/// writes the sample; the renderer mirrors that cursor into its draw-only
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
        &ambition_characters::actor::BodyCombat,
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
        // ⭐ NAMED WHERE IT LIVES. Both of these are
        // `crate::anim`'s; reaching them through
        // `crate::features` was two hops of the monolith's own facade
        // republishing a peer domain, which is the shape that makes a module
        // look coupled to the monolith when it is not.
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
/// `ActorControlFrame` intent (movement + melee/special edges), and publish the
/// per-frame attack INTENT (`BossAttackIntent`) the moveset trigger reads. Since §A1
/// slice 1b this tick NO LONGER writes the `BossAttackState` component — that
/// telegraph/strike read-model is projected SOLELY from the live `MovePlayback` by
/// `project_boss_attack_state_from_move`, and the volume / damage / debug-overlay
/// paths read that projected value.
///
/// E6(c): the autonomous boss arm builds the `BossPatternContext` directly from
/// its selected target instead of laundering that target through
/// `BrainSnapshot::target_pos`; player-possessed bosses still use the generic
/// player-brain snapshot because controller input is the point of that path.
pub fn tick_boss_brains_system(
    world_time: Res<WorldTime>,
    // The composed collision read-API rather than its three ingredients.
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    // A possessed boss carries `DrivingParticipant(slot)` and reads its
    // controller frame from here, through the SAME universal-control path every
    // driven body uses. Bosses are valid controllable bodies (architecturally); design
    // gating of WHICH boss is possessable lives above, in the possession target
    // filter — not as a "bosses can never be controlled" barrier in this tick.
    slot_controls: Res<ambition_characters::control::SlotControls>,
    mut bosses: Query<
        (
            bevy::ecs::entity::Entity,
            crate::BossClusterRef,
            // The boss's HP authority (§A1) — liveness is `health.alive()`.
            &ambition_characters::actor::BodyHealth,
            &mut Brain,
            // Possession keys on driver authority; the boss brain remains attached
            // and simply stops deciding while a participant drives the body.
            Option<&ambition_characters::control::DrivingParticipant>,
            &mut ActorControl,
            // The per-frame attack INTENT the trigger reads (§A1 intent/projection
            // split): the driver (autonomous pattern OR possession) writes which
            // profile it wants to fire here; `trigger_boss_attack_moves` reads it.
            // The `BossAttackState` read-model is NOT written here (§A1 slice 1b) — the
            // projection owns it — so this tick no longer borrows it.
            &mut ambition_characters::brain::BossAttackIntent,
            &ambition_combat::components::ActorTarget,
            // The boss's authored special repertoire (body CAPABILITY, persisted
            // across a brain swap). Read only by the possession arm to map input
            // onto the boss's own moves; `Option` for test fixtures that spawn a
            // boss without it.
            Option<&ambition_characters::brain::BossCapability>,
            // The projected live-move read-model (LAST frame's — the projection
            // runs after this tick). The autonomous pattern OBSERVES its own
            // playing move through it: cycle mode sustains its request through
            // the observed windup and rests once the move ends, instead of
            // running a parallel windup/active clock. Read-only; the projection
            // stays the sole writer.
            Option<&BossAttackState>,
        ),
        With<FeatureSimEntity>,
    >,
    // Any body a boss may be aiming at — its COLLISION extent, read-only. NOT
    // `CenteredAabb`: that is the coarse footprint, and a boss publishes it from
    // its `BodyEnvelope` render envelope (AJ5.1), so asking it about a boss
    // target would answer with a render quad. `BodyKinematics::size` is the box
    // the movement seam actually sweeps, for every body including a boss —
    // `integrate_boss_bodies` heals `kin.size` onto the authored `combat_size`
    // every tick, which is the same extent `BossPatternCfg::combat_size` gives
    // the asking side.
    target_bodies: Query<&ambition_platformer2d_core::BodyKinematics>,
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
    ) in &mut bosses
    {
        let boss = feature.as_boss_ref();
        if !health.alive() {
            // Dead boss: zero out the control frame + fire intent so the trigger starts
            // nothing this frame; the projection clears its `BossAttackState` read-model.
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            intent.clear();
            continue;
        }

        // POSSESSED BOSS: driven from slot input through the player brain, the
        // same universal path every controlled body uses. It steers by
        // `velocity_target` (bosses float / SNAP-integrate in `update_ecs_bosses`)
        // at the shared body run capability, AND commands its own authored specials
        // through a deterministic input→special mapping over `BossCapability` — the
        // boss body's full kit, nothing special-cased (unified-actors I2/I7).
        //
        // the scripted pattern is not stashed any more; it is simply not
        // asked. Its `Brain` sits on the body untouched for the whole
        // possession — the human is the policy choosing from the same repertoire
        // the pattern would, and the moment the seat leaves, the pattern resumes
        // from the state it was in.
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

            // Map controller input onto the boss's authored repertoire and publish it as
            // this frame's fire INTENT (§A1 slice 1b). `trigger_boss_attack_moves` reads
            // it and starts the matching move; the move's OWN duration is the fire-rate
            // gate (a live `MovePlayback` blocks re-trigger, invariant I3), so the
            // possession path needs no separate `active_remaining` bookkeeping. The
            // `BossAttackState` read-model is written SOLELY by the projection from that
            // live move — no direct write here. A possessed strike fires as a REAL
            // strike (R1.4: possession grants the full kit; the hitbox carries the
            // possessor's effective faction, stamped in `advance_move_playback`), and
            // when this boss RIDES a limb-rigged mount, the projected `BossAttackState`
            // drives `route_boss_strikes_to_limbs` exactly as the autonomous pattern
            // does — press down+attack aboard the giant and both hands slam (G5).
            //
            // The mapping is the G5 CONTROLLER→VERB MAP (`possessed_attack_choice`):
            // the profile's authored `possessed_verbs` resolved through the same
            // directional-verb chain an actor melee uses, falling back to the legacy
            // deterministic mapping (primary strike / signature special) for a boss
            // that authors no verbs. Verb bindings are BLIND.
            intent.clear();
            if let Some(profile) =
                possessed_attack_choice(&frame, &boss.config.behavior, capability, boss.kin.facing)
            {
                intent.active_profile = Some(profile);
            }
            continue;
        }

        // Non-BossPattern brains on a boss (test fixtures) emit no fire intent — the
        // same guard the bespoke `pattern_brain_mut` match used before the
        // universal-tick fold. The projection clears their `BossAttackState`.
        if !matches!(
            &*brain,
            Brain::StateMachine(StateMachineCfg::BossPattern { .. })
        ) {
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            intent.clear();
            continue;
        }

        // §A7 BOSS PERCEPTION = OMNISCIENT (the basic mode, `Perception::default()`).
        // A boss is relentless: it KNOWS where its foe is anywhere in its arena — you
        // cannot juke it — so it reads the global `ActorTarget` `select_actor_targets`
        // maintains, the same omniscient datum every body carries. This is a documented
        // POLICY, not a carve-out: omniscience is a first-class perception (see
        // `Perception`), applied to the boss via the DEFAULT (it carries no `Perception`
        // component, unlike sighted actors granted `Sighted` by `ensure_perception`).
        // A boss that wanted bounded, juke-able senses would carry `Perception::Sighted`
        // and branch here exactly as `tick_actor_brains` does; none do today.
        let target_pos = target.pos;
        // The target's own BODY, not just where it stands. A contact chase asks
        // whether two bodies touch; answering that from positions alone made
        // every wide body's contact unreachable (see `lateral_body_gap`). Every
        // body carries `BodyKinematics`, player and actor and boss alike, so this
        // asks one question of one component. A target we cannot find a body for
        // reads as a point, which is exactly what the old arithmetic assumed.
        let target_body_size = target
            .entity
            .and_then(|entity| target_bodies.get(entity).ok())
            .map_or(ae::Vec2::ZERO, |kin| kin.size);

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
                    encounter_phase: boss.status.encounter_phase,
                    actor_pos: boss.kin.pos,
                    target_pos,
                    target_body_size,
                    world_size: feature_world.size,
                    front_wall_clearance,
                    dt,
                    // BD1's situation buckets + `OnHitTaken`. The brain remembers
                    // its own last HP, so a hit is a DROP in this pool — no
                    // per-tick damage channel exists and none is invented.
                    actor_facing: boss.kin.facing,
                    hp_current: health.current(),
                    hp_max: health.max(),
                    // The brain's observation of its own live move, from the
                    // projected read-model (one frame stale, deterministically):
                    // a telegraphing move reads as `striking: false`, a striking
                    // move as `striking: true`, no move as `None`.
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

        // Geometry strikes and content-technique specials now share this path:
        // the profile request starts one authored move, whose active windows own
        // hit volumes or sustained `Effect{key}` emission. There is no direct
        // boss-special dispatch beside the moveset runtime.
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

/// Boss PRESENTATION — decay the boss's body-generic reaction timers and sync the
/// sprite-animation mirrors (`BossPatternTimer`, `BossPhase`, death anim).
///
/// Since fable AD2 this system moves no body and emits no damage: movement is
/// [`integrate_boss_bodies`] (the shared flight-limb arm); STRIKE damage is the
/// moveset's own hitboxes (`trigger_boss_attack_moves` → `advance_move_playback` →
/// `apply_hitbox_damage`); BODY-CONTACT damage is the shared `apply_actor_contact_damage`.
/// The old `boss_attack_damage` / `sync_boss_strike_hitboxes` polls are gone — a boss's
/// offense and body flow through the
/// SAME systems every actor uses.
pub fn update_ecs_bosses(
    world_time: Res<WorldTime>,
    mut bosses: Query<
        (
            &ambition_characters::actor::BodyHealth,
            &mut ambition_characters::actor::BodyCombat,
            &mut BossPatternTimer,
            &mut BossDeathAnimation,
            &mut BossPhase,
            &Brain,
        ),
        // The player carries the unified `BodyKinematics`; exclude it so this boss
        // query is provably disjoint (boss / player are mutually exclusive archetypes).
        (
            With<FeatureSimEntity>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
        ),
    >,
) {
    // Sim clock: bosses must slow with bullet-time (ADR 0010).
    let dt = world_time.sim_dt();
    for (health, mut boss_combat, mut pattern_timer, mut death_anim, mut phase, brain) in
        &mut bosses
    {
        let alive = health.alive();
        // Body-generic reaction timers (hit_flash + i-frame + the §A2 stagger set)
        // decay here for bosses through the SAME `BodyCombat` decay the actor tick
        // runs — the boss is excluded from the actor tick, so it decays its own,
        // but via the one shared method, not a hand-copy (§A1).
        boss_combat.decay_reaction_timers(dt);
        // Mirror the brain's `pattern_timer` (living in `BossPatternState`) into the
        // presentation-side `BossPatternTimer` for sprite-animation consumers.
        // Defaults to 0 for a non-BossPattern brain (test fixtures).
        pattern_timer.0 = match brain {
            Brain::StateMachine(StateMachineCfg::BossPattern { state, .. }) => state.pattern_timer,
            _ => 0.0,
        };
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
