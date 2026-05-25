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
    action_set::ActionRequest, boss_pattern::tick_boss_pattern, ActorActionMessage, ActorControl,
    BossAttackState, BossPatternContext, Brain, StateMachineCfg,
};
use crate::content::features::bosses::BossSpriteMetrics;
use crate::features::{
    boss_attack_damage, boss_special_for_profile, bounding_aabb, BossVolumeContext,
};
use crate::presentation::character_sprites::registry::SheetRegistry;
use ambition_engine::AabbExt;
use bevy::prelude::{Commands, MessageWriter};

/// Marker that a boss entity has had its sprite metrics applied
/// (once-per-boss derivation gate). Inserted by
/// [`derive_boss_sprite_metrics`] when it walks a new boss.
#[derive(Component, Clone, Copy, Debug)]
pub struct BossSpriteMetricsApplied;

/// Map a boss `BossBehaviorProfile::id` to its sprite-registry
/// target id. The sprite generator's `target` field doesn't always
/// match the boss internal id — clockwork_warden / gradient_sentinel
/// both share the generic `"boss"` sheet. Future RON-authored
/// boss specs can carry their own sprite target string; for now
/// this match is the single mapping point.
pub fn sprite_target_for_boss(behavior_id: &str) -> &str {
    match behavior_id {
        "clockwork_warden" | "gradient_sentinel" => "boss",
        other => other,
    }
}

/// Read the sprite registry for each freshly-spawned boss and copy
/// its `body_metrics` into `BossRuntime::sprite_metrics`. Also
/// derives an updated `combat_size` from the bounding box of the
/// body parts so the boss's collision + soft world-bounds clamp
/// scales with the visible sprite body instead of the LDtk
/// BossSpawn AABB.
///
/// Gated by the `BossSpriteMetricsApplied` marker so each boss is
/// processed exactly once. Skips bosses whose sprite target isn't
/// in the registry (the boss keeps its authored / fallback
/// combat_size).
///
/// When the boss's brain is `BossPattern { cfg, .. }`, the system
/// also writes the derived combat_size into `cfg.combat_size` so
/// the brain's soft world-bounds clamp matches the new physical
/// envelope (otherwise the brain would still clamp against the
/// stale 64×80 spawn AABB).
pub fn derive_boss_sprite_metrics(
    mut commands: Commands,
    registry: Option<Res<SheetRegistry>>,
    mut bosses: Query<
        (Entity, &mut BossFeature, Option<&mut Brain>),
        (With<FeatureSimEntity>, Without<BossSpriteMetricsApplied>),
    >,
) {
    let Some(registry) = registry else {
        // Headless / minimal-plugin tests don't init the sprite
        // registry. With no metadata available, the derivation is a
        // no-op — boss keeps its hardcoded `combat_size`.
        return;
    };
    if registry.is_empty() {
        // Registry hasn't loaded yet — retry next frame. Don't
        // insert the gate marker so the next tick re-attempts.
        return;
    }
    for (entity, mut feature, brain_opt) in &mut bosses {
        let boss = &mut feature.boss;
        let target = sprite_target_for_boss(&boss.behavior.id);
        let Some((metrics, frame_w, frame_h)) = registry.body_metrics(target) else {
            // No metadata for this boss — leave defaults alone.
            commands.entity(entity).insert(BossSpriteMetricsApplied);
            continue;
        };

        let snapshot = BossSpriteMetrics {
            frame_width: frame_w,
            frame_height: frame_h,
            body_pixel_bbox: metrics.body_pixel_bbox,
            body_pixel_parts: metrics.body_pixel_parts.clone(),
        };
        let has_body = snapshot.has_body();
        boss.sprite_metrics = Some(snapshot);

        if has_body {
            // Derive combat_size from the bounding AABB of all body
            // parts (or the single bbox). Use the current sprite
            // render size (boss.size) as the scale base so a larger
            // boss instance (e.g. boss-lab variant) scales up
            // correctly.
            let snapshot = boss.sprite_metrics.as_ref().expect("just inserted");
            let body_aabbs = crate::features::world_space_body_aabbs_from_parts(
                &snapshot.body_pixel_parts,
                snapshot.body_pixel_bbox,
                frame_w,
                frame_h,
                boss.pos,
                boss.size,
            );
            if let Some(bound) = bounding_aabb(&body_aabbs) {
                let derived = bound.half_size() * 2.0;
                boss.behavior.combat_size = Some(derived);
                // Mirror into the brain cfg so the soft world-bounds
                // clamp uses the new value too.
                if let Some(mut brain) = brain_opt {
                    if let Brain::StateMachine(StateMachineCfg::BossPattern { cfg, .. }) =
                        &mut *brain
                    {
                        cfg.combat_size = derived;
                    }
                }
            }
        }
        commands.entity(entity).insert(BossSpriteMetricsApplied);
    }
}

