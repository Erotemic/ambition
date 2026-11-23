//! OBS3: a custom brain module that binds the SIGHTED think port reads the
//! target its body's perception admitted, not where the target is now.
//!
//! `fixtures/extension_fixture_modules`' `aimer` fires at the target its
//! trigger names: `aim` through the authoritative `ActorThinkPort`,
//! `sighted_aim` through `ActorSightedThinkPort`. The shot's direction says
//! which knowledge each module received. A third, sighted sandbag wears a
//! character whose native brain perceives nothing (`StandStill`) and has no
//! projectile slot (its fire is stripped at the gate, as any such body's is),
//! so its belief exists only because the module's demand maintains it; its
//! projection is read directly.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::characters::perception::{ModuleObservation, PerceptionMemory, PerceptionRequirement};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::platformer::schedule::{PlayerInputSet, SimScheduleExt, WorldPrepSet};
use bevy::prelude::*;

/// `ambition_extension_fixture_modules::AIMER_BRAIN` / `SIGHTED_AIMER_BRAIN`.
const AIMER_BRAIN: &str = "fixture_aimer";
const SIGHTED_AIMER_BRAIN: &str = "fixture_sighted_aimer";
const AIMER: &str = "obs3_aimer";
const SIGHTED: &str = "obs3_sighted_aimer";
const SANDBAG: &str = "obs3_sighted_sandbag";

/// Each aimer's latest gated fire since the window was last cleared: a
/// module asks every tick, and the frames its body's weapon refuses carry none.
#[derive(Resource, Default)]
struct Fires(Vec<(String, Vec2)>);

fn record_fires(mut seen: ResMut<Fires>, bodies: Query<(&FeatureId, &ActorControl)>) {
    for (id, control) in &bodies {
        if id.as_str() == AIMER || id.as_str() == SIGHTED {
            if let Some(fire) = control.0.fire {
                seen.0.retain(|(name, _)| name != id.as_str());
                seen.0.push((id.as_str().to_owned(), fire.dir));
            }
        }
    }
}

fn fire_of(sim: &Platformer2dSimHarness, id: &str) -> Option<Vec2> {
    sim.world().resource::<Fires>().0.iter().find(|(name, _)| name == id).map(|(_, fire)| *fire)
}

fn body(sim: &mut Platformer2dSimHarness, id: &str) -> Entity {
    let world = sim.world_mut();
    world
        .query::<(Entity, &FeatureId)>()
        .iter(world)
        .find(|(_, feature)| feature.as_str() == id)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("{id} is spawned"))
}

fn player(sim: &mut Platformer2dSimHarness) -> (Entity, Vec2) {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(Entity, &BodyKinematics), PrimaryPlayerOnly>();
    let (entity, kin) = q.single(world).expect("primary player");
    (entity, kin.pos)
}

#[test]
fn a_sighted_module_brain_reads_its_bodys_belief_and_an_authoritative_one_the_live_target() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wasm = ambition_platformer2d::extension::build_module_crate(&root, "ambition_extension_fixture_modules")
        .expect("the fixture crate builds for wasm32-unknown-unknown");
    let options = crate::common::sandbox_options()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_extension_module_files(vec![wasm]);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds");
    let (player_body, start) = player(&mut sim);
    for (id, key, character) in [
        (SIGHTED, SIGHTED_AIMER_BRAIN, "perfect_cellular_automaton"),
        (AIMER, AIMER_BRAIN, "perfect_cellular_automaton"),
        (SANDBAG, SIGHTED_AIMER_BRAIN, "sandbag"),
    ] {
        sim.spawn_enemy_character_at(
            id,
            id,
            (start.x + 120.0, start.y),
            (14.0, 23.0),
            CharacterBrain::Custom(key.to_string()),
            character,
        );
    }
    let sim_schedule = sim.app_mut().sim_schedule();
    sim.app_mut().init_resource::<Fires>().add_systems(
        sim_schedule,
        record_fires.after(PlayerInputSet::ControlGate).before(WorldPrepSet::BeforeIntegrate),
    );
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let sighted = body(&mut sim, SIGHTED);
    let aimer = body(&mut sim, AIMER);
    let sandbag = body(&mut sim, SANDBAG);
    for body in [sighted, sandbag] {
        assert_eq!(
            sim.world().get::<ModuleObservation>(body).map(|o| o.demand),
            Some(PerceptionRequirement::TargetBelief),
            "the sighted binding resolves the body's demand"
        );
    }
    assert_eq!(
        sim.world().get::<ModuleObservation>(aimer).map(|o| o.demand),
        Some(PerceptionRequirement::None),
        "an authoritative binding demands nothing of perception"
    );
    assert!(
        sim.world().get::<PerceptionMemory>(sandbag).is_some_and(|memory| !memory.0.is_empty()),
        "the module's demand maintains the belief of a body whose native brain perceives nothing"
    );
    // With the foe in view, both aim at it: it is to their left.
    let believed = |sim: &Platformer2dSimHarness| {
        sim.world().get::<ModuleObservation>(sandbag).and_then(|o| o.decision).and_then(|d| d.believed)
    };
    let (_, seen_at) = player(&mut sim);
    let in_view = believed(&sim).expect("the sandbag believes in the foe it sees");
    assert!((in_view - seen_at).length() < 48.0, "the projection is the seen foe: {in_view:?} vs {seen_at:?}");
    for id in [SIGHTED, AIMER] {
        let fire = fire_of(&sim, id).unwrap_or_else(|| panic!("{id} fires at a visible foe"));
        assert!(fire.x < -0.5, "{id} aims at the visible foe on its left: {fire:?} (foe at {seen_at:?})");
    }

    // The foe leaves the view but stays in the room, far to the right.
    let away = Vec2::new(start.x + 120.0 + 1200.0, start.y);
    sim.world_mut().get_mut::<BodyKinematics>(player_body).expect("the player's body").pos = away;
    sim.world_mut().resource_mut::<Fires>().0.clear();
    for _ in 0..12 {
        sim.step(AgentAction::default());
    }
    let (_, now) = player(&mut sim);
    assert!((now - away).length() < 100.0, "premise: the player stays where it was put, at {now:?}");
    let live = fire_of(&sim, AIMER).expect("the authoritative aimer still has its target");
    assert!(live.x > 0.5, "the authoritative module reads the live target, to the right now: {live:?}");
    if let Some(estimate) = fire_of(&sim, SIGHTED) {
        assert!(
            estimate.x < 0.0,
            "the sighted module reads its body's pursuit estimate, not the new position: {estimate:?}"
        );
    }
    if let Some(estimate) = believed(&sim) {
        assert!(
            (estimate - now).length() > 400.0,
            "the projection did not follow the foe out of view: {estimate:?} vs {now:?}"
        );
    }
}
