//! **The GGRS round-trip the review asked for and the sentinel could not run.**
//!
//! Every other proof in this crate checks that the capability OFFERS its four
//! halves and that a composition installs them. None of them proved the offer
//! is sufficient: that after a real rewind the cooldown, the profile and the
//! velocity are what they were. The queue recorded it as deferred —
//!
//! > *"a GGRS round-trip asserting cooldown, profile and velocity after a real
//! > rewind. That needs a rollback host, and `capability_demo` has none —
//! > building one inside the sentinel would make it a host. Recorded rather
//! > than faked."*
//!
//! — which is answered by `Platformer2dSimHarness::app_mut()`: a fixture can
//! now install systems onto a rollback session instead of a crate having to
//! become a host. And the sentinel is untouched, because this arrives as a
//! **dev-dependency**. The footprint ratchet reads `cargo tree --edges normal`,
//! so the capability's own closure is exactly what it was; the same reasoning
//! `ambition_platformer2d_runtime` and the SDK already sit here under.
//!
//! ⛔ **a sync test is not a flavour of "run it twice".** Every frame is saved,
//! rewound and resimulated, and `rollback_health` compares the checksums. So an
//! assertion here is not about one frame's value — it is that the value is
//! reconstructible from the snapshot on every frame it existed.
//!
//! ⚠ if `PulseCooldown` were NOT registered, this is the test that fails: the
//! body would restore and the gate would not, and a pulse would fire twice from
//! one charge on the resimulated frame. That is the desync the offer exists to
//! prevent, and until now nothing ran it.

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::lifecycle::insert_session_world_component;
use ambition_platformer2d::world::rooms::{RoomSet, RoomSpec};
use ambition_platformer2d_rollback_ggrs::AmbitionRollbackApp;
use ambition_sim_harness::{
    AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use bevy::prelude::Entity;
use capability_demo::{
    PulseBody, PulseCooldown, PulsePlugin, PulseProfile, PulseProfiles, PulseRequested,
    BODY_ROLLBACK_STATE, PULSE_CAPABILITY, ROLLBACK_STATE,
};

/// Cooldown ticks the authored profile arms. Long enough that a rewind window
/// lands inside it — a cooldown that expired before the first resimulation
/// would make every assertion below trivially true.
const COOLDOWN_TICKS: u32 = 20;

fn compose(
    app: &mut bevy::prelude::App,
    _options: &Platformer2dSimHarnessOptions,
) -> Result<(), String> {
    // A bare one-room session, the same shape `composes_below_the_app.rs` uses:
    // the proof is about the capability, not about anyone's content.
    let world = ae::World::new(
        "pulse_room",
        ae::Vec2::new(800.0, 600.0),
        ae::Vec2::new(100.0, 100.0),
        Vec::new(),
    );
    let set = RoomSet::from_parts(
        "pulse_room",
        vec![RoomSpec::new("pulse_room", world)],
        Vec::new(),
    );
    insert_session_world_component(app.world_mut(), set);
    app.insert_resource(ControlFrame::default());

    // ⛔ **the HOST half, which the harness does not supply.** Choosing the GGRS
    // schedule is not the same as installing GGRS: `AmbitionRollbackPlugin` is
    // what adds `GgrsPlugin`, the snapshot storage and `RollbackOrdered`.
    // Without it, `require_rollback` still fires its hook and the first
    // rollback-marked entity spawned panics inside bevy_ggrs on a resource
    // nobody added — which is a composition error wearing an engine's error
    // message.
    app.add_plugins(ambition_platformer2d_rollback_ggrs::AmbitionRollbackPlugin);
    // The rollback host publishes its intra-tick phase for the presented-pose
    // layer. A headless capability host has no presentation and still has to
    // provide the resource that plugin writes into — otherwise
    // `sample_ggrs_accumulator_phase` fails parameter validation on frame one.
    app.init_resource::<ambition_platformer2d::sim_view::PresentationPhase>();
    app.add_plugins(PulsePlugin::default());
    // The PROFILE under test, authored rather than defaulted — so "the profile
    // survived" means something a default could not have provided.
    app.insert_resource(PulseProfiles::from_prepared(vec![PulseProfile {
        name: "round_trip".into(),
        radius: 64.0,
        force: 12.0,
        cooldown_ticks: COOLDOWN_TICKS,
    }]));

    // The composition installs what the capability OFFERS. This is the one line
    // the crate's docs promise a host needs.
    app.rollback_component_clone_probed::<PulseCooldown>(
        PULSE_CAPABILITY,
        ROLLBACK_STATE,
        |cooldown| u64::from(cooldown.remaining_ticks),
    );
    app.rollback_component_clone_probed::<PulseBody>(
        PULSE_CAPABILITY,
        BODY_ROLLBACK_STATE,
        |body| {
            // A checksum projection, so it must be a canonical function of the
            // value: raw float bits, not a rounded position.
            u64::from(body.pos.x.to_bits()) << 32 | u64::from(body.vel.x.to_bits())
        },
    );
    // Presence of the body component is what marks the entity for GGRS.
    //
    // ⚠ its own NAME, not `BODY_ROLLBACK_STATE`. The registry refuses two
    // registrations of different KINDS under one name — the clone above and this
    // marker are different facts about the same type — and the engine's own
    // domains follow the same split (`entity:transform_beat` for the marker,
    // `actor.transform_beat` for the clone).
    app.require_rollback::<PulseBody>(PULSE_CAPABILITY, "entity:pulse_body");
    Ok(())
}

fn harness() -> Platformer2dSimHarness {
    Platformer2dSimHarness::build(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
        compose,
    )
    .expect("the capability composes onto a sync-test harness")
}

fn cooldown_of(sim: &mut Platformer2dSimHarness, body: Entity) -> u32 {
    sim.world_mut()
        .entity(body)
        .get::<PulseCooldown>()
        .expect("the firer keeps its cooldown")
        .remaining_ticks
}

#[test]
fn the_cooldown_the_profile_and_the_velocity_survive_a_real_rewind() {
    let mut sim = harness();

    let firer = sim
        .world_mut()
        .spawn((PulseBody::default(), PulseCooldown::default()))
        .id();
    let bystander = sim
        .world_mut()
        .spawn((
            PulseBody {
                pos: ae::Vec2::new(20.0, 0.0),
                vel: ae::Vec2::ZERO,
            },
            capability_demo::PulseAffected,
        ))
        .id();
    sim.rebase_rollback_history()
        .expect("the spawned bodies become the rollback baseline");

    sim.world_mut()
        .write_message(PulseRequested { body: firer });
    sim.step(AgentAction::default());
    sim.rollback_health().expect("the firing frame is clean");

    // **The profile reached the runtime.** A default profile arms 45 ticks; the
    // authored one arms COOLDOWN_TICKS, so this number IS the profile.
    let armed = cooldown_of(&mut sim, firer);
    assert_eq!(
        armed, COOLDOWN_TICKS,
        "the pulse armed {armed} ticks, so the authored profile did not reach \
         the runtime — the compiler would be validating content nothing reads"
    );

    // **The velocity a pulse imparted is rollback state too.** Read it before
    // the rewinds so a later drift is visible as a change rather than as an
    // absence.
    let pushed = sim
        .world_mut()
        .entity(bystander)
        .get::<PulseBody>()
        .expect("the bystander keeps its body")
        .vel;
    assert!(
        pushed.x.abs() > f32::EPSILON,
        "the pulse pushed nobody ({pushed:?}), so the frames below would prove \
         only that zero survives a rewind"
    );

    // **Every frame from here is saved, rewound and resimulated.** The cooldown
    // must age by exactly one per tick across all of it: a gate that failed to
    // rewind would age twice on a resimulated frame, and one that failed to
    // save would snap back.
    for tick in 1..=10u32 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("tick {tick} desynced: {error}"));
        let expected = COOLDOWN_TICKS.saturating_sub(tick);
        let actual = cooldown_of(&mut sim, firer);
        assert_eq!(
            actual, expected,
            "after {tick} resimulated tick(s) the cooldown reads {actual}, not \
             {expected} — the gate is not being rewound with the body it gates"
        );
    }

    let after = sim
        .world_mut()
        .entity(bystander)
        .get::<PulseBody>()
        .expect("the bystander survives the rewinds")
        .vel;
    assert_eq!(
        after, pushed,
        "the imparted velocity changed across the rewind window: {pushed:?} -> \
         {after:?}"
    );
}
