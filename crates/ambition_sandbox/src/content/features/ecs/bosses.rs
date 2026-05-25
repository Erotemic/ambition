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
//!    `BossAttackState` is the single source of truth for boss
//!    attack state — debug overlay, damage application, and
//!    vulnerable-volume rendering all read from it via the pure
//!    helpers in `content/features/boss_attack_geometry`.
//! 3. [`update_ecs_bosses`] — **integration only**. Reads
//!    `ActorControl::0.desired_vel`, integrates the boss body via
//!    `BossRuntime::integrate_body`, syncs presentation mirrors,
//!    and publishes both strike and body-contact damage by calling
//!    `boss_attack_damage` against the boss's `BossAttackState` —
//!    no runtime attack-state fields are involved.

use super::*;

use crate::brain::{
    boss_pattern::tick_boss_pattern, ActorControl, BossAttackState, BossPatternContext, Brain,
    StateMachineCfg,
};
use crate::features::{boss_attack_damage, BossVolumeContext};

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
/// update the `BossAttackState` component. `BossAttackState` is the
/// single source of truth for boss attack state from this point on;
/// the runtime no longer carries mirror fields and the volume /
/// damage / debug-overlay paths all query it.
pub fn tick_boss_brains_system(
    world_time: Res<WorldTime>,
    world: Res<crate::GameWorld>,
    mut bosses: Query<
        (
            &BossFeature,
            &mut Brain,
            &mut ActorControl,
            &mut BossAttackState,
            &super::super::components::ActorTarget,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let dt = world_time.sim_dt();
    for (feature, mut brain, mut control, mut attack_state, target) in &mut bosses {
        let boss = &feature.boss;
        if !boss.alive {
            // Dead boss: zero out frame + attack state so any
            // downstream consumer sees a coherent "no intent".
            control.0 = ae::ActorControlFrame::neutral();
            attack_state.clear();
            continue;
        }

        let StateMachineCfg::BossPattern { cfg, state } = pattern_brain_mut(&mut brain) else {
            // Boss has a non-BossPattern brain (test fixture). Leave
            // ActorControl + BossAttackState neutral so a future
            // brain swap doesn't leak stale intent.
            control.0 = ae::ActorControlFrame::neutral();
            attack_state.clear();
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
    }
}

/// Helper: dig out the `&mut StateMachineCfg` from a `Brain`.
/// Bosses never spawn with `Brain::Player`; the `unreachable!` arm
/// is a safety net for that invariant.
fn pattern_brain_mut(brain: &mut Brain) -> &mut StateMachineCfg {
    match brain {
        Brain::StateMachine(cfg) => cfg,
        Brain::Player(_) => unreachable!("Boss entities are never spawned with Brain::Player"),
    }
}

/// Integrate ECS-authored bosses + publish damage. The brain
/// (`tick_boss_brains_system`) owns intent and has already written
/// `ActorControl` + `BossAttackState` by the time this system runs.
///
/// This system:
/// 1. Integrates the boss body using `ActorControl::0.desired_vel`.
/// 2. Syncs presentation mirrors (`FeatureAabb`, `BossPatternTimer`,
///    `BossPhase`).
/// 3. Publishes attack + body-contact damage via the pure
///    `boss_attack_damage` helper, which reads `BossAttackState`
///    directly (no runtime mirror fields involved).
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
            &BossAttackState,
            &Brain,
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
    for (mut aabb, mut feature, mut pattern_timer, mut phase, control, attack_state, brain) in
        &mut bosses
    {
        let boss = &mut feature.boss;
        // Integration: take the brain-emitted desired_vel and let
        // `step_kinematic` translate it into a collision-resolved
        // position change. The brain decided what we want; the
        // runtime decides what's actually possible.
        boss.integrate_body(&feature_world, control.0.desired_vel, dt);
        aabb.center = boss.pos;
        aabb.half_size = boss.render_size() * 0.5;
        // Mirror the brain's pattern_timer (now living in
        // `BossPatternState`) into the presentation-side
        // `BossPatternTimer` component for sprite-animation
        // consumers. Defaults to 0 when the boss has a non-BossPattern
        // brain (test fixtures).
        pattern_timer.0 = match brain {
            Brain::StateMachine(StateMachineCfg::BossPattern { state, .. }) => state.pattern_timer,
            _ => 0.0,
        };
        *phase = BossPhase::from_alive(boss.alive);
        if player_vulnerable && boss.alive {
            let ctx = BossVolumeContext::from_runtime(boss, attack_state);
            if let Some(damage) = boss_attack_damage(&ctx, player_body) {
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
