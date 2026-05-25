//! The Bevy gameplay system that ticks projectiles, samples motion
//! input, and routes hits through ECS-native feature damage messages.

use ambition_engine as ae;
use bevy::prelude::*;

use super::collision::{resolve_world_collision, WorldHitOutcome, WorldHitPolicy};
use super::diagnostics::log_press_diagnostics;
use super::state::{PlayerProjectile, PlayerProjectileState, ProjectileTraceEvent};
use crate::audio::SfxMessage;
use crate::features::{
    ActorCombatState, ActorDisposition, BossFeature, BreakableFeature, DamageEvent, DamageSource,
    FeatureAabb, FeatureId, FeatureSimEntity,
};
use crate::presentation::fx::VfxMessage;
use crate::trace::GameplayTraceBuffer;
use crate::GameWorld;

pub fn update_projectiles(
    world_time: Res<crate::WorldTime>,
    world: Res<GameWorld>,
    // Projectile spawn origin/direction reads input from the primary
    // local player's `PlayerInputFrame` component (the per-player input
    // migration target, OVERNIGHT-TODO #17.5). The single global
    // `Res<ControlFrame>` is still written by the input pipeline and
    // mirrored into this component by `sync_local_player_input_frame`,
    // so today the `PrimaryPlayerOnly` filter keeps single-player
    // behavior identical. Future co-op / network builds populate this
    // component on additional player entities without ever competing
    // for the global resource — projectile spawn becomes per-player at
    // that point by simply dropping the filter.
    player_input_q: Query<
        (&crate::player::PlayerBody, &crate::player::PlayerInputFrame),
        crate::player::PrimaryPlayerOnly,
    >,
    user_settings: Res<crate::persistence::settings::UserSettings>,
    mut state: ResMut<PlayerProjectileState>,
    mut trace: ResMut<GameplayTraceBuffer>,
    mut feature_damage: MessageWriter<DamageEvent>,
    ecs_breakables: Query<(&FeatureId, &FeatureAabb, &BreakableFeature), With<FeatureSimEntity>>,
    ecs_actors: Query<
        (
            &FeatureId,
            &FeatureAabb,
            &ActorDisposition,
            &ActorCombatState,
        ),
        With<FeatureSimEntity>,
    >,
    ecs_bosses: Query<
        (
            &FeatureId,
            &FeatureAabb,
            &BossFeature,
            &crate::brain::BossAttackState,
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
    state.clock += dt;
    state.spawner.tick(dt);

    // Resolve the primary local player's body + input frame from the
    // ECS. `PrimaryPlayerOnly` keeps single-player behavior; future
    // multiplayer drops the filter and iterates over input-bearing
    // players.
    let primary = player_input_q.single().ok();

    // Sample motion for Hadouken recognition. Both `ControlFrame::axis_y`
    // and `MotionDirection::from_axis` use the +Y-DOWN convention
    // (the engine matcher returns `Down` for y > 0; pinned by the
    // `motion_direction_quantization` engine test). Pass axis_y
    // straight through — an earlier negation here was inverting the
    // sign and silently mapping every "press Down" sample to `Up`,
    // which made every QCF detection fail forever.
    let control_frame = primary.map(|(_, input)| input.frame).unwrap_or_default();
    let dir = ae::MotionDirection::from_axis(control_frame.axis_x, control_frame.axis_y, 0.55);
    let now = state.clock;
    state.motion_buffer.push(dir, now);

    let mut events: Vec<ProjectileTraceEvent> = Vec::new();
    let mut still_alive = Vec::with_capacity(state.bodies.len());
    let mut bodies = std::mem::take(&mut state.bodies);
    for mut p in bodies.drain(..) {
        let alive = p.body.tick(dt);
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
        let damage_event = DamageEvent {
            volume: p.body.aabb(),
            damage: p.body.damage,
            source: DamageSource::PlayerProjectile { kind: p.body.kind },
            ignored_targets: Vec::new(),
        };
        let ecs_breakable_hit =
            crate::features::ecs_damage_event_hits_breakable(&damage_event, &ecs_breakables);
        let ecs_actor_hit =
            crate::features::ecs_damage_event_hits_actor(&damage_event, &ecs_actors);
        let ecs_boss_hit = crate::features::ecs_damage_event_hits_boss(&damage_event, &ecs_bosses);
        if ecs_breakable_hit || ecs_actor_hit || ecs_boss_hit {
            feature_damage.write(damage_event);
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

    let Some((body, _input)) = primary else {
        return;
    };
    let facing = if body.facing.abs() < f32::EPSILON {
        1.0
    } else {
        body.facing.signum()
    };
    let origin = ae::Vec2::new(
        body.pos.x + facing * (body.size.x * 0.5 + 4.0),
        body.pos.y - body.size.y * 0.20,
    );
    let direction = ae::Vec2::new(facing, 0.0);
    let damage_mult = user_settings.gameplay.player_damage_multiplier;

    // Press edge: try Hadouken tiers first (most-specific motion gate
    // wins), else start charging a Fireball. Order matters — the
    // grace shape is a SUBSEQUENCE of the full QCF, so check Super
    // first; otherwise a 3-step input would fire a weak Hadouken.
    if control_frame.projectile_pressed {
        let super_qcf = state.motion_buffer.detect_quarter_circle();
        let half_circle = state.motion_buffer.detect_half_circle();
        let grace_qcf = state.motion_buffer.detect_quarter_circle_grace();

        let motion_kind =
            if (super_qcf.is_some() || half_circle.is_some()) && state.unlocked.hadouken_super {
                Some(ae::ProjectileKind::HadoukenSuper)
            } else if grace_qcf.is_some() && state.unlocked.hadouken {
                Some(ae::ProjectileKind::Hadouken)
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
    } else if control_frame.projectile_held {
        if let Some(t) = state.charging.as_mut() {
            *t += dt;
        }
    } else if control_frame.projectile_released {
        if let Some(hold) = state.charging.take() {
            let tier = state.charge_tuning.tier_for_hold(hold);
            try_fire_projectile(
                &mut state,
                ae::ProjectileKind::Fireball,
                origin,
                direction,
                damage_mult,
                tier,
                &mut events,
            );
        }
    }

    let tick = trace.current_tick();
    for event in events {
        trace.push_event(event.into_trace_event(tick));
    }
}

/// Run the spawner's cooldown / resource-meter checks for `kind`,
/// apply the (Fireball-only) charge tier, and append the result to
/// the body list and trace events. Pulled out of `update_projectiles`
/// so the press path and the release path share one code path —
/// keeps "spawned a projectile" a single place to grep.
fn try_fire_projectile(
    state: &mut PlayerProjectileState,
    kind: ae::ProjectileKind,
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
            state.bodies.push(PlayerProjectile {
                body: ae::ProjectileBody::from_spec(spec),
            });
            events.push(ProjectileTraceEvent::Fired { kind });
        }
        Err(ae::SpawnFailure::OutOfResource) => {
            events.push(ProjectileTraceEvent::BlockedByResource { kind });
        }
        Err(ae::SpawnFailure::Cooldown) => {}
    }
}
