//! Boss systems: brain tick (intent), encounter-phase forwarding,
//! sandbox-aware integration, and contact-damage publication.
//!
//! Three Bevy systems live here, chained in the `WorldPrep` set:
//!
//! 1. [`sync_boss_encounter_phase`] — copy the active encounter
//!    phase from `BossEncounterRegistry` into each boss's
//!    `BossRuntime::encounter_phase` mirror. Runs first so the
//!    brain tick below sees the current phase this frame.
//! 2. [`tick_boss_brains_system`] — for every boss with a
//!    `Brain::StateMachine(BossPattern)`, build a
//!    [`BossPatternContext`], call [`tick_boss_pattern`], and
//!    write the resulting [`ActorControlFrame`] + [`BossAttackState`].
//!    Also syncs `BossAttackState` → `BossRuntime` mirror fields
//!    (`active_strike_profile`, `telegraph_profile`, `attack_timer`,
//!    `attack_windup_timer`) so legacy volume / damage readers
//!    keep working without per-call changes.
//! 3. [`update_ecs_bosses`] — **integration only**. Reads
//!    `ActorControl::0.desired_vel`, integrates the boss body via
//!    `BossRuntime::integrate_body`, syncs `FeatureAabb` /
//!    `BossPatternTimer` / `BossPhase`, and publishes body-contact
//!    damage. Does NOT call `boss.update(...)`, does NOT overwrite
//!    `ActorControl`, does NOT choose pattern steps. Per the
//!    "move boss policy out of BossRuntime" migration: the brain
//!    decides; the runtime integrates.

use super::*;

use crate::brain::{
    boss_pattern::tick_boss_pattern, ActorControl, BossAttackState, BossPatternContext, Brain,
    StateMachineCfg,
};

/// Sync each boss's `encounter_phase` mirror from `BossEncounterRegistry`.
/// Runs before [`tick_boss_brains_system`] so the brain sees this
/// frame's phase.
pub fn sync_boss_encounter_phase(
    encounter_registry: Res<crate::boss_encounter::BossEncounterRegistry>,
    mut bosses: Query<&mut BossFeature, With<FeatureSimEntity>>,
) {
    for mut feature in &mut bosses {
        let boss = &mut feature.boss;
        let encounter_id = crate::boss_encounter::encounter_id_from_name(&boss.name);
        if let Some(state) = encounter_registry.get(&encounter_id) {
            boss.encounter_phase = state.phase;
        }
    }
}

