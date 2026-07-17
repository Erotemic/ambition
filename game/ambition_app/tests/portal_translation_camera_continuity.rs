// Portal camera continuity integration test: drives the real portal_lab sim and
// a headless main-camera harness. Built only when the portal mechanic, portal
// presentation resources, and RL stepping API are enabled.
#![cfg(all(feature = "portal", feature = "portal_render", feature = "rl_sim"))]

use crate::common::{base, hold_right};

use ambition::actors::actor::{BodyKinematics, PlayerEntity, PrimaryPlayer};
use ambition::input::ControlFrame;
use ambition::platformer::camera_layers::MainCamera;
use ambition::platformer::schedule::GameMode;
use ambition::portal::{PlacedPortal, PortalTransit};
use ambition::portal_presentation::{
    PortalCameraContinuityConfig, PortalCameraContinuityHostView, PortalCameraContinuitySelection,
    PortalCameraContinuityState, PortalWorldFrame,
};
use ambition::render::rendering::{camera_follow, CameraViewState};
use ambition_app::app::{SandboxSet, SandboxSimulationPlugin, StartRoomOverride};
use ambition_app::AgentAction;
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
        // camera_follow is the PRESENTATION half now (E4-17): the sim's
        // CameraObservationPlugin (inside SandboxSimulationPlugin's engine
        // group) resolves the snapshot as a tail observer after
        // CoreSimulation; the rig composes the render-side apply after it,
        // exactly like the real host, and owns the render-side
        // CameraViewState resource it registers.
        app.init_resource::<CameraViewState>();
        app.add_systems(
            Update,
            (
                ambition::host::portal::sync_portal_world_frame
                    .before(ambition::host::portal::apply_portal_camera_continuity),
                ambition::host::portal::sync_portal_camera_continuity_focus
                    .before(ambition::host::portal::apply_portal_camera_continuity),
                ambition::host::portal::apply_portal_camera_continuity
                    .after(SandboxSet::CoreSimulation)
                    .before(camera_follow),
                // Same-frame clamp pad into the sim resolve, like the host.
                ambition::render::rendering::publish_portal_camera_clamp
                    .after(ambition::host::portal::apply_portal_camera_continuity)
                    .before(ambition::sim_view::camera_snapshot::resolve_camera_observation),
                camera_follow
                    .after(ambition::host::portal::apply_portal_camera_continuity)
                    .after(ambition::sim_view::camera_snapshot::resolve_camera_observation),
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
        *world.resource_mut::<ambition::platformer::camera_ease::CameraEaseState>() =
            ambition::platformer::camera_ease::CameraEaseState::default();
        *world.resource_mut::<PortalCameraContinuityHostView>() =
            PortalCameraContinuityHostView::default();
        world.resource_mut::<PortalCameraContinuityState>().clear();
    }

    fn portal_near(&mut self, pos: Vec2) -> PlacedPortal {
        let world = self.app.world_mut();
        let mut portals = world.query::<&PlacedPortal>();
        portals
            .iter(world)
            .cloned()
            .filter(|p| !p.channel.is_gun_pair())
            .min_by(|a, b| {
                a.pos
                    .distance_squared(pos)
                    .total_cmp(&b.pos.distance_squared(pos))
            })
            .expect("authored portal near requested position")
    }

    fn portal_by_channel(&mut self, channel: ambition::portal::PortalChannel) -> PlacedPortal {
        let world = self.app.world_mut();
        let mut portals = world.query::<&PlacedPortal>();
        portals
            .iter(world)
            .cloned()
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
            let body_before =
                ambition::portal::pieces::map_point(current.player_pos, &exit_frame, &enter_frame);
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
            let body_before =
                ambition::portal::pieces::map_point(current.player_pos, &exit_frame, &enter_frame);
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

/// Walking through the thin-wall doorway pair (c136/c137) must keep the
/// APPARENT (screen-space) player position smooth for the WHOLE walk — the
/// engage frame, every anchored frame, the anchor-release frame, and the
/// settle afterwards. Jon's report: the character visibly jumps crossing the
/// thin wall in Continuous mode, which the per-crossing checks above (both on
/// wide pairs) never covered frame-by-frame.
#[test]
fn thin_wall_walk_keeps_apparent_player_position_smooth() {
    let mut harness = HeadlessCameraHarness::new();

    // Locate the thin-wall doorway pair: partner-linked, opposed normals,
    // faces less than ~48px apart.
    let (entry, exit) = {
        let world = harness.app.world_mut();
        let mut portals = world.query::<&PlacedPortal>();
        let all: Vec<PlacedPortal> = portals
            .iter(world)
            .cloned()
            .filter(|p| !p.channel.is_gun_pair())
            .collect();
        let mut found = None;
        for p in &all {
            if let Some(q) = all.iter().find(|q| q.channel == p.channel.partner()) {
                let opposed = p.normal.dot(q.normal) < -0.9;
                let thin = p.pos.distance(q.pos) <= 48.0;
                // Walk left-to-right: entry face points left (-x).
                if opposed && thin && p.normal.x < -0.9 {
                    found = Some((p.clone(), q.clone()));
                    break;
                }
            }
        }
        found.expect("portal_lab should author a thin-wall doorway pair (c136/c137)")
    };

    let start = entry.pos + entry.normal * 120.0;
    harness.place_player(start, Vec2::ZERO);
    // Let the camera settle on the start position first, so the walk itself
    // is the only motion being measured.
    for _ in 0..90 {
        harness.step(base());
    }
    let mut previous = harness.step(base());

    // Walk right through the doorway and keep walking; the whole pass must
    // read as ordinary walking. The body's VISUAL is continuous by the clip
    // pieces (the slices tile across the seam even as the authoritative pos
    // snaps by the wall thickness), so what the player actually SEES jump is
    // the CAMERA: any one-frame camera step much larger than a frame of walk
    // speed (270/60 = 4.5px) is the world lurching behind the character.
    let mut crossed = false;
    let mut max_camera_step = 0.0_f32;
    let mut max_smooth_screen_step = 0.0_f32;
    let mut worst: Option<(usize, CameraSample, CameraSample)> = None;
    for frame in 0..240 {
        let current = harness.step(hold_right());
        let snapped = previous.player_pos.distance(current.player_pos) > 20.0;
        if snapped {
            crossed = true;
            // At the snap frame the AUTHORITATIVE screen offset jumps by
            // design; the visual invariant is map-aware continuity.
            let body_before = ambition::portal::pieces::map_point(
                current.player_pos,
                &exit.frame(),
                &entry.frame(),
            );
            let mapped_step =
                ((body_before - previous.camera_center) - previous.screen_pos()).length();
            assert!(
                mapped_step <= 6.0,
                "map-aware body continuity at the snap frame: {mapped_step:.2}px \
                 (frame {frame}, prev {previous:?}, cur {current:?})"
            );
        } else {
            max_smooth_screen_step =
                max_smooth_screen_step.max((current.screen_pos() - previous.screen_pos()).length());
        }
        let camera_step = (current.camera_center - previous.camera_center).length();
        if camera_step > max_camera_step {
            max_camera_step = camera_step;
            worst = Some((frame, previous, current));
        }
        previous = current;
        if current.player_pos.x > exit.pos.x + 200.0 {
            break;
        }
    }
    assert!(crossed, "the walk should transit the thin-wall pair");
    assert!(
        max_camera_step <= 12.0,
        "a thin-wall doorway is a doorway, not a teleport: the camera must \
         never lurch the world (max one-frame camera step {max_camera_step:.2}px \
         at {worst:#?})"
    );
    assert!(
        max_smooth_screen_step <= 12.0,
        "between snaps the apparent player position must move like ordinary \
         walking, got a {max_smooth_screen_step:.2}px step"
    );
}