/// Sync each boss's `encounter_phase` mirror from `BossEncounterRegistry`.
/// Runs before [`tick_boss_brains_system`] so the brain sees this
/// frame's phase.
///
/// **Encounter id resolution**: uses `boss.behavior.id` (canonical
/// id resolved at spawn from the brain's `PhaseScript:` payload),
/// not `encounter_id_from_name(boss.name)`. The two diverge when an
/// LDtk BossSpawn carries a flavor name like "System Boss" plus a
/// `PhaseScript:clockwork_warden` brain — `boss.name` ≠
/// `behavior.id`, and using the name would miss the registry
/// entry, leaving the boss permanently Dormant (no attacks).
pub fn sync_boss_encounter_phase(
    encounter_registry: Res<crate::boss_encounter::BossEncounterRegistry>,
    mut bosses: Query<&mut BossFeature, With<FeatureSimEntity>>,
    mut last_logged: bevy::ecs::system::Local<
        std::collections::HashMap<String, ae::BossEncounterPhase>,
    >,
) {
    for mut feature in &mut bosses {
        let boss = &mut feature.boss;
        let lookup = encounter_registry.get(&boss.behavior.id);
        let new_phase = lookup.map(|s| s.phase);
        // Log phase transitions per boss so we can see in the logs
        // when (or if) Dormant → Intro → Phase1 actually fires.
        let prev = last_logged.get(&boss.behavior.id).copied();
        if new_phase != prev {
            match (lookup, new_phase) {
                (Some(_), Some(phase)) => {
                    bevy::log::info!(
                        target: "ambition::boss_encounter",
                        "sync_phase: boss={} (behavior.id={}) phase {:?} → {:?}",
                        boss.id,
                        boss.behavior.id,
                        prev,
                        phase,
                    );
                    last_logged.insert(boss.behavior.id.clone(), phase);
                }
                (None, _) => {
                    bevy::log::warn!(
                        target: "ambition::boss_encounter",
                        "sync_phase: boss={} behavior.id={} NOT IN encounter_registry (boss.encounter_phase stays {:?})",
                        boss.id,
                        boss.behavior.id,
                        boss.encounter_phase,
                    );
                    last_logged.insert(boss.behavior.id.clone(), boss.encounter_phase);
                }
                _ => {}
            }
        }
        if let Some(phase) = new_phase {
            boss.encounter_phase = phase;
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
            bevy::ecs::entity::Entity,
            &BossFeature,
            &mut Brain,
            &mut ActorControl,
            &mut BossAttackState,
            &super::super::components::ActorTarget,
        ),
        With<FeatureSimEntity>,
    >,
    mut action_messages: MessageWriter<ActorActionMessage>,
) {
    let dt = world_time.sim_dt();
    for (entity, feature, mut brain, mut control, mut attack_state, target) in &mut bosses {
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

        // Boss-side Special direct-write: the Gradient Sentinel has
        // four distinct specials (OverfitVolley / MinimaTrap /
        // SaddlePoint / GradientCascade) which doesn't fit
        // `ActionSet`'s single special slot. Rather than grow the
        // ActionSet schema or the ActorControlFrame, the boss tick
        // writes `ActorActionMessage::Special { spec }` directly
        // using `boss_special_for_profile` to look up the spec from
        // the live `BossAttackState.active_profile`. The boss's
        // `ActionSet.special` is set to `None` for multi-special
        // bosses (see `spawn_boss`) so the generic
        // `emit_brain_action_messages` resolver doesn't fire a
        // duplicate. GNU-ton's apple rain takes the same path so all
        // boss specials share one wiring.
        if frame.special_pressed {
            if let Some(profile) = attack_state.active_profile.as_ref() {
                if let Some(spec) = boss_special_for_profile(profile, boss) {
                    action_messages.write(ActorActionMessage {
                        actor: entity,
                        request: ActionRequest::Special { spec },
                    });
                }
            }
        }
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
