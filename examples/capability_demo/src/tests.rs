//! The sentinel's own proof.
//!
//! Each test below is one of the four halves of the capability contract, and
//! the point of every one is what it did NOT have to touch.

use super::*;
use ambition_causal::{CausalRecording, FactValue, RecordingPolicy, SubjectKey};
use ambition_content_pack::{
    compile, AssetsUnchecked, ContentPackDraft, ContentPackManifest, ModuleNamespace, PackId,
    PackVersion, SchemaId, SchemaRegistry, SourceDeclaration,
};
use ambition_platformer2d_core::Vec2;

const PROFILES: &str = r#"(
    profiles: [
        (name: "gentle", radius: 64.0, force: 100.0, cooldown_ticks: 30),
        (name: "cannon", radius: 200.0, force: 900.0, cooldown_ticks: 90),
    ],
)"#;

fn compile_profiles(name: &str, text: &str) -> Result<Vec<PulseProfile>, String> {
    let root = std::env::temp_dir().join(format!("capability_demo_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    std::fs::write(root.join("pulse.ron"), text).expect("write");

    // A registry with ONLY this capability's schema in it. The compiler has
    // never heard of pulses; the capability hands it one.
    let mut registry = SchemaRegistry::new();
    registry.register(pulse_schema()).expect("fresh registry");

    let draft = ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("pulse_pack".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("demo".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: "pulse.ron".into(),
                schema: SchemaId::new(PULSE_SCHEMA),
                version: ambition_content_pack::SchemaVersion(1),
            }],
        },
    )
    .map_err(|f| f.render())?;

    let pack = compile(&draft, &registry, &AssetsUnchecked).map_err(|f| f.render())?;
    Ok(pack
        .lowered::<Vec<PulseProfile>>(&SchemaId::new(PULSE_SCHEMA))
        .expect("a Runtime schema lowers its artifact")
        .clone())
}

/// The same compilation, returning the PACK — what a composition actually holds.
fn compile_pack(
    name: &str,
    text: &str,
) -> Result<ambition_content_pack::PreparedContentPack, String> {
    let root = std::env::temp_dir().join(format!("capability_demo_test/{name}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    std::fs::write(root.join("pulse.ron"), text).expect("write");
    let mut registry = SchemaRegistry::new();
    registry.register(pulse_schema()).expect("fresh registry");
    let draft = ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("pulse_pack".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("demo".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: "pulse.ron".into(),
                schema: SchemaId::new(PULSE_SCHEMA),
                version: ambition_content_pack::SchemaVersion(1),
            }],
        },
    )
    .map_err(|f| f.render())?;
    compile(&draft, &registry, &AssetsUnchecked).map_err(|f| f.render())
}

/// HALF 1 — an authored schema, registered by the capability that owns it.
///
/// `ambition_content_pack` has no pulse knowledge. No central content enum was
/// edited to make this compile.
#[test]
fn the_capability_registers_its_own_content_schema() {
    let profiles = compile_profiles("valid", PROFILES).expect("compiles");
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[1].name, "cannon");
    assert_eq!(profiles[1].force, 900.0);
}

/// And its own refusals, in the compiler's vocabulary rather than a panic.
#[test]
fn the_capabilitys_schema_refuses_its_own_nonsense() {
    let failure = compile_profiles(
        "no_radius",
        r#"(profiles: [(name: "dud", radius: 0.0, force: 100.0, cooldown_ticks: 10)])"#,
    )
    .expect_err("a pulse with no radius pushes nothing");
    assert!(failure.contains("radius"), "{failure}");

    let failure = compile_profiles(
        "unknown_field",
        r#"(profiles: [(name: "d", radius: 1.0, force: 1.0, cooldown_ticks: 1, colour: "red")])"#,
    )
    .expect_err("an unconsumed field must not be dropped in silence");
    assert!(failure.contains("unknown-field"), "{failure}");
}

