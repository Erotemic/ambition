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
use crate::engine_core::AabbExt;
use crate::features::{
    boss_attack_damage, boss_special_for_profile, bounding_aabb, BossVolumeContext,
};
use crate::presentation::character_sprites::registry::SheetRegistry;
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
        // GNU-ton's hand-tuned `gnu_ton_part_aabb` math was migrated
        // into the standard `body_metrics` pipeline (the
        // `gnu_ton_boss_spritesheet.ron`'s `animations` block) on
        // 2026-05-26. Map the behavior id to that sheet target so
        // `derive_boss_sprite_metrics` picks up the per-animation
        // hitboxes / hurtboxes.
        "gnu_ton" => "gnu_ton_boss",
        other => other,
    }
}

/// World-space size of the rendered sprite quad for a boss, given the
/// boss's spawn / collision size and its sprite target.
///
/// The visible sprite is rendered at `max(size) * collision_scale`,
/// where `collision_scale` is per-sheet (1.6 for the clockwork /
/// gradient sentinel `BOSS_SHEET`, 1.25 for the mockingbird sheet,
/// 4.5 for GNU-ton). The hurtbox / hitbox math needs THIS value
/// (not `boss.size`) as the world scale so the cyan / red / yellow
/// boxes cover the visible body. Otherwise the boxes end up half
/// the size of what the player sees.
///
/// Unknown targets get a 1.0 scale fallback (sprite renders at
/// `boss.size`) — that's the safe "no sprite spec known" case used
/// by test fixtures and bosses without a registered sheet.
pub fn sprite_render_size_for(target: &str, boss_size: ae::Vec2) -> ae::Vec2 {
    use crate::boss_encounter::sprites;
    let spec = match target {
        "boss" => Some(sprites::BOSS_SHEET),
        "mockingbird" => Some(sprites::MOCKINGBIRD_SHEET),
        // `gnu_ton_boss` is the actual sheet target ID emitted by the
        // gnu_ton spritesheet RON. `gnu_ton_body` / `gnu_ton_hands` /
        // `gnu_ton` (legacy aliases) stay mapped for compatibility.
        "gnu_ton_boss" | "gnu_ton" | "gnu_ton_body" | "gnu_ton_hands" => {
            Some(sprites::GNU_TON_SHEET)
        }
        _ => None,
    };
    let Some(spec) = spec else {
        return boss_size;
    };
    let bevy_size = bevy::math::Vec2::new(boss_size.x, boss_size.y);
    let render = spec.render_size(bevy_size);
    ae::Vec2::new(render.x, render.y)
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

        let sprite_render_size = sprite_render_size_for(target, boss.size);
        let mut snapshot = BossSpriteMetrics {
            frame_width: frame_w,
            frame_height: frame_h,
            body_pixel_bbox: metrics.body_pixel_bbox,
            body_pixel_parts: metrics.body_pixel_parts.clone(),
            sprite_render_size,
            // Provisional — overwritten below once we know whether
            // there's a body bbox to derive an offset from.
            combat_offset: ae::Vec2::ZERO,
            animations: metrics.animations.clone(),
        };

        // Derive combat_size + combat_offset from the bounding AABB
        // of all body parts (or the single bbox). Use the SPRITE
        // RENDER SIZE (not `boss.size`) as the world-scale base —
        // the visible sprite is rendered at
        // `boss_asset.spec.render_size(boss.size) = max(boss.size)
        // * collision_scale`, which is bigger than the LDtk spawn
        // AABB. Scaling combat_size to render size means the orange
        // (combat) box and magenta (body-contact damage) box both
        // cover the visible body instead of half of it.
        //
        // The `combat_offset` (bound.center() - boss.pos) captures
        // the fact that the body bbox isn't necessarily centered in
        // the sprite frame — without it, `boss.aabb()` sits at
        // `boss.pos`, but the visible body is offset ~41 px above,
        // so the pogo zone and orange debug box land "below" the
        // visible sprite and pogo doesn't fire where the player aims.
        let body_aabbs = crate::features::world_space_body_aabbs_from_parts(
            &snapshot.body_pixel_parts,
            snapshot.body_pixel_bbox,
            frame_w,
            frame_h,
            boss.pos,
            sprite_render_size,
        );
        let derive_result = bounding_aabb(&body_aabbs);
        if let Some(bound) = derive_result {
            snapshot.combat_offset = bound.center() - boss.pos;
        }
        boss.sprite_metrics = Some(snapshot);
        if let Some(bound) = derive_result {
            let derived = bound.half_size() * 2.0;
            boss.behavior.combat_size = Some(derived);
            // Mirror into the brain cfg so the soft world-bounds
            // clamp uses the new value too.
            if let Some(mut brain) = brain_opt {
                if let Brain::StateMachine(StateMachineCfg::BossPattern { cfg, .. }) = &mut *brain {
                    cfg.combat_size = derived;
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
        std::collections::HashMap<String, crate::boss_encounter::BossEncounterPhase>,
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
            control.0 = crate::actor_control::ActorControlFrame::neutral();
            attack_state.clear();
            continue;
        }

        let StateMachineCfg::BossPattern { cfg, state } = pattern_brain_mut(&mut brain) else {
            // Boss has a non-BossPattern brain (test fixture). Leave
            // ActorControl + BossAttackState neutral so a future
            // brain swap doesn't leak stale intent.
            control.0 = crate::actor_control::ActorControlFrame::neutral();
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
        let mut frame = crate::actor_control::ActorControlFrame::neutral();
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
    mut hit_events: MessageWriter<HitEvent>,
    // Bosses target the primary player today. Real multiplayer
    // boss AI (per-player targeting, agro lists, phase transitions
    // that respond to multiple players) is a deeper redesign than
    // the iterate-all-players pattern used by hazards / projectiles
    // — see OVERNIGHT-TODO #17.8 "Generalize enemy targeting." The
    // `PrimaryPlayerOnly` filter documents the targeting decision
    // at the query rather than leaving it as an implicit
    // `single()` semantic.
    // Per-boss target via `ActorTarget` (populated by
    // `select_actor_targets`). Read each boss's targeted player by
    // Entity from the all-players query — single-player behavior is
    // preserved because there's only one player today.
    player_query: Query<
        (
            &crate::player::PlayerKinematics,
            &crate::player::PlayerOffense,
            &crate::player::PlayerDodgeState,
            &crate::player::PlayerShieldState,
            &crate::player::PlayerCombatState,
        ),
        With<crate::player::PlayerEntity>,
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
            &super::super::components::ActorTarget,
        ),
        With<FeatureSimEntity>,
    >,
) {
    // Sim clock: bosses must slow with bullet-time (ADR 0010); a
    // boss locked-on to the player should not get free hits when
    // the player triggers bullet-time mid-pattern.
    let dt = world_time.sim_dt();
    let feature_world = world_with_sandbox_solids(&world.0, &platform_set.0, &overlay);
    for (
        mut aabb,
        mut feature,
        mut pattern_timer,
        mut phase,
        control,
        attack_state,
        brain,
        actor_target,
    ) in &mut bosses
    {
        // Resolve this boss's targeted player. If the target's
        // entity has despawned or no players exist, skip the body-
        // contact check — body still integrates so the boss keeps
        // animating its pattern. `target_entity` is threaded onto
        // the emitted `HitEvent::target` so the player-side reader
        // lands boss-attack damage on this specific player.
        let target_entity = actor_target.entity;
        let target_player = target_entity.and_then(|e| player_query.get(e).ok());
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
        let (Some(target_entity), Some((kin, offense, dodge, shield, combat))) =
            (target_entity, target_player)
        else {
            continue;
        };
        let player_body = kin.aabb();
        let dodge_rolling = dodge.roll_timer > 0.0;
        let player_vulnerable =
            !offense.invincible && !dodge_rolling && !shield.parrying() && combat.vulnerable();
        if player_vulnerable && boss.alive {
            let ctx = BossVolumeContext::from_runtime(boss, attack_state);
            if let Some(damage) = boss_attack_damage(&ctx, target_entity, player_body) {
                let pos = damage
                    .knockback
                    .as_ref()
                    .map(|k| k.impact_pos)
                    .unwrap_or_else(|| damage.volume.center());
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
                hit_events.write(damage);
            }
        }
    }
}
