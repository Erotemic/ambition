//! **The capability, mounted by a real composition through the public SDK.**
//!
//! The unit tests prove each contribution in isolation: a schema the compiler
//! accepts, an action the registry holds, rollback state a host can install,
//! facts the log records. What none of them shows is the thing an author
//! actually does — write ONE module that mounts the capability and declares
//! what it brings, and have the composition install all of it.
//!
//! ⚠ **`ambition` is a DEV-dependency.** A mechanic must not depend on the
//! facade that mounts it; the capability's real closure is still seven
//! foundation crates. This file is the consumer, not the capability.

use ambition::app::{GameModule, ModuleDraft, ModuleManifest, PlatformerApp};
use ambition::input::{InstalledActions, SemanticActionId, GAMEPLAY_CONTEXT};
use bevy::prelude::*;

/// A game that wants the pulse. This is the whole integration an author writes.
struct GameWithPulse;

impl GameModule for GameWithPulse {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest::new("game_with_pulse")
    }

    fn define(&self, module: &mut ModuleDraft) {
        module
            .experience("game_with_pulse")
            .launcher_route("home")
            .gameplay_route("game_with_pulse/play");
        module
            // behaviour
            .capability(ambition_pulse::PulsePlugin)
            // + the semantic action it contributes
            .actions(&[ambition_pulse::PULSE_ACTION])
            // + what it needs rewound
            .requires_rollback(ambition_pulse::REQUIRED_ROLLBACK);
    }
}

/// **All three contributions arrive from one declaration.**
#[test]
fn one_module_mounts_the_capability_and_the_composition_installs_everything() {
    let mut app = App::new();
    // ⚠ the composition refuses LATER (nothing registers the gameplay route,
    // which is a separate and correct refusal). What this test is about is
    // whether the contributions arrived before that, so the result is not
    // asserted — the resources are.
    let outcome = PlatformerApp::headless()
        .rollback(2)
        .mount(GameWithPulse)
        .install_into(&mut app);

    // The ACTION reached the composition's vocabulary, beside the engine's.
    let actions = app
        .world()
        .get_resource::<InstalledActions>()
        .expect("the composition builds an action registry");
    assert_eq!(
        actions.get(SemanticActionId("pulse")).map(|d| d.capability),
        Some(ambition_pulse::PULSE_CAPABILITY),
        "the capability's action is in the game's vocabulary"
    );
    assert!(
        actions.for_context(GAMEPLAY_CONTEXT).any(|d| d.id.0 == "jump"),
        "and the engine's own is there unasked, so one query answers both"
    );

    // The BEHAVIOUR is installed: the plugin's own resources exist.
    assert!(
        app.world()
            .get_resource::<ambition_pulse::PulseProfiles>()
            .is_some(),
        "the mechanic's tuning resource is present, so `capability(..)` ran"
    );

    // ⚠ and the ROLLBACK requirement was CHECKED. The composition declared
    // `rollback(2)` and nothing registered `pulse.cooldown`, so it must refuse
    // with that reason — a capability whose rewind state is skipped desyncs, and
    // this is the composition catching it rather than a player discovering it.
    let message = format!("{outcome:?}");
    assert!(
        message.contains(ambition_pulse::ROLLBACK_STATE),
        "the missing rewind state is named by the refusal: {message}"
    );
    assert!(
        message.contains("twice from one charge"),
        "and carries the capability's own reason, not a generic complaint: {message}"
    );
}

/// **The content half, through the facade a game actually uses.**
///
/// The capability offers a schema; the game adds it to the engine's registry and
/// validates its own authored file. No `ambition_content_pack` import here —
/// only `ambition::content`.
#[test]
fn a_game_validates_the_capabilitys_authored_content_through_the_facade() {
    use ambition::content::{
        AssetsUnchecked, ContentPackDraft, ContentPackManifest, ModuleNamespace, PackId,
        PackVersion, SchemaId, SchemaVersion, SourceDeclaration, compile, engine_schemas,
    };

    let root = std::env::temp_dir().join("ambition_pulse_sdk/pack");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp pack");
    std::fs::write(
        root.join("pulse.ron"),
        r#"(profiles: [(name: "heavy", radius: 180.0, force: 800.0, cooldown_ticks: 60)])"#,
    )
    .expect("write");

    let mut registry = engine_schemas();
    registry
        .register(ambition_pulse::pulse_schema())
        .expect("the capability's schema is new to the engine's registry");

    let draft = ContentPackDraft::read_manifest(
        root,
        ContentPackManifest {
            id: PackId("game_with_pulse".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("game_with_pulse".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: "pulse.ron".into(),
                schema: SchemaId::new(ambition_pulse::PULSE_SCHEMA),
                version: SchemaVersion(1),
            }],
        },
    )
    .expect("draft reads");

    let pack = compile(&draft, &registry, &AssetsUnchecked).expect("the game's tuning compiles");
    let profiles = pack
        .lowered::<Vec<ambition_pulse::PulseProfile>>(&SchemaId::new(ambition_pulse::PULSE_SCHEMA))
        .expect("a Runtime schema lowers what the game will run");
    assert_eq!(profiles[0].name, "heavy");
    assert_eq!(profiles[0].radius, 180.0);
}