/// HALF 2 — a semantic action, beside the engine's own vocabulary.
///
/// `Platformer2dInputActionMonolith` is a closed enum and was not touched.
#[test]
fn the_capability_registers_its_own_semantic_action() {
    let mut registry = ambition_input::ActionRegistry::with_engine_actions();
    let engine_actions = registry.len();
    register_actions(&mut registry).expect("a fresh id");

    assert_eq!(registry.len(), engine_actions + 1);
    let def = registry
        .get(ambition_input::SemanticActionId("pulse"))
        .expect("registered");
    assert_eq!(def.capability, PULSE_CAPABILITY);
    assert!(
        registry
            .for_context(ambition_input::GAMEPLAY_CONTEXT)
            .any(|d| d.id.0 == "pulse"),
        "and it is offered where it is meaningful, beside `jump` and `attack`"
    );
}

/// HALF 3 — rollback state the capability OFFERS and a composition installs.
///
/// the capability does not register it itself, and that is deliberate: the
/// registration trait lives in `ambition_platformer2d_runtime`, so self-registering would
/// drag the whole simulation into a mechanic that uses none of it. Offering is
/// also what the other two halves already do — a schema and an action are
/// offered, and whoever composes installs them.
///
/// This test IS the composition. `ambition_platformer2d_runtime` is a dev-dependency, so it
/// is in scope here and absent from the crate's real closure.
#[test]
fn a_composition_installs_the_rollback_state_the_capability_offers() {
    use ambition_platformer2d_rollback_ggrs::AmbitionRollbackApp;

    let mut app = App::new();
    app.add_plugins(PulsePlugin::default());
    // The one line a host writes. If a capability's offer needed more than
    // this, "offer rather than register" would be a worse trade than the
    // dependency it avoids.
    app.rollback_component_clone_probed::<PulseCooldown>(
        PULSE_CAPABILITY,
        ROLLBACK_STATE,
        |cooldown| u64::from(cooldown.remaining_ticks),
    );

    let dump = app
        .world()
        .get_resource::<ambition_platformer2d_runtime::rollback::RollbackRegistry>()
        .expect("registered")
        .schema_dump();
    assert!(
        dump.contains(ROLLBACK_STATE),
        "the cooldown has to be in the schema or a rewind would restore the body and not the \
         gate, and a pulse would fire twice from one charge on the resimulated frame:\n{dump}"
    );
    assert!(
        dump.contains(PULSE_CAPABILITY),
        "and under the CAPABILITY's own owner label, not the runtime's:\n{dump}"
    );
}

/// and the plugin alone must NOT register it, or the offer is a lie and
/// the dependency it was meant to avoid comes back the first time somebody
/// assumes the plugin is enough.
#[test]
fn the_plugin_alone_registers_no_rollback_state() {
    let mut app = App::new();
    app.add_plugins(PulsePlugin::default());
    let registered = app
        .world()
        .get_resource::<ambition_platformer2d_runtime::rollback::RollbackRegistry>()
        .map(|r| r.schema_dump().contains(ROLLBACK_STATE))
        .unwrap_or(false);
    assert!(
        !registered,
        "installing the mechanic must not silently register rollback state — a composition \
         that never asked for it would carry schema it did not choose"
    );
}

