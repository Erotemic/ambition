//! The Bevy gameplay system that ticks projectiles, samples motion
//! input, and routes hits through `FeatureRuntime::apply_damage_event`.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::prelude::*;

use super::diagnostics::log_press_diagnostics;
use super::feedback::forward_damage_feedback;
use super::state::{PlayerProjectile, PlayerProjectileState, ProjectileTraceEvent};
use crate::audio::SfxMessage;
use crate::features::{DamageEvent, DamageSource, FeatureEventBus};
use crate::fx::VfxMessage;
use crate::input::ControlFrame;
use crate::physics::DebrisBurstMessage;
use crate::trace::GameplayTraceBuffer;
use crate::{GameWorld, SandboxRuntime};

pub fn update_projectiles(
    time: Res<Time>,
    world: Res<GameWorld>,
    mut runtime: ResMut<SandboxRuntime>,
    control_frame: Res<ControlFrame>,
    user_settings: Res<crate::settings::UserSettings>,
    mut state: ResMut<PlayerProjectileState>,
    mut trace: ResMut<GameplayTraceBuffer>,
    mut feature_bus: ResMut<FeatureEventBus>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
    mut debris: MessageWriter<DebrisBurstMessage>,
) {
    let dt = time.delta_secs();
    state.clock += dt;
    state.spawner.tick(dt);

    // Sample motion for Hadouken recognition. Both `ControlFrame::axis_y`
    // and `MotionDirection::from_axis` use the +Y-DOWN convention
    // (the engine matcher returns `Down` for y > 0; pinned by the
    // `motion_direction_quantization` engine test). Pass axis_y
    // straight through — an earlier negation here was inverting the
    // sign and silently mapping every "press Down" sample to `Up`,
    // which made every QCF detection fail forever.
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
        // breakables / NPCs) via the unified pathway. If anything was
        // hit, the projectile expires this frame (no piercing today).
        let damage_event = DamageEvent {
            volume: p.body.aabb(),
            damage: p.body.damage,
            source: DamageSource::PlayerProjectile { kind: p.body.kind },
        };
        let report = runtime.features.apply_damage_event(&damage_event);
        if report.any_actor_hit() {
            forward_damage_feedback(&mut vfx, &mut debris, &report.events);
            // Forward boss-damage / quest / flag writes / NPC-struck
            // events so the rest of the systems (boss encounter, save
            // flags, quest registry) react to projectile hits the same
            // way they react to slash hits.
            feature_bus.ingest(&report.events);
            sfx.write(SfxMessage::Hit { pos: p.body.pos });
            events.push(ProjectileTraceEvent::Hit {
                kind: p.body.kind,
                damage: p.body.damage,
            });
            continue;
        }

        // Step 2: solid-wall test. Fireball bounces off floors (per
        // its `bounces_remaining` budget); side / ceiling / out-of-
        // budget hits expire. Hadouken spawns with 0 bounces, so the
        // first solid hit always expires it.
        let aabb = p.body.aabb();
        let solid_hit = world.0.blocks.iter().find(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. }
            ) && block.aabb.strict_intersects(aabb)
        });
        if let Some(block) = solid_hit {
            match p.body.resolve_solid_hit(block.aabb) {
                ae::ProjectileSolidHit::Bounced => {
                    sfx.write(SfxMessage::Hit { pos: p.body.pos });
                    still_alive.push(p);
                    continue;
                }
                ae::ProjectileSolidHit::Expired => {
                    events.push(ProjectileTraceEvent::Hit {
                        kind: p.body.kind,
                        damage: p.body.damage,
                    });
                    vfx.write(VfxMessage::Impact { pos: p.body.pos });
                    continue;
                }
            }
        }

        still_alive.push(p);
    }
    state.bodies = still_alive;

    let facing = if runtime.player.facing.abs() < f32::EPSILON {
        1.0
    } else {
        runtime.player.facing.signum()
    };
    let origin = ae::Vec2::new(
        runtime.player.pos.x + facing * (runtime.player.size.x * 0.5 + 4.0),
        runtime.player.pos.y - runtime.player.size.y * 0.20,
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
