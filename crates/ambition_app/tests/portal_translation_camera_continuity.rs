// Portal camera continuity integration test: drives the real portal_lab sim and
// a headless main-camera harness. Built only when the portal mechanic, portal
// presentation resources, and RL stepping API are enabled.
#![cfg(all(feature = "portal", feature = "portal_render", feature = "rl_sim"))]

mod common;
use common::{base, hold_right};

use ambition_app::app::{SandboxSet, SandboxSimulationPlugin, StartRoomOverride};
use ambition_app::AgentAction;
use ambition_gameplay_core::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
use ambition_gameplay_core::game_mode::GameMode;
use ambition_gameplay_core::portal::{
    PlacedPortal, PortalCameraContinuityConfig, PortalCameraContinuityHostView,
    PortalCameraContinuitySelection, PortalCameraContinuityState, PortalTransit, PortalWorldFrame,
};
use ambition_gameplay_core::session::camera_layers::MainCamera;
use ambition_input::ControlFrame;
use ambition_render::rendering::{camera_follow, CameraViewState};
use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::transform::TransformPlugin;

#[derive(Clone, Copy, Debug)]
struct CameraSample {
    player_pos: Vec2,
    player_vel: Vec2,
    camera_center: Vec2,
    active_transit: bool,
    camera_roll: f32,
}

impl CameraSample {
    fn screen_pos(self) -> Vec2 {
        self.player_pos - self.camera_center
    }
}

struct HeadlessCameraHarness {
    app: App,
}

impl HeadlessCameraHarness {
    fn new() -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(AssetPlugin::default());
        app.add_plugins(ImagePlugin::default());
        app.add_plugins(TransformPlugin);
        app.add_plugins(StatesPlugin);
        app.init_state::<GameMode>();
        app.insert_resource(StartRoomOverride("portal_lab".to_string()));
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(1.0 / 60.0),
        ));
        app.add_plugins(SandboxSimulationPlugin);
        app.init_resource::<PortalWorldFrame>();
        app.init_resource::<PortalCameraContinuitySelection>();
        app.init_resource::<PortalCameraContinuityConfig>();
        app.init_resource::<PortalCameraContinuityState>();
        app.init_resource::<PortalCameraContinuityHostView>();
        app.add_systems(
            Update,
            (
                ambition_gameplay_core::portal::sync_portal_world_frame
                    .before(ambition_gameplay_core::portal::apply_portal_camera_continuity),
                ambition_gameplay_core::portal::sync_portal_camera_continuity_focus
                    .before(ambition_gameplay_core::portal::apply_portal_camera_continuity),
                ambition_gameplay_core::portal::apply_portal_camera_continuity
                    .after(SandboxSet::CoreSimulation)
                    .before(camera_follow),
                camera_follow.after(ambition_gameplay_core::portal::apply_portal_camera_continuity),
            ),
        );
        app.world_mut().spawn((
            Transform::default(),
            Projection::Orthographic(OrthographicProjection::default_2d()),
            MainCamera,
            Name::new("headless continuity test main camera"),
        ));
        app.update();

        {
            let world = app.world_mut();
            let mut config = world.resource_mut::<PortalCameraContinuityConfig>();
            config.debug_log = false;
            config.max_entry_screen_offset = Vec2::new(1200.0, 900.0);
        }

        Self { app }
    }

    fn step(&mut self, action: AgentAction) -> CameraSample {
        *self.app.world_mut().resource_mut::<ControlFrame>() = action.into();
        self.app.update();
        self.sample()
    }

    fn sample(&mut self) -> CameraSample {
        let world = self.app.world_mut();
        let mut player =
            world.query_filtered::<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>();
        let kin = *player.single(world).expect("primary player body");
        let active_transit = {
            let mut transit =
                world.query_filtered::<&PortalTransit, (With<PlayerEntity>, With<PrimaryPlayer>)>();
            transit.single(world).is_ok()
        };
        let view = world.resource::<CameraViewState>();
        let camera_roll = world.resource::<PortalCameraContinuityState>().roll_radians;
        CameraSample {
            player_pos: kin.pos,
            player_vel: kin.vel,
            camera_center: view.center_world,
            active_transit,
            camera_roll,
        }
    }

    fn place_player(&mut self, pos: Vec2, vel: Vec2) {
        let world = self.app.world_mut();
        let mut player = world
            .query_filtered::<&mut BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>();
        let mut kin = player.single_mut(world).expect("primary player body");
        kin.pos = pos;
        kin.vel = vel;
        kin.facing = if vel.x >= 0.0 { 1.0 } else { -1.0 };
        *world.resource_mut::<ambition_gameplay_core::CameraEaseState>() =
            ambition_gameplay_core::CameraEaseState::default();
        *world.resource_mut::<PortalCameraContinuityHostView>() =
            PortalCameraContinuityHostView::default();
        world.resource_mut::<PortalCameraContinuityState>().clear();
    }

    fn portal_near(&mut self, pos: Vec2) -> PlacedPortal {
        let world = self.app.world_mut();
        let mut portals = world.query::<&PlacedPortal>();
        portals
            .iter(world)
            .copied()
            .filter(|p| !p.channel.is_gun_pair())
            .min_by(|a, b| {
                a.pos
                    .distance_squared(pos)
                    .total_cmp(&b.pos.distance_squared(pos))
            })
            .expect("authored portal near requested position")
    }

    fn portal_by_channel(
        &mut self,
        channel: ambition_gameplay_core::portal::PortalChannel,
    ) -> PlacedPortal {
        let world = self.app.world_mut();
        let mut portals = world.query::<&PlacedPortal>();
        portals
            .iter(world)
            .copied()
            .find(|p| p.channel == channel)
            .expect("linked portal partner")
    }
}