/// HALF 4 — causal facts, quoting the authored content that supplied them.
#[test]
fn the_capability_publishes_its_own_causal_facts() {
    let mut app = App::new();
    app.add_plugins(PulsePlugin::default());
    app.init_resource::<CausalRecording>();
    app.world_mut()
        .resource_mut::<CausalRecording>()
        .set_policy(RecordingPolicy::All)
        .set_tick(7);
    app.world_mut()
        .insert_resource(PulseProfiles::from_prepared(
            compile_profiles("facts", PROFILES).expect("compiles"),
        ));

    let firer = app
        .world_mut()
        .spawn((
            PulseBody {
                pos: Vec2::new(0.0, 0.0),
                ..Default::default()
            },
            PulseCooldown::default(),
        ))
        .id();
    // One inside the 64px radius, one outside it.
    let near = app
        .world_mut()
        .spawn((
            PulseBody {
                pos: Vec2::new(20.0, 0.0),
                ..Default::default()
            },
            PulseAffected,
        ))
        .id();
    app.world_mut().spawn((
        PulseBody {
            pos: Vec2::new(400.0, 0.0),
            ..Default::default()
        },
        PulseAffected,
    ));

    app.world_mut()
        .write_message(PulseRequested { body: firer });
    app.update();

    // The mechanic worked…
    let pushed = app.world().entity(near).get::<PulseBody>().unwrap().vel;
    assert!(pushed.x > 0.0, "the near body was pushed away: {pushed:?}");

    // …and it EXPLAINED itself, naming the authored profile.
    let log = app.world().resource::<CausalRecording>();
    let why = log.explain(7, &SubjectKey::Unstable(firer.to_bits()));
    let fired = why
        .first("pulse_fired")
        .expect("the pulse published a fact");
    assert_eq!(
        fired.get("pushed"),
        Some(&FactValue::Int(1)),
        "one body was in range and one was not — the count is the finding"
    );
    assert_eq!(
        fired.content.as_deref(),
        Some("pulse:pulse_profile/gentle"),
        "the fact quotes the AUTHORED profile, which is what 'which content supplied the \
         active value' means"
    );
}

/// "I pressed it and nothing happened" is a fact, not a silence.
#[test]
fn a_refused_pulse_says_why() {
    let mut app = App::new();
    app.add_plugins(PulsePlugin::default());
    app.init_resource::<CausalRecording>();
    app.world_mut()
        .resource_mut::<CausalRecording>()
        .set_policy(RecordingPolicy::All)
        .set_tick(3);
    let firer = app
        .world_mut()
        .spawn((
            PulseBody::default(),
            PulseCooldown {
                remaining_ticks: 20,
            },
        ))
        .id();
    app.world_mut()
        .write_message(PulseRequested { body: firer });
    app.update();

    let log = app.world().resource::<CausalRecording>();
    let why = log.explain(3, &SubjectKey::Unstable(firer.to_bits()));
    assert!(
        why.first("pulse_refused").is_some(),
        "a body on cooldown has to explain itself — that IS the question somebody brings"
    );
    assert!(why.first("pulse_fired").is_none());
}

/// The cooldown is real: a pulse arms it, and it ages.
#[test]
fn firing_arms_the_cooldown_and_it_ages() {
    let mut app = App::new();
    app.add_plugins(PulsePlugin::default());
    app.world_mut()
        .insert_resource(PulseProfiles::from_prepared(vec![PulseProfile {
            name: "test".into(),
            radius: 10.0,
            force: 1.0,
            cooldown_ticks: 3,
        }]));
    let firer = app
        .world_mut()
        .spawn((PulseBody::default(), PulseCooldown::default()))
        .id();

    app.world_mut()
        .write_message(PulseRequested { body: firer });
    app.update();
    assert_eq!(
        app.world()
            .entity(firer)
            .get::<PulseCooldown>()
            .unwrap()
            .remaining_ticks,
        3
    );
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .entity(firer)
            .get::<PulseCooldown>()
            .unwrap()
            .remaining_ticks,
        1
    );
}

