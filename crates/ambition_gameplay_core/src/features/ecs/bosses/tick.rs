//! The per-frame boss tick: encounter-phase sync + brain ticking + the main
//! ECS boss update (`update_ecs_bosses`).

use super::super::*;

use crate::features::BossVolumeContext;
use ambition_characters::brain::{ActorControl, BossAttackState, Brain, StateMachineCfg};
use ambition_engine_core::AabbExt;
use bevy::prelude::{Commands, Component, Entity};

/// Marks a Boss-faction strike [`crate::combat::hitbox::Hitbox`] whose geometry is
/// re-derived each tick from the owning boss's live `active_attack_volumes` (one
/// marker per volume `part`). This is fable AD2's generalization: a sprite-frame-
/// driven / multi-part boss strike (GNU-ton's hands) tracks the drawn pose
/// frame-by-frame through the SHARED hitbox pipeline (`apply_hitbox_damage`'s Boss
/// branch), instead of the bespoke per-tick `boss_attack_damage` poll.
#[derive(Component, Clone, Copy, Debug)]
pub struct FrameDrivenBossStrike {
    pub part: usize,
}

/// Aggressor push for a boss strike (matches the old `boss_attack_damage` strike arm).
const BOSS_STRIKE_KNOCKBACK: f32 = 1.25;
/// Backstop lifetime, refreshed every tick the strike stays live; the reconcile
/// despawns explicitly on strike-end, this just reaps a leaked hitbox if the boss
/// despawns mid-strike.
const BOSS_STRIKE_LIFETIME_S: f32 = 0.2;