fn assert_near_vec(label: &str, got: Vec2, expected: Vec2, epsilon: f32) {
    let delta = got - expected;
    assert!(
        delta.length() <= epsilon,
        "{label}: got ({:.3}, {:.3}), expected ({:.3}, {:.3}), delta ({:.3}, {:.3}), |delta| {:.3} > {epsilon}",
        got.x,
        got.y,
        expected.x,
        expected.y,
        delta.x,
        delta.y,
        delta.length(),
    );
}

#[test]
fn c141_to_c140_preserves_screen_position_and_continues_right() {
    let mut harness = HeadlessCameraHarness::new();
    let entry = harness.portal_near(Vec2::new(2792.0, 248.0));
    let exit = harness.portal_by_channel(entry.channel.partner());

    assert_near_vec("c141 position", entry.pos, Vec2::new(2792.0, 248.0), 2.0);
    assert_near_vec("c141 normal", entry.normal, Vec2::new(-1.0, 0.0), 0.01);
    assert_near_vec("c140 position", exit.pos, Vec2::new(2552.0, 248.0), 2.0);
    assert_near_vec("c140 normal", exit.normal, Vec2::new(1.0, 0.0), 0.01);

    let start = entry.pos + entry.normal * 32.0;
    harness.place_player(start, Vec2::ZERO);
    let mut previous = harness.step(base());

    let mut crossed = false;
    for _ in 0..120 {
        let current = harness.step(hold_right());
        let jumped = previous.player_pos.distance(current.player_pos) > 100.0;
        if jumped {
            let enter_frame = entry.frame();
            let exit_frame = exit.frame();
            let body_before = ambition_gameplay_core::portal::pieces::map_point(
                current.player_pos,
                &exit_frame,
                &enter_frame,
            );
            let screen_before = body_before - previous.camera_center;
            let screen_after = current.player_pos - current.camera_center;
            assert_near_vec(
                "body screen-space continuity",
                screen_after,
                screen_before,
                1.5,
            );

            let entry_portal_screen_before = entry.pos - previous.camera_center;
            let exit_portal_screen_after = exit.pos - current.camera_center;
            assert_near_vec(
                "portal aperture screen-space continuity",
                exit_portal_screen_after,
                entry_portal_screen_before,
                1.5,
            );

            let visible_step = (current.screen_pos() - previous.screen_pos()).length();
            assert!(
                visible_step <= 18.0,
                "the handoff should look like one normal rightward step, not a snap: visible screen delta {visible_step:.3}px, previous={previous:?}, current={current:?}",
            );
            assert!(
                current.camera_roll.abs() < 0.001,
                "translation wall-wall portal must not roll the camera, roll={}",
                current.camera_roll,
            );
            assert!(
                current.player_vel.x > 30.0,
                "held-right motion should continue right through c141->c140, vel={:?}",
                current.player_vel,
            );

            let mut last = current;
            let mut max_visible_step = visible_step;
            let mut clear_frame = None;
            for follow_frame in 0..45 {
                let next = harness.step(hold_right());
                max_visible_step =
                    max_visible_step.max((next.screen_pos() - last.screen_pos()).length());
                if clear_frame.is_none() && !next.active_transit {
                    clear_frame = Some(follow_frame);
                }
                last = next;
            }
            assert!(
                clear_frame.is_some(),
                "the player should clear the exit aperture promptly after c141->c140"
            );
            assert!(
                max_visible_step <= 24.0,
                "screen-space motion should stay continuous through transit clear; max visible step {max_visible_step:.3}px",
            );
            assert!(
                last.player_pos.x > current.player_pos.x + 8.0,
                "after emerging at c140, holding right should keep increasing world x: current={:?}, last={:?}",
                current.player_pos,
                last.player_pos,
            );
            assert!(
                last.player_vel.x > 30.0,
                "after c141->c140 clear, held right should still drive positive x velocity, vel={:?}",
                last.player_vel,
            );

            crossed = true;
            break;
        }
        previous = current;
    }

    assert!(
        crossed,
        "the player should transit c141 -> c140 while holding right"
    );
}