/// A composition that skips the offer is CAUGHT, not left to desync.
///
/// The offer keeps this capability's closure to foundations, and the price was
/// that nothing forced a host to accept it. `REQUIRED_ROLLBACK` is the
/// obligation, declared next to the thing that has it — the same shape the
/// content compiler uses for "a `Runtime` schema must lower an artifact".
#[test]
fn a_composition_that_forgets_the_rollback_state_is_told_which_and_why() {
    use ambition_platformer2d_rollback_ggrs::AmbitionRollbackApp;
    use ambition_platformer2d_runtime::rollback::RollbackRegistry;

    // A host that installed the mechanic and nothing else.
    let mut forgetful = App::new();
    forgetful.add_plugins(PulsePlugin::default());
    forgetful.init_resource::<RollbackRegistry>();
    let missing = forgetful
        .world()
        .resource::<RollbackRegistry>()
        .missing_required_state(REQUIRED_ROLLBACK);
    // BOTH halves of the contract are outstanding: the cooldown AND the body.
    let names: Vec<&str> = missing.iter().map(|m| m.name).collect();
    assert_eq!(
        names,
        vec![ROLLBACK_STATE, BODY_ROLLBACK_STATE],
        "the contract must name every piece of authoritative state the mechanic \
         introduces — a list with only the cheap half passes while the desync \
         remains"
    );
    assert!(
        missing[0].why.contains("twice from one charge"),
        "and it says what BREAKS, so a host knows this is a desync rather than an optional \
         extra: {:?}",
        missing[0].why
    );
    assert!(
        missing[1].why.contains("still carrying the old push"),
        "the body's reason names its own failure, not the cooldown's: {:?}",
        missing[1].why
    );

    // A host that accepted the WHOLE offer.
    let mut complete = App::new();
    complete.add_plugins(PulsePlugin::default());
    complete.rollback_component_clone_probed::<PulseCooldown>(
        PULSE_CAPABILITY,
        ROLLBACK_STATE,
        |cooldown| u64::from(cooldown.remaining_ticks),
    );
    complete.rollback_component_clone_probed::<PulseBody>(
        PULSE_CAPABILITY,
        BODY_ROLLBACK_STATE,
        |body| body.vel.x.to_bits() as u64,
    );
    assert!(complete
        .world()
        .resource::<RollbackRegistry>()
        .missing_required_state(REQUIRED_ROLLBACK)
        .is_empty());
}

/// the OWNER is part of the requirement. Two capabilities may both
/// reasonably want a `cooldown`; a name registered by somebody else is not this
/// capability's state, and treating it as satisfied would be the worst kind of
/// pass — one that reports safety while the desync is still there.
#[test]
fn another_capabilitys_registration_does_not_satisfy_this_one() {
    use ambition_platformer2d_rollback_ggrs::AmbitionRollbackApp;
    use ambition_platformer2d_runtime::rollback::RollbackRegistry;

    let mut app = App::new();
    app.add_plugins(PulsePlugin::default());
    // Register BOTH of this capability's names — under the wrong owner.
    app.rollback_component_clone_probed::<PulseCooldown>(
        "some_other_capability",
        ROLLBACK_STATE,
        |cooldown| u64::from(cooldown.remaining_ticks),
    );
    app.rollback_component_clone_probed::<PulseBody>(
        "some_other_capability",
        BODY_ROLLBACK_STATE,
        |body| body.vel.x.to_bits() as u64,
    );
    let missing = app
        .world()
        .resource::<RollbackRegistry>()
        .missing_required_state(REQUIRED_ROLLBACK);
    assert_eq!(
        missing.len(),
        REQUIRED_ROLLBACK.len(),
        "the same NAMES under another owner are different registrations, so \
         every requirement is still outstanding"
    );
}

/// The capability runs in the HOST'S schedule, not in `Update`.
///
/// * render-rate coupled under a fixed-tick host — cooldowns aged once per
///   frame instead of once per tick;
/// * not resimulated under a rollback host — GGRS replays the SIM schedule,
///   so a rewind restored `PulseCooldown` without re-running the behaviour that
///   produced the surrounding result. Snapshotting state does not help when the
///   systems that move it never replay.
///
/// The seam is `SimScheduleExt::sim_schedule()`: the HOST names the authoritative schedule and
/// the capability asks.
///
/// So this test gives the host a schedule of its own.
#[test]
fn the_capability_runs_in_the_hosts_schedule_rather_than_in_update() {
    use bevy::ecs::schedule::ScheduleLabel;

    #[derive(ScheduleLabel, Clone, Copy, Debug, Hash, PartialEq, Eq)]
    struct HostSim;

    let mut app = App::new();
    // The host chooses.
    app.set_sim_schedule(HostSim);
    app.add_plugins(PulsePlugin::default());

    let firer = app
        .world_mut()
        .spawn((PulseBody::default(), PulseCooldown::default()))
        .id();
    let near = app
        .world_mut()
        .spawn((
            PulseBody {
                pos: Vec2::new(20.0, 0.0),
                ..Default::default()
            },
            PulseAffected,
        ))
        .id();
    app.world_mut()
        .write_message(PulseRequested { body: firer });

    // A render update is NOT a simulation tick.
    app.update();
    assert_eq!(
        app.world().entity(near).get::<PulseBody>().unwrap().vel,
        Vec2::ZERO,
        "the pulse fired on a render update — the capability is in `Update`, so \
         under a fixed-tick host its cooldowns follow the frame rate and under a \
         rollback host its behaviour is never resimulated"
    );

    // The host's own schedule is where the simulation happens.
    app.world_mut().run_schedule(HostSim);
    assert!(
        app.world().entity(near).get::<PulseBody>().unwrap().vel.x > 0.0,
        "the pulse did not fire in the schedule the host declared authoritative"
    );
}

