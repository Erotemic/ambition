//! PATH1: a shipped moving hazard is prepared once by the shared path law,
//! placed at its seed, and travels its prepared path under rollback.

#![cfg(feature = "rl_sim")]
use ambition_app::{AgentAction, AmbitionSim, Platformer2dSimHarness};
use ambition_platformer2d::combat::hazard_runtime::HazardFeature;
use bevy::prelude::*;

fn movers(sim: &mut Platformer2dSimHarness) -> Vec<(String, Vec2, Vec2)> {
    let world = sim.world_mut();
    let mut q = world.query::<&HazardFeature>();
    let mut rows: Vec<_> = q
        .iter(world)
        .filter_map(|feature| {
            let motion = feature.hazard.motion.as_ref()?;
            Some((feature.hazard.id.clone(), feature.hazard.pos, motion.position()))
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

/// The LDtk-authored hazards of `basement_hazards` reach publication through
/// the one preparation: each moving hazard's pose is its path phase's
/// position from the first published tick, it travels, and its pose and
/// phase stay committed together. It runs under sync-test rollback, so every
/// step is resimulated from restored phases and checked.
#[test]
fn a_shipped_moving_hazard_starts_at_its_seed_and_travels_its_prepared_path() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("basement_hazards").with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    sim.step(AgentAction::default());
    let start = movers(&mut sim);
    assert!(!start.is_empty(), "the room publishes at least one moving hazard");
    for (id, pos, on_path) in &start {
        assert!((*pos - *on_path).length() < 1e-3, "hazard `{id}` is not where its phase is: {pos:?} vs {on_path:?}");
    }
    sim.step_n(AgentAction::default(), 30);
    let later = movers(&mut sim);
    assert_eq!(later.len(), start.len());
    for ((id, before, _), (_, after, on_path)) in start.iter().zip(&later) {
        assert!((*after - *before).length() > 1.0, "hazard `{id}` did not travel: {before:?} -> {after:?}");
        assert!((*after - *on_path).length() < 1e-3, "hazard `{id}` left its phase: {after:?} vs {on_path:?}");
    }
}

fn routed(sim: &mut Platformer2dSimHarness) -> Vec<(Vec2, Vec2)> {
    use ambition_platformer2d::{body_seed::ActorMotionPath, engine_core::BodyKinematics};
    let world = sim.world_mut();
    let mut q = world.query::<(&ActorMotionPath, &BodyKinematics)>();
    q.iter(world).filter_map(|(path, kin)| Some((kin.pos, path.0.as_ref()?.target()))).collect()
}

/// PATH2: the gallery goblin in `basement_enemies` (route `(520,650)` to
/// `(820,650)`, PingPong at 95) keeps its admitted placement, walks its route
/// through ordinary intent, reaches the far waypoint and reverses there. Its
/// route progress advances from its accepted arrival; nothing assigns it a
/// path position. Under sync-test rollback, so the progress is restored and
/// resimulated every step.
#[test]
fn a_route_driven_walker_reaches_its_route_target_and_reverses() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::fixed_60hz_room_options("basement_enemies").with_sync_test_rollback_settings(4, 12),
    )
    .unwrap();
    sim.step(AgentAction::default());
    let start = routed(&mut sim);
    assert_eq!(start.len(), 1, "one routed body in the room: {start:?}");
    // The authored spawn box is 44 wide at x 500: its center is 522, and the
    // route starts at 520. It was not moved onto the route.
    assert!((start[0].0.x - 522.0).abs() < 1.0, "the body was relocated: {:?}", start[0].0);
    // The player stands well out of its notice (aggro 140), so it patrols.
    sim.teleport_player((120.0, start[0].0.y));
    let (mut far, mut reversed, mut after_reverse) = (f32::MIN, false, None);
    for _ in 0..900 {
        sim.step(AgentAction::default());
        let (pos, target) = routed(&mut sim)[0];
        far = far.max(pos.x);
        if !reversed && far >= 816.0 && target.x == 520.0 {
            reversed = true;
            after_reverse = Some(pos.x);
        }
        if let (true, Some(at)) = (reversed, after_reverse) {
            if pos.x < at - 40.0 {
                return;
            }
        }
    }
    panic!("the walker did not reach 820 and come back (furthest x {far}, reversed {reversed})");
}