/// Tick every boss's `BossPattern` brain: advance the cursor, emit
/// `ActorControlFrame` intent (movement + melee/special edges), and
/// update the `BossAttackState` component. Then mirror
/// `BossAttackState` → `BossRuntime` execution fields so legacy
/// volume / damage readers keep working.
pub fn tick_boss_brains_system(
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    mut bosses: Query<
        (
            &mut BossFeature,
            &mut Brain,
            &mut ActorControl,
            &mut BossAttackState,
            &super::super::components::ActorTarget,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    for (mut feature, mut brain, mut control, mut attack_state, target) in &mut bosses {
        let boss = &mut feature.boss;
        if !boss.alive {
            // Dead boss: zero out frame + attack state so any
            // downstream consumer sees a coherent "no intent".
            control.0 = ae::ActorControlFrame::neutral();
            attack_state.clear();
            sync_runtime_mirror_from_attack_state(boss, &attack_state);
            continue;
        }

        // Free-running clocks the runtime still owns (legacy
        // cycle-mode volume rendering reads `pattern_timer`).
        boss.tick_runtime_clocks(dt);

        let StateMachineCfg::BossPattern { cfg, state } = pattern_brain_mut(&mut brain) else {
            // Boss has a non-BossPattern brain (e.g. test fixture
            // attaches StandStill). Skip the tick + mirror clear so
            // the brain's neutral output stays authoritative.
            control.0 = ae::ActorControlFrame::neutral();
            attack_state.clear();
            sync_runtime_mirror_from_attack_state(boss, &attack_state);
            continue;
        };

        let ctx = BossPatternContext {
            encounter_phase: boss.encounter_phase,
            actor_pos: boss.pos,
            target_pos: target.pos,
            world_size: world.0.size,
            dt,
        };
        let mut frame = ae::ActorControlFrame::neutral();
        tick_boss_pattern(cfg, state, &ctx, &mut frame, &mut attack_state);
        control.0 = frame;
        sync_runtime_mirror_from_attack_state(boss, &attack_state);
    }
}

/// Helper: dig out the `&mut BossPattern { cfg, state }` from a
/// `Brain`. Returns `None` for non-BossPattern brains.
fn pattern_brain_mut(brain: &mut Brain) -> &mut StateMachineCfg {
    match brain {
        Brain::StateMachine(cfg) => cfg,
        // Player-brain bosses aren't a thing today; treat as the
        // empty path. The caller's match below handles it as a
        // non-`BossPattern` variant.
        Brain::Player(_) => {
            // Compile-error proxy: we can't return a real ref to a
            // non-existent inner type, so we cheat with a static
            // mut wouldn't compile either. Instead use the slightly
            // ugly trick of stashing a StandStill in place. This
            // branch is unreachable in production (only spawn paths
            // attach BossPattern) but we want a safe fallback.
            unreachable!("Boss entities are never spawned with Brain::Player")
        }
    }
}

/// Push `BossAttackState` into the `BossRuntime` mirror fields the
/// legacy volume / damage readers (`attack_volumes`,
/// `attack_telegraph_volumes`, `player_damage`) still consult. Once
/// those readers migrate to query the `BossAttackState` component
/// directly, this sync goes away.
fn sync_runtime_mirror_from_attack_state(boss: &mut BossRuntime, attack_state: &BossAttackState) {
    boss.active_strike_profile = attack_state.active_profile.clone();
    boss.telegraph_profile = attack_state.telegraph_profile.clone();
    boss.attack_timer = attack_state.active_remaining;
    boss.attack_windup_timer = attack_state.telegraph_remaining;
}

/// Integrate ECS-authored bosses + publish body-contact damage.
///
/// **Integration only.** The brain (`tick_boss_brains_system`) owns
/// the intent decision and has already written `ActorControl` +
/// `BossAttackState` by the time this system runs. This system reads
/// `ActorControl::0.desired_vel`, hands it to
/// `BossRuntime::integrate_body`, syncs presentation mirrors
/// (`FeatureAabb`, `BossPatternTimer`, `BossPhase`), and emits
/// `PlayerDamageEvent` on body contact.
pub fn update_ecs_bosses(
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    platform_set: Res<crate::MovingPlatformSet>,
    overlay: Res<FeatureEcsWorldOverlay>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
    mut vfx: MessageWriter<crate::presentation::fx::VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
    mut player_damage: MessageWriter<PlayerDamageEvent>,
    // Bosses target the primary player today. Real multiplayer
    // boss AI (per-player targeting, agro lists, phase transitions
    // that respond to multiple players) is a deeper redesign than
    // the iterate-all-players pattern used by hazards / projectiles
    // — see OVERNIGHT-TODO #17.8 "Generalize enemy targeting." The
    // `PrimaryPlayerOnly` filter documents the targeting decision
    // at the query rather than leaving it as an implicit
    // `single()` semantic.
    player_query: Query<
        (
            &crate::player::PlayerBody,
            &crate::player::PlayerCombatState,
        ),
        crate::player::PrimaryPlayerOnly,
    >,
    mut bosses: Query<
        (
            &mut FeatureAabb,
            &mut BossFeature,
            &mut BossPatternTimer,
            &mut BossPhase,
            &ActorControl,
        ),
        With<FeatureSimEntity>,
    >,
) {
    // Sim clock: bosses must slow with bullet-time (ADR 0010); a
    // boss locked-on to the player should not get free hits when
    // the player triggers bullet-time mid-pattern.
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    let Ok((pb, combat)) = player_query.single() else {
        return;
    };
    let player_body = pb.aabb();
    let player_vulnerable =
        !pb.invincible && !pb.dodge_rolling && !pb.parrying && combat.vulnerable();
    for (mut aabb, mut feature, mut pattern_timer, mut phase, control) in &mut bosses {
        let boss = &mut feature.boss;
        // Integration: take the brain-emitted desired_vel and let
        // `step_kinematic` translate it into a collision-resolved
        // position change. The brain decided what we want; the
        // runtime decides what's actually possible.
        boss.integrate_body(&feature_world, control.0.desired_vel, dt);
        aabb.center = boss.pos;
        aabb.half_size = boss.render_size() * 0.5;
        pattern_timer.0 = boss.pattern_timer;
        *phase = BossPhase::from_alive(boss.alive);
        if player_vulnerable && boss.alive {
            if let Some(damage) = boss.player_damage(player_body) {
                let pos = damage.impact_pos;
                sfx.write(crate::audio::SfxMessage::Play {
                    id: ambition_sfx::ids::PLAYER_DAMAGE,
                    pos,
                });
                vfx.write(VfxMessage::Impact { pos });
                vfx.write(VfxMessage::Burst {
                    pos,
                    count: 14,
                    speed: 300.0,
                    color: [1.0, 0.34, 0.28, 0.88],
                    kind: ParticleKind::Shard,
                });
                debris.write(DebrisBurstMessage {
                    pos,
                    cue: PhysicsDebrisCue::Impact,
                });
                player_damage.write(damage);
            }
        }
    }
}