/// The same input over the same number of SIM TICKS gives the same result,
/// whichever schedule the host picked.
///
/// The equivalence the review asked for, expressed without standing up two real
/// hosts: one app leaves the sim schedule at its `Update` default, the other
/// names its own. Step each the same number of simulation ticks and the cooldown
/// and the pushed body must agree.
///
/// what this pins is that pulse's result is a function of TICKS, not of
/// frames. That is the property bare `Update` broke, and it is the one a
/// rollback host needs in order to replay anything.
#[test]
fn the_result_is_a_function_of_sim_ticks_not_of_which_schedule_runs_them() {
    use bevy::ecs::schedule::ScheduleLabel;

    #[derive(ScheduleLabel, Clone, Copy, Debug, Hash, PartialEq, Eq)]
    struct HostSim;

    fn run(host_schedule: bool, ticks: usize) -> (u32, f32) {
        let mut app = App::new();
        if host_schedule {
            app.set_sim_schedule(HostSim);
        }
        app.add_plugins(PulsePlugin::default());
        let firer = app
            .world_mut()
            .spawn((PulseBody::default(), PulseCooldown::default()))
            .id();
        let near = app
            .world_mut()
            .spawn((
                PulseBody {
                    pos: Vec2::new(20.0, 0.0),
                    ..Default::default()
                },
                PulseAffected,
            ))
            .id();
        app.world_mut()
            .write_message(PulseRequested { body: firer });
        for _ in 0..ticks {
            if host_schedule {
                // `update()` still runs so messages advance exactly as they do
                // under a real host; the SIM work happens in the host schedule.
                app.update();
                app.world_mut().run_schedule(HostSim);
            } else {
                app.update();
            }
        }
        (
            app.world()
                .entity(firer)
                .get::<PulseCooldown>()
                .unwrap()
                .remaining_ticks,
            app.world().entity(near).get::<PulseBody>().unwrap().vel.x,
        )
    }

    assert_eq!(
        run(false, 5),
        run(true, 5),
        "five simulation ticks produced different pulse state depending on WHICH \
         schedule ran them — the capability's result is not a function of ticks"
    );
}

