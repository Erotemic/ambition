//! The per-frame boss tick: encounter-phase sync + brain ticking + the main
//! ECS boss update (`update_ecs_bosses`).

use super::super::*;

use ambition_characters::brain::{
    ActorControl, BossAttackIntent, BossAttackState, Brain, StateMachineCfg,
};
use ambition_engine_core::AabbExt;
use bevy::prelude::{Commands, Entity};

/// Copy the profile fields the trigger reads from the boss pattern brain's
/// freshly-computed attack projection (`BossPatternState.attack_state`) onto its
/// [`BossAttackIntent`] (fable review §A1 intent/projection split). This is what
/// `trigger_boss_attack_moves` reads to start a move. Since §A1 slice 1b the ECS
/// [`BossAttackState`] component is NO LONGER the intent source and is no longer
/// written here — it is written SOLELY by `project_boss_attack_state_from_move` as a
/// pure projection of the live `MovePlayback`.
fn mirror_intent(attack_state: &BossAttackState, intent: &mut BossAttackIntent) {
    intent.telegraph_profile = attack_state.telegraph_profile.clone();
    intent.active_profile = attack_state.active_profile.clone();
}

/// Sync each boss's `encounter_phase` mirror from the entity-local
/// [`BossPhaseState`] copy (`BossEncounter.encounter`). The mirror is a convenience
/// field the brain (`BossPatternContext`) reads; the `BossEncounter.encounter`
/// phase machine — ticked by `update_boss_encounters` — is the source of truth.
/// Keyed per-entity by construction, so two of the same archetype sync
/// independent phases.
///
/// Runs before [`tick_boss_brains_system`] so the brain sees this frame's phase.
pub fn sync_boss_encounter_phase(
    mut bosses: Query<super::super::boss_clusters::BossClusterQueryData, With<FeatureSimEntity>>,
    mut last_logged: bevy::ecs::system::Local<
        std::collections::HashMap<String, crate::boss_encounter::BossEncounterPhase>,
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
                        target: "ambition::boss_encounter",
                        "sync_phase: boss={} (behavior.id={}) phase {:?} → {:?}",
                        boss_id,
                        behavior_id,
                        prev,
                        phase,
                    );
                    last_logged.insert(boss_id.clone(), phase);
                }
                None => {
                    bevy::log::warn!(
                        target: "ambition::boss_encounter",
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

/// TRIGGER the boss's data-driven attack MOVES: while a strike profile is the boss's
/// active attack, ensure its move is playing (insert `MovePlayback` from the boss's
/// `ActorMoveset` for that profile's [`move_id`](ambition_characters::brain::BossAttackProfile::move_id)).
/// This is the ONE trigger for EVERY boss strike — geometry AND special — so a boss's
/// melee runs through the SAME moveset runtime an actor's swing does (fable review
/// §A1: the moveset is the boss's melee system too), retiring the bespoke
/// `sync_boss_strike_hitboxes` per-tick geometry poll AND the boss-only
/// `dispatch_boss_special`:
///
/// - A **geometry** profile's move carries the strike's static hit volumes on its
///   Active window; `advance_move_playback` spawns/despawns the Boss-faction strike
///   hitbox through the shared `apply_hitbox_damage` path.
/// - A **special** profile's move SUSTAINS `Effect{key}` every strike frame; the
///   `Effect{key}`→`Special{key}` bridge fires the content technique.
///
/// `Without<MovePlayback>` gates re-trigger; the move duration equals the authored
/// strike window (both on the boss's proper time = sim time undilated), so the strike
/// lasts exactly the window. A **possessed** boss (its `active_profile` set from
/// controller input in `tick_boss_brains_system`) fires SPECIALS here too — but its
/// GEOMETRY strikes are suppressed (the old `sync_boss_strike_hitboxes` skipped
/// player-controlled bosses; possessed-boss geometry with effective faction is a
/// follow-up).
pub fn trigger_boss_attack_moves(
    mut commands: Commands,
    bosses: Query<
        (
            Entity,
            &BossAttackIntent,
            &crate::combat::moveset::ActorMoveset,
            &crate::actor::BodyKinematics,
            Option<&Brain>,
            Option<&crate::combat::moveset::MovePlayback>,
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
    for (entity, attack_intent, moveset, kin, brain, playback) in &bosses {
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

        // A move is already playing: don't re-trigger (its own duration is the
        // fire-rate gate) — but ABORT a still-in-WINDUP move the pattern has
        // abandoned (phase change / suppress / rest cleared the intent, or it moved
        // on to a different profile). This is the telegraph-edge trigger's parity
        // with the old strike-edge behavior: an interrupted windup must NOT strike.
        // A move already in its Active window is committed (the Smash convention) and
        // runs to completion.
        if let Some(pb) = playback {
            let move_profile = BossAttackProfile::from_move_id(&pb.spec.id);
            let in_windup = pb.t < active_start(&pb.spec);
            let intent_wants_this = intent.is_some_and(|(p, _)| *p == move_profile);
            if in_windup && !intent_wants_this {
                commands
                    .entity(entity)
                    .remove::<crate::combat::moveset::MovePlayback>();
            }
            continue;
        }

        let Some((profile, from_telegraph)) = intent else {
            continue;
        };
        // Possessed-boss GEOMETRY strikes stay suppressed (parity with the retired
        // sync); its specials still fire (they carry the firer's effective faction).
        if !profile.is_special() && brain.is_some_and(|b| b.is_player()) {
            continue;
        }
        if let Some(spec) = moveset.0.move_by_id(&profile.move_id()) {
            // Telegraph edge → `t0 = 0` plays the windup THROUGH the move (so the
            // projected telegraph read-model + a future bound anim clip slave to the
            // one move timeline). Strike/possession edge → `t0 = tel` starts at the
            // strike, so the hitbox is live the same frame as the pre-Slice-D move.
            let t0 = if from_telegraph { 0.0 } else { active_start(spec) };
            commands.entity(entity).insert(
                crate::combat::moveset::MovePlayback::new_at(spec.clone(), kin.facing, t0),
            );
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
            Option<&crate::combat::moveset::MovePlayback>,
            &mut BossAttackState,
        ),
        With<FeatureSimEntity>,
    >,
) {
    use ambition_characters::brain::BossAttackProfile;
    for (playback, mut attack_state) in &mut bosses {
        // No live move → no telegraph, no strike. As the SOLE writer (§A1 slice 1b) the
        // projection clears a resting boss here rather than leaving a stale strike the
        // brain used to clear.
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

/// Tick every boss's `BossPattern` brain: advance the cursor, emit
/// `ActorControlFrame` intent (movement + melee/special edges), and publish the
/// per-frame attack INTENT (`BossAttackIntent`) the moveset trigger reads. Since §A1
/// slice 1b this tick NO LONGER writes the `BossAttackState` component — that
/// telegraph/strike read-model is projected SOLELY from the live `MovePlayback` by
/// `project_boss_attack_state_from_move`, and the volume / damage / debug-overlay
/// paths read that projected value.
pub fn tick_boss_brains_system(
    world_time: Res<WorldTime>,
    world: Res<ambition_engine_core::RoomGeometry>,
    platform_set: Res<crate::MovingPlatformSet>,
    overlay: Res<FeatureEcsWorldOverlay>,
    // A possessed boss carries `Brain::Player(slot)` and reads its controller
    // frame from here, through the SAME universal-brain path every controlled
    // body uses. Bosses are valid controllable bodies (architecturally); design
    // gating of WHICH boss is possessable lives above, in the possession target
    // filter — not as a "bosses can never be controlled" barrier in this tick.
    slot_controls: Res<ambition_characters::brain::SlotControls>,
    mut bosses: Query<
        (
            bevy::ecs::entity::Entity,
            super::super::boss_clusters::BossClusterRef,
            // The boss's HP authority (§A1) — liveness is `health.alive()`.
            &ambition_characters::actor::BodyHealth,
            &mut Brain,
            &mut ActorControl,
            // The per-frame attack INTENT the trigger reads (§A1 intent/projection
            // split): the driver (autonomous pattern OR possession) writes which
            // profile it wants to fire here; `trigger_boss_attack_moves` reads it.
            // The `BossAttackState` read-model is NOT written here (§A1 slice 1b) — the
            // projection owns it — so this tick no longer borrows it.
            &mut ambition_characters::brain::BossAttackIntent,
            &super::super::super::components::ActorTarget,
            // The boss's authored special repertoire (body CAPABILITY, persisted
            // across a brain swap). Read only by the possession arm to map input
            // onto the boss's own moves; `Option` for test fixtures that spawn a
            // boss without it.
            Option<&ambition_characters::brain::BossCapability>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    for (_entity, feature, health, mut brain, mut control, mut intent, target, capability) in
        &mut bosses
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
        // boss body's full kit, nothing special-cased (unified-actors I2/I7). The
        // scripted pattern is suspended (its brain is stashed); the human is the
        // policy choosing from the same repertoire the pattern would.
        if let Some(slot) = brain.player_slot() {
            let mut snapshot = ambition_characters::brain::BrainSnapshot::idle();
            snapshot.actor_pos = boss.kin.pos;
            snapshot.actor_vel = boss.kin.vel;
            snapshot.actor_facing = boss.kin.facing;
            snapshot.actor_aerial = true;
            snapshot.max_run_speed = ae::MAX_RUN_SPEED;
            snapshot.dt = dt;
            snapshot.player_input = Some(slot_controls.get(slot));
            let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
            brain.tick(&snapshot, &mut frame);
            control.0 = frame;

            // Map controller input onto the boss's authored repertoire and publish it as
            // this frame's fire INTENT (§A1 slice 1b). `trigger_boss_attack_moves` reads
            // it and starts the matching move; the move's OWN duration is the fire-rate
            // gate (a live `MovePlayback` blocks re-trigger, invariant I3), so the
            // possession path needs no separate `active_remaining` bookkeeping. The
            // `BossAttackState` read-model is written SOLELY by the projection from that
            // live move — no direct write here.
            //
            // Deterministic mapping (tuning/design is a follow-up): attack → the boss's
            // primary authored strike; special (blink button) / projectile → its
            // SIGNATURE content special (falling back to the next strike if the boss
            // authors only geometry moves). A boss with no authored special is a no-op.
            //
            // BLIND (Jon feel-checks): a possessed GEOMETRY strike is suppressed by the
            // trigger (parity with the retired `sync_boss_strike_hitboxes`, which never
            // struck for a player-controlled boss). With `BossAttackState` now
            // projection-only, a suppressed geometry strike starts no move and so shows
            // no strike POSE either (its damage was already suppressed). A SPECIAL still
            // fires — its move projects the pose and sustains the content technique with
            // the firer's effective Player faction. Restoring the geometry strike as a
            // REAL strike (routed through the moveset with effective faction) is the
            // effective-faction follow-up.
            intent.clear();
            let choice = if frame.melee_pressed {
                capability.and_then(|c| c.slot(0).cloned())
            } else if frame.special_pressed || frame.projectile_pressed {
                capability.and_then(|c| c.signature_special().or_else(|| c.slot(1)).cloned())
            } else {
                None
            };
            if let Some((profile, _strike_seconds)) = choice {
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

        // The front-wall standoff the pattern probes with — read before the brain
        // borrow that `brain.tick` needs.
        let front_wall_standoff = match &*brain {
            Brain::StateMachine(StateMachineCfg::BossPattern { cfg, .. }) => {
                cfg.macro_tuning.front_wall_standoff
            }
            _ => 0.0,
        };
        let front_wall_clearance =
            boss_front_wall_clearance(&feature_world, &boss, target.pos, front_wall_standoff);

        // §A1 slice 3c: the boss brain ticks through the UNIVERSAL `Brain::tick`
        // path like every other body — no bespoke `tick_boss_pattern` call site.
        // Fill the BossPattern fields onto the shared snapshot; the dispatcher
        // routes to `tick_boss_pattern`, which writes the attack projection INTO
        // `BossPatternState.attack_state`. Mirror that into the ECS component below.
        let mut snapshot = ambition_characters::brain::BrainSnapshot::idle();
        snapshot.actor_pos = boss.kin.pos;
        snapshot.target_pos = target.pos;
        snapshot.dt = dt;
        snapshot.boss_encounter_phase = Some(boss.status.encounter_phase);
        snapshot.world_size = world.0.size;
        snapshot.front_wall_clearance = front_wall_clearance;
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        brain.tick(&snapshot, &mut frame);
        // Publish the brain's fire INTENT for the trigger (§A1 split): the profile the
        // pattern wants this frame (telegraph edge → windup; strike → strike), read
        // STRAIGHT from the brain's freshly-ticked `BossPatternState.attack_state`. The
        // ECS `BossAttackState` component is no longer written here (§A1 slice 1b) — the
        // projection derives it from the move this intent starts.
        if let Some(bps) = brain.boss_pattern_state() {
            mirror_intent(&bps.attack_state, &mut intent);
        } else {
            intent.clear();
        }

        // Boss specials run through the SHARED moveset now (fable review §A1): a
        // multi-special boss (the Gradient Sentinel authors four; GNU-ton its apple
        // rain) can't fit `ActionSet`'s single special slot, so its `ActionSet.special`
        // is `None` and the boss carries an `ActorMoveset` (one sustain-move per key,
        // built at spawn). Mirroring the brain's `active_profile` into the ECS
        // `BossAttackState` above is the whole wiring — `trigger_boss_attack_moves`
        // reads it and starts the move, whose per-frame `Effect{key}` fires the content
        // technique through `dispatch_move_events`. One path for every boss special AND
        // the actor's; the bespoke `dispatch_boss_special` is retired.
        control.0 = frame;
    }
}

pub(crate) fn boss_front_wall_clearance(
    world: &ae::World,
    boss: &super::super::boss_clusters::BossRef<'_>,
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

/// PHASE — integrate boss bodies through the ONE shared per-body integrator
/// (`integrate_actor_body`), the SAME function every actor body flows through
/// (fable-review-2026-07-04 R1.1). A boss IS an aerial actor: its `BossPattern`
/// brain wrote a `velocity_target` into `ActorControl` upstream, and the shared
/// integrator moves it through the SAME flight limb every aerial actor uses
/// (`ActorMut::update`) in DIRECT-VELOCITY mode — the commanded velocity is taken
/// verbatim. There is no longer a bespoke boss `em.update` + `CenteredAabb` publish;
/// the boss's coarse render footprint rides the body-generic [`crate::combat::BodyEnvelope`]
/// (the envelope split, AJ5.1), so the one integrator's publish rule
/// (`footprint = envelope ?? kin.size`) reproduces the render-sized box.
///
/// This is the boss sibling of the player's `integrate_home_body` arm and the
/// actor arm of `integrate_sim_bodies`: three disjoint archetypes, ONE shared
/// integrator. It keeps its own scheduled slot (chain 1, between
/// `tick_boss_brains_system` and `update_ecs_bosses`) so the boss's presentation
/// systems still read this frame's already-moved position. Byte-identical to the
/// old bespoke arm: a boss's flight produces no jump/dash/land move-events (no
/// movement FX), never `shark_charge_crash`es (its caps lack `charge_crash_explodes`),
/// and its stagger timers are always zero (the boss victim path arms none), so
/// every extra thing `integrate_actor_body` does is a no-op here.
///
/// (Merging this into `integrate_sim_bodies`' actor query with NO boss arm is a
/// later slice — it needs the chain-2 movement phase reordered ahead of the
/// chain-1 boss presentation systems, a schedule change kept separate from this
/// integrator-sharing one.)
pub fn integrate_boss_bodies(
    world_time: Res<WorldTime>,
    world: Res<ambition_engine_core::RoomGeometry>,
    platform_set: Res<crate::MovingPlatformSet>,
    overlay: Res<FeatureEcsWorldOverlay>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    steering: Res<super::super::actors::ActorSteering>,
    gravity: crate::physics::GravityCtx,
    mut sfx: bevy::prelude::MessageWriter<ambition_sfx::SfxMessage>,
    mut vfx: bevy::prelude::MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut hit_events: bevy::prelude::MessageWriter<HitEvent>,
    mut bosses: Query<
        (
            Entity,
            super::super::actor_clusters::ActorClusterQueryData,
            &super::super::boss_clusters::BossConfig,
            &crate::combat::BodyEnvelope,
            Option<&mut ActorControl>,
            Option<&mut crate::player::BodyAnimFacts>,
            &super::super::super::components::ActorTarget,
            &mut CenteredAabb,
            &mut ambition_characters::actor::BodyCombat,
        ),
        (With<FeatureSimEntity>, Without<crate::actor::PlayerEntity>),
    >,
) {
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    let combat_tuning = feel_tuning.feature_combat_tuning();
    for (entity, mut cq, boss_config, envelope, mut control, mut anim, target, mut aabb, mut combat) in
        &mut bosses
    {
        // Self-heal the collision envelope onto `kin.size` (the seam sweeps it),
        // robust to the profile / spawn-override / sprite-derive timing that writes
        // `behavior.combat_size`. The coarse render footprint stays in `BodyEnvelope`.
        let combat_size = boss_config.behavior.combat_size.unwrap_or(cq.kin.size);
        let mut em = cq.as_actor_mut();
        em.kin.size = combat_size;
        super::super::actors::integrate_actor_body(
            entity,
            &mut em,
            &mut aabb,
            &mut combat,
            control.as_deref_mut(),
            anim.as_deref_mut(),
            // The boss's coarse render envelope publishes the `CenteredAabb`
            // (byte-identical to the old render-sized box); an ordinary actor
            // would pass `None` and publish from `kin.size`.
            Some(envelope.0),
            target.pos,
            // A boss is never mounted.
            false,
            &feature_world,
            combat_tuning,
            &steering,
            &gravity,
            dt,
            *feel_tuning,
            &mut sfx,
            &mut vfx,
            &mut hit_events,
        );
    }
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
        (With<FeatureSimEntity>, Without<crate::actor::PlayerEntity>),
    >,
) {
    // Sim clock: bosses must slow with bullet-time (ADR 0010).
    let dt = world_time.sim_dt();
    for (health, mut boss_combat, mut pattern_timer, mut death_anim, mut phase, brain) in &mut bosses
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
mod attack_moveset_tests {
    use super::*;
    use ambition_characters::brain::{BossAttackProfile, BossCapability};

    fn warden_behavior() -> crate::features::bosses::BossBehaviorProfile {
        crate::features::bosses::BossBehaviorProfile::clockwork_warden()
    }

    /// Boss-fold slice (fable review §A1): EVERY boss strike runs through the SHARED
    /// moveset. `boss_attack_moveset` builds one move per profile — a GEOMETRY strike
    /// gets an Active-window hit volume (from `volumes_for_profile`), a SPECIAL gets a
    /// sustain-`Effect` move — and `trigger_boss_attack_moves` starts whichever profile
    /// is the boss's `active_profile`. This pins BOTH new links (geometry + special).
    #[test]
    fn a_boss_geometry_profile_triggers_its_hit_volume_move() {
        let cap = BossCapability {
            specials: vec![
                (BossAttackProfile::FloorSlam, 0.3), // geometry → hit-volume move
                (BossAttackProfile::Special("apple_rain".to_string()), 2.0),
            ],
        };
        let combat_size = ambition_engine_core::Vec2::new(80.0, 80.0);
        let moveset =
            crate::features::boss_attack_moveset(&cap, &warden_behavior(), combat_size, &[])
                .expect("a boss with strikes → a moveset");
        // BOTH profiles now author a move — geometry AND special.
        assert_eq!(moveset.0.moves.len(), 2, "geometry + special both became moves");
        let slam = moveset
            .0
            .move_by_id("floor_slam")
            .expect("the geometry profile became a hit-volume move");
        assert_eq!(slam.duration_s, 0.3);
        let active = &slam.windows[0];
        assert!(matches!(active.tag, ambition_entity_catalog::WindowTag::Active));
        assert!(
            !active.volumes.is_empty(),
            "FloorSlam authors a body-local hit volume"
        );
        assert!(active.sustain_effect.is_none(), "geometry is not a sustain");
        assert!(
            moveset.0.move_by_id("apple_rain").is_some(),
            "the Special profile still became a sustain-move"
        );

        // Trigger a geometry strike: the driver's INTENT (§A1 split) names FloorSlam
        // as the active profile → the trigger starts the FloorSlam move.
        let mut app = App::new();
        app.add_systems(Update, trigger_boss_attack_moves);
        let intent = BossAttackIntent {
            active_profile: Some(BossAttackProfile::FloorSlam),
            ..Default::default()
        };
        let boss = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                intent,
                moveset,
                crate::actor::BodyKinematics {
                    pos: ambition_engine_core::Vec2::ZERO,
                    vel: ambition_engine_core::Vec2::ZERO,
                    size: ambition_engine_core::Vec2::new(80.0, 80.0),
                    facing: 1.0,
                },
            ))
            .id();
        app.update();
        let pb = app
            .world()
            .get::<crate::combat::moveset::MovePlayback>(boss)
            .expect("the active geometry profile started its moveset move");
        assert_eq!(pb.spec.id, "floor_slam");
        assert!(
            !pb.spec.windows[0].volumes.is_empty(),
            "the triggered move carries the strike hit volume"
        );
    }

    /// Build the (trigger → advance → project) chain the E53 flip runs, on one boss
    /// whose FloorSlam move spans a 0.2s telegraph + 0.3s strike.
    fn telegraph_boss_app() -> (App, Entity) {
        let cap = BossCapability {
            specials: vec![(BossAttackProfile::FloorSlam, 0.3)],
        };
        let combat_size = ambition_engine_core::Vec2::new(80.0, 80.0);
        let moveset = crate::features::boss_attack_moveset(
            &cap,
            &warden_behavior(),
            combat_size,
            &[(BossAttackProfile::FloorSlam, 0.2)],
        )
        .expect("a boss with a telegraphed strike → a moveset");

        let mut app = App::new();
        app.init_resource::<ambition_time::WorldTime>();
        {
            let mut wt = app.world_mut().resource_mut::<ambition_time::WorldTime>();
            wt.scaled_dt = 0.05;
            wt.raw_dt = 0.05;
        }
        app.add_message::<crate::combat::moveset::MoveEventMessage>();
        app.add_systems(
            Update,
            (
                trigger_boss_attack_moves,
                crate::combat::moveset::advance_move_playback,
                project_boss_attack_state_from_move,
            )
                .chain(),
        );
        // §A1 split: the trigger reads the INTENT (telegraph edge → play the windup);
        // the projection WRITES the read-model `BossAttackState` from the live move.
        let intent = BossAttackIntent {
            telegraph_profile: Some(BossAttackProfile::FloorSlam),
            ..Default::default()
        };
        let boss = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                intent,
                BossAttackState::default(),
                moveset,
                crate::combat::components::ActorFaction::Boss,
                crate::actor::BodyKinematics {
                    pos: ambition_engine_core::Vec2::ZERO,
                    vel: ambition_engine_core::Vec2::ZERO,
                    size: combat_size,
                    facing: 1.0,
                },
            ))
            .id();
        (app, boss)
    }

    /// E53 Slice D: a Telegraph-step intent starts the move at its WINDUP (`t0 = 0`),
    /// the projection reports `telegraph_profile` while the move is in windup, then
    /// flips to `active_profile` once the move's clock reaches the strike window —
    /// `BossAttackState` is DERIVED from the live move, both halves.
    #[test]
    fn telegraph_edge_trigger_projects_windup_then_strike() {
        let (mut app, boss) = telegraph_boss_app();

        // Frame 1: the telegraph intent starts the move at t0=0; one advance puts it
        // ~0.05s into the 0.2s windup — the projection reports a TELEGRAPH, no strike.
        app.update();
        let st = app.world().get::<BossAttackState>(boss).unwrap();
        assert_eq!(st.telegraph_profile, Some(BossAttackProfile::FloorSlam));
        assert_eq!(st.active_profile, None, "windup has no live strike yet");

        // Advance past the 0.2s telegraph into the strike window: the projection now
        // reports the STRIKE, telegraph cleared, and active_elapsed folds in the
        // telegraph offset (t ≈ 0.25 > 0.2).
        for _ in 0..4 {
            app.update();
        }
        let st = app.world().get::<BossAttackState>(boss).unwrap();
        assert_eq!(st.active_profile, Some(BossAttackProfile::FloorSlam));
        assert_eq!(st.telegraph_profile, None, "strike clears the telegraph");
        assert!(
            st.active_elapsed > 0.2,
            "active_elapsed folds in the telegraph offset; got {}",
            st.active_elapsed
        );
    }

    /// E53 Slice D: a windup the pattern ABANDONS (intent cleared — phase change /
    /// suppress / rest) must NOT strike. The still-in-windup move is despawned before
    /// its Active window opens, so no spurious hitbox — parity with the old
    /// strike-edge trigger (which simply never started a move for an interrupted
    /// telegraph).
    #[test]
    fn interrupted_windup_is_aborted_before_the_strike() {
        let (mut app, boss) = telegraph_boss_app();
        app.update();
        assert!(
            app.world().get::<crate::combat::moveset::MovePlayback>(boss).is_some(),
            "the telegraph started a move"
        );
        // The pattern abandons the windup (e.g. a phase transition cleared intent):
        // clearing the INTENT (§A1 split) is what the trigger observes to abort.
        app.world_mut()
            .get_mut::<BossAttackIntent>(boss)
            .unwrap()
            .clear();
        app.update();
        assert!(
            app.world().get::<crate::combat::moveset::MovePlayback>(boss).is_none(),
            "an abandoned windup is aborted before it can strike"
        );
    }
}
