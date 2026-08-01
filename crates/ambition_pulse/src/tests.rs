//! **The sentinel's own proof.**
//!
//! Each test below is one of the four halves of the capability contract, and
//! the point of every one is what it did NOT have to touch.

use super::*;
use ambition_causal::{CausalRecording, FactValue, RecordingPolicy, SubjectKey};
use ambition_content_pack::{
    AssetsUnchecked, ContentPackDraft, ContentPackManifest, ModuleNamespace, PackId, PackVersion,
    SchemaId, SchemaRegistry, SourceDeclaration, compile,
};
use ambition_engine_core::Vec2;

const PROFILES: &str = r#"(
    profiles: [
        (name: "gentle", radius: 64.0, force: 100.0, cooldown_ticks: 30),
        (name: "cannon", radius: 200.0, force: 900.0, cooldown_ticks: 90),
    ],
)"#;

fn compile_profiles(name: &str, text: &str) -> Result<Vec<PulseProfile>, String> {
    let root = std::env::temp_dir().join(format!("ambition_pulse_test/{name}"));
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

/// **HALF 1 — an authored schema, registered by the capability that owns it.**
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

/// **HALF 2 — a semantic action, beside the engine's own vocabulary.**
///
/// `SandboxAction` is a closed enum and was not touched.
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

/// **HALF 3 — rollback state the capability OFFERS and a composition installs.**
///
/// ⛔ the capability does not register it itself, and that is deliberate: the
/// registration trait lives in `ambition_runtime`, so self-registering would
/// drag the whole simulation into a mechanic that uses none of it. Offering is
/// also what the other two halves already do — a schema and an action are
/// offered, and whoever composes installs them.
///
/// This test IS the composition. `ambition_runtime` is a dev-dependency, so it
/// is in scope here and absent from the crate's real closure.
#[test]
fn a_composition_installs_the_rollback_state_the_capability_offers() {
    use ambition_runtime::rollback::AmbitionRollbackApp;

    let mut app = App::new();
    app.add_plugins(PulsePlugin);
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
        .get_resource::<ambition_runtime::rollback::RollbackRegistry>()
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

/// ⚠ **and the plugin alone must NOT register it**, or the offer is a lie and
/// the dependency it was meant to avoid comes back the first time somebody
/// assumes the plugin is enough.
#[test]
fn the_plugin_alone_registers_no_rollback_state() {
    let mut app = App::new();
    app.add_plugins(PulsePlugin);
    let registered = app
        .world()
        .get_resource::<ambition_runtime::rollback::RollbackRegistry>()
        .map(|r| r.schema_dump().contains(ROLLBACK_STATE))
        .unwrap_or(false);
    assert!(
        !registered,
        "installing the mechanic must not silently register rollback state — a composition \
         that never asked for it would carry schema it did not choose"
    );
}

/// **HALF 4 — causal facts, quoting the authored content that supplied them.**
#[test]
fn the_capability_publishes_its_own_causal_facts() {
    let mut app = App::new();
    app.add_plugins(PulsePlugin);
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

    app.world_mut().write_message(PulseRequested { body: firer });
    app.update();

    // The mechanic worked…
    let pushed = app.world().entity(near).get::<PulseBody>().unwrap().vel;
    assert!(pushed.x > 0.0, "the near body was pushed away: {pushed:?}");

    // …and it EXPLAINED itself, naming the authored profile.
    let log = app.world().resource::<CausalRecording>();
    let why = log.explain(7, &SubjectKey::Unstable(firer.to_bits()));
    let fired = why.first("pulse_fired").expect("the pulse published a fact");
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

/// ⚠ **"I pressed it and nothing happened" is a fact, not a silence.**
#[test]
fn a_refused_pulse_says_why() {
    let mut app = App::new();
    app.add_plugins(PulsePlugin);
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
    app.world_mut().write_message(PulseRequested { body: firer });
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
    app.add_plugins(PulsePlugin);
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

    app.world_mut().write_message(PulseRequested { body: firer });
    app.update();
    assert_eq!(
        app.world().entity(firer).get::<PulseCooldown>().unwrap().remaining_ticks,
        3
    );
    app.update();
    app.update();
    assert_eq!(
        app.world().entity(firer).get::<PulseCooldown>().unwrap().remaining_ticks,
        1
    );
}

/// ⛔ **A composition that skips the offer is CAUGHT, not left to desync.**
///
/// The offer keeps this capability's closure to foundations, and the price was
/// that nothing forced a host to accept it. `REQUIRED_ROLLBACK` is the
/// obligation, declared next to the thing that has it — the same shape the
/// content compiler uses for "a `Runtime` schema must lower an artifact".
#[test]
fn a_composition_that_forgets_the_rollback_state_is_told_which_and_why() {
    use ambition_runtime::rollback::{AmbitionRollbackApp, RollbackRegistry};

    // A host that installed the mechanic and nothing else.
    let mut forgetful = App::new();
    forgetful.add_plugins(PulsePlugin);
    forgetful.init_resource::<RollbackRegistry>();
    let missing = forgetful
        .world()
        .resource::<RollbackRegistry>()
        .missing_required_state(REQUIRED_ROLLBACK);
    assert_eq!(missing.len(), 1, "the cooldown is unregistered");
    assert_eq!(missing[0].name, ROLLBACK_STATE);
    assert!(
        missing[0].why.contains("twice from one charge"),
        "and it says what BREAKS, so a host knows this is a desync rather than an optional \
         extra: {:?}",
        missing[0].why
    );

    // A host that accepted the offer.
    let mut complete = App::new();
    complete.add_plugins(PulsePlugin);
    complete.rollback_component_clone_probed::<PulseCooldown>(
        PULSE_CAPABILITY,
        ROLLBACK_STATE,
        |cooldown| u64::from(cooldown.remaining_ticks),
    );
    assert!(
        complete
            .world()
            .resource::<RollbackRegistry>()
            .missing_required_state(REQUIRED_ROLLBACK)
            .is_empty()
    );
}

/// ⚠ **the OWNER is part of the requirement.** Two capabilities may both
/// reasonably want a `cooldown`; a name registered by somebody else is not this
/// capability's state, and treating it as satisfied would be the worst kind of
/// pass — one that reports safety while the desync is still there.
#[test]
fn another_capabilitys_registration_does_not_satisfy_this_one() {
    use ambition_runtime::rollback::{AmbitionRollbackApp, RollbackRegistry};

    let mut app = App::new();
    app.add_plugins(PulsePlugin);
    app.rollback_component_clone_probed::<PulseCooldown>(
        "some_other_capability",
        ROLLBACK_STATE,
        |cooldown| u64::from(cooldown.remaining_ticks),
    );
    let missing = app
        .world()
        .resource::<RollbackRegistry>()
        .missing_required_state(REQUIRED_ROLLBACK);
    assert_eq!(
        missing.len(),
        1,
        "the same NAME under another owner is a different registration"
    );
}
