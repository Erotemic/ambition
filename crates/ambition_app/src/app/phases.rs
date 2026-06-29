use bevy::prelude::*;

use ambition_engine_core as ae;
use ambition_gameplay_core::audio::SfxMessage;
use ambition_gameplay_core::combat::attack::engine_input_from_actor_control;
use ambition_gameplay_core::features;
use ambition_gameplay_core::player::handle_player_events;
use ambition_gameplay_core::time::feel::SandboxFeelTuning;
use ambition_render::fx::VfxMessage;

use super::world_flow::{reset_sandbox, sandbox_dt};

/// How a ledge-grabbing player should react to the moving platform that carries
/// them this frame: ride along with it, or be knocked off because the carry
/// would shove them into a wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LedgePlatformCarry {
    Carry,
    KnockOff,
}

/// Decide [`LedgePlatformCarry`] for a ledge-grabbing player about to be carried
/// by `delta`. `world` is the base collision world, which **excludes** the
/// moving platform (it's composited in separately), so a solid overlap here is a
/// genuine *other* wall — meaning the platform would push the player through it
/// (#126 "ledge grab on a moving platform into a wall pushes you through").
/// Pure, so the wall decision is unit-testable without the full phase context.
pub(super) fn ledge_platform_carry(
    world: &ae::World,
    player_aabb: ae::Aabb,
    delta: ae::Vec2,
) -> LedgePlatformCarry {
    use ambition_engine_core::AabbExt;
    let carried = player_aabb.translated(delta);
    let into_wall = world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, ae::BlockKind::Solid) && carried.strict_intersects(b.aabb));
    if into_wall {
        LedgePlatformCarry::KnockOff
    } else {
        LedgePlatformCarry::Carry
    }
}

/// The unified player body tick — control phase **and** simulation phase in ONE
/// combined engine call (`ae::update_player_with_tuning_clusters`), the exact
/// shape the actor path uses (`ActorMut::integrate_body` →
/// `update_body_with_tuning_clusters`). The player and a brain-driven actor now
/// run the SAME body-tick entry; the only difference is the input frame.
///
/// THE TWO-CLOCK SPLIT IS AN INPUT AFFORDANCE, NOT A SIMULATION STRUCTURE.
/// Precision-blink bullet-time keeps the player's aim responsive while the world
/// slows. That used to be two separate Bevy systems (control@real-dt then
/// sim@scaled-dt). It is now carried entirely by `InputState::control_dt`: the
/// human sets `control_dt = real frame_dt` (so the engine runs the control phase
/// at real time and the simulation phase at `sim_dt`), while a brain leaves
/// `control_dt = 0` and runs everything at sim time — it needs no think-time to
/// aim. Same body, same engine entry; the affordance lives in the input.
///
/// `ActorControl` is the single source of truth for player input — the player
/// brain translates every verb the simulation consumes (movement, jump, dash,
/// attack, interact, shield, pogo, blink, fly_toggle, fast_fall,
/// projectile-charge, aim). The hitstun gate applies inside
/// `engine_input_from_actor_control`.
///
/// Returns `Return` if the engine asked for a sandbox reset (drown / hazard /
/// out-of-bounds / death). The non-primary player clone runs the same core but
/// never triggers the world-global reset / camera shake — those are primary-only.
#[allow(clippy::too_many_arguments)]
pub(super) fn player_body_phase(
    actor_control: ambition_characters::actor::control::ActorControlFrame,
    world: &ae::World,
    clusters: &mut ae::BodyClustersMut<'_>,
    sim_state: &mut ambition_gameplay_core::SandboxSimState,
    clock: &mut ambition_gameplay_core::time::clock_state::ClockState,
    safety: &mut ambition_gameplay_core::player::PlayerSafetyState,
    moving_platforms: &[ambition_gameplay_core::world::platforms::MovingPlatformState],
    attack: &mut Option<ambition_gameplay_core::MeleeSwing>,
    sfx_writer: &mut MessageWriter<SfxMessage>,
    vfx_writer: &mut MessageWriter<VfxMessage>,
    shake: &mut ambition_gameplay_core::time::camera_ease::CameraShakeState,
    tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    feature_ecs_overlay: &features::FeatureEcsWorldOverlay,
    reset_room_features: &mut MessageWriter<features::ResetRoomFeaturesEvent>,
    anim: &mut ambition_gameplay_core::player::PlayerAnimState,
    combat: &mut ambition_gameplay_core::actor::BodyCombat,
    interaction: &mut ambition_gameplay_core::player::PlayerInteractionState,
    blink_cam: &mut ambition_gameplay_core::player::PlayerBlinkCameraState,
    is_primary: bool,
) {
    // ONE input frame. `control_dt = frame_dt` (real time) IS the precision-blink
    // affordance: the combined entry below runs the control phase at this rate and
    // the simulation phase at the scaled `sim_dt`. `sim_state.time_scale` /
    // `clock.time_scale` were set this frame by the time-control pipeline in
    // SandboxSet::PlayerInput (emit → apply → smooth). The hitstun gate applies
    // inside the helper.
    let input = engine_input_from_actor_control(
        actor_control,
        feel,
        combat.hitstun_timer,
        combat.recoil_lock_timer,
        frame_dt,
    );
    let sim_dt = sandbox_dt(combat.hitstop_timer, clock.time_scale, frame_dt);

    // Pre-sim LEDGE-platform carry. Platforms are advanced once (by
    // `advance_moving_platforms`) ahead of this whole tick, so we read this frame's
    // delta. Standing-on-platform RIDING is EMERGENT in the movement sweep
    // (`integrate_velocity_clusters` carries any grounded body by its support's
    // velocity — the same rule enemies ride by), so there is no player-specific ride
    // code. What stays player-specific is the LEDGE carry: hanging off a moving
    // platform's edge (only the player ledge-grabs) leaves the body un-grounded, so
    // the sweep carry can't apply.
    let player_aabb_pre = clusters.kinematics.aabb();
    let player_size_pre = clusters.kinematics.size;
    let active_ledge_platform = clusters.ledge.grab.and_then(|grab| {
        moving_platforms.iter().position(|platform| {
            platform.matches_ledge_contact_in_frame(
                grab.contact,
                player_size_pre,
                tuning.gravity_dir,
            )
        })
    });
    if let Some(platform_delta) =
        active_ledge_platform.map(|idx| moving_platforms[idx].last_delta())
    {
        match ledge_platform_carry(world, player_aabb_pre, platform_delta) {
            // #126: the platform is about to carry the hanging player INTO a wall.
            // Don't ride into it (that clips through) — knock off the ledge and fall.
            LedgePlatformCarry::KnockOff => {
                clusters.ledge.knock_off_on_hit();
            }
            // Carry both the player and the stored ledge contact so hang / climb /
            // roll interpolation stays platform-relative after the platform moves.
            LedgePlatformCarry::Carry => {
                clusters.kinematics.pos += platform_delta;
                if let Some(grab) = clusters.ledge.grab.as_mut() {
                    grab.contact.anchor += platform_delta;
                    grab.contact.climb_target += platform_delta;
                }
            }
        }
    }

    let collision_world =
        features::world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
    let was_grounded = clusters.ground.on_ground;
    let pre_sim_vy = clusters.kinematics.vel.y;

    // THE single combined body tick: control phase (at `input.control_dt`, the real
    // clock) then simulation phase (at `sim_dt`, the scaled clock) — the same entry
    // the actor body uses, plus the player respawn POLICY below.
    let events =
        ae::update_player_with_tuning_clusters(&collision_world, clusters, input, sim_dt, tuning);

    // Hard-fall screen shake: pure trigger in `time::camera_ease`. Avoids tiny hops,
    // saturates above terminal velocity via `kick()`'s cap. `pre_sim_vy` is the
    // velocity entering the combined tick (the control phase rarely changes a
    // falling body's descent, so the landing read is unchanged in practice).
    let shake_amplitude = ambition_gameplay_core::time::camera_ease::hard_fall_shake_amplitude(
        was_grounded,
        clusters.ground.on_ground,
        pre_sim_vy,
    );
    if is_primary && shake_amplitude > 0.0 {
        shake.kick(shake_amplitude);
        sfx_writer.write(SfxMessage::Play {
            id: ambition_sfx::ids::PLAYER_LAND,
            pos: clusters.kinematics.pos,
        });
    }
    // Player respawn POLICY — the one thing the actor path does NOT do (an actor
    // owns its own hazard reaction; it never teleports to the player spawn). A
    // flagged reset from either the control or simulation half lands here.
    if events.reset && is_primary {
        reset_sandbox(
            world,
            sfx_writer,
            vfx_writer,
            clusters,
            sim_state,
            clock,
            safety,
            attack,
            anim,
            combat,
            interaction,
            blink_cam,
            tuning,
            feel,
        );
        reset_room_features.write(features::ResetRoomFeaturesEvent {
            reason: features::RoomResetReason::PlayerDeath,
        });
        return;
    }
    handle_player_events(
        sfx_writer,
        vfx_writer,
        clusters,
        combat,
        blink_cam,
        anim,
        events,
        Some(was_grounded),
    );
}