/// Reconcile per-frame boss STRIKE hitboxes (fable AD2). Replaces the
/// `boss_attack_damage` strike poll: while a boss's `active_attack_volumes` is
/// non-empty, maintain one Boss-faction `Hitbox` per volume part — updating its
/// geometry each tick so a frame-driven strike tracks the drawn pose — and despawn
/// them when the strike ends. Damage then flows through the ONE shared
/// `apply_hitbox_damage` Boss branch (deduped hit-once-per-victim per strike via
/// `HitboxHits`). Possessed / dead bosses maintain none (the old
/// `!boss_player_controlled && alive` gate).
#[allow(clippy::type_complexity)]
pub fn sync_boss_strike_hitboxes(
    mut commands: Commands,
    gravity: crate::physics::GravityCtx,
    bosses: Query<
        (
            Entity,
            super::super::boss_clusters::BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &BossAttackState,
            &Brain,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<FeatureSimEntity>,
    >,
    mut strikes: Query<(
        Entity,
        &FrameDrivenBossStrike,
        &mut crate::combat::hitbox::Hitbox,
        &mut crate::combat::hitbox::HitboxLifetime,
    )>,
) {
    use crate::combat::hitbox::{Hitbox, HitboxAnchor, HitboxHits, HitboxLifetime};
    use crate::features::active_attack_volumes;

    // Live strike volumes for a boss this tick (empty ⇒ no strike / gated off).
    let volumes_for = |entity: Entity| -> Vec<ae::Aabb> {
        let Ok((_, feature, health, attack_state, brain, anim)) = bosses.get(entity) else {
            return Vec::new();
        };
        if !health.alive() || brain.is_player() {
            return Vec::new();
        }
        let ctx = BossVolumeContext::from_ref(feature.as_boss_ref(), attack_state)
            .with_animation_frame(anim);
        active_attack_volumes(&ctx)
    };

    // Pass 1 — update existing strike hitboxes in place (preserving `HitboxHits`
    // dedup), or despawn ones whose strike/part ended.
    let mut present: std::collections::HashSet<(Entity, usize)> = std::collections::HashSet::new();
    for (hb_entity, marker, mut hitbox, mut lifetime) in &mut strikes {
        let owner = hitbox.owner;
        let volumes = volumes_for(owner);
        let Some(v) = volumes.get(marker.part).copied() else {
            commands.entity(hb_entity).despawn();
            continue;
        };
        let owner_pos = bosses.get(owner).map(|(_, f, ..)| f.kin.pos).unwrap_or_default();
        hitbox.half_extent = v.half_size();
        hitbox.anchor = HitboxAnchor::FollowOwner {
            local_offset: v.center() - owner_pos,
        };
        hitbox.frame_down = gravity.dir_at(owner_pos);
        lifetime.remaining_s = BOSS_STRIKE_LIFETIME_S;
        present.insert((owner, marker.part));
    }

    // Pass 2 — spawn any missing part for a live strike.
    for (boss_entity, feature, health, attack_state, brain, anim) in &bosses {
        if !health.alive() || brain.is_player() {
            continue;
        }
        let ctx = BossVolumeContext::from_ref(feature.as_boss_ref(), attack_state)
            .with_animation_frame(anim);
        let volumes = active_attack_volumes(&ctx);
        let owner_pos = feature.kin.pos;
        let frame_down = gravity.dir_at(owner_pos);
        let damage = feature.config.behavior.attack_damage.max(1);
        for (i, v) in volumes.iter().enumerate() {
            if present.contains(&(boss_entity, i)) {
                continue;
            }
            commands.spawn((
                Hitbox {
                    owner: boss_entity,
                    source: crate::combat::components::ActorFaction::Boss,
                    anchor: HitboxAnchor::FollowOwner {
                        local_offset: v.center() - owner_pos,
                    },
                    half_extent: v.half_size(),
                    shape: None,
                    facing: 1.0,
                    damage,
                    knockback_strength: BOSS_STRIKE_KNOCKBACK,
                    knock_x: 0.0,
                    frame_down,
                },
                HitboxLifetime {
                    remaining_s: BOSS_STRIKE_LIFETIME_S,
                },
                HitboxHits::default(),
                FrameDrivenBossStrike { part: i },
            ));
        }
    }
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

/// TRIGGER the boss's data-driven special MOVES: while a `Special(key)` profile is
/// the boss's active attack, ensure its sustain-move is playing (insert `MovePlayback`
/// from the boss's `ActorMoveset` for that key). The move's per-frame `Effect{key}`
/// (a sustain window, [`crate::features::boss_special_moveset`]) fires the content
/// technique through the SHARED `dispatch_move_events` bridge — so the boss's special
/// runs through the SAME moveset runtime the actor's does, retiring the boss-only
/// `dispatch_boss_special` (fable review §A1: the boss special path unifies with the
/// actor's). `Without<MovePlayback>` gates re-trigger; the move duration equals the
/// strike window (both on sim time), so the sustain lasts exactly the strike. Geometry
/// profiles have no move → they stay on `sync_boss_strike_hitboxes`. Possession routes
/// here too: its input map sets `active_profile`, and this fires the mapped move.
pub fn trigger_boss_special_moves(
    mut commands: Commands,
    bosses: Query<
        (
            Entity,
            &BossAttackState,
            &crate::combat::moveset::ActorMoveset,
            &crate::actor::BodyKinematics,
        ),
        (
            With<FeatureSimEntity>,
            Without<crate::combat::moveset::MovePlayback>,
        ),
    >,
) {
    for (entity, attack_state, moveset, kin) in &bosses {
        let Some(key) = attack_state
            .active_profile
            .as_ref()
            .and_then(|p| p.special_key())
        else {
            continue;
        };
        if let Some(spec) = moveset.0.move_by_id(key) {
            commands.entity(entity).insert(
                crate::combat::moveset::MovePlayback::new(spec.clone(), kin.facing),
            );
        }
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
                    // setting `active_profile` above is enough — `trigger_boss_special_moves`
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
        // `BossAttackState` above is the whole wiring — `trigger_boss_special_moves`
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
/// frame-driven Boss hitboxes ([`sync_boss_strike_hitboxes`] → `apply_hitbox_damage`);
/// BODY-CONTACT damage is the shared `apply_actor_contact_damage`. The old
/// `boss_attack_damage` poll is gone — a boss's offense and body flow through the
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
mod special_moveset_tests {
    use super::*;
    use ambition_characters::brain::{BossAttackProfile, BossCapability};

    /// Boss-fold slice (fable review §A1): the boss's content-technique special runs
    /// through the SHARED moveset. (a) `boss_special_moveset` generates a sustain-move
    /// per `Special(key)` (skipping geometry profiles); (b) `trigger_boss_special_moves`
    /// starts that move while the profile is the boss's `active_profile`. The
    /// sustain→`Effect{key}`→`Special{key}`→technique tail is pinned by the moveset
    /// dispatch/sustain tests, so this covers the new boss-side links end to end.
    #[test]
    fn a_boss_special_profile_triggers_its_sustain_move() {
        let cap = BossCapability {
            specials: vec![
                (BossAttackProfile::FloorSlam, 0.3), // geometry → no move
                (BossAttackProfile::Special("apple_rain".to_string()), 2.0),
            ],
        };
        let moveset =
            crate::features::boss_special_moveset(&cap).expect("a content special → a moveset");
        assert!(
            moveset.0.move_by_id("apple_rain").is_some(),
            "the Special profile became a move"
        );
        assert_eq!(
            moveset.0.moves.len(),
            1,
            "the geometry profile authored NO move (it stays on the hitbox path)"
        );

        let mut app = App::new();
        app.add_systems(Update, trigger_boss_special_moves);
        let mut attack_state = BossAttackState::default();
        attack_state.active_profile = Some(BossAttackProfile::Special("apple_rain".to_string()));
        let boss = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                attack_state,
                moveset,
                crate::actor::BodyKinematics {
                    pos: ambition_engine_core::Vec2::ZERO,
                    vel: ambition_engine_core::Vec2::ZERO,
                    size: ambition_engine_core::Vec2::new(40.0, 40.0),
                    facing: 1.0,
                },
            ))
            .id();
        app.update();
        let pb = app
            .world()
            .get::<crate::combat::moveset::MovePlayback>(boss)
            .expect("the active Special profile started its moveset move");
        assert_eq!(pb.spec.id, "apple_rain");
        // The move SUSTAINS the effect for the strike window (2.0s), so the
        // technique gets the per-frame "active this tick" signal.
        assert_eq!(pb.spec.duration_s, 2.0);
        assert_eq!(
            pb.spec.windows[0].sustain_effect.as_deref(),
            Some("apple_rain")
        );
    }
}
