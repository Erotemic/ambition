//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_characters::actor::BodyCombat;
use ambition_engine_core::BodyKinematics;
use ambition_engine_core::{BodyBaseSize, BodyMotionFacts, BodyOffense, BodyShieldState};
use ambition_platformer_primitives::markers::PlayerEntity;
use bevy::prelude::{App, MessageReader, ResMut, Resource, Update};

#[derive(Resource, Default)]
struct HitLog(Vec<HitSource>);

fn record_hits(mut reader: MessageReader<HitEvent>, mut log: ResMut<HitLog>) {
    for e in reader.read() {
        log.0.push(e.source.clone());
    }
}

fn spawn_player(app: &mut App, pos: ae::Vec2) {
    app.world_mut().spawn((
        PlayerEntity,
        BodyKinematics {
            pos,
            size: ae::Vec2::new(28.0, 46.0),
            facing: 1.0,
            ..Default::default()
        },
        // The published combat footprint every body carries (§A6).
        ae::CenteredAabb::from_center_size(pos, ae::Vec2::new(28.0, 46.0)),
        BodyBaseSize {
            base_size: ae::Vec2::new(28.0, 46.0),
        },
        BodyOffense::default(),
        BodyMotionFacts::default(),
        BodyShieldState::default(),
        BodyCombat::default(),
        ambition_platformer_primitives::frame_env::ResolvedMotionFrame::default(),
    ));
}

fn spawn_hazard(app: &mut App, id: &str, pos: ae::Vec2) {
    let aabb = ae::Aabb::new(pos, ae::Vec2::new(16.0, 16.0));
    let hazard = HazardRuntime::new(id, id, aabb, crate::DamageVolume::new(id, aabb, 1));
    app.world_mut().spawn((
        FeatureSimEntity,
        FeatureName::new(id),
        CenteredAabb::from_center_size(pos, ae::Vec2::new(32.0, 32.0)),
        HazardFeature::new(hazard),
    ));
}

fn app_with_hazard_system() -> App {
    let mut app = App::new();
    app.insert_resource(ambition_time::WorldTime::default());
    app.init_resource::<HitLog>();
    app.add_message::<HitEvent>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<VfxMessage>();
    app.add_message::<DebrisBurstMessage>();
    app.add_systems(Update, (update_ecs_hazards, record_hits).chain());
    app
}

#[test]
fn player_touching_a_hazard_emits_a_hazard_hit() {
    let mut app = app_with_hazard_system();
    let pos = ae::Vec2::new(100.0, 100.0);
    spawn_player(&mut app, pos);
    spawn_hazard(&mut app, "spikes", pos);
    app.update();
    assert!(
        app.world()
            .resource::<HitLog>()
            .0
            .iter()
            .any(|s| matches!(s, HitSource::Hazard)),
        "overlapping a hazard should emit a HitSource::Hazard hit"
    );
}

#[test]
fn player_clear_of_a_hazard_takes_no_hit() {
    let mut app = app_with_hazard_system();
    spawn_player(&mut app, ae::Vec2::new(100.0, 100.0));
    spawn_hazard(&mut app, "spikes", ae::Vec2::new(900.0, 900.0));
    app.update();
    assert!(
        app.world().resource::<HitLog>().0.is_empty(),
        "a hazard the player is clear of should not emit a hit"
    );
}

/// Fable review 2026-07-02 §A4: hazards are world danger for EVERY body —
/// an NPC standing in the spikes takes the same hit a player would
/// (previously the damage query was player-scoped and NPCs were immune).
#[test]
fn a_non_player_body_touching_a_hazard_takes_the_hit_too() {
    let mut app = app_with_hazard_system();
    // A player far away (the system requires at least one player to run
    // its damage pass).
    spawn_player(&mut app, ae::Vec2::new(900.0, 900.0));
    let pos = ae::Vec2::new(100.0, 100.0);
    let victim = app
        .world_mut()
        .spawn((
            ambition_platformer_primitives::lifecycle::FeatureSimEntity,
            ae::CenteredAabb::from_center_size(pos, ae::Vec2::new(24.0, 40.0)),
            BodyOffense::default(),
            BodyMotionFacts::default(),
            BodyShieldState::default(),
            BodyCombat::default(),
            ambition_characters::actor::BodyHealth::new(ambition_characters::actor::Health::new(5)),
        ))
        .id();
    spawn_hazard(&mut app, "spikes", pos);
    app.update();
    let world = app.world_mut();
    let mut reader = world
        .resource_mut::<bevy::prelude::Messages<HitEvent>>()
        .get_cursor();
    let world = app.world();
    let hits: Vec<_> = reader
        .read(world.resource::<bevy::prelude::Messages<HitEvent>>())
        .collect();
    assert!(
        hits.iter().any(|e| matches!(e.source, HitSource::Hazard)
            && matches!(e.target, HitTarget::Actor(v) if v == victim)),
        "an overlapping non-player body should take a pre-resolved hazard hit; got {hits:?}"
    );
}

/// CC2 (the sweep law): a body leaps ACROSS a hazard in one frame, ending
/// CLEAR of it. The old discrete endpoint overlap missed the tunnel; the
/// swept path catches it. This is the tunneling class §7.6 retires.
#[test]
fn a_fast_body_cannot_tunnel_through_a_hazard_between_frames() {
    let mut app = app_with_hazard_system();
    {
        let mut wt = app.world_mut().resource_mut::<ambition_time::WorldTime>();
        wt.scaled_dt = 0.1;
        wt.raw_dt = 0.1;
    }
    let end = ae::Vec2::new(160.0, 100.0);
    // vel * dt = 200 px this frame → the path started at x = -40, crossing
    // the hazard at x = 100, and ENDED clear at x = 160.
    app.world_mut().spawn((
        PlayerEntity,
        BodyKinematics {
            pos: end,
            vel: ae::Vec2::new(2000.0, 0.0),
            size: ae::Vec2::new(28.0, 46.0),
            facing: 1.0,
            ..Default::default()
        },
        ae::CenteredAabb::from_center_size(end, ae::Vec2::new(28.0, 46.0)),
        BodyBaseSize {
            base_size: ae::Vec2::new(28.0, 46.0),
        },
        BodyOffense::default(),
        BodyMotionFacts::default(),
        BodyShieldState::default(),
        BodyCombat::default(),
        ambition_platformer_primitives::frame_env::ResolvedMotionFrame::default(),
    ));
    spawn_hazard(&mut app, "spikes", ae::Vec2::new(100.0, 100.0));
    // Sanity: at the END position the player is CLEAR of the hazard, so a
    // discrete check would emit nothing — any hit here is the swept catch.
    assert!(
        !ae::Aabb::new(end, ae::Vec2::new(14.0, 23.0)).strict_intersects(ae::Aabb::new(
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(16.0, 16.0)
        )),
        "test setup: the end position must be clear of the hazard"
    );
    app.update();
    assert!(
        app.world()
            .resource::<HitLog>()
            .0
            .iter()
            .any(|s| matches!(s, HitSource::Hazard)),
        "a body whose path crossed the hazard should take the hit (no tunneling)"
    );
}