#[cfg(test)]
mod ledge_carry_tests {
    use super::{ledge_platform_carry, LedgePlatformCarry};
    use ambition_engine_core as ae;

    fn world_with_right_wall() -> ae::World {
        // A solid wall occupying x[100,120], full height; open space to its left.
        ae::World::new(
            "ledge_carry_test",
            ae::Vec2::new(400.0, 400.0),
            ae::Vec2::new(20.0, 50.0),
            vec![ae::Block::solid(
                "wall",
                ae::Vec2::new(100.0, 0.0),
                ae::Vec2::new(20.0, 400.0),
            )],
        )
    }

    // A ledge-grabbing player hugging the left of the wall (right edge at x=92).
    fn player() -> ae::Aabb {
        ae::Aabb::new(ae::Vec2::new(80.0, 50.0), ae::Vec2::new(12.0, 20.0))
    }

    #[test]
    fn carry_into_a_wall_knocks_the_player_off() {
        // A rightward platform delta would push the player's right edge (92) past
        // the wall face (100) and into it — #126: knock off, don't clip through.
        assert_eq!(
            ledge_platform_carry(&world_with_right_wall(), player(), ae::Vec2::new(30.0, 0.0)),
            LedgePlatformCarry::KnockOff,
        );
    }

    #[test]
    fn carry_away_from_walls_rides_normally() {
        // Leftward (away) — and a small rightward nudge that stays clear — both
        // ride along with the platform.
        let world = world_with_right_wall();
        assert_eq!(
            ledge_platform_carry(&world, player(), ae::Vec2::new(-30.0, 0.0)),
            LedgePlatformCarry::Carry,
        );
        assert_eq!(
            ledge_platform_carry(&world, player(), ae::Vec2::new(5.0, 0.0)),
            LedgePlatformCarry::Carry,
        );
    }
}