/// AUTHORED NUMBERS REACH A FIRED PULSE.
///
/// the review's second finding, and the authority split the content-compiler
/// program exists to remove: the schema was registered, packs validated and
/// LOWERED correctly, and `PulsePlugin` then called
/// `init_resource::<PulseProfiles>()` — the built-in defaults. A game could
/// author a radius, watch the compiler accept it, mount the capability, and
/// pulse at the default radius forever. The compiler was validating content
/// the runtime ignored, which is worse than not validating it.
///
/// So this test does not inspect the lowered artifact — that passed the whole
/// time. It authors profiles that differ from the defaults in every field,
/// compiles them, mounts the capability, FIRES, and measures the body.
///
/// PROBED: with `PulsePlugin::default()` in place of `from_prepared`, the pushed
/// body reports the default force and the assertion names both numbers.
#[test]
fn authored_profile_values_reach_a_fired_pulse_not_just_the_lowered_artifact() {
    use bevy::ecs::schedule::ScheduleLabel;

    #[derive(ScheduleLabel, Clone, Copy, Debug, Hash, PartialEq, Eq)]
    struct HostSim;

    // Every field differs from `PulseProfile::default()` (96 / 420 / 45), and
    // the FIRST profile is the active one.
    const AUTHORED: &str = r#"(
    profiles: [
        (name: "authored", radius: 500.0, force: 1234.0, cooldown_ticks: 7),
    ],
)"#;

    let pack = compile_pack("acceptance", AUTHORED).expect("the pack compiles");
    let plugin = PulsePlugin::from_prepared(&pack).expect("the pack prepared profiles");

    let mut app = App::new();
    app.set_sim_schedule(HostSim);
    app.add_plugins(plugin);

    let firer = app
        .world_mut()
        .spawn((PulseBody::default(), PulseCooldown::default()))
        .id();
    // 300px away: inside the AUTHORED 500 radius and far outside the default 96,
    // so "did the authored radius apply" is answerable from whether it moved.
    let near = app
        .world_mut()
        .spawn((
            PulseBody {
                pos: Vec2::new(300.0, 0.0),
                ..Default::default()
            },
            PulseAffected,
        ))
        .id();

    app.world_mut()
        .write_message(PulseRequested { body: firer });
    app.update();
    app.world_mut().run_schedule(HostSim);

    let pushed = app.world().entity(near).get::<PulseBody>().unwrap().vel;
    assert!(
        pushed.x > 0.0,
        "a body 300px away was not pushed, so the AUTHORED radius of 500 never \
         reached the runtime — the capability is still running the built-in 96"
    );
    // force 1234 with linear falloff at 300/500 → 1234 * (1 - 0.6) = 493.6
    assert!(
        (pushed.x - 493.6).abs() < 0.5,
        "pushed at {pushed:?}; the authored force of 1234 with linear falloff at \
         300/500 is 493.6. A different number means a different profile applied"
    );
    let cooldown = app
        .world()
        .entity(firer)
        .get::<PulseCooldown>()
        .unwrap()
        .remaining_ticks;
    assert_eq!(
        cooldown, 7,
        "the authored cooldown of 7 did not reach the runtime (the default is 45)"
    );
}

/// A pack that prepared no profiles is a REFUSAL, not a silent default.
///
/// The other half of the seam: a composition that asked for authored tuning and
/// got none must hear about it. Silently running the defaults is exactly the
/// behaviour the finding above describes.
#[test]
fn a_pack_without_pulse_profiles_refuses_rather_than_defaulting() {
    // A pack whose schema registry has no pulse schema lowers no pulse artifact.
    let root = std::env::temp_dir().join("capability_demo_test/empty_pack");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp dir");
    let draft = ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("pulse_pack".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("demo".into()),
            requires: Vec::new(),
            sources: Vec::new(),
        },
    )
    .expect("an empty pack is a valid draft");
    let pack = compile(&draft, &SchemaRegistry::new(), &AssetsUnchecked).expect("compiles");

    let refusal = PulsePlugin::from_prepared(&pack).expect_err("no profiles is a refusal");
    let rendered = refusal.to_string();
    assert!(
        rendered.contains("demo") && rendered.contains("PulsePlugin::default()"),
        "the refusal must name the pack and the deliberate alternative, got: {rendered}"
    );
}

/// The active profile is FROZEN at composition, so a rewind cannot disagree
/// about which one was live.
///
/// That made the active selection MUTABLE SIMULATION STATE which nothing rewound and which the
/// rollback contract never mentioned — so the radius, force and cooldown a pulse used could differ
/// between an original tick and its resimulation, while every declared requirement was satisfied .
///
/// Freezing is the cheaper of the two honest answers — the other being to make
/// the selection rollback-owned — and nothing called `select`, so nothing was
/// lost. A game that wants to switch profiles mid-match must now decide to make
/// that rollback-owned rather than inherit it by accident.
///
/// what this test can assert is the TYPE-LEVEL fact: choosing happens on an
/// owned value before the resource exists. `with_active` consumes `self`, so
/// there is no `&mut` path to the live resource at all — which is why the
/// property needs no runtime guard.
#[test]
fn the_active_profile_is_chosen_before_the_resource_exists_not_during_the_sim() {
    let profiles =
        PulseProfiles::from_prepared(compile_profiles("frozen", PROFILES).expect("compiles"));
    assert_eq!(profiles.active().name, "gentle", "the first is active");

    // Choosing is a COMPOSITION-time move on an owned value…
    let cannon = profiles
        .clone()
        .with_active("cannon")
        .expect("the authored name resolves");
    assert_eq!(cannon.active().name, "cannon");
    assert_eq!(cannon.active().radius, 200.0);

    // …and an unknown name is answerable rather than silently ignored.
    assert!(
        PulseProfiles::from_prepared(compile_profiles("frozen2", PROFILES).expect("compiles"))
            .with_active("no_such_profile")
            .is_none(),
        "an unknown profile name must be a `None` the composition can act on, \
         not a silent fallback to whatever was already active"
    );
}

