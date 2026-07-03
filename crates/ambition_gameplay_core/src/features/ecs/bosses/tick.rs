//! The per-frame boss tick: encounter-phase sync + brain ticking + the main
//! ECS boss update (`update_ecs_bosses`).

use super::super::*;

use ambition_characters::brain::{ActorControl, BossAttackState, Brain, StateMachineCfg};
use ambition_engine_core::AabbExt;
use bevy::prelude::{Commands, Entity};

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
            &BossAttackState,
            &crate::combat::moveset::ActorMoveset,
            &crate::actor::BodyKinematics,
            Option<&Brain>,
        ),
        (
            With<FeatureSimEntity>,
            Without<crate::combat::moveset::MovePlayback>,
        ),
    >,
) {
    for (entity, attack_state, moveset, kin, brain) in &bosses {
        let Some(profile) = attack_state.active_profile.as_ref() else {
            continue;
        };
        // Possessed-boss GEOMETRY strikes stay suppressed (parity with the retired
        // sync); its specials still fire (they carry the firer's effective faction).
        if !profile.is_special() && brain.is_some_and(|b| b.is_player()) {
            continue;
        }
        if let Some(spec) = moveset.0.move_by_id(&profile.move_id()) {
            // Start the move at its Active-window edge (the telegraph offset `tel`),
            // so the strike is live THIS frame — identical hitbox timing to the
            // pre-E53 strike-only move, and possession's instant strike is preserved
            // (`boss_possession_specials`). The move still ENCODES `tel` so the
            // projected `active_elapsed` folds in the telegraph offset. The telegraph-
            // edge (`t0 = 0`, plays the windup) trigger is the E53 Slice-D flip.
            let t0 = spec
                .windows
                .iter()
                .find(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
                .map(|w| w.start_s)
                .unwrap_or(0.0);
            commands.entity(entity).insert(
                crate::combat::moveset::MovePlayback::new_at(spec.clone(), kin.facing, t0),
            );
        }
    }
}

/// PROJECT the ACTIVE (strike) half of [`BossAttackState`] from the live boss
/// [`MovePlayback`] (E53 Slice B+C). While a boss move's clock is inside its Active
/// window, the strike read-model — `active_profile` / `active_remaining` /
/// `active_elapsed` — is DERIVED from the move (the move is the authority) instead
/// of trusted from the pattern cursor's mirror. The values are provably equal to
/// the brain's write (the move carries the telegraph offset `tel` as its Active
/// start, so `active_elapsed == t == tel + strike_elapsed`), so this is
/// behavior-preserving; it flips WHO owns the strike timing to the shared move
/// runtime, mirroring `project_moveset_melee_to_body_melee`.
///
/// ADDITIVE + non-destructive: it only writes while a move is in its Active window,
/// and never touches the TELEGRAPH fields (still brain-written) — so a boss with no
/// `ActorMoveset` (test fixtures), a boss mid-telegraph, and a resting boss all keep
/// the brain's mirror untouched. Runs AFTER `advance_move_playback` so `t` is
/// current. The telegraph half + retiring the brain write is Slice D.
pub fn project_boss_attack_state_from_move(
    mut bosses: Query<
        (&crate::combat::moveset::MovePlayback, &mut BossAttackState),
        With<FeatureSimEntity>,
    >,
) {
    use ambition_characters::brain::BossAttackProfile;
    for (playback, mut attack_state) in &mut bosses {
        let t = playback.t;
        let Some(active) = playback
            .spec
            .windows
            .iter()
            .find(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
        else {
            continue;
        };
        if t < active.start_s || t >= active.end_s {
            // Outside the strike (telegraph, or the finished tail): leave the brain's
            // mirror as-is. Slice D projects the telegraph half here too.
            continue;
        }
        attack_state.active_profile = Some(BossAttackProfile::from_move_id(&playback.spec.id));
        attack_state.active_remaining = (active.end_s - t).max(0.0);
        attack_state.active_elapsed = t;
    }
}

/// Tick every boss's `BossPattern` brain: advance the cursor, emit
/// `ActorControlFrame` intent (movement + melee/special edges), and
/// update the `BossAttackState` component. `BossAttackState` is the
/// single source of truth for boss attack state — the volume / damage /
/// debug-overlay paths all query it.
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
            &mut BossAttackState,
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
    for (_entity, feature, health, mut brain, mut control, mut attack_state, target, capability) in
        &mut bosses
    {
        let boss = feature.as_boss_ref();
        if !health.alive() {
            // Dead boss: zero out frame + attack state so any
            // downstream consumer sees a coherent "no intent".
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            attack_state.clear();
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

            // Drive the boss's authored specials from controller input through the
            // SAME `BossAttackState` the autonomous pattern sets — so every
            // downstream consumer (telegraph/active volumes, `boss_attack_damage`,
            // the `Special` EFFECTS techniques, sprite anim) is unchanged. The
            // active window is the body's own fire-rate enforcement (invariant I3):
            // while a strike is live it ticks down and a fresh press is ignored; a
            // press when idle starts the mapped special. A boss with no authored
            // special is a no-op — the body simply has no move to command.
            if attack_state.active_remaining > 0.0 {
                attack_state.active_remaining = (attack_state.active_remaining - dt).max(0.0);
                attack_state.active_elapsed += dt;
                if attack_state.active_remaining <= 0.0 {
                    attack_state.clear();
                }
            } else {
                attack_state.clear();
                // Deterministic mapping (tuning/design is a follow-up): attack →
                // the boss's primary authored strike; special (blink button) /
                // projectile → its SIGNATURE content special (falling back to the
                // next strike if the boss authors only geometry moves).
                let choice = if frame.melee_pressed {
                    capability.and_then(|c| c.slot(0).cloned())
                } else if frame.special_pressed || frame.projectile_pressed {
                    capability.and_then(|c| c.signature_special().or_else(|| c.slot(1)).cloned())
                } else {
                    None
                };
                if let Some((profile, strike_seconds)) = choice {
                    attack_state.telegraph_profile = None;
                    attack_state.telegraph_remaining = 0.0;
                    attack_state.telegraph_elapsed = 0.0;
                    attack_state.active_profile = Some(profile.clone());
                    attack_state.active_remaining = strike_seconds;
                    attack_state.active_elapsed = 0.0;
                    // Content-technique specials fire through the SHARED moveset:
                    // setting `active_profile` above is enough — `trigger_boss_attack_moves`
                    // reads it and starts the sustain-move, whose per-frame `Effect{key}`
                    // fires the technique. The spawned effects inherit the firer's
                    // EFFECTIVE faction (Player while possessed), so they strike the
                    // boss's former allies. Geometry profiles have no move → their
                    // damage flows through the frame-driven hitbox path.
                }
            }
            continue;
        }

        // Non-BossPattern brains on a boss (test fixtures) emit no intent and clear
        // the attack mirror — the same guard the bespoke `pattern_brain_mut` match
        // used before the universal-tick fold.
        if !matches!(
            &*brain,
            Brain::StateMachine(StateMachineCfg::BossPattern { .. })
        ) {
            control.0 = ambition_characters::actor::control::ActorControlFrame::neutral();
            attack_state.clear();
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
        if let Some(bps) = brain.boss_pattern_state() {
            *attack_state = bps.attack_state.clone();
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

/// PHASE — integrate boss bodies through the SHARED movement seam (archetype swap
/// AS4c). A boss IS an aerial actor: its `BossPattern` brain wrote a
/// `velocity_target` into `ActorControl` upstream, and this arm moves it through the
/// SAME flight limb every aerial actor uses (`ActorMut::update`) in DIRECT-VELOCITY
/// mode — the commanded velocity is taken verbatim, byte-identical to the boss's old
/// bespoke SNAP float (`step_floating_body`). It is the boss sibling of the player's
/// `integrate_home_body` arm: shares the one movement seam, but keeps the
/// boss-specific footprint publish (the render-basis-sized `CenteredAabb`, oriented
/// to the boss's reference frame). Runs after `tick_boss_brains_system` (intent) and
/// before `update_ecs_bosses` (presentation + attack-damage publish, which read this
/// frame's already-moved position).
pub fn integrate_boss_bodies(
    world_time: Res<WorldTime>,
    world: Res<ambition_engine_core::RoomGeometry>,
    platform_set: Res<crate::MovingPlatformSet>,
    overlay: Res<FeatureEcsWorldOverlay>,
    feel_tuning: Res<crate::time::feel::SandboxFeelTuning>,
    gravity: crate::physics::GravityCtx,
    mut bosses: Query<
        (
            super::super::actor_clusters::ActorClusterQueryData,
            &super::super::boss_clusters::BossConfig,
            &super::super::boss_clusters::BossEncounter,
            &ActorControl,
            &super::super::super::components::ActorTarget,
            &mut CenteredAabb,
        ),
        (With<FeatureSimEntity>, Without<crate::actor::PlayerEntity>),
    >,
) {
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    let combat_tuning = feel_tuning.feature_combat_tuning();
    for (mut actor, boss_config, boss_encounter, control, target, mut aabb) in &mut bosses {
        // Self-heal the collision envelope onto `kin.size` (the seam sweeps it),
        // robust to the profile / spawn-override / sprite-derive timing that writes
        // `behavior.combat_size`. The render basis stays in `BossEncounter.render_size`.
        let combat_size = boss_config.behavior.combat_size.unwrap_or(actor.kin.size);
        let mut em = actor.as_actor_mut();
        em.kin.size = combat_size;
        let gravity_dir = gravity.dir_at(em.kin.pos);
        // Direct-velocity flight (the boss's `ActorConfig.tuning.flight_direct_velocity`
        // is set): `control.0.velocity_target` is taken verbatim by the flight limb.
        // A dead boss returns early inside `update` (no move), matching the old skip.
        let _ = em.update(
            &feature_world,
            target.pos,
            combat_tuning,
            None,
            dt,
            false,
            control.0,
            gravity_dir,
            *feel_tuning,
            // No stagger gate: the boss's old bespoke float applied none, so keep the
            // movement byte-identical (boss hitstun handling is a later decision).
            (0.0, 0.0),
        );
        aabb.center = em.kin.pos;
        // Orient the render footprint to the boss's reference frame (identity under
        // vertical gravity, so replay stays byte-identical there).
        let boss_frame = ambition_engine_core::AccelerationFrame::new(gravity_dir);
        aabb.half_size = boss_frame.to_world_half(boss_encounter.render_size * 0.5);
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

        // Trigger a geometry strike: setting `active_profile` starts the FloorSlam move.
        let mut app = App::new();
        app.add_systems(Update, trigger_boss_attack_moves);
        let mut attack_state = BossAttackState::default();
        attack_state.active_profile = Some(BossAttackProfile::FloorSlam);
        let boss = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                attack_state,
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
}