#[test]
fn c135_to_c134_preserves_screen_position_and_keeps_falling() {
    let mut harness = HeadlessCameraHarness::new();
    let entry = harness.portal_near(Vec2::new(900.0, 900.0));
    let exit = harness.portal_by_channel(entry.channel.partner());

    assert_near_vec("c135 position", entry.pos, Vec2::new(900.0, 900.0), 2.0);
    assert_near_vec("c135 normal", entry.normal, Vec2::new(0.0, -1.0), 0.01);
    assert_near_vec("c134 position", exit.pos, Vec2::new(900.0, 220.0), 2.0);
    assert_near_vec("c134 normal", exit.normal, Vec2::new(0.0, 1.0), 0.01);

    let start = entry.pos + entry.normal * 32.0;
    harness.place_player(start, Vec2::new(0.0, 260.0));
    let mut previous = harness.step(base());

    let mut crossed = false;
    for _ in 0..120 {
        let current = harness.step(base());
        let jumped = previous.player_pos.distance(current.player_pos) > 300.0;
        if jumped {
            let enter_frame = entry.frame();
            let exit_frame = exit.frame();
            let body_before = ambition_gameplay_core::portal::pieces::map_point(
                current.player_pos,
                &exit_frame,
                &enter_frame,
            );
            let screen_before = body_before - previous.camera_center;
            let screen_after = current.player_pos - current.camera_center;
            assert_near_vec(
                "body screen-space continuity",
                screen_after,
                screen_before,
                1.5,
            );

            let entry_portal_screen_before = entry.pos - previous.camera_center;
            let exit_portal_screen_after = exit.pos - current.camera_center;
            assert_near_vec(
                "portal aperture screen-space continuity",
                exit_portal_screen_after,
                entry_portal_screen_before,
                1.5,
            );

            let visible_step = (current.screen_pos() - previous.screen_pos()).length();
            assert!(
                visible_step <= 24.0,
                "the floor-ceiling handoff should look like one falling step, not a snap: visible screen delta {visible_step:.3}px, previous={previous:?}, current={current:?}",
            );
            assert!(
                current.camera_roll.abs() < 0.001,
                "translation floor-ceiling portal must not roll the camera, roll={}",
                current.camera_roll,
            );
            assert!(
                current.player_vel.y > 30.0,
                "falling into c135 should continue falling out of c134, vel={:?}",
                current.player_vel,
            );

            let mut last = current;
            let mut max_visible_step = visible_step;
            let mut clear_frame = None;
            for follow_frame in 0..45 {
                let next = harness.step(base());
                max_visible_step =
                    max_visible_step.max((next.screen_pos() - last.screen_pos()).length());
                if clear_frame.is_none() && !next.active_transit {
                    clear_frame = Some(follow_frame);
                }
                last = next;
            }
            assert!(
                clear_frame.is_some(),
                "the player should clear the exit aperture promptly after c135->c134"
            );
            assert!(
                max_visible_step <= 30.0,
                "screen-space motion should stay continuous through floor-ceiling transit clear; max visible step {max_visible_step:.3}px",
            );
            assert!(
                last.player_pos.y > current.player_pos.y + 8.0,
                "after emerging at c134, gravity should keep increasing world y: current={:?}, last={:?}",
                current.player_pos,
                last.player_pos,
            );

            crossed = true;
            break;
        }
        previous = current;
    }

    assert!(
        crossed,
        "the player should transit c135 -> c134 while falling"
    );
}