/// The rollback contract names every piece of authoritative state the
/// mechanic introduces, which is the property the contract exists to have.
///
/// A checkable restatement of: enumerate what `fire_pulses` reads or writes that can differ
/// between an original tick and its resimulation, and require each to appear in
/// `REQUIRED_ROLLBACK` or to have no mutable path.
#[test]
fn every_piece_of_authoritative_pulse_state_is_in_the_contract() {
    let names: Vec<&str> = REQUIRED_ROLLBACK.iter().map(|r| r.name).collect();
    assert!(
        names.contains(&ROLLBACK_STATE),
        "the cooldown gates the action: {names:?}"
    );
    assert!(
        names.contains(&BODY_ROLLBACK_STATE),
        "the pushed velocity IS the mechanic's observable effect: {names:?}"
    );
    // The third piece — the active profile — is covered by construction rather
    // than by registration: there is no `&mut` path to it after composition.
    // `the_active_profile_is_chosen_before_the_resource_exists_not_during_the_sim`
    // is that argument, and this assertion is here so the three are counted
    // together in one place.
    assert_eq!(
        names.len(),
        2,
        "a new piece of authoritative state was added without deciding whether \
         it is rewound or frozen: {names:?}"
    );
    for req in REQUIRED_ROLLBACK {
        assert_eq!(
            req.owner, PULSE_CAPABILITY,
            "every requirement is this capability's own"
        );
        assert!(
            req.why.len() > 40,
            "each names what BREAKS, not just what is missing: {:?}",
            req.why
        );
    }
}

/// Reordering the profiles changes the pack's identity, because it changes
/// which pulse the game fires.
///
/// A fingerprint that cannot tell those two packs apart is useless for the four things it exists
/// for: cache invalidation, packaging, session compatibility, and telling two peers apart.
///
/// The cause is that this file is POSITIONAL — `from_prepared` pins `active: 0`
/// — while `define` was keyed only by name, and the pack sorts definitions by
/// content id.
#[test]
fn swapping_two_profiles_moves_the_fingerprint() {
    const SWAPPED: &str = r#"(
    profiles: [
        (name: "cannon", radius: 200.0, force: 900.0, cooldown_ticks: 90),
        (name: "gentle", radius: 64.0, force: 100.0, cooldown_ticks: 30),
    ],
)"#;
    let original = compile_pack("order_original", PROFILES).expect("compiles");
    let swapped = compile_pack("order_swapped", SWAPPED).expect("compiles");

    // The runtime really does read a different profile — without this the test
    // could pass over two packs that behave identically, which would make the
    // fingerprint claim meaningless.
    let live = |pack: &ambition_content_pack::PreparedContentPack| {
        let profiles: &Vec<PulseProfile> = pack
            .lowered(&SchemaId::new(PULSE_SCHEMA))
            .expect("the schema lowered its profiles");
        PulseProfiles::from_prepared(profiles.clone()).active()
    };
    assert_ne!(
        live(&original).name,
        live(&swapped).name,
        "the two packs must actually run different pulses, or this proves nothing"
    );

    assert_ne!(
        original.fingerprint, swapped.fingerprint,
        "two packs that fire different pulses report the same identity:\n--- original ---\n{}\n--- swapped ---\n{}",
        original.canonical_bytes(),
        swapped.canonical_bytes()
    );
}
