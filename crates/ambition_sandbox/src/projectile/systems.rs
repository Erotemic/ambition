//! The Bevy gameplay system that ticks projectiles, samples motion
//! input, and routes hits through ECS-native feature damage messages.

use crate::engine_core as ae;
use bevy::prelude::*;

use super::collision::{resolve_world_collision, WorldHitOutcome, WorldHitPolicy};
use super::diagnostics::log_press_diagnostics;
use super::state::{PlayerProjectileState, ProjectileTraceEvent};
use crate::audio::SfxMessage;
use crate::features::{
    ActorCombatState, ActorDisposition, BossClusterRef, BossConfig, BreakableFeature, FeatureAabb,
    FeatureId, FeatureSimEntity, HitEvent, HitSource,
};
use crate::presentation::fx::VfxMessage;
use crate::trace::GameplayTraceBuffer;
use crate::GameWorld;

pub fn update_projectiles(
    world_time: Res<crate::WorldTime>,
    world: Res<GameWorld>,
    gravity: crate::physics::GravityCtx,
    // Per-player projectile state lives on the player entity itself
    // (was a singleton `Res<PlayerProjectileState>`). Iterates every
    // player so co-op / possession builds get one independent charge
    // timer + body list per player without sharing a singleton.
    mut player_q: Query<
        (
            Entity,
            &crate::player::PlayerKinematics,
            &mut crate::projectile::PlayerProjectileState,
            &mut crate::player::PlayerAnimState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    mut brain_actions: MessageReader<crate::brain::ActorActionMessage>,
    user_settings: Res<crate::persistence::settings::UserSettings>,
    mut trace: ResMut<GameplayTraceBuffer>,
    mut feature_damage: MessageWriter<HitEvent>,
    ecs_breakables: Query<(&FeatureId, &FeatureAabb, &BreakableFeature), With<FeatureSimEntity>>,
    ecs_actors: Query<
        (
            &FeatureId,
            &FeatureAabb,
            &ActorDisposition,
            &ActorCombatState,
        ),
        (With<FeatureSimEntity>, Without<BossConfig>),
    >,
    ecs_bosses: Query<
        (
            &FeatureId,
            &FeatureAabb,
            BossClusterRef,
            &crate::brain::BossAttackState,
            // Live rendered frame, so the projectile hit-check uses the
            // same head position as the damage path + the drawn hurtbox.
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<FeatureSimEntity>,
    >,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    // Sim clock: projectile motion + spawner pacing freeze in
    // bullet-time alongside the rest of the world (ADR 0010). A
    // projectile in mid-arc should not advance while the world
    // is stopped.
    let dt = world_time.sim_dt();
    // Localized gravity: each projectile body resolves its own gravity by
    // position below, so a shot crossing a gravity column bends the column's way.

    // Build a per-actor map of PlayerProjectileTick infos so the
    // per-player loop below can look up each player's tick info by
    // entity without re-iterating the message stream. The brain-side
    // emitter (`emit_player_projectile_tick_messages`) produces
    // exactly one per player-brain actor per tick.
    let tick_infos: std::collections::HashMap<Entity, PlayerProjectileTickInfo> = brain_actions
        .read()
        .filter_map(|msg| match msg.request {
            crate::brain::ActionRequest::PlayerProjectileTick {
                axis,
                aim,
                press,
                held,
                released,
            } => Some((
                msg.actor,
                PlayerProjectileTickInfo {
                    axis,
                    aim,
                    press,
                    held,
                    released,
                },
            )),
            _ => None,
        })
        .collect();

    let damage_mult = user_settings.gameplay.player_damage_multiplier;
    for (player_entity, kin, mut state, mut anim) in &mut player_q {
        let tick_info = tick_infos.get(&player_entity).copied().unwrap_or_default();
        state.clock += dt;
        state.spawner.tick(dt);

        // Sample motion for Hadouken recognition. Both the action message
        // axis and `MotionDirection::from_axis` use the +Y-DOWN convention
        // (the engine matcher returns `Down` for y > 0; pinned by the
        // `motion_direction_quantization` engine test). Pass axis through
        // unchanged — an earlier negation here was inverting the sign
        // and silently mapping every "press Down" sample to `Up`, which
        // made every QCF detection fail forever.
        let dir =
            crate::projectile::MotionDirection::from_axis(tick_info.axis.x, tick_info.axis.y, 0.55);
        let now = state.clock;
        state.motion_buffer.push(dir, now);

        let mut events: Vec<ProjectileTraceEvent> = Vec::new();
        let mut still_alive = Vec::with_capacity(state.bodies.len());
        let mut bodies = std::mem::take(&mut state.bodies);
        for mut p in bodies.drain(..) {
            let gravity_sign = gravity.sign_at(p.body.pos);
            let alive = p.body.tick(dt, gravity_sign);
            if !alive {
                events.push(ProjectileTraceEvent::Expired { kind: p.body.kind });
                continue;
            }

            // Step 1: damage check against actors (enemies / bosses /
            // breakables / NPCs) via the unified pathway. The damage event
            // is only written when a hit is actually detected — a stray
            // projectile mid-flight must not deal damage to anything its
            // future AABB happens to brush in a later frame. The projectile
            // also expires on the first hit (no piercing today), so one
            // shot = one damage event = one damage application.
            let hit_event = HitEvent {
                volume: p.body.aabb(),
                damage: p.body.damage,
                source: HitSource::PlayerProjectile { kind: p.body.kind },
                attacker: Some(player_entity),
                target: crate::features::HitTarget::Volume,
                mode: crate::features::HitMode::Knockback,
                knockback: None,
                ignored_targets: Vec::new(),
            };
            let ecs_breakable_hit =
                crate::features::ecs_hit_event_hits_breakable(&hit_event, &ecs_breakables);
            let ecs_actor_hit = crate::features::ecs_hit_event_hits_actor(&hit_event, &ecs_actors);
            let ecs_boss_hit = crate::features::ecs_hit_event_hits_boss(&hit_event, &ecs_bosses);
            if ecs_breakable_hit || ecs_actor_hit || ecs_boss_hit {
                feature_damage.write(hit_event);
                sfx.write(SfxMessage::Hit { pos: p.body.pos });
                events.push(ProjectileTraceEvent::Hit {
                    kind: p.body.kind,
                    damage: p.body.damage,
                });
                continue;
            }

            // Step 2: world-collision test, dispatched through the shared
            // `WorldHitPolicy::PlayerBouncing` helper so the "solid first,
            // then one-way" priority + bouncing semantics live in one
            // place (OVERNIGHT-TODO #17.7). Player Fireballs (with
            // `bounces_remaining > 0`) bounce off floors and pass through
            // one-way platforms unless landing-from-above; Hadouken (0
            // bounces) expires on the first solid hit.
            match resolve_world_collision(&mut p.body, &world.0, WorldHitPolicy::PlayerBouncing) {
                WorldHitOutcome::Bounced { pos } => {
                    sfx.write(SfxMessage::Hit { pos });
                    still_alive.push(p);
                    continue;
                }
                WorldHitOutcome::Expired { pos } => {
                    events.push(ProjectileTraceEvent::Hit {
                        kind: p.body.kind,
                        damage: p.body.damage,
                    });
                    vfx.write(VfxMessage::Impact { pos });
                    continue;
                }
                WorldHitOutcome::Continue => {}
            }

            still_alive.push(p);
        }
        state.bodies = still_alive;

        let facing = if kin.facing.abs() < f32::EPSILON {
            1.0
        } else {
            kin.facing.signum()
        };
        let origin = ae::Vec2::new(
            kin.pos.x + facing * (kin.size.x * 0.5 + 4.0),
            kin.pos.y - kin.size.y * 0.20,
        );
        let direction = ae::Vec2::new(facing, 0.0);

        // Length before the input processing block so we can detect "a
        // projectile spawned this frame" without threading a bool through
        // every fire branch.
        let bodies_before = state.bodies.len();

        // Press edge: try Hadouken tiers first (most-specific motion gate
        // wins), else start charging a Fireball. Order matters — the
        // grace shape is a SUBSEQUENCE of the full QCF, so check Super
        // first; otherwise a 3-step input would fire a weak Hadouken.
        if tick_info.press {
            let super_qcf = state.motion_buffer.detect_quarter_circle();
            let half_circle = state.motion_buffer.detect_half_circle();
            let grace_qcf = state.motion_buffer.detect_quarter_circle_grace();

            let motion_kind = if (super_qcf.is_some() || half_circle.is_some())
                && state.unlocked.hadouken_super
            {
                Some(crate::projectile::ProjectileKind::HadoukenSuper)
            } else if grace_qcf.is_some() && state.unlocked.hadouken {
                Some(crate::projectile::ProjectileKind::Hadouken)
            } else {
                None
            };

            // Debug log on every fire-press so the player can see
            // exactly what the motion recognizer saw and why a given
            // press did or didn't upgrade to a Hadouken. Run with
            // `RUST_LOG=ambition_sandbox::projectile=info` (or
            // `RUST_LOG=info` more broadly) to surface these.
            log_press_diagnostics(
                &state.motion_buffer,
                super_qcf,
                half_circle,
                grace_qcf,
                motion_kind,
            );

            if let Some(kind) = motion_kind {
                // Motion gesture committed — fire immediately, do not
                // start a charge for this press.
                try_fire_projectile(
                    &mut state,
                    kind,
                    origin,
                    direction,
                    damage_mult,
                    0,
                    &mut events,
                );
                state.motion_buffer.clear();
                state.charging = None;
            } else if state.unlocked.fireball {
                // Begin charging the Fireball. Release-edge below
                // commits the charged shot.
                state.charging = Some(0.0);
            }
        } else if tick_info.held {
            if let Some(t) = state.charging.as_mut() {
                *t += dt;
            }
        } else if tick_info.released {
            if let Some(hold) = state.charging.take() {
                let tier = state.charge_tuning.tier_for_hold(hold);
                try_fire_projectile(
                    &mut state,
                    crate::projectile::ProjectileKind::Fireball,
                    origin,
                    direction,
                    damage_mult,
                    tier,
                    &mut events,
                );
            }
        }

        // Mirror projectile state onto the player's animation flags. `aim`
        // tracks the held-charge pose every frame; `shoot` is a short
        // post-fire pulse triggered only on the frame the body count grew.
        // SHOOT_ANIM_HOLD_SECS is short enough that a rapid-fire stream
        // visibly stutters between Shoot and Idle/Walk rather than locking
        // out the locomotion read.
        const SHOOT_ANIM_HOLD_SECS: f32 = 0.18;
        let charging = state.charging.is_some();
        if anim.aim_anim_active != charging {
            anim.aim_anim_active = charging;
        }
        if state.bodies.len() > bodies_before {
            anim.shoot_anim_timer = SHOOT_ANIM_HOLD_SECS;
        }

        let tick = trace.current_tick();
        for event in events {
            trace.push_event(event.into_trace_event(tick));
        }
    } // end per-player loop
}

/// Run the spawner's cooldown / resource-meter checks for `kind`,
/// apply the (Fireball-only) charge tier, and append the result to
/// the body list and trace events. Pulled out of `update_projectiles`
/// so the press path and the release path share one code path —
/// keeps "spawned a projectile" a single place to grep.
fn try_fire_projectile(
    state: &mut PlayerProjectileState,
    kind: crate::projectile::ProjectileKind,
    origin: ae::Vec2,
    direction: ae::Vec2,
    damage_mult: f32,
    charge_tier: u8,
    events: &mut Vec<ProjectileTraceEvent>,
) {
    match state
        .spawner
        .try_spawn(kind, origin, direction, damage_mult)
    {
        Ok(spec) => {
            let spec = spec.with_charge_tier(charge_tier);
            state.bodies.push(crate::projectile::InFlightProjectile {
                body: crate::projectile::ProjectileBody::from_spec(spec),
                owner_id: String::new(),
            });
            events.push(ProjectileTraceEvent::Fired { kind });
        }
        Err(crate::projectile::SpawnFailure::OutOfResource) => {
            events.push(ProjectileTraceEvent::BlockedByResource { kind });
        }
        Err(crate::projectile::SpawnFailure::Cooldown) => {}
    }
}

/// Flattened view of the `PlayerProjectileTick` request — used inside
/// `update_projectiles` after destructuring the matched
/// `ActorActionMessage`. A separate type so the "no message arrived
/// this tick" fallback can rely on `Default`.
#[derive(Clone, Copy, Debug, Default)]
struct PlayerProjectileTickInfo {
    axis: ae::Vec2,
    #[allow(dead_code, reason = "carried for future aim-driven fire direction")]
    aim: ae::Vec2,
    press: bool,
    held: bool,
    released: bool,
}
